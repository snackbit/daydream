#[path = "shutdown.rs"]
mod shutdown;

use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex,
    },
    thread,
    time::Duration,
};

use gstreamer as gst;
use gstreamer::prelude::*;
use gstreamer_app as gst_app;
use tracing::{error, info, trace, warn};

use super::super::{
    benchmark::{RecorderBenchmarkSegmentInput, RecorderBenchmarkSession},
    capture_runtime::{FrameReceiver, SharedCaptureTiming},
    errors::dependency_error,
    types::RecorderError,
};
#[cfg(test)]
use super::frame_pump::{frame_interval, pump_synthetic_frames};
use super::{
    audio::{install_audio_cutoff_probes, log_audio_cutoff_results, send_audio_source_eos},
    duration_to_clock_time,
    frame_pump::{pump_composed_frames, FramePumpStop},
    pipeline::{audio_branches, build_pipeline, AudioBranch},
    types::{FragmentEvent, MediaSegmentInput},
};

const EOS_TIMEOUT: Duration = Duration::from_secs(10);
#[cfg(not(test))]
const FRAME_PUMP_STOP_TIMEOUT: Duration = Duration::from_secs(5);
#[cfg(test)]
const FRAME_PUMP_STOP_TIMEOUT: Duration = Duration::from_millis(100);

#[derive(Debug)]
pub(crate) struct MediaWriterSegment {
    appsrc: gst_app::AppSrc,
    extra_video_sources: Vec<gst_app::AppSrc>,
    audio_branches: Vec<AudioBranch>,
    bus: gst::Bus,
    frame_pump: Option<thread::JoinHandle<()>>,
    frame_pump_result: mpsc::Receiver<Result<(), String>>,
    frame_pump_completion: Option<Result<(), String>>,
    pipeline: gst::Pipeline,
    benchmark: Option<RecorderBenchmarkSession>,
    started_at_ms: u64,
    stop_frames: Option<mpsc::Sender<FramePumpStop>>,
    fragments: mpsc::Receiver<FragmentEvent>,
    fragment_overflow: Arc<AtomicBool>,
    open_fragments: Arc<Mutex<std::collections::BTreeMap<std::path::PathBuf, bool>>>,
    fragment_seen: Arc<AtomicBool>,
    reference_eos: Arc<AtomicBool>,
}

#[cfg(test)]
pub(crate) fn start_synthetic_segment(
    input: &MediaSegmentInput,
) -> Result<MediaWriterSegment, RecorderError> {
    let canvas_width = input.canvas_width;
    let canvas_height = input.canvas_height;
    let frame_interval = frame_interval(input.fps);
    let benchmark = benchmark_session_for_input(input);

    let fps = input.fps;
    start_segment_with_frame_pump(input, true, benchmark, move |appsrc, stop, _sources| {
        thread::spawn(move || {
            pump_synthetic_frames(
                appsrc,
                canvas_width,
                canvas_height,
                fps,
                frame_interval,
                stop,
            )
        })
    })
}

pub(crate) fn start_segment_from_frames(
    input: &MediaSegmentInput,
    frames: FrameReceiver,
    timing: Option<SharedCaptureTiming>,
) -> Result<MediaWriterSegment, RecorderError> {
    let fps = input.fps;
    let started_at_ms = input.started_at_ms;
    let benchmark = benchmark_session_for_input(input);
    let frame_pump_benchmark = benchmark.clone();
    start_segment_with_frame_pump(input, false, benchmark, move |appsrc, stop, sources| {
        thread::spawn(move || {
            if !sources.is_empty() {
                return super::frame_pump::pump_source_frames(
                    sources,
                    frames,
                    stop,
                    timing,
                    fps,
                    started_at_ms,
                    frame_pump_benchmark,
                );
            }
            pump_composed_frames(
                appsrc,
                frames,
                stop,
                timing,
                fps,
                started_at_ms,
                frame_pump_benchmark,
            )
        })
    })
}

impl MediaWriterSegment {
    pub(crate) fn apply_cutoff(&self, timestamp_ms: u64) -> Result<(), RecorderError> {
        install_audio_cutoff_probes(
            &self.pipeline,
            &self.audio_branches,
            self.started_at_ms,
            timestamp_ms,
        )
        .map(|_| ())
    }
    pub(crate) fn flush_handle(&self) -> gst::Pipeline {
        self.pipeline.clone()
    }
    #[cfg(test)]
    pub(super) fn for_supervision_test(
        pipeline: gst::Pipeline,
        bus: gst::Bus,
        appsrc: gst_app::AppSrc,
        result: mpsc::Receiver<Result<(), String>>,
    ) -> Self {
        Self {
            appsrc,
            extra_video_sources: Vec::new(),
            audio_branches: vec![],
            bus,
            frame_pump: None,
            frame_pump_result: result,
            frame_pump_completion: None,
            pipeline,
            benchmark: None,
            started_at_ms: 0,
            stop_frames: None,
            fragments: mpsc::channel().1,
            fragment_overflow: Arc::new(AtomicBool::new(false)),
            open_fragments: Arc::new(Mutex::new(std::collections::BTreeMap::new())),
            fragment_seen: Arc::new(AtomicBool::new(false)),
            reference_eos: Arc::new(AtomicBool::new(false)),
        }
    }
    pub(crate) fn poll_error(&mut self) -> Option<RecorderError> {
        if self.fragment_overflow.load(Ordering::SeqCst) {
            return Some(dependency_error("recorder_fragment_queue_overflow","Recording fragment metadata could not keep up. Capture stopped to preserve the saved files."));
        }
        while let Some(message) = self.bus.pop() {
            match message.view() {
                gst::MessageView::Error(_) => return Some(dependency_error("recorder_media_writer_failed", "The media writer stopped after a recording error. Check audio devices and available storage.")),
                gst::MessageView::Eos(_) => return Some(dependency_error("recorder_media_writer_failed", "The media writer ended unexpectedly.")),
                _ => (),
            }
        }
        match self.frame_pump_result.try_recv() {
            Ok(result) => {
                self.frame_pump_completion = Some(result);
                Some(dependency_error(
                    "recorder_media_writer_frame_write_failed",
                    "The video frame writer stopped unexpectedly.",
                ))
            }
            Err(mpsc::TryRecvError::Disconnected) => Some(dependency_error(
                "recorder_media_writer_frame_write_failed",
                "The video frame writer stopped unexpectedly.",
            )),
            Err(mpsc::TryRecvError::Empty) => None,
        }
    }
    #[cfg(test)]
    pub(crate) fn benchmark(&self) -> Option<RecorderBenchmarkSession> {
        self.benchmark.clone()
    }

    pub(crate) fn drain_fragments(&mut self) -> Vec<FragmentEvent> {
        self.fragments.try_iter().collect()
    }
}

pub(super) fn start_segment_with_frame_pump(
    input: &MediaSegmentInput,
    _stop_frame_pump_on_stop: bool,
    benchmark: Option<RecorderBenchmarkSession>,
    spawn_frame_pump: impl FnOnce(
        gst_app::AppSrc,
        mpsc::Receiver<FramePumpStop>,
        Vec<(crate::storage::RecordingVideoSource, gst_app::AppSrc)>,
    ) -> thread::JoinHandle<Result<(), String>>,
) -> Result<MediaWriterSegment, RecorderError> {
    info!(
        output_path = %input.output_path.to_string_lossy(),
        canvas_width = input.canvas_width,
        canvas_height = input.canvas_height,
        fps = input.fps,
        desktop_audio_enabled = input.desktop_audio_source_id.is_some(),
        microphone_enabled = input.microphone_audio_source_id.is_some(),
        "media_writer_segment_start_requested"
    );
    gst::init().map_err(|error| {
        dependency_error(
            "recorder_media_writer_unavailable",
            format!("GStreamer initialization failed: {error}."),
        )
    })?;
    let pipeline = build_pipeline(input)?;
    let appsrc = pipeline
        .by_name("video_src")
        .ok_or_else(|| {
            dependency_error(
                "recorder_media_writer_unavailable",
                "GStreamer video appsrc is missing from the pipeline.",
            )
        })?
        .downcast::<gst_app::AppSrc>()
        .map_err(|_error| {
            dependency_error(
                "recorder_media_writer_unavailable",
                "GStreamer video_src is not an appsrc.",
            )
        })?;
    let bus = pipeline.bus().ok_or_else(|| {
        dependency_error(
            "recorder_media_writer_unavailable",
            "GStreamer pipeline bus is unavailable.",
        )
    })?;
    let reference_eos = Arc::new(AtomicBool::new(false));
    let eos_seen = reference_eos.clone();
    let mux = pipeline.by_name("mux").ok_or_else(|| {
        dependency_error(
            "recorder_media_writer_pipeline_failed",
            "Recording muxer is missing.",
        )
    })?;
    let reference_pad = mux.static_pad("video").ok_or_else(|| {
        dependency_error(
            "recorder_media_writer_pipeline_failed",
            "Recording reference video pad is missing.",
        )
    })?;
    reference_pad.add_probe(gst::PadProbeType::EVENT_DOWNSTREAM, move |_, info| {
        if info
            .event()
            .is_some_and(|event| matches!(event.view(), gst::EventView::Eos(_)))
        {
            eos_seen.store(true, Ordering::SeqCst);
        }
        gst::PadProbeReturn::Ok
    });
    let (stop_frames, receiver) = mpsc::channel();
    let open_fragments = Arc::new(Mutex::new(std::collections::BTreeMap::new()));
    if let Some(splitmux) = pipeline.by_name("mux") {
        let assigned = open_fragments.clone();
        let first_path = input.output_path.clone();
        let first_index = input.segment_index;
        splitmux.connect("format-location", false, move |values| {
            let index = values.get(1)?.get::<u32>().ok()?;
            let path = if index == first_index {
                first_path.clone()
            } else {
                first_path.parent()?.join(format!("{index:04}.mkv"))
            };
            // Register fragment identity before the streaming thread can publish
            // opened/closed events. A close for another fragment cannot finish it.
            assigned.lock().ok()?.entry(path.clone()).or_insert(false);
            Some(path.to_string_lossy().to_string().to_value())
        });
    }
    let (fragment_sender, fragments) = mpsc::sync_channel(16);
    let fragment_overflow = Arc::new(AtomicBool::new(false));
    let overflow = fragment_overflow.clone();
    let started = input.started_at_ms;
    let fragment_seen = Arc::new(AtomicBool::new(false));
    let open_count = open_fragments.clone();
    let seen = fragment_seen.clone();
    bus.set_sync_handler(move |_, message| {
        if let gst::MessageView::Element(element) = message.view() {
            if let Some(structure) = element.structure() {
                let name = structure.name();
                if matches!(
                    name.as_str(),
                    "splitmuxsink-fragment-opened" | "splitmuxsink-fragment-closed"
                ) {
                    if let (Ok(path), Ok(time)) = (
                        structure.get::<String>("location"),
                        structure.get::<gst::ClockTime>("running-time"),
                    ) {
                        let event = FragmentEvent {
                            opened: name == "splitmuxsink-fragment-opened",
                            path: path.into(),
                            timestamp_ms: started.saturating_add(time.mseconds()),
                        };
                        if event.opened {
                            seen.store(true, Ordering::SeqCst);
                        }
                        if let Ok(mut fragments) = open_count.lock() {
                            fragments.insert(event.path.clone(), !event.opened);
                        } else {
                            overflow.store(true, Ordering::SeqCst);
                        }
                        if fragment_sender.try_send(event).is_err() {
                            overflow.store(true, Ordering::SeqCst);
                        }
                    } else {
                        overflow.store(true, Ordering::SeqCst);
                    }
                }
            }
        }
        gst::BusSyncReply::Pass
    });
    let (frame_pump_result_sender, frame_pump_result) = mpsc::channel();
    let sources = input
        .video_sources
        .iter()
        .enumerate()
        .map(|(index, source)| {
            let element = if index == 0 {
                appsrc.clone()
            } else {
                pipeline
                    .by_name(&format!("video_src_{index}"))
                    .ok_or_else(|| {
                        dependency_error(
                            "recorder_media_writer_pipeline_failed",
                            "A video input is missing.",
                        )
                    })?
                    .downcast::<gst_app::AppSrc>()
                    .map_err(|_| {
                        dependency_error(
                            "recorder_media_writer_pipeline_failed",
                            "A video input is invalid.",
                        )
                    })?
            };
            Ok((source.clone(), element))
        })
        .collect::<Result<Vec<_>, RecorderError>>()?;
    let extra_video_sources = sources
        .iter()
        .skip(1)
        .map(|(_, source)| source.clone())
        .collect();
    let worker = spawn_frame_pump(appsrc.clone(), receiver, sources);
    let frame_pump = thread::spawn(move || {
        let result = match worker.join() {
            Ok(result) => result,
            Err(_error) => Err("GStreamer frame writer thread panicked.".to_string()),
        };
        let _ = frame_pump_result_sender.send(result);
    });
    if let Err(error) = pipeline.set_state(gst::State::Playing) {
        error!(error = ?error, "media_writer_pipeline_start_failed");
        let _ = stop_frames.send(FramePumpStop {
            stop_at_ms: input.started_at_ms,
        });
        let _ = appsrc.end_of_stream();
        let _ = pipeline.set_state(gst::State::Null);
        let _ = frame_pump.join();
        return Err(dependency_error(
            "recorder_media_writer_start_failed",
            format!("Failed to start GStreamer pipeline: {error:?}."),
        ));
    }
    info!(
        output_path = %input.output_path.to_string_lossy(),
        "media_writer_pipeline_started"
    );

    Ok(MediaWriterSegment {
        appsrc,
        extra_video_sources,
        audio_branches: audio_branches(input),
        bus,
        frame_pump: Some(frame_pump),
        frame_pump_result,
        frame_pump_completion: None,
        pipeline,
        benchmark,
        started_at_ms: input.started_at_ms,
        stop_frames: Some(stop_frames),
        fragments,
        fragment_overflow,
        open_fragments,
        fragment_seen,
        reference_eos,
    })
}

fn benchmark_session_for_input(input: &MediaSegmentInput) -> Option<RecorderBenchmarkSession> {
    RecorderBenchmarkSession::from_env(RecorderBenchmarkSegmentInput {
        benchmarks_dir: input.benchmarks_dir.clone(),
        canvas_height: input.canvas_height,
        canvas_width: input.canvas_width,
        desired_fps: input.fps,
        recording_id: input.recording_id.clone(),
        segment_id: input.segment_id.clone(),
        segment_index: input.segment_index,
    })
}
