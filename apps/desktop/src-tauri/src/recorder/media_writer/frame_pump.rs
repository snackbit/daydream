use std::{sync::mpsc, time::Duration};

use gstreamer as gst;
use gstreamer_app as gst_app;
use tracing::info;

#[cfg(test)]
use super::super::capture_runtime::CapturedFrameFormat;
#[cfg(test)]
use super::video_frame::black_yuv420p_frame;
use super::{
    super::{
        benchmark::{RecorderBenchmarkSession, RecorderBenchmarkStage},
        capture_runtime::{
            record_frame_sent_to_writer, CapturedFrame, FrameReceiver, SharedCaptureTiming,
        },
    },
    video_frame::{prepare_video_frame_for_gstreamer, video_caps, PreparedVideoFrame},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct FramePumpStop {
    pub(super) stop_at_ms: u64,
}

#[cfg(test)]
pub(super) fn pump_synthetic_frames(
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

pub(super) fn pump_composed_frames(
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

pub(super) fn frame_is_after_stop(frame_timestamp_ms: u64, stop_at_ms: u64) -> bool {
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

pub(super) fn live_frame_pts(
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

pub(super) fn frame_interval(fps: f64) -> Duration {
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
