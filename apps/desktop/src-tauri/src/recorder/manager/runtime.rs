use super::super::{
    capture_runtime::CapturePreviewSession,
    types::{CaptureSource, RecorderPreviewStatus, RecorderStatus},
};
use super::ActiveRecording;

#[derive(Debug)]
pub(super) struct RecorderRuntime {
    pub(super) active: Option<ActiveRecording>,
    pub(super) capture_sources: Vec<CaptureSource>,
    pub(super) preview: Option<ActivePreview>,
    pub(super) preview_start_generation: u64,
    pub(super) preview_status: RecorderPreviewStatus,
    pub(super) status: RecorderStatus,
    pub(super) pause_owner: Option<PauseOwner>,
    pub(super) protection_monitor: Option<super::super::window_protection::WindowProtectionMonitor>,
    pub(super) pending_failure: Option<super::super::types::RecorderError>,
    pub(super) automatic_resume_pending: bool,
    pub(super) cancel_pending_resume: bool,
    pub(super) pause_generation: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PauseOwner {
    Manual,
    Automatic,
}

#[derive(Debug)]
pub(super) struct ActivePreview {
    pub(super) scene_id: String,
    pub(super) session: Box<dyn CapturePreviewSession>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum PreviewStartDecision {
    AlreadyCurrent(RecorderPreviewStatus),
    Pending {
        generation: u64,
        status: RecorderPreviewStatus,
    },
}
