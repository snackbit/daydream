use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecallCitation {
    pub evidence_id: String,
    pub recording_id: String,
    pub segment_id: String,
    pub source_kind: String,
    pub source_track_id: String,
    pub source_id: String,
    pub source_frame_ref: Option<String>,
    pub document_id: Option<String>,
    pub start_offset_ms: i64,
    pub end_offset_ms: i64,
    pub absolute_start_ms: i64,
    pub absolute_end_ms: i64,
    pub content_revision: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecallEvidence {
    #[serde(flatten)]
    pub citation: RecallCitation,
    pub text: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecallSearchHit {
    pub evidence: RecallEvidence,
    pub score: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecallSearchInput {
    pub query: String,
    pub day_id: Option<String>,
    pub limit: usize,
    pub timezone: Option<String>,
    #[serde(default)]
    pub source_kinds: Vec<String>,
    pub from_ms: Option<i64>,
    pub to_ms: Option<i64>,
    pub cursor: Option<String>,
    pub mode: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecallSearchResult {
    pub privacy_revision: i64,
    pub hits: Vec<RecallSearchHit>,
    pub complete: bool,
    pub unavailable_reason: Option<String>,
    pub next_cursor: Option<String>,
    pub semantic_used: bool,
    pub indexed_evidence_count: usize,
    pub eligible_evidence_count: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecallDayCard {
    pub privacy_revision: i64,
    pub day_id: String,
    pub start_ms: i64,
    pub end_ms: i64,
    pub timezone: String,
    pub recording_count: usize,
    pub evidence_count: usize,
    pub summary: Option<String>,
    pub reviewed: bool,
    pub status: String,
    pub input_fingerprint: String,
    pub captured_duration_ms: i64,
    pub thumbnail_path: Option<String>,
    #[serde(default)]
    pub thumbnail_recording_id: Option<String>,
    pub redaction_count: usize,
    pub recording_active: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecallSession {
    pub id: String,
    pub title: String,
    pub category: String,
    pub title_correction: Option<String>,
    pub category_correction: Option<String>,
    pub summary: String,
    pub start_ms: i64,
    pub end_ms: i64,
    pub citations: Vec<RecallCitation>,
    pub correction: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecallSuggestion {
    pub id: String,
    pub title: String,
    pub description: String,
    pub citations: Vec<RecallCitation>,
    pub accepted_task_id: Option<String>,
    pub dismissed: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecallTask {
    pub id: String,
    pub day_id: String,
    pub title: String,
    pub notes: String,
    pub status: String,
    pub citations: Vec<RecallCitation>,
    pub source_withheld: bool,
    pub updated_at_ms: i64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecallFeedback {
    pub target_id: String,
    pub rating: Option<String>,
    pub comment: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecallDay {
    #[serde(default)]
    pub evidence: Vec<RecallEvidence>,
    pub privacy_revision: i64,
    pub day: RecallDayCard,
    #[serde(default)]
    pub recordings: Vec<RecallDayRecording>,
    pub sessions: Vec<RecallSession>,
    pub suggestions: Vec<RecallSuggestion>,
    pub tasks: Vec<RecallTask>,
    pub feedback: Vec<RecallFeedback>,
    pub coverage: Vec<RecallCoverageInterval>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecallDayRecording {
    pub recording_id: String,
    pub title: Option<String>,
    pub start_ms: i64,
    pub end_ms: Option<i64>,
    pub start_offset_ms: Option<i64>,
    pub duration_ms: Option<i64>,
    pub availability: String,
    pub processing_status: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecallCoverageInterval {
    pub source_track_ids: Vec<String>,
    pub recording_id: String,
    pub segment_id: Option<String>,
    pub start_ms: i64,
    pub end_ms: i64,
    pub kind: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListRecallDaysInput {
    pub before_day_id: Option<String>,
    pub limit: usize,
    pub timezone: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecallDayInput {
    pub day_id: String,
    pub timezone: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolveRecallEvidenceInput {
    pub evidence_id: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetRecallFeedbackInput {
    pub day_id: String,
    pub target_id: String,
    pub rating: Option<String>,
    pub comment: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetRecallSessionCorrectionInput {
    pub day_id: String,
    pub session_id: String,
    pub correction: Option<String>,
    pub title_correction: Option<String>,
    pub category_correction: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DismissRecallSuggestionInput {
    pub day_id: String,
    pub suggestion_id: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcceptRecallSuggestionInput {
    pub day_id: String,
    pub suggestion_id: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRecallTaskInput {
    pub task_id: String,
    pub title: String,
    pub notes: String,
    pub status: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetRecallDayReviewedInput {
    pub day_id: String,
    pub reviewed: bool,
    pub input_fingerprint: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecallJob {
    pub id: String,
    pub day_id: Option<String>,
    pub kind: String,
    pub status: String,
    pub progress: f64,
    pub error_code: Option<String>,
    pub updated_at_ms: i64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ControlRecallJobInput {
    pub job_id: String,
    pub action: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecallStatus {
    pub privacy_revision: i64,
    pub indexed_evidence_count: usize,
    pub pending_evidence_count: usize,
    pub embedding_ready: bool,
    pub generation_ready: bool,
    pub embedding_unavailable_reason: Option<String>,
    pub generation_unavailable_reason: Option<String>,
    pub jobs: Vec<RecallJob>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecallStateChanged {
    pub day_id: Option<String>,
    pub privacy_revision: i64,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestRecallProviderInput {
    pub kind: String,
    pub settings: crate::settings::RecallSettings,
    pub key_draft: Option<String>,
}
impl std::fmt::Debug for TestRecallProviderInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("TestRecallProviderInput([redacted])")
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecallProviderTestResult {
    pub kind: String,
    pub status: String,
    pub error_code: Option<String>,
    pub elapsed_ms: u64,
}
