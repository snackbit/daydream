use std::{sync::mpsc, thread, time::Duration};

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
    types::MediaSegmentInput,
};

const EOS_TIMEOUT: Duration = Duration::from_secs(10);
#[cfg(not(test))]
const FRAME_PUMP_STOP_TIMEOUT: Duration = Duration::from_secs(5);
#[cfg(test)]
const FRAME_PUMP_STOP_TIMEOUT: Duration = Duration::from_millis(100);

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
    #[cfg(test)]
    pub(super) fn for_supervision_test(
        pipeline: gst::Pipeline,
        bus: gst::Bus,
        appsrc: gst_app::AppSrc,
        result: mpsc::Receiver<Result<(), String>>,
    ) -> Self {
        Self {
            appsrc,
            audio_branches: vec![],
            bus,
            frame_pump: None,
            frame_pump_result: result,
            pipeline,
            benchmark: None,
            started_at_ms: 0,
            stop_frames: None,
        }
    }
    pub(crate) fn abort(mut self) {
        if let Some(stop) = self.stop_frames.take() {
            let _ = stop.send(FramePumpStop {
                stop_at_ms: self.started_at_ms,
            });
        }
        let _ = self.pipeline.set_state(gst::State::Null);
        if self
            .frame_pump_result
            .recv_timeout(FRAME_PUMP_STOP_TIMEOUT)
            .is_ok()
        {
            if let Some(worker) = self.frame_pump.take() {
                let _ = worker.join();
            }
        }
    }
    pub(crate) fn poll_error(&mut self) -> Option<RecorderError> {
        while let Some(message) = self.bus.pop() {
            match message.view() {
                gst::MessageView::Error(_) => return Some(dependency_error("recorder_media_writer_failed", "The media writer stopped after a recording error. Check audio devices and available storage.")),
                gst::MessageView::Eos(_) => return Some(dependency_error("recorder_media_writer_failed", "The media writer ended unexpectedly.")),
                _ => (),
            }
        }
        match self.frame_pump_result.try_recv() {
            Ok(_) | Err(mpsc::TryRecvError::Disconnected) => Some(dependency_error(
                "recorder_media_writer_frame_write_failed",
                "The video frame writer stopped unexpectedly.",
            )),
            Err(mpsc::TryRecvError::Empty) => None,
        }
    }
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

pub(super) fn start_segment_with_frame_pump(
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
