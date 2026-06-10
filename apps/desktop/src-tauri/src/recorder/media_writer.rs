use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc, Arc,
    },
    thread,
    time::Duration,
};

use gstreamer as gst;
use gstreamer::prelude::*;
use gstreamer_app as gst_app;
use gstreamer_pbutils as gst_pbutils;
use serde::{Deserialize, Serialize};
use tracing::{error, info, trace, warn};

use super::{
    benchmark::{RecorderBenchmarkSegmentInput, RecorderBenchmarkSession, RecorderBenchmarkStage},
    capture_runtime::{
        record_frame_sent_to_writer, CapturedFrame, CapturedFrameFormat, FrameReceiver,
        SharedCaptureTiming,
    },
    errors::dependency_error,
    types::{RecorderError, RecorderMediaWriterCapabilities, RecorderMediaWriterHealth},
};

const REQUIRED_VIDEO_ELEMENTS: &[&str] = &[
    "appsrc",
    "queue",
    "videoconvert",
    "x264enc",
    "h264parse",
    "matroskamux",
    "filesink",
];
const REQUIRED_AUDIO_ELEMENTS: &[&str] = &["pulsesrc", "audioconvert", "audioresample", "opusenc"];
const GSTREAMER_PACKAGE_HINTS: &[GstreamerPackageHint] = &[
    GstreamerPackageHint::new(
        "appsrc",
        "gstreamer1.0-plugins-base",
        "gstreamer1-plugins-base",
    ),
    GstreamerPackageHint::new(
        "audioconvert",
        "gstreamer1.0-plugins-base",
        "gstreamer1-plugins-base",
    ),
    GstreamerPackageHint::new(
        "audioresample",
        "gstreamer1.0-plugins-base",
        "gstreamer1-plugins-base",
    ),
    GstreamerPackageHint::new("filesink", "libgstreamer1.0-0", "gstreamer1"),
    GstreamerPackageHint::new(
        "h264parse",
        "gstreamer1.0-plugins-bad",
        "gstreamer1-plugins-bad-free",
    ),
    GstreamerPackageHint::new(
        "matroskamux",
        "gstreamer1.0-plugins-good",
        "gstreamer1-plugins-good",
    ),
    GstreamerPackageHint::new(
        "opusenc",
        "gstreamer1.0-plugins-base",
        "gstreamer1-plugins-base",
    ),
    GstreamerPackageHint::new(
        "pulsesrc",
        "gstreamer1.0-pulseaudio",
        "gstreamer1-plugins-good",
    ),
    GstreamerPackageHint::new("queue", "libgstreamer1.0-0", "gstreamer1"),
    GstreamerPackageHint::new(
        "videoconvert",
        "gstreamer1.0-plugins-base",
        "gstreamer1-plugins-base",
    ),
    GstreamerPackageHint::new(
        "x264enc",
        "gstreamer1.0-plugins-ugly",
        "gstreamer1-plugins-ugly",
    ),
];
const DESKTOP_AUDIO_BITRATE_BPS: u32 = 96_000;
const MICROPHONE_AUDIO_BITRATE_BPS: u32 = 64_000;
const LOW_LATENCY_QUEUE: &str = "queue max-size-time=0 max-size-bytes=0 max-size-buffers=0";
const PROBE_TIMEOUT: Duration = Duration::from_secs(5);
const EOS_TIMEOUT: Duration = Duration::from_secs(10);
const SOURCE_EOS_TIMEOUT: Duration = Duration::from_secs(2);
#[cfg(not(test))]
const FRAME_PUMP_STOP_TIMEOUT: Duration = Duration::from_secs(5);
#[cfg(test)]
const FRAME_PUMP_STOP_TIMEOUT: Duration = Duration::from_millis(100);

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MediaSegmentInput {
    pub(crate) benchmarks_dir: PathBuf,
    pub(crate) canvas_height: u32,
    pub(crate) canvas_width: u32,
    pub(crate) desktop_audio_source_id: Option<String>,
    pub(crate) fps: f64,
    pub(crate) microphone_audio_source_id: Option<String>,
    pub(crate) output_path: PathBuf,
    pub(crate) recording_id: String,
    pub(crate) segment_id: String,
    pub(crate) segment_index: u32,
    pub(crate) started_at_ms: u64,
    pub(crate) video_bitrate_kbps: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MediaWriterStreamIndexes {
    pub(crate) desktop_audio_stream_index: Option<i64>,
    pub(crate) microphone_audio_stream_index: Option<i64>,
    pub(crate) video_stream_index: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MediaSegmentProbe {
    pub(crate) audio: Vec<MediaSegmentAudioProbe>,
    pub(crate) available: bool,
    pub(crate) duration_ms: Option<u64>,
    pub(crate) error: Option<String>,
    pub(crate) probe_backend: String,
    pub(crate) streams: Vec<MediaSegmentStreamProbe>,
    pub(crate) video: Option<MediaSegmentVideoProbe>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MediaSegmentStreamProbe {
    pub(crate) codec_name: Option<String>,
    pub(crate) codec_type: Option<String>,
    pub(crate) duration_ms: Option<u64>,
    pub(crate) index: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MediaSegmentVideoProbe {
    pub(crate) codec_name: Option<String>,
    pub(crate) duration_ms: Option<u64>,
    pub(crate) estimated_frame_count: Option<u64>,
    pub(crate) height: Option<u32>,
    pub(crate) index: i64,
    pub(crate) pixel_format: Option<String>,
    pub(crate) width: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MediaSegmentAudioProbe {
    pub(crate) channels: Option<u32>,
    pub(crate) codec_name: Option<String>,
    pub(crate) duration_ms: Option<u64>,
    pub(crate) index: i64,
    pub(crate) sample_rate: Option<u32>,
}

#[derive(Debug)]
pub(crate) struct MediaWriterSegment {
    appsrc: gst_app::AppSrc,
    audio_branches: Vec<AudioBranch>,
    bus: gst::Bus,
    frame_pump: Option<thread::JoinHandle<()>>,
    frame_pump_result: mpsc::Receiver<Result<(), String>>,
    pipeline: gst::Pipeline,
    benchmark: Option<RecorderBenchmarkSession>,
    started_at_ms: u64,
    stop_frames: Option<mpsc::Sender<FramePumpStop>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct AudioBranch {
    source_name: &'static str,
    cutoff_queue_name: &'static str,
}

#[derive(Debug)]
struct AudioCutoffProbe {
    source_name: &'static str,
    dropped_buffers: Arc<AtomicU64>,
}

#[derive(Clone, Copy, Debug)]
struct GstreamerPackageHint {
    deb: &'static str,
    element: &'static str,
    rpm: &'static str,
}

impl GstreamerPackageHint {
    const fn new(element: &'static str, deb: &'static str, rpm: &'static str) -> Self {
        Self { deb, element, rpm }
    }
}

#[derive(Debug)]
struct PreparedVideoFrame {
    data: Vec<u8>,
    format: CapturedFrameFormat,
    height: u32,
    width: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FramePumpStop {
    stop_at_ms: u64,
}

pub(crate) fn collect_media_writer_health() -> RecorderMediaWriterHealth {
    let (available, init_error) = match gst::init() {
        Ok(()) => (true, None),
        Err(error) => (
            false,
            Some(format!("GStreamer initialization failed: {error}.")),
        ),
    };
    let capabilities = if available {
        collect_capabilities()
    } else {
        RecorderMediaWriterCapabilities::default()
    };
    let error = init_error.or_else(|| capability_error(&capabilities));

    RecorderMediaWriterHealth {
        available: error.is_none(),
        capabilities,
        error,
        version: available.then(gst_version_string),
        writer: Some("gstreamer".to_string()),
    }
}

pub(crate) fn validate_segment_capabilities(
    capabilities: &RecorderMediaWriterCapabilities,
    input: &MediaSegmentInput,
) -> Result<(), RecorderError> {
    if let Some(error) = segment_capability_error(capabilities, input) {
        return Err(dependency_error("recorder_media_writer_unavailable", error));
    }

    Ok(())
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
    start_segment_with_frame_pump(input, true, benchmark, move |appsrc, stop| {
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
    start_segment_with_frame_pump(input, false, benchmark, move |appsrc, stop| {
        thread::spawn(move || {
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
    pub(crate) fn benchmark(&self) -> Option<RecorderBenchmarkSession> {
        self.benchmark.clone()
    }

    pub(crate) fn stop(mut self, stop_at_ms: u64) -> Result<(), RecorderError> {
        info!(stop_at_ms, "media_writer_stop_requested");
        let audio_cutoff_probes = install_audio_cutoff_probes(
            &self.pipeline,
            &self.audio_branches,
            self.started_at_ms,
            stop_at_ms,
        )?;
        if let Some(stop_frames) = self.stop_frames.take() {
            let _ = stop_frames.send(FramePumpStop { stop_at_ms });
            trace!("media_writer_frame_pump_stop_signaled");
        }

        if let Err(error) = send_audio_source_eos(&self.pipeline, &self.audio_branches) {
            let _ = self.pipeline.set_state(gst::State::Null);
            return Err(error);
        }

        match self.frame_pump_result.recv_timeout(FRAME_PUMP_STOP_TIMEOUT) {
            Ok(Ok(())) => trace!("media_writer_frame_pump_stopped"),
            Ok(Err(error)) => {
                error!(error = %error, "media_writer_frame_pump_failed");
                let _ = self.pipeline.set_state(gst::State::Null);
                return Err(dependency_error(
                    "recorder_media_writer_frame_write_failed",
                    format!("Failed to feed GStreamer video frames: {error}"),
                ));
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                error!("media_writer_frame_pump_stop_timeout");
                let _ = self.pipeline.set_state(gst::State::Null);
                return Err(dependency_error(
                    "recorder_media_writer_frame_write_failed",
                    "Timed out waiting for the GStreamer frame writer to stop.",
                ));
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                error!("media_writer_frame_pump_result_disconnected");
                let _ = self.pipeline.set_state(gst::State::Null);
                return Err(dependency_error(
                    "recorder_media_writer_frame_write_failed",
                    "GStreamer frame writer stopped without reporting a result.",
                ));
            }
        }
        if let Some(frame_pump) = self.frame_pump.take() {
            let _ = frame_pump.join();
        }

        let _ = self.appsrc.end_of_stream();
        trace!("media_writer_appsrc_eos_sent");

        trace!("media_writer_waiting_for_bus_eos");
        let result = wait_for_eos_or_error(&self.bus);
        let _ = self.pipeline.set_state(gst::State::Null);
        log_audio_cutoff_results(&audio_cutoff_probes);
        match &result {
            Ok(()) => info!("media_writer_stop_finalized"),
            Err(error) => error!(
                error_code = %error.code,
                error_category = ?error.category,
                "media_writer_stop_failed"
            ),
        }
        if let Ok(()) = &result {
            if let Some(benchmark) = &self.benchmark {
                benchmark.finish();
            }
        }

        result
    }
}

fn start_segment_with_frame_pump(
    input: &MediaSegmentInput,
    _stop_frame_pump_on_stop: bool,
    benchmark: Option<RecorderBenchmarkSession>,
    spawn_frame_pump: impl FnOnce(
        gst_app::AppSrc,
        mpsc::Receiver<FramePumpStop>,
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
    let (stop_frames, receiver) = mpsc::channel();
    let (frame_pump_result_sender, frame_pump_result) = mpsc::channel();
    let worker = spawn_frame_pump(appsrc.clone(), receiver);
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
        audio_branches: audio_branches(input),
        bus,
        frame_pump: Some(frame_pump),
        frame_pump_result,
        pipeline,
        benchmark,
        started_at_ms: input.started_at_ms,
        stop_frames: Some(stop_frames),
    })
}

pub(crate) fn benchmark_session_for_input(
    input: &MediaSegmentInput,
) -> Option<RecorderBenchmarkSession> {
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

pub(crate) fn stream_indexes(input: &MediaSegmentInput) -> MediaWriterStreamIndexes {
    let mut next_stream_index = 1;
    let desktop_audio_stream_index = input.desktop_audio_source_id.as_ref().map(|_source_id| {
        let index = next_stream_index;
        next_stream_index += 1;
        index
    });
    let microphone_audio_stream_index =
        input.microphone_audio_source_id.as_ref().map(|_source_id| {
            let index = next_stream_index;
            next_stream_index += 1;
            index
        });

    MediaWriterStreamIndexes {
        desktop_audio_stream_index,
        microphone_audio_stream_index,
        video_stream_index: 0,
    }
}

#[cfg(test)]
fn audio_source_names(input: &MediaSegmentInput) -> Vec<&'static str> {
    audio_branches(input)
        .into_iter()
        .map(|branch| branch.source_name)
        .collect()
}

fn audio_branches(input: &MediaSegmentInput) -> Vec<AudioBranch> {
    let mut names = Vec::with_capacity(2);
    if input.desktop_audio_source_id.is_some() {
        names.push(AudioBranch {
            source_name: "desktop_audio_src",
            cutoff_queue_name: "desktop_audio_cutoff_queue",
        });
    }
    if input.microphone_audio_source_id.is_some() {
        names.push(AudioBranch {
            source_name: "microphone_audio_src",
            cutoff_queue_name: "microphone_audio_cutoff_queue",
        });
    }

    names
}

pub(crate) fn probe_segment_best_effort(segment_path: &Path) -> MediaSegmentProbe {
    match probe_segment(segment_path) {
        Ok(probe) => probe,
        Err(error) => unavailable_segment_probe(error),
    }
}

pub(crate) fn probe_json_string(probe: &MediaSegmentProbe) -> String {
    serde_json::to_string(probe).unwrap_or_else(|error| {
        serde_json::to_string(&unavailable_segment_probe(error.to_string()))
            .expect("serialize unavailable media probe")
    })
}

fn build_pipeline(input: &MediaSegmentInput) -> Result<gst::Pipeline, RecorderError> {
    let launch = build_pipeline_description(input);
    trace!(
        output_path = %input.output_path.to_string_lossy(),
        launch = %launch,
        "media_writer_pipeline_description_built"
    );
    let element = gst::parse::launch(&launch).map_err(|error| {
        dependency_error(
            "recorder_media_writer_pipeline_failed",
            format!("Failed to build GStreamer pipeline: {error}."),
        )
    })?;
    element.downcast::<gst::Pipeline>().map_err(|_error| {
        dependency_error(
            "recorder_media_writer_pipeline_failed",
            "GStreamer launch description did not produce a pipeline.",
        )
    })
}

fn build_pipeline_description(input: &MediaSegmentInput) -> String {
    let key_int_max = key_int_max(input.fps, 20);
    let mut description = format!(
        "appsrc name=video_src is-live=true format=time do-timestamp=false block=true \
         ! {queue} \
         ! videoconvert ! video/x-raw,format=I420 \
         ! x264enc bitrate={bitrate} key-int-max={key_int_max} speed-preset=medium \
             tune=zerolatency rc-lookahead=0 bframes=0 sync-lookahead=0 \
         ! h264parse ! {queue} ! mux. ",
        bitrate = input.video_bitrate_kbps,
        queue = LOW_LATENCY_QUEUE,
    );
    if let Some(source_id) = &input.desktop_audio_source_id {
        description.push_str(&audio_branch(
            source_id,
            DESKTOP_AUDIO_BITRATE_BPS,
            "desktop_audio_src",
        ));
    }
    if let Some(source_id) = &input.microphone_audio_source_id {
        description.push_str(&audio_branch(
            source_id,
            MICROPHONE_AUDIO_BITRATE_BPS,
            "microphone_audio_src",
        ));
    }
    description.push_str(&format!(
        "matroskamux name=mux ! filesink location={}",
        launch_value(input.output_path.as_os_str())
    ));

    description
}

fn audio_branch(source_id: &str, bitrate: u32, name: &str) -> String {
    let cutoff_queue_name = match name {
        "desktop_audio_src" => "desktop_audio_cutoff_queue",
        "microphone_audio_src" => "microphone_audio_cutoff_queue",
        _ => "audio_cutoff_queue",
    };
    format!(
        "pulsesrc name={name} device={device} ! {queue} ! audioconvert ! audioresample \
         ! audio/x-raw,rate=48000 ! opusenc bitrate={bitrate} ! {cutoff_queue} ! mux. ",
        name = name,
        device = launch_value(source_id),
        bitrate = bitrate,
        queue = LOW_LATENCY_QUEUE,
        cutoff_queue = low_latency_queue_with_name(cutoff_queue_name),
    )
}

fn low_latency_queue_with_name(name: &str) -> String {
    format!("queue name={name} max-size-time=0 max-size-bytes=0 max-size-buffers=0")
}

fn wait_for_eos_or_error(bus: &gst::Bus) -> Result<(), RecorderError> {
    loop {
        let Some(message) = bus.timed_pop(duration_to_clock_time(EOS_TIMEOUT)) else {
            warn!("media_writer_eos_timeout");
            return Err(dependency_error(
                "recorder_media_writer_finalize_failed",
                "Timed out waiting for GStreamer to finalize the segment.",
            ));
        };
        match message.view() {
            gst::MessageView::Eos(_eos) => {
                info!("media_writer_eos_received");
                return Ok(());
            }
            gst::MessageView::Error(error) => {
                error!(
                    source = %error
                        .src()
                        .map(|source| source.path_string())
                        .unwrap_or_else(|| "unknown".into()),
                    error = %error.error(),
                    "media_writer_bus_error"
                );
                return Err(dependency_error(
                    "recorder_media_writer_failed",
                    format!(
                        "GStreamer error from {}: {}.",
                        error
                            .src()
                            .map(|source| source.path_string())
                            .unwrap_or_else(|| "unknown".into()),
                        error.error()
                    ),
                ));
            }
            _ => {}
        }
    }
}

fn send_audio_source_eos(
    pipeline: &gst::Pipeline,
    audio_branches: &[AudioBranch],
) -> Result<(), RecorderError> {
    for branch in audio_branches {
        let source_name = branch.source_name;
        let source = pipeline.by_name(source_name).ok_or_else(|| {
            dependency_error(
                "recorder_media_writer_finalize_failed",
                format!("GStreamer audio source {source_name} is missing from the pipeline."),
            )
        })?;
        trace!(
            source = source_name,
            "media_writer_audio_source_eos_requested"
        );
        send_element_eos_with_timeout(source, source_name)?;
        trace!(source = source_name, "media_writer_audio_source_eos_sent");
    }

    Ok(())
}

fn install_audio_cutoff_probes(
    pipeline: &gst::Pipeline,
    audio_branches: &[AudioBranch],
    started_at_ms: u64,
    stop_at_ms: u64,
) -> Result<Vec<AudioCutoffProbe>, RecorderError> {
    let cutoff_pts = duration_to_clock_time(Duration::from_millis(
        stop_at_ms.saturating_sub(started_at_ms),
    ));
    audio_branches
        .iter()
        .map(|branch| {
            let queue = pipeline.by_name(branch.cutoff_queue_name).ok_or_else(|| {
                dependency_error(
                    "recorder_media_writer_finalize_failed",
                    format!(
                        "GStreamer audio cutoff queue {} is missing from the pipeline.",
                        branch.cutoff_queue_name
                    ),
                )
            })?;
            let pad = queue.static_pad("src").ok_or_else(|| {
                dependency_error(
                    "recorder_media_writer_finalize_failed",
                    format!(
                        "GStreamer audio cutoff queue {} has no src pad.",
                        branch.cutoff_queue_name
                    ),
                )
            })?;
            let dropped_buffers = Arc::new(AtomicU64::new(0));
            let dropped_for_probe = dropped_buffers.clone();
            pad.add_probe(gst::PadProbeType::BUFFER, move |_pad, info| {
                if let Some(buffer) = info.buffer() {
                    if audio_buffer_is_after_cutoff(buffer, cutoff_pts) {
                        dropped_for_probe.fetch_add(1, Ordering::Relaxed);
                        return gst::PadProbeReturn::Drop;
                    }
                }
                gst::PadProbeReturn::Ok
            });

            Ok(AudioCutoffProbe {
                source_name: branch.source_name,
                dropped_buffers,
            })
        })
        .collect()
}

fn log_audio_cutoff_results(probes: &[AudioCutoffProbe]) {
    for probe in probes {
        info!(
            source = probe.source_name,
            dropped_buffers = probe.dropped_buffers.load(Ordering::Relaxed),
            "media_writer_audio_cutoff_applied"
        );
    }
}

fn audio_buffer_is_after_cutoff(buffer: &gst::BufferRef, cutoff_pts: gst::ClockTime) -> bool {
    buffer.pts().is_some_and(|pts| pts > cutoff_pts)
}

fn send_element_eos_with_timeout(
    element: gst::Element,
    source_name: &'static str,
) -> Result<(), RecorderError> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let sent = element.send_event(gst::event::Eos::new());
        let _ = sender.send(sent);
    });

    match receiver.recv_timeout(SOURCE_EOS_TIMEOUT) {
        Ok(true) => Ok(()),
        Ok(false) => Err(dependency_error(
            "recorder_media_writer_finalize_failed",
            format!("GStreamer audio source {source_name} rejected EOS."),
        )),
        Err(mpsc::RecvTimeoutError::Timeout) => {
            error!(
                source = source_name,
                "media_writer_audio_source_eos_timeout"
            );
            Err(dependency_error(
                "recorder_media_writer_finalize_failed",
                format!("Timed out sending EOS to GStreamer audio source {source_name}."),
            ))
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => Err(dependency_error(
            "recorder_media_writer_finalize_failed",
            format!("GStreamer audio source {source_name} EOS sender stopped unexpectedly."),
        )),
    }
}

#[cfg(test)]
fn pump_synthetic_frames(
    appsrc: gst_app::AppSrc,
    canvas_width: u32,
    canvas_height: u32,
    fps: f64,
    frame_interval: Duration,
    stop: mpsc::Receiver<FramePumpStop>,
) -> Result<(), String> {
    let frame = black_yuv420p_frame(canvas_width, canvas_height)?;
    let mut frame_index = 0_u64;

    loop {
        if let Err(error) = push_yuv420p_frame(
            &appsrc,
            frame.clone(),
            canvas_width,
            canvas_height,
            frame_index,
            fps,
            frame_interval,
        ) {
            match stop.try_recv() {
                Ok(_) | Err(mpsc::TryRecvError::Disconnected) => return Ok(()),
                Err(mpsc::TryRecvError::Empty) => return Err(error),
            }
        }
        frame_index += 1;

        match stop.recv_timeout(frame_interval) {
            Ok(_) | Err(mpsc::RecvTimeoutError::Disconnected) => return Ok(()),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
    }
}

fn pump_composed_frames(
    appsrc: gst_app::AppSrc,
    frames: FrameReceiver,
    stop: mpsc::Receiver<FramePumpStop>,
    timing: Option<SharedCaptureTiming>,
    fps: f64,
    started_at_ms: u64,
    benchmark: Option<RecorderBenchmarkSession>,
) -> Result<(), String> {
    let frame_interval = frame_interval(fps);
    let mut last_pts = None;

    loop {
        match stop.try_recv() {
            Ok(stop_request) => {
                return drain_ready_composed_frames(
                    &appsrc,
                    &frames,
                    DrainReadyFrames {
                        started_at_ms,
                        last_pts: &mut last_pts,
                        fps,
                        frame_interval,
                        stop_at_ms: stop_request.stop_at_ms,
                        timing: timing.as_ref(),
                        benchmark: benchmark.as_ref(),
                    },
                );
            }
            Err(mpsc::TryRecvError::Disconnected) => return Ok(()),
            Err(mpsc::TryRecvError::Empty) => {}
        }

        match frames.recv_timeout(Duration::from_millis(50)) {
            Ok(frame) => {
                if let Err(error) = push_composed_frame(
                    &appsrc,
                    frame,
                    ComposedFramePushContext {
                        benchmark: benchmark.as_ref(),
                        fps,
                        frame_interval,
                        last_pts: &mut last_pts,
                        started_at_ms,
                        timing: timing.as_ref(),
                    },
                ) {
                    match stop.try_recv() {
                        Ok(_) | Err(mpsc::TryRecvError::Disconnected) => return Ok(()),
                        Err(mpsc::TryRecvError::Empty) => return Err(error),
                    }
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => return Ok(()),
        }
    }
}

fn drain_ready_composed_frames(
    appsrc: &gst_app::AppSrc,
    frames: &FrameReceiver,
    context: DrainReadyFrames<'_>,
) -> Result<(), String> {
    let mut drained_frames = 0_u64;
    let mut dropped_after_stop = 0_u64;
    loop {
        match frames.recv_timeout(Duration::ZERO) {
            Ok(frame) => {
                if frame_is_after_stop(frame.timestamp_ms, context.stop_at_ms) {
                    dropped_after_stop = dropped_after_stop.saturating_add(1);
                    continue;
                }
                push_composed_frame(
                    appsrc,
                    frame,
                    ComposedFramePushContext {
                        benchmark: context.benchmark,
                        fps: context.fps,
                        frame_interval: context.frame_interval,
                        last_pts: context.last_pts,
                        started_at_ms: context.started_at_ms,
                        timing: context.timing,
                    },
                )?;
                drained_frames = drained_frames.saturating_add(1);
            }
            Err(mpsc::RecvTimeoutError::Timeout | mpsc::RecvTimeoutError::Disconnected) => {
                info!(
                    stop_at_ms = context.stop_at_ms,
                    drained_frames, dropped_after_stop, "media_writer_stop_cutoff_applied"
                );
                return Ok(());
            }
        }
    }
}

struct DrainReadyFrames<'a> {
    benchmark: Option<&'a RecorderBenchmarkSession>,
    fps: f64,
    frame_interval: Duration,
    last_pts: &'a mut Option<Duration>,
    started_at_ms: u64,
    stop_at_ms: u64,
    timing: Option<&'a SharedCaptureTiming>,
}

struct ComposedFramePushContext<'a> {
    benchmark: Option<&'a RecorderBenchmarkSession>,
    fps: f64,
    frame_interval: Duration,
    last_pts: &'a mut Option<Duration>,
    started_at_ms: u64,
    timing: Option<&'a SharedCaptureTiming>,
}

fn frame_is_after_stop(frame_timestamp_ms: u64, stop_at_ms: u64) -> bool {
    frame_timestamp_ms > stop_at_ms
}

fn push_composed_frame(
    appsrc: &gst_app::AppSrc,
    frame: CapturedFrame,
    context: ComposedFramePushContext<'_>,
) -> Result<(), String> {
    let pts = live_frame_pts(
        frame.timestamp_ms,
        context.started_at_ms,
        context.frame_interval,
        *context.last_pts,
    );
    let timestamp_ms = frame.timestamp_ms;
    let convert_started = std::time::Instant::now();
    let frame_format = frame.format;
    let frame = prepare_video_frame_for_gstreamer(frame).map_err(|error| error.message)?;
    if let Some(benchmark) = context.benchmark {
        benchmark.record_stage(
            RecorderBenchmarkStage::WriterConvert,
            convert_started.elapsed(),
            Some(timestamp_ms),
            None,
            vec![frame_format.label().to_string()],
            Some(frame.data.len().try_into().unwrap_or(u64::MAX)),
        );
    }
    let push_started = std::time::Instant::now();
    push_raw_video_frame_at_pts(appsrc, frame, pts, context.fps, context.frame_interval)?;
    if let Some(benchmark) = context.benchmark {
        benchmark.record_stage(
            RecorderBenchmarkStage::WriterPush,
            push_started.elapsed(),
            Some(timestamp_ms),
            None,
            Vec::new(),
            None,
        );
    }
    *context.last_pts = Some(pts);
    if let Some(timing) = context.timing {
        record_frame_sent_to_writer(timing);
    }

    Ok(())
}

#[cfg(test)]
fn push_yuv420p_frame(
    appsrc: &gst_app::AppSrc,
    frame: Vec<u8>,
    width: u32,
    height: u32,
    frame_index: u64,
    fps: f64,
    frame_interval: Duration,
) -> Result<(), String> {
    let pts = duration_mul(frame_interval, frame_index);
    push_raw_video_frame_at_pts(
        appsrc,
        PreparedVideoFrame {
            data: frame,
            format: CapturedFrameFormat::Yuv420p,
            height,
            width,
        },
        pts,
        fps,
        frame_interval,
    )
}

fn push_raw_video_frame_at_pts(
    appsrc: &gst_app::AppSrc,
    frame: PreparedVideoFrame,
    pts: Duration,
    fps: f64,
    frame_interval: Duration,
) -> Result<(), String> {
    let caps = video_caps(&frame, fps)?;
    let mut buffer = gst::Buffer::from_mut_slice(frame.data);
    {
        let buffer_ref = buffer
            .get_mut()
            .ok_or_else(|| "GStreamer frame buffer is not writable.".to_string())?;
        buffer_ref.set_pts(gst::ClockTime::from_nseconds(
            pts.as_nanos().try_into().unwrap_or(u64::MAX),
        ));
        buffer_ref.set_duration(gst::ClockTime::from_nseconds(
            frame_interval.as_nanos().try_into().unwrap_or(u64::MAX),
        ));
    }
    let sample = gst::Sample::builder().buffer(&buffer).caps(&caps).build();
    appsrc
        .push_sample(&sample)
        .map(|_success| ())
        .map_err(|error| error.to_string())
}

fn prepare_video_frame_for_gstreamer(
    frame: CapturedFrame,
) -> Result<PreparedVideoFrame, RecorderError> {
    frame.validate()?;
    if frame_is_tightly_packed_for_gstreamer(&frame)? {
        return Ok(PreparedVideoFrame {
            data: frame.data,
            format: frame.format,
            height: frame.height,
            width: frame.width,
        });
    }

    repack_video_frame_for_gstreamer(frame)
}

fn frame_is_tightly_packed_for_gstreamer(frame: &CapturedFrame) -> Result<bool, RecorderError> {
    Ok(match frame.format {
        CapturedFrameFormat::Yuv444p => frame.stride == frame.width,
        CapturedFrameFormat::Yuv420p => frame.stride == frame.width,
        CapturedFrameFormat::Rgbx
        | CapturedFrameFormat::Rgba
        | CapturedFrameFormat::Xrgb
        | CapturedFrameFormat::Argb
        | CapturedFrameFormat::Bgrx
        | CapturedFrameFormat::Bgra => {
            frame.stride
                == frame
                    .width
                    .checked_mul(4)
                    .ok_or_else(|| "Packed video frame stride overflow.".to_string())
                    .map_err(|message| {
                        dependency_error("recorder_media_writer_frame_write_failed", message)
                    })?
        }
    })
}

fn repack_video_frame_for_gstreamer(
    frame: CapturedFrame,
) -> Result<PreparedVideoFrame, RecorderError> {
    let data = match frame.format {
        CapturedFrameFormat::Yuv444p => repack_yuv444p_frame(&frame),
        CapturedFrameFormat::Yuv420p => repack_yuv420p_frame(&frame),
        CapturedFrameFormat::Rgbx
        | CapturedFrameFormat::Rgba
        | CapturedFrameFormat::Xrgb
        | CapturedFrameFormat::Argb
        | CapturedFrameFormat::Bgrx
        | CapturedFrameFormat::Bgra => repack_packed_rgba_frame(&frame),
    }?;
    Ok(PreparedVideoFrame {
        data,
        format: frame.format,
        height: frame.height,
        width: frame.width,
    })
}

fn repack_packed_rgba_frame(frame: &CapturedFrame) -> Result<Vec<u8>, RecorderError> {
    let width_bytes = frame.width.checked_mul(4).ok_or_else(|| {
        dependency_error(
            "recorder_media_writer_frame_write_failed",
            "Packed video frame width overflow.",
        )
    })? as usize;
    let mut output = vec![0; width_bytes * frame.height as usize];
    for row in 0..frame.height as usize {
        let source_start = row * frame.stride as usize;
        let output_start = row * width_bytes;
        output[output_start..output_start + width_bytes]
            .copy_from_slice(&frame.data[source_start..source_start + width_bytes]);
    }
    Ok(output)
}

fn repack_yuv444p_frame(frame: &CapturedFrame) -> Result<Vec<u8>, RecorderError> {
    let width = frame.width as usize;
    let height = frame.height as usize;
    let source_plane_len = frame.stride as usize * height;
    let output_plane_len = width * height;
    let mut output = vec![0; output_plane_len * 3];
    for plane in 0..3 {
        for row in 0..height {
            let source_start = plane * source_plane_len + row * frame.stride as usize;
            let output_start = plane * output_plane_len + row * width;
            output[output_start..output_start + width]
                .copy_from_slice(&frame.data[source_start..source_start + width]);
        }
    }
    Ok(output)
}

fn repack_yuv420p_frame(frame: &CapturedFrame) -> Result<Vec<u8>, RecorderError> {
    let width = frame.width as usize;
    let height = frame.height as usize;
    let y_source_stride = frame.stride as usize;
    let uv_source_stride = y_source_stride / 2;
    let y_output_len = width * height;
    let uv_width = width / 2;
    let uv_height = height / 2;
    let uv_output_len = uv_width * uv_height;
    let y_source_len = y_source_stride * height;
    let uv_source_len = uv_source_stride * uv_height;
    let mut output = vec![0; y_output_len + uv_output_len * 2];
    for row in 0..height {
        let source_start = row * y_source_stride;
        let output_start = row * width;
        output[output_start..output_start + width]
            .copy_from_slice(&frame.data[source_start..source_start + width]);
    }
    for plane in 0..2 {
        for row in 0..uv_height {
            let source_start = y_source_len + plane * uv_source_len + row * uv_source_stride;
            let output_start = y_output_len + plane * uv_output_len + row * uv_width;
            output[output_start..output_start + uv_width]
                .copy_from_slice(&frame.data[source_start..source_start + uv_width]);
        }
    }
    Ok(output)
}

fn video_caps(frame: &PreparedVideoFrame, fps: f64) -> Result<gst::Caps, String> {
    let format = gst_video_format(frame.format);
    let mut builder = gst::Caps::builder("video/x-raw")
        .field("format", format)
        .field("framerate", gst_framerate_fraction(fps));
    if frame.width > 0 {
        builder = builder.field("width", frame.width as i32);
    }
    if frame.height > 0 {
        builder = builder.field("height", frame.height as i32);
    }
    Ok(builder.build())
}

fn gst_video_format(format: CapturedFrameFormat) -> &'static str {
    match format {
        CapturedFrameFormat::Yuv444p => "Y444",
        CapturedFrameFormat::Yuv420p => "I420",
        CapturedFrameFormat::Rgbx => "RGBx",
        CapturedFrameFormat::Rgba => "RGBA",
        CapturedFrameFormat::Xrgb => "xRGB",
        CapturedFrameFormat::Argb => "ARGB",
        CapturedFrameFormat::Bgrx => "BGRx",
        CapturedFrameFormat::Bgra => "BGRA",
    }
}

fn live_frame_pts(
    frame_timestamp_ms: u64,
    started_at_ms: u64,
    frame_interval: Duration,
    last_pts: Option<Duration>,
) -> Duration {
    let raw_pts = Duration::from_millis(frame_timestamp_ms.saturating_sub(started_at_ms));
    match last_pts {
        Some(last_pts) if raw_pts <= last_pts => last_pts.saturating_add(frame_interval),
        Some(last_pts) => raw_pts.max(last_pts.saturating_add(frame_interval)),
        None => raw_pts,
    }
}

fn probe_segment(segment_path: &Path) -> Result<MediaSegmentProbe, String> {
    gst::init().map_err(|error| format!("GStreamer initialization failed: {error}."))?;
    let discoverer =
        gst_pbutils::Discoverer::new(gst::ClockTime::from_seconds(PROBE_TIMEOUT.as_secs()))
            .map_err(|error| format!("Failed to create GStreamer discoverer: {error}."))?;
    let uri = gst::glib::filename_to_uri(segment_path, None::<&str>)
        .map_err(|error| format!("Failed to build file URI for media probe: {error}."))?;
    let info = discoverer
        .discover_uri(uri.as_ref())
        .map_err(|error| format!("GStreamer media discovery failed: {error}."))?;
    let duration_ms = info.duration().map(clock_time_ms);
    let video_streams = info.video_streams();
    let audio_streams = info.audio_streams();
    let video = video_streams
        .first()
        .map(|stream| video_probe(0, stream, duration_ms));
    let audio = audio_streams
        .iter()
        .enumerate()
        .map(|(index, stream)| audio_probe(index as i64 + 1, stream, duration_ms))
        .collect::<Vec<_>>();
    let mut streams = Vec::new();
    if let Some(video) = &video {
        streams.push(MediaSegmentStreamProbe {
            codec_name: video.codec_name.clone(),
            codec_type: Some("video".to_string()),
            duration_ms: video.duration_ms,
            index: video.index,
        });
    }
    streams.extend(audio.iter().map(|audio| MediaSegmentStreamProbe {
        codec_name: audio.codec_name.clone(),
        codec_type: Some("audio".to_string()),
        duration_ms: audio.duration_ms,
        index: audio.index,
    }));

    Ok(MediaSegmentProbe {
        audio,
        available: true,
        duration_ms,
        error: None,
        probe_backend: "gstreamer-discoverer".to_string(),
        streams,
        video,
    })
}

fn video_probe(
    index: i64,
    stream: &gst_pbutils::DiscovererVideoInfo,
    duration_ms: Option<u64>,
) -> MediaSegmentVideoProbe {
    MediaSegmentVideoProbe {
        codec_name: stream_caps_name(stream),
        duration_ms,
        estimated_frame_count: None,
        height: Some(stream.height()),
        index,
        pixel_format: stream_caps_field(stream, "format"),
        width: Some(stream.width()),
    }
}

fn audio_probe(
    index: i64,
    stream: &gst_pbutils::DiscovererAudioInfo,
    duration_ms: Option<u64>,
) -> MediaSegmentAudioProbe {
    MediaSegmentAudioProbe {
        channels: Some(stream.channels()),
        codec_name: stream_caps_name(stream),
        duration_ms,
        index,
        sample_rate: Some(stream.sample_rate()),
    }
}

fn stream_caps_name(stream: &impl gst_pbutils::prelude::DiscovererStreamInfoExt) -> Option<String> {
    stream.caps().and_then(|caps| {
        caps.structure(0)
            .map(|structure| structure.name().to_string())
    })
}

fn stream_caps_field(
    stream: &impl gst_pbutils::prelude::DiscovererStreamInfoExt,
    field: &str,
) -> Option<String> {
    stream.caps().and_then(|caps| {
        caps.structure(0)
            .and_then(|structure| structure.get::<String>(field).ok())
    })
}

fn unavailable_segment_probe(error: impl Into<String>) -> MediaSegmentProbe {
    MediaSegmentProbe {
        audio: Vec::new(),
        available: false,
        duration_ms: None,
        error: Some(error.into()),
        probe_backend: "gstreamer-discoverer".to_string(),
        streams: Vec::new(),
        video: None,
    }
}

fn collect_capabilities() -> RecorderMediaWriterCapabilities {
    RecorderMediaWriterCapabilities {
        appsrc: element_available("appsrc"),
        audio_convert: element_available("audioconvert"),
        audio_resample: element_available("audioresample"),
        file_sink: element_available("filesink"),
        h264_parse: element_available("h264parse"),
        matroska_muxer: element_available("matroskamux"),
        opus_encoder: element_available("opusenc"),
        pulse_source: element_available("pulsesrc"),
        queue: element_available("queue"),
        video_convert: element_available("videoconvert"),
        x264_encoder: element_available("x264enc"),
    }
}

fn element_available(name: &str) -> bool {
    gst::ElementFactory::find(name).is_some()
}

fn capability_error(capabilities: &RecorderMediaWriterCapabilities) -> Option<String> {
    missing_capability_error(capabilities, REQUIRED_VIDEO_ELEMENTS)
}

fn segment_capability_error(
    capabilities: &RecorderMediaWriterCapabilities,
    input: &MediaSegmentInput,
) -> Option<String> {
    let mut missing = Vec::new();
    for element in REQUIRED_VIDEO_ELEMENTS {
        if !capability_for_element(capabilities, element) {
            missing.push(*element);
        }
    }
    if input.desktop_audio_source_id.is_some() || input.microphone_audio_source_id.is_some() {
        for element in REQUIRED_AUDIO_ELEMENTS {
            if !capability_for_element(capabilities, element) {
                missing.push(*element);
            }
        }
    }
    if missing.is_empty() {
        None
    } else {
        Some(format!(
            "GStreamer is missing required media writer elements: {}. {}",
            missing.join(", "),
            missing_package_hint(&missing)
        ))
    }
}

fn missing_capability_error(
    capabilities: &RecorderMediaWriterCapabilities,
    elements: &[&str],
) -> Option<String> {
    let missing = elements
        .iter()
        .copied()
        .filter(|element| !capability_for_element(capabilities, element))
        .collect::<Vec<_>>();
    if missing.is_empty() {
        None
    } else {
        Some(format!(
            "GStreamer is missing required media writer elements: {}. {}",
            missing.join(", "),
            missing_package_hint(&missing)
        ))
    }
}

fn missing_package_hint(missing: &[&str]) -> String {
    let deb_packages = package_hints_for(missing, |hint| hint.deb);
    let rpm_packages = package_hints_for(missing, |hint| hint.rpm);

    format!(
        "Install the Linux GStreamer runtime/plugins declared in apps/desktop/src-tauri/tauri.conf.json: Debian/Ubuntu {}; RPM-family {}.",
        deb_packages.join(" "),
        rpm_packages.join(" ")
    )
}

fn package_hints_for(
    missing: &[&str],
    package_for: impl Fn(&GstreamerPackageHint) -> &'static str,
) -> Vec<&'static str> {
    let mut packages = Vec::new();
    for element in missing {
        let Some(hint) = GSTREAMER_PACKAGE_HINTS
            .iter()
            .find(|hint| hint.element == *element)
        else {
            continue;
        };
        let package = package_for(hint);
        if !packages.contains(&package) {
            packages.push(package);
        }
    }
    packages
}

fn capability_for_element(capabilities: &RecorderMediaWriterCapabilities, element: &str) -> bool {
    match element {
        "appsrc" => capabilities.appsrc,
        "audioconvert" => capabilities.audio_convert,
        "audioresample" => capabilities.audio_resample,
        "filesink" => capabilities.file_sink,
        "h264parse" => capabilities.h264_parse,
        "matroskamux" => capabilities.matroska_muxer,
        "opusenc" => capabilities.opus_encoder,
        "pulsesrc" => capabilities.pulse_source,
        "queue" => capabilities.queue,
        "videoconvert" => capabilities.video_convert,
        "x264enc" => capabilities.x264_encoder,
        _ => false,
    }
}

#[cfg(test)]
fn black_yuv420p_frame(canvas_width: u32, canvas_height: u32) -> Result<Vec<u8>, String> {
    if !canvas_width.is_multiple_of(2) || !canvas_height.is_multiple_of(2) {
        return Err("I420 frame dimensions must be even.".to_string());
    }
    let plane_len = (canvas_width as usize)
        .checked_mul(canvas_height as usize)
        .ok_or_else(|| "Frame dimensions overflow.".to_string())?;
    let chroma_len = (canvas_width as usize / 2)
        .checked_mul(canvas_height as usize / 2)
        .ok_or_else(|| "Frame dimensions overflow.".to_string())?;
    let frame_len = plane_len
        .checked_add(chroma_len * 2)
        .ok_or_else(|| "Frame dimensions overflow.".to_string())?;
    let mut frame = vec![0; frame_len];
    frame[plane_len..].fill(128);

    Ok(frame)
}

fn frame_interval(fps: f64) -> Duration {
    if fps.is_finite() && fps > 0.0 {
        Duration::from_secs_f64(1.0 / fps)
    } else {
        Duration::from_secs(1)
    }
}

#[cfg(test)]
fn duration_mul(duration: Duration, multiplier: u64) -> Duration {
    let nanos = duration.as_nanos().saturating_mul(multiplier as u128);
    Duration::from_nanos(nanos.try_into().unwrap_or(u64::MAX))
}

fn gst_framerate_fraction(fps: f64) -> gst::Fraction {
    if !fps.is_finite() || fps <= 0.0 {
        return gst::Fraction::new(1, 1);
    }
    if fps.fract() == 0.0 {
        return gst::Fraction::new(fps.round().clamp(1.0, 60.0) as i32, 1);
    }
    if (fps - 0.5).abs() < f64::EPSILON {
        return gst::Fraction::new(1, 2);
    }
    if (fps - 0.2).abs() < f64::EPSILON {
        return gst::Fraction::new(1, 5);
    }
    if (fps - 0.1).abs() < f64::EPSILON {
        return gst::Fraction::new(1, 10);
    }

    gst::Fraction::new((fps * 1000.0).round().clamp(1.0, 60_000.0) as i32, 1000)
}

#[cfg(test)]
fn gst_fraction(fps: f64) -> String {
    let fraction = gst_framerate_fraction(fps);
    format!("{}/{}", fraction.numer(), fraction.denom())
}

fn key_int_max(fps: f64, seconds: u32) -> u32 {
    (fps * seconds as f64).round().max(1.0) as u32
}

fn clock_time_ms(value: gst::ClockTime) -> u64 {
    value.mseconds()
}

fn duration_to_clock_time(duration: Duration) -> gst::ClockTime {
    gst::ClockTime::from_nseconds(duration.as_nanos().try_into().unwrap_or(u64::MAX))
}

fn gst_version_string() -> String {
    let version = gst::version();
    format!("{}.{}.{}.{}", version.0, version.1, version.2, version.3)
}

fn launch_value(value: impl AsRef<std::ffi::OsStr>) -> String {
    let value = value.as_ref().to_string_lossy();
    if !value.chars().any(char::is_whitespace) {
        return value.to_string();
    }

    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

#[cfg(test)]
mod tests {
    use std::{fs, sync::mpsc, thread, time::Duration};

    use gstreamer as gst;
    use uuid::Uuid;

    use crate::recorder::capture_runtime::{
        bounded_frame_channel, CapturedFrame, CapturedFrameFormat,
    };
    use crate::recorder::types::RecorderMediaWriterCapabilities;

    use super::{
        audio_buffer_is_after_cutoff, audio_source_names, black_yuv420p_frame,
        build_pipeline_description, duration_to_clock_time, frame_is_after_stop,
        frame_is_tightly_packed_for_gstreamer, gst_fraction, gst_video_format, key_int_max,
        live_frame_pts, prepare_video_frame_for_gstreamer, probe_segment_best_effort,
        start_segment_from_frames, start_segment_with_frame_pump, start_synthetic_segment,
        stream_indexes, validate_segment_capabilities, video_caps, MediaSegmentInput,
        PreparedVideoFrame,
    };

    #[test]
    fn gstreamer_fraction_formats_low_fps_values() {
        assert_eq!(gst_fraction(1.0), "1/1");
        assert_eq!(gst_fraction(0.5), "1/2");
        assert_eq!(gst_fraction(0.2), "1/5");
        assert_eq!(gst_fraction(0.1), "1/10");
        assert_eq!(gst_fraction(120.0), "60/1");
    }

    #[test]
    fn video_caps_declare_configured_low_fps_framerate() {
        gst::init().expect("initialize gstreamer");
        let frame = PreparedVideoFrame {
            data: black_yuv420p_frame(1280, 720).expect("black yuv frame"),
            format: CapturedFrameFormat::Yuv420p,
            height: 720,
            width: 1280,
        };
        let caps = video_caps(&frame, 0.2).expect("video caps");
        let framerate = caps
            .structure(0)
            .and_then(|structure| structure.get::<gst::Fraction>("framerate").ok())
            .expect("video caps should include framerate");

        assert_eq!(framerate, gst::Fraction::new(1, 5));
    }

    #[test]
    fn keyframe_interval_uses_frame_count_for_low_fps() {
        assert_eq!(key_int_max(1.0, 20), 20);
        assert_eq!(key_int_max(0.2, 20), 4);
        assert_eq!(key_int_max(0.1, 20), 2);
    }

    #[test]
    fn live_frame_pts_preserves_wall_clock_time_instead_of_frame_count() {
        let frame_interval = Duration::from_millis(200);
        let started_at_ms = 1_000;
        let first = live_frame_pts(1_000, started_at_ms, frame_interval, None);
        let second = live_frame_pts(2_000, started_at_ms, frame_interval, Some(first));
        let third = live_frame_pts(3_000, started_at_ms, frame_interval, Some(second));

        assert_eq!(first, Duration::ZERO);
        assert_eq!(second, Duration::from_secs(1));
        assert_eq!(third, Duration::from_secs(2));
    }

    #[test]
    fn live_frame_pts_clamps_duplicate_or_backwards_timestamps_forward() {
        let frame_interval = Duration::from_millis(200);
        let started_at_ms = 1_000;
        let first = live_frame_pts(1_000, started_at_ms, frame_interval, None);
        let duplicate = live_frame_pts(1_000, started_at_ms, frame_interval, Some(first));
        let backwards = live_frame_pts(900, started_at_ms, frame_interval, Some(duplicate));

        assert_eq!(first, Duration::ZERO);
        assert_eq!(duplicate, Duration::from_millis(200));
        assert_eq!(backwards, Duration::from_millis(400));
    }

    #[test]
    fn frame_stop_cutoff_allows_stop_timestamp_and_drops_later_frames() {
        assert!(!frame_is_after_stop(1_000, 1_000));
        assert!(!frame_is_after_stop(999, 1_000));
        assert!(frame_is_after_stop(1_001, 1_000));
    }

    #[test]
    fn audio_stop_cutoff_allows_stop_pts_and_drops_later_buffers() {
        gst::init().expect("initialize gstreamer");
        let cutoff = duration_to_clock_time(Duration::from_millis(1_000));
        let mut at_cutoff = gst::Buffer::new();
        at_cutoff
            .get_mut()
            .expect("writable buffer")
            .set_pts(cutoff);
        let mut after_cutoff = gst::Buffer::new();
        after_cutoff
            .get_mut()
            .expect("writable buffer")
            .set_pts(duration_to_clock_time(Duration::from_millis(1_001)));
        let no_pts = gst::Buffer::new();

        assert!(!audio_buffer_is_after_cutoff(at_cutoff.as_ref(), cutoff));
        assert!(audio_buffer_is_after_cutoff(after_cutoff.as_ref(), cutoff));
        assert!(!audio_buffer_is_after_cutoff(no_pts.as_ref(), cutoff));
    }

    #[test]
    fn stream_indexes_keep_video_first_and_audio_tracks_separate() {
        let input = segment_input(Some("desktop"), Some("mic"));
        let indexes = stream_indexes(&input);

        assert_eq!(indexes.video_stream_index, 0);
        assert_eq!(indexes.desktop_audio_stream_index, Some(1));
        assert_eq!(indexes.microphone_audio_stream_index, Some(2));
    }

    #[test]
    fn audio_source_names_match_configured_audio_branches() {
        assert!(audio_source_names(&segment_input(None, None)).is_empty());
        assert_eq!(
            audio_source_names(&segment_input(Some("desktop"), None)),
            vec!["desktop_audio_src"]
        );
        assert_eq!(
            audio_source_names(&segment_input(None, Some("mic"))),
            vec!["microphone_audio_src"]
        );
        assert_eq!(
            audio_source_names(&segment_input(Some("desktop"), Some("mic"))),
            vec!["desktop_audio_src", "microphone_audio_src"]
        );
    }

    #[test]
    fn pipeline_uses_low_latency_video_encoder_and_explicit_queues() {
        let mut input = segment_input(Some("desktop"), Some("mic"));
        input.video_bitrate_kbps = 2_500;
        let pipeline = build_pipeline_description(&input);

        assert!(pipeline.contains("tune=zerolatency"));
        assert!(pipeline.contains("x264enc bitrate=2500"));
        assert!(pipeline.contains("rc-lookahead=0"));
        assert!(pipeline.contains("bframes=0"));
        assert!(pipeline.contains("sync-lookahead=0"));
        assert!(pipeline.contains("! videoconvert ! video/x-raw,format=I420 ! x264enc"));
        assert!(!pipeline.contains("caps=video/x-raw,format=I420"));
        assert_eq!(
            pipeline
                .matches("queue max-size-time=0 max-size-bytes=0 max-size-buffers=0")
                .count(),
            6
        );
        assert!(pipeline.contains("pulsesrc name=desktop_audio_src"));
        assert!(pipeline.contains("queue name=desktop_audio_cutoff_queue"));
        assert!(pipeline.contains("pulsesrc name=microphone_audio_src"));
        assert!(pipeline.contains("queue name=microphone_audio_cutoff_queue"));
    }

    #[test]
    fn gst_video_formats_match_raw_video_caps_names() {
        assert_eq!(gst_video_format(CapturedFrameFormat::Yuv420p), "I420");
        assert_eq!(gst_video_format(CapturedFrameFormat::Yuv444p), "Y444");
        assert_eq!(gst_video_format(CapturedFrameFormat::Rgba), "RGBA");
        assert_eq!(gst_video_format(CapturedFrameFormat::Bgra), "BGRA");
    }

    #[test]
    fn writer_frame_prep_keeps_tightly_packed_rgba_without_colorspace_conversion() {
        let width = 5_120;
        let height = 1_440;
        let data = vec![42; width * height * 4];
        let frame = CapturedFrame::new(
            "monitor",
            1,
            width as u32,
            height as u32,
            (width * 4) as u32,
            CapturedFrameFormat::Rgba,
            data.clone(),
        )
        .expect("valid rgba frame");

        assert!(
            frame_is_tightly_packed_for_gstreamer(&frame).expect("tight frame check"),
            "full-canvas RGBA frames should be handed to GStreamer without Rust conversion"
        );
        let started = std::time::Instant::now();
        let prepared = prepare_video_frame_for_gstreamer(frame).expect("prepare frame");

        assert_eq!(prepared.format, CapturedFrameFormat::Rgba);
        assert_eq!(prepared.width, width as u32);
        assert_eq!(prepared.height, height as u32);
        assert_eq!(prepared.data.len(), data.len());
        assert!(
            started.elapsed() < Duration::from_millis(50),
            "tight RGBA prep should be a move, not a per-pixel conversion"
        );
    }

    #[test]
    fn writer_frame_prep_repacks_stride_without_changing_format() {
        let frame = CapturedFrame::new(
            "monitor",
            1,
            2,
            2,
            12,
            CapturedFrameFormat::Rgba,
            vec![
                1, 2, 3, 4, 5, 6, 7, 8, 99, 99, 99, 99, 9, 10, 11, 12, 13, 14, 15, 16, 88, 88, 88,
                88,
            ],
        )
        .expect("valid padded rgba frame");

        assert!(!frame_is_tightly_packed_for_gstreamer(&frame).expect("tight frame check"));
        let prepared = prepare_video_frame_for_gstreamer(frame).expect("prepare padded frame");

        assert_eq!(prepared.format, CapturedFrameFormat::Rgba);
        assert_eq!(
            prepared.data,
            vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]
        );
    }

    #[test]
    fn video_only_segment_does_not_require_audio_plugins() {
        let capabilities = video_only_capabilities();
        let input = segment_input(None, None);

        validate_segment_capabilities(&capabilities, &input)
            .expect("video-only segment should not require optional audio elements");
    }

    #[test]
    fn audio_segment_requires_audio_plugins() {
        let capabilities = video_only_capabilities();
        let input = segment_input(Some("desktop"), None);

        let error = validate_segment_capabilities(&capabilities, &input)
            .expect_err("audio segment should require audio elements");

        assert_eq!(error.code, "recorder_media_writer_unavailable");
        assert!(error.message.contains("pulsesrc"));
        assert!(error.message.contains("opusenc"));
        assert!(error.message.contains("gstreamer1.0-pulseaudio"));
        assert!(error.message.contains("gstreamer1-plugins-good"));
    }

    #[test]
    fn synthetic_segment_stop_finalizes_non_empty_mkv() {
        let output_path =
            std::env::temp_dir().join(format!("daydream-media-writer-{}.mkv", Uuid::now_v7()));
        let mut input = segment_input(None, None);
        input.output_path = output_path.clone();

        let segment = start_synthetic_segment(&input).expect("start synthetic media segment");
        thread::sleep(Duration::from_millis(150));
        segment
            .stop(input.started_at_ms.saturating_add(150))
            .expect("stop synthetic media segment");

        let metadata = fs::metadata(&output_path).expect("read finalized media segment metadata");
        assert!(metadata.len() > 0);

        let probe = probe_segment_best_effort(&output_path);
        assert!(probe.available, "{probe:?}");
        assert_eq!(
            probe
                .video
                .as_ref()
                .and_then(|video| video.codec_name.as_deref()),
            Some("video/x-h264")
        );

        let _ = fs::remove_file(output_path);
    }

    #[test]
    fn live_frame_segment_stop_drains_queued_frame_before_eos() {
        let output_path =
            std::env::temp_dir().join(format!("daydream-live-media-writer-{}.mkv", Uuid::now_v7()));
        let mut input = segment_input(None, None);
        input.output_path = output_path.clone();
        let (sender, receiver) = bounded_frame_channel(3);
        let frame = CapturedFrame::new(
            "portal-monitor-a",
            1,
            input.canvas_width,
            input.canvas_height,
            input.canvas_width,
            CapturedFrameFormat::Yuv420p,
            black_yuv420p_frame(input.canvas_width, input.canvas_height).expect("black yuv frame"),
        )
        .expect("captured frame");
        let segment =
            start_segment_from_frames(&input, receiver, None).expect("start live media segment");

        sender.send(frame).expect("send captured frame");
        drop(sender);
        segment
            .stop(input.started_at_ms.saturating_add(1))
            .expect("stop live media segment");

        let metadata = fs::metadata(&output_path).expect("read finalized media segment metadata");
        assert!(metadata.len() > 0);

        let probe = probe_segment_best_effort(&output_path);
        assert!(probe.available, "{probe:?}");
        assert_eq!(
            probe
                .video
                .as_ref()
                .and_then(|video| video.codec_name.as_deref()),
            Some("video/x-h264")
        );

        let _ = fs::remove_file(output_path);
    }

    #[test]
    fn segment_stop_returns_error_when_frame_pump_does_not_exit() {
        let output_path = std::env::temp_dir().join(format!(
            "daydream-stuck-media-writer-{}.mkv",
            Uuid::now_v7()
        ));
        let mut input = segment_input(None, None);
        input.output_path = output_path.clone();
        let (release_sender, release_receiver) = mpsc::channel();
        let segment = start_segment_with_frame_pump(&input, false, None, move |_appsrc, _stop| {
            thread::spawn(move || {
                let _ = release_receiver.recv();
                Ok(())
            })
        })
        .expect("start media segment");

        let error = segment
            .stop(input.started_at_ms)
            .expect_err("stuck frame pump should fail stop");
        let _ = release_sender.send(());

        assert_eq!(error.code, "recorder_media_writer_frame_write_failed");
        assert!(error.message.contains("Timed out"));

        let _ = fs::remove_file(output_path);
    }

    fn segment_input(desktop: Option<&str>, mic: Option<&str>) -> MediaSegmentInput {
        MediaSegmentInput {
            benchmarks_dir: std::env::temp_dir(),
            canvas_height: 720,
            canvas_width: 1280,
            desktop_audio_source_id: desktop.map(ToString::to_string),
            fps: 1.0,
            microphone_audio_source_id: mic.map(ToString::to_string),
            output_path: "recording.mkv".into(),
            recording_id: "recording-1".to_string(),
            segment_id: "segment-1".to_string(),
            segment_index: 1,
            started_at_ms: 0,
            video_bitrate_kbps: 1_500,
        }
    }

    fn video_only_capabilities() -> RecorderMediaWriterCapabilities {
        RecorderMediaWriterCapabilities {
            appsrc: true,
            audio_convert: false,
            audio_resample: false,
            file_sink: true,
            h264_parse: true,
            matroska_muxer: true,
            opus_encoder: false,
            pulse_source: false,
            queue: true,
            video_convert: true,
            x264_encoder: true,
        }
    }
}
