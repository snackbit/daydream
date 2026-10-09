use std::{
    cell::RefCell,
    collections::HashMap,
    os::fd::{AsFd, BorrowedFd, OwnedFd},
    rc::Rc,
    sync::{mpsc, Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use pipewire as pw;
use pw::{properties::properties, spa};
use tauri::{AppHandle, Emitter};
use tracing::{debug, info, warn};

use super::portal::{close_portal_session_blocking, LivePortalSession, LivePortalStream};
use crate::recorder::{
    benchmark::{RecorderBenchmarkSession, RecorderBenchmarkStage},
    capture_runtime::{
        new_capture_timing, record_capture_frame, record_compose_duration, record_composed_frame,
        record_dmabuf_frame_read_back_to_cpu, record_scheduler_tick, CapturePreviewSession,
        CaptureRuntimeSource, CaptureRuntimeSourceHandle, CaptureSession, CapturedDmaBufFrame,
        CapturedDmaBufFrameInput, CapturedFrame, CapturedFrameFormat, CapturedFrameStorage,
        FrameSender, FrameTrySendError, RuntimeWorker, SharedCaptureTiming,
    },
    commands::RECORDER_PREVIEW_FRAME_EVENT,
    compositor::{SceneCompositorInput, SceneCompositorPlan},
    errors::{
        capture_adapter_failed_error, capture_format_unsupported_error,
        capture_source_missing_error,
    },
    manager::{now_ms, RecordingSegmentRuntime},
    media_writer::{self, MediaSegmentInput},
    platform::linux::gpu_compositor::LinuxGpuFrameCompositor,
    preview::encode_cropped_preview_frame,
    types::{
        CaptureFrameStorageKind, CaptureSourceKind, RecorderError, RecorderPreviewFrame,
        RecorderScene,
    },
};

mod frame_convert;
mod intake;
mod preflight;
mod preview;
mod sampler;
mod scheduler;
mod session;
use frame_convert::*;
use intake::*;
#[cfg(test)]
pub(crate) use preflight::preflight_live_capture;
pub(crate) use preflight::{
    prepared_live_capture, start_live_segment, start_preview_session, LiveCapturePreflight,
    PreparedLiveCapture,
};
use preview::*;
use sampler::*;
use scheduler::*;
pub(crate) use session::RunningPreviewSession;
use session::*;

const PIPEWIRE_READY_TIMEOUT: Duration = Duration::from_secs(5);
const PIPEWIRE_LATEST_FRAME_LOG_INTERVAL: Duration = Duration::from_secs(5);

fn raw_video_params(node_id: u32, fps: f64) -> Result<Vec<u8>, RecorderError> {
    let framerate = pipewire_framerate_fraction(fps);
    let obj = pw::spa::pod::object!(
        pw::spa::utils::SpaTypes::ObjectParamFormat,
        pw::spa::param::ParamType::EnumFormat,
        pw::spa::pod::property!(
            pw::spa::param::format::FormatProperties::MediaType,
            Id,
            pw::spa::param::format::MediaType::Video
        ),
        pw::spa::pod::property!(
            pw::spa::param::format::FormatProperties::MediaSubtype,
            Id,
            pw::spa::param::format::MediaSubtype::Raw
        ),
        pw::spa::pod::property!(
            pw::spa::param::format::FormatProperties::VideoFormat,
            Choice,
            Enum,
            Id,
            pw::spa::param::video::VideoFormat::RGBx,
            pw::spa::param::video::VideoFormat::RGBx,
            pw::spa::param::video::VideoFormat::RGBA,
            pw::spa::param::video::VideoFormat::xRGB,
            pw::spa::param::video::VideoFormat::ARGB,
            pw::spa::param::video::VideoFormat::BGRx,
            pw::spa::param::video::VideoFormat::BGRA,
            pw::spa::param::video::VideoFormat::I420,
            pw::spa::param::video::VideoFormat::Y444,
        ),
        pw::spa::pod::property!(
            pw::spa::param::format::FormatProperties::VideoSize,
            Choice,
            Range,
            Rectangle,
            pw::spa::utils::Rectangle {
                width: 1920,
                height: 1080
            },
            pw::spa::utils::Rectangle {
                width: 1,
                height: 1
            },
            pw::spa::utils::Rectangle {
                width: 8192,
                height: 8192
            }
        ),
        pw::spa::pod::property!(
            pw::spa::param::format::FormatProperties::VideoFramerate,
            Choice,
            Range,
            Fraction,
            framerate,
            pw::spa::utils::Fraction { num: 0, denom: 1 },
            pw::spa::utils::Fraction { num: 60, denom: 1 }
        ),
    );

    pw::spa::pod::serialize::PodSerializer::serialize(
        std::io::Cursor::new(Vec::new()),
        &pw::spa::pod::Value::Object(obj),
    )
    .map(|serialized| serialized.0.into_inner())
    .map_err(|error| {
        capture_adapter_failed_error(format!(
            "Failed to build PipeWire raw video format parameters for node {node_id}: {error}."
        ))
    })
}

fn pipewire_framerate_fraction(fps: f64) -> pw::spa::utils::Fraction {
    if !fps.is_finite() || fps <= 0.0 {
        return pw::spa::utils::Fraction { num: 1, denom: 1 };
    }
    if fps.fract() == 0.0 {
        return pw::spa::utils::Fraction {
            num: fps.round().clamp(1.0, 60.0) as u32,
            denom: 1,
        };
    }
    if (fps - 0.5).abs() < f64::EPSILON {
        return pw::spa::utils::Fraction { num: 1, denom: 2 };
    }
    if (fps - 0.2).abs() < f64::EPSILON {
        return pw::spa::utils::Fraction { num: 1, denom: 5 };
    }
    if (fps - 0.1).abs() < f64::EPSILON {
        return pw::spa::utils::Fraction { num: 1, denom: 10 };
    }

    pw::spa::utils::Fraction {
        num: (fps * 1000.0).round().clamp(1.0, 60_000.0) as u32,
        denom: 1000,
    }
}

fn pipewire_error(error: pw::Error) -> RecorderError {
    capture_adapter_failed_error(format!("PipeWire capture adapter failed: {error}."))
}

#[cfg(test)]
mod tests;
