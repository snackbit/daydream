use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub enum ReprocessingOperation {
    Transcription,
    Ocr,
    ScreenObservations,
}
impl ReprocessingOperation {
    pub(super) fn kind(&self) -> &'static str {
        match self {
            Self::Transcription => "audio",
            Self::Ocr => "ocr",
            Self::ScreenObservations => "vlm",
        }
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReprocessingRange {
    pub start_offset_ms: i64,
    pub end_offset_ms: i64,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReprocessingTarget {
    pub track_id: String,
    pub ranges: Option<Vec<ReprocessingRange>>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreviewRecordingReprocessingInput {
    pub recording_id: String,
    pub targets: Vec<ReprocessingTarget>,
    pub operations: Option<Vec<ReprocessingOperation>>,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StartRecordingReprocessingInput {
    pub recording_id: String,
    pub targets: Vec<ReprocessingTarget>,
    pub operations: Option<Vec<ReprocessingOperation>>,
    pub plan_fingerprint: String,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReprocessingOperationInfo {
    pub operation: ReprocessingOperation,
    pub enabled: bool,
    pub remote: bool,
    pub provider_label: String,
    pub blocked_reason: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecordingReprocessingScope {
    pub id: String,
    pub track_id: String,
    pub source_label: String,
    pub operation: ReprocessingOperation,
    pub start_offset_ms: i64,
    pub end_offset_ms: i64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecordingReprocessingProgress {
    #[serde(flatten)]
    pub scope: RecordingReprocessingScope,
    pub job_id: String,
    pub status: String,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingReprocessingPreview {
    pub recording_id: String,
    pub plan_fingerprint: String,
    pub scopes: Vec<RecordingReprocessingScope>,
    pub operations: Vec<ReprocessingOperationInfo>,
    pub blocked_reason: Option<String>,
}
#[derive(Clone, Debug, Serialize)]
pub(super) struct PlannedSection {
    pub segment_id: String,
    pub native_track_id: Option<String>,
    pub role: Option<String>,
    pub source_track_id: String,
    pub label: String,
    pub operation: ReprocessingOperation,
    pub start: i64,
    pub end: i64,
}
pub(super) struct Plan {
    pub preview: RecordingReprocessingPreview,
    pub sections: Vec<PlannedSection>,
}
