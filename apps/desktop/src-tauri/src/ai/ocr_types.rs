use super::types::ProcessingJob;
use crate::storage::OcrBlockInput;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OcrHistoryEntry {
    pub id: String,
    pub recording_id: String,
    pub segment_id: String,
    pub source_track_id: String,
    pub frame_timestamp_ms: i64,
    pub source_frame_ref: String,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub engine: String,
    pub provider: String,
    pub model: Option<String>,
    pub config_fingerprint: String,
    pub text: String,
    pub provenance: String,
    pub status: String,
    pub error_code: Option<String>,
    pub observed_start_ms: i64,
    pub observed_end_ms: i64,
    pub occurrence_count: usize,
    pub support_timestamps_ms: Vec<i64>,
    pub cache_hit: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OcrDocument {
    #[serde(flatten)]
    pub entry: OcrHistoryEntry,
    pub content_id: Option<String>,
    pub blocks: Vec<OcrBlockInput>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetOcrDocumentInput {
    pub recording_id: String,
    pub source_track_id: String,
    pub timestamp_ms: i64,
    pub document_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OcrDocumentSelection {
    pub recording_id: String,
    pub source_track_id: String,
    pub document: Option<OcrDocument>,
    pub job: Option<ProcessingJob>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OcrHistoryCursor {
    pub timestamp_ms: i64,
    pub id: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetOcrHistoryInput {
    pub recording_id: String,
    pub source_track_id: Option<String>,
    pub limit: Option<usize>,
    pub cursor: Option<OcrHistoryCursor>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OcrHistoryPage {
    pub items: Vec<OcrHistoryEntry>,
    pub next_cursor: Option<OcrHistoryCursor>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReprocessRecordingOcrInput {
    pub recording_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OcrTimelineFrame {
    pub document_id: String,
    pub error_code: Option<String>,
    pub frame_timestamp_ms: i64,
    pub id: String,
    pub ocr_output_count: i64,
    pub recording_id: String,
    pub segment_id: String,
    pub source_frame_ref: String,
    pub source_track_id: String,
    pub status: String,
    pub sample_end_ms: i64,
    pub text_empty: Option<bool>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetOcrTimelineFramesInput {
    pub recording_id: String,
    pub source_track_id: Option<String>,
    pub start_timestamp_ms: Option<i64>,
    pub end_timestamp_ms: Option<i64>,
    pub limit: Option<usize>,
    pub cursor: Option<OcrHistoryCursor>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OcrTimelineFramePage {
    pub recording_id: String,
    pub frames: Vec<OcrTimelineFrame>,
    pub next_cursor: Option<OcrHistoryCursor>,
}
