use std::fmt::Debug;

use super::types::RecorderError;

mod channel;
mod cpu_frame;
mod dmabuf_frame;
mod formats;
mod sources;
#[cfg(test)]
mod tests;
mod timing;
mod worker;

pub(crate) use channel::{bounded_frame_channel, FrameReceiver, FrameSender, FrameTrySendError};
pub(crate) use cpu_frame::CapturedFrame;
pub(crate) use dmabuf_frame::{CapturedDmaBufFrame, CapturedDmaBufFrameInput};
pub(crate) use formats::{CapturedFrameFormat, CapturedFrameStorage, WriterFrameHandoffPlan};
pub(crate) use sources::{CaptureRuntimeSource, CaptureRuntimeSourceHandle};
pub(crate) use timing::{
    capture_timing_snapshot, new_capture_timing, record_capture_frame, record_compose_duration,
    record_composed_frame, record_dmabuf_frame_read_back_to_cpu, record_frame_sent_to_writer,
    record_scheduler_tick, SharedCaptureTiming,
};
pub(crate) use worker::RuntimeWorker;

pub(crate) trait CaptureSession: Send + Debug {
    fn stop(self: Box<Self>, stop_at_ms: u64) -> Result<(), RecorderError>;
}

pub(crate) trait CapturePreviewSession: Send + Debug {
    fn stop(self: Box<Self>) -> Result<(), RecorderError>;
}
