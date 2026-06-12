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
