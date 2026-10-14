use super::BackgroundTaskCursor;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingProcessingLane {
    pub kind: String,
    pub applicability: String,
    pub status: Option<String>,
    pub job_ids: Vec<String>,
    pub total: i64,
    pub completed: i64,
    pub failed: i64,
    pub skipped: i64,
    pub cancelled: i64,
    pub running: i64,
    pub queued: i64,
    pub progress: Option<f64>,
    pub input_open: bool,
    pub error_code: Option<String>,
    pub next_retry_at_ms: Option<i64>,
    pub activity: Vec<ProcessingLaneActivity>,
    pub eta_ms: Option<i64>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessingLaneActivity {
    pub chunk_id: String,
    pub job_id: String,
    pub source_track_id: Option<String>,
    pub source_label: Option<String>,
    pub start_offset_ms: Option<i64>,
    pub end_offset_ms: Option<i64>,
    pub status: String,
    pub phase: String,
    pub started_at_ms: Option<i64>,
    pub phase_started_at_ms: Option<i64>,
    pub last_progress_at_ms: Option<i64>,
    pub next_retry_at_ms: Option<i64>,
    pub waiting_reason: Option<String>,
    pub worker_active: Option<bool>,
    pub total_units: Option<i64>,
    pub completed_units: Option<i64>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingIndexCoverage {
    pub applicability: String,
    pub indexed: i64,
    pub total: i64,
    pub pending: i64,
    pub status: Option<String>,
    pub error_code: Option<String>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingProcessingGroup {
    pub recording_id: String,
    pub title: String,
    pub started_at_ms: Option<i64>,
    pub updated_at_ms: i64,
    pub status: String,
    pub input_open: bool,
    pub eta_ms: Option<i64>,
    pub job_ids: Vec<String>,
    pub lanes: Vec<RecordingProcessingLane>,
    pub index_coverage: RecordingIndexCoverage,
}
#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListRecordingProcessingGroupsInput {
    pub recording_id: Option<String>,
    #[serde(default)]
    pub recording_ids: Vec<String>,
    pub filter: Option<String>,
    pub limit: Option<usize>,
    pub cursor: Option<BackgroundTaskCursor>,
    #[serde(default)]
    pub known_recording_ids: Vec<String>,
}
#[derive(Default, Serialize)]
pub struct RecordingProcessingCounts {
    pub all: usize,
    pub active: usize,
    pub attention: usize,
    pub history: usize,
    pub running: usize,
    pub queued: usize,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingProcessingGroupPage {
    pub groups: Vec<RecordingProcessingGroup>,
    pub next_cursor: Option<BackgroundTaskCursor>,
    pub counts: RecordingProcessingCounts,
    pub reconciled_groups: Vec<RecordingProcessingGroup>,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingProcessingUnitCursor {
    pub chunk_id: String,
    pub unit_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetRecordingProcessingLaneInput {
    pub recording_id: String,
    pub source_kind: String,
    pub limit: Option<usize>,
    pub cursor: Option<RecordingProcessingUnitCursor>,
    #[serde(default)]
    pub history: bool,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingProcessingUnit {
    #[serde(flatten)]
    pub unit: super::BackgroundTaskUnit,
    pub job_id: String,
    pub source_track_id: Option<String>,
    pub source_role: Option<String>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingProcessingLaneDetail {
    pub lane: RecordingProcessingLane,
    pub units: Vec<RecordingProcessingUnit>,
    pub next_cursor: Option<RecordingProcessingUnitCursor>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ControlRecordingProcessingGroupsInput {
    pub recording_id: String,
    pub action: String,
    pub source_kind: Option<String>,
    #[serde(default)]
    pub use_current_settings: bool,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingProcessingJobActionResult {
    pub job_id: String,
    pub succeeded: bool,
    pub error_code: Option<String>,
}
#[derive(Serialize)]
pub struct RecordingProcessingActionResult {
    pub results: Vec<RecordingProcessingJobActionResult>,
}
