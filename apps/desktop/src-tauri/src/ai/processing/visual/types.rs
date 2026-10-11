use std::path::{Path, PathBuf};

use serde::Deserialize;

use super::super::super::*;
use super::super::TempProcessingDirectory;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::ai) struct VisualContextFrame {
    pub(in crate::ai) frame_timestamp_ms: i64,
    pub(in crate::ai) path: PathBuf,
    pub(in crate::ai) source_frame_ref: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::ai) struct ExtractedVisualFrame {
    pub(in crate::ai) abrupt_transition: bool,
    pub(in crate::ai) after_context: Option<VisualContextFrame>,
    pub(in crate::ai) before_context: Option<VisualContextFrame>,
    pub(in crate::ai) frame_timestamp_ms: i64,
    pub(in crate::ai) full_checkpoint: bool,
    pub(in crate::ai) height: i64,
    pub(in crate::ai) path: PathBuf,
    pub(in crate::ai) region: VisualRegion,
    pub(in crate::ai) signature: String,
    pub(in crate::ai) source_frame_ref: String,
    pub(in crate::ai) width: i64,
}

pub(in crate::ai) struct ExtractedVisualFrameSet {
    pub(in crate::ai) ocr_frames: Vec<ExtractedVisualFrame>,
    pub(in crate::ai) vlm_frames: Vec<ExtractedVisualFrame>,
    pub(in crate::ai) _temp_dir: TempProcessingDirectory,
}

impl ExtractedVisualFrame {
    pub(in crate::ai) fn path(&self) -> &Path {
        &self.path
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::ai) struct VisualImage {
    pub(in crate::ai) height: usize,
    pub(in crate::ai) pixels: Vec<u8>,
    pub(in crate::ai) width: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::ai) struct OcrCandidate {
    pub(in crate::ai) confidence: Option<f64>,
    pub(in crate::ai) region: VisualRegion,
    pub(in crate::ai) text: String,
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::ai) struct PendingVlmObservation {
    pub(in crate::ai) confidence: Option<f64>,
    pub(in crate::ai) frame_timestamp_ms: i64,
    pub(in crate::ai) observation_kind: String,
    pub(in crate::ai) region: VisualRegion,
    pub(in crate::ai) source_frame_ref: String,
    pub(in crate::ai) text: String,
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::ai) struct PreviousVisualObservation {
    pub(in crate::ai) frame_timestamp_ms: i64,
    pub(in crate::ai) kind: String,
    pub(in crate::ai) text: String,
}

#[derive(Clone, Default)]
pub(in crate::ai) struct VisionRequestConfig {
    pub(in crate::ai) route: Option<crate::model_registry::types::Route>,
    pub(in crate::ai) storage: Option<crate::storage::AppStorage>,
    pub(in crate::ai) api_key: String,
    pub(in crate::ai) base_url: String,
    pub(in crate::ai) model: String,
    pub(in crate::ai) provider: String,
}
impl std::fmt::Debug for VisionRequestConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VisionRequestConfig")
            .field("provider", &self.provider)
            .field("model", &self.model)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Deserialize)]
pub(super) struct VlmChatCompletionResponse {
    pub(super) choices: Vec<VlmChatCompletionChoice>,
}

#[derive(Debug, Deserialize)]
pub(super) struct VlmChatCompletionChoice {
    pub(super) message: VlmChatCompletionMessage,
}

#[derive(Debug, Deserialize)]
pub(super) struct VlmChatCompletionMessage {
    pub(super) content: Option<String>,
}
