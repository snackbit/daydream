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
    pub(in crate::ai) frames: Vec<ExtractedVisualFrame>,
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

#[derive(Clone, Debug)]
pub(in crate::ai) struct VisionRequestConfig {
    pub(in crate::ai) api_key: String,
    pub(in crate::ai) base_url: String,
    pub(in crate::ai) model: String,
    pub(in crate::ai) provider: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct VlmChatResponse {
    pub(super) choices: Vec<VlmChatChoice>,
}

#[derive(Debug, Deserialize)]
pub(super) struct VlmChatChoice {
    pub(super) message: VlmChatMessage,
}

#[derive(Debug, Deserialize)]
pub(super) struct VlmChatMessage {
    pub(super) content: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::ai) struct VlmObservationPayload {
    pub(in crate::ai) observations: Vec<VlmObservationCandidate>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::ai) struct VlmObservationCandidate {
    pub(in crate::ai) confidence: Option<f64>,
    pub(in crate::ai) frame_timestamp_ms: i64,
    pub(in crate::ai) kind: String,
    pub(in crate::ai) region: VlmObservationRegion,
    pub(in crate::ai) text: String,
}

#[derive(Debug, Deserialize)]
pub(in crate::ai) struct VlmObservationRegion {
    pub(in crate::ai) height: i64,
    pub(in crate::ai) width: i64,
    pub(in crate::ai) x: i64,
    pub(in crate::ai) y: i64,
}
