#[derive(Clone, Debug, PartialEq)]
pub struct RecordingTrackRow {
    pub codec: Option<String>,
    pub created_at_ms: i64,
    pub id: String,
    pub kind: String,
    pub label: String,
    pub path: Option<String>,
    pub recording_id: String,
    pub source_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UpsertRecordingTrackInput {
    pub codec: Option<String>,
    pub created_at_ms: i64,
    pub id: String,
    pub kind: String,
    pub label: String,
    pub path: Option<String>,
    pub recording_id: String,
    pub source_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SaveAiModelAssetInput {
    pub checksum: Option<String>,
    pub checksum_algorithm: Option<String>,
    pub display_label: String,
    pub downloaded_bytes: i64,
    pub download_status: String,
    pub etag: Option<String>,
    pub error_code: Option<String>,
    pub id: String,
    pub local_path: Option<String>,
    pub provider: String,
    pub self_test_status: String,
    pub size_bytes: Option<i64>,
    pub source_url: Option<String>,
    pub timestamp_ms: i64,
    pub total_bytes: Option<i64>,
    pub verification_status: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AiModelAssetRow {
    pub checksum: Option<String>,
    pub checksum_algorithm: Option<String>,
    pub created_at_ms: i64,
    pub display_label: String,
    pub downloaded_bytes: i64,
    pub download_status: String,
    pub etag: Option<String>,
    pub error_code: Option<String>,
    pub id: String,
    pub local_path: Option<String>,
    pub provider: String,
    pub self_test_status: String,
    pub size_bytes: Option<i64>,
    pub source_url: Option<String>,
    pub total_bytes: Option<i64>,
    pub updated_at_ms: i64,
    pub verification_status: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProcessingJobRow {
    pub created_at_ms: i64,
    pub error: Option<String>,
    pub id: String,
    pub kind: String,
    pub recording_id: String,
    pub status: String,
    pub updated_at_ms: i64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CreateProcessingJobInput {
    pub created_at_ms: i64,
    pub error: Option<String>,
    pub id: String,
    pub kind: String,
    pub recording_id: String,
    pub status: String,
    pub updated_at_ms: i64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CreateProcessingChunkInput {
    pub attempt_count: i64,
    pub created_at_ms: i64,
    pub end_offset_ms: Option<i64>,
    pub error_code: Option<String>,
    pub id: String,
    pub job_id: String,
    pub progress: f64,
    pub recording_id: String,
    pub result_code: Option<String>,
    pub segment_id: Option<String>,
    pub source_kind: String,
    pub start_offset_ms: Option<i64>,
    pub status: String,
    pub track_id: Option<String>,
    pub updated_at_ms: i64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProcessingChunkRow {
    pub attempt_count: i64,
    pub created_at_ms: i64,
    pub end_offset_ms: Option<i64>,
    pub error_code: Option<String>,
    pub id: String,
    pub job_id: String,
    pub progress: f64,
    pub recording_id: String,
    pub result_code: Option<String>,
    pub segment_id: Option<String>,
    pub source_kind: String,
    pub start_offset_ms: Option<i64>,
    pub status: String,
    pub track_id: Option<String>,
    pub updated_at_ms: i64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CreateCaptureEventInput {
    pub id: String,
    pub kind: String,
    pub occurred_at_ms: i64,
    pub offset_ms: i64,
    pub payload_json: String,
    pub recording_id: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CaptureEventRow {
    pub id: String,
    pub kind: String,
    pub occurred_at_ms: i64,
    pub offset_ms: i64,
    pub payload_json: String,
    pub recording_id: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CreateTranscriptSegmentInput {
    pub chunk_id: String,
    pub confidence: Option<f64>,
    pub created_at_ms: i64,
    pub end_offset_ms: i64,
    pub id: String,
    pub job_id: String,
    pub model: Option<String>,
    pub provider: String,
    pub recording_id: String,
    pub segment_id: String,
    pub source_role: String,
    pub start_offset_ms: i64,
    pub text: String,
    pub track_id: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TranscriptSegmentRow {
    pub chunk_id: String,
    pub confidence: Option<f64>,
    pub created_at_ms: i64,
    pub end_offset_ms: i64,
    pub id: String,
    pub job_id: String,
    pub model: Option<String>,
    pub provider: String,
    pub recording_id: String,
    pub segment_id: String,
    pub source_role: String,
    pub start_offset_ms: i64,
    pub text: String,
    pub track_id: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TranscriptWordRow {
    pub end_offset_ms: i64,
    pub id: String,
    pub probability: Option<f64>,
    pub start_offset_ms: i64,
    pub text: String,
    pub transcript_segment_id: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CreateTranscriptWordInput {
    pub end_offset_ms: i64,
    pub id: String,
    pub probability: Option<f64>,
    pub start_offset_ms: i64,
    pub text: String,
    pub transcript_segment_id: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CreateActivityIntervalInput {
    pub chunk_id: Option<String>,
    pub end_offset_ms: i64,
    pub id: String,
    pub job_id: String,
    pub recording_id: String,
    pub regions_json: Option<String>,
    pub score: Option<f64>,
    pub source_kind: String,
    pub start_offset_ms: i64,
    pub state: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ActivityIntervalRow {
    pub chunk_id: Option<String>,
    pub end_offset_ms: i64,
    pub id: String,
    pub job_id: String,
    pub recording_id: String,
    pub regions_json: Option<String>,
    pub score: Option<f64>,
    pub source_kind: String,
    pub start_offset_ms: i64,
    pub state: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CreateVisualRegionInput {
    pub chunk_id: String,
    pub confidence: Option<f64>,
    pub frame_timestamp_ms: i64,
    pub id: String,
    pub job_id: String,
    pub model: Option<String>,
    pub observation_kind: Option<String>,
    pub prompt_version: Option<String>,
    pub provider: String,
    pub recording_id: String,
    pub region_height: i64,
    pub region_width: i64,
    pub region_x: i64,
    pub region_y: i64,
    pub source_frame_ref: String,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OcrRegionRow {
    pub chunk_id: String,
    pub confidence: Option<f64>,
    pub frame_timestamp_ms: i64,
    pub id: String,
    pub job_id: String,
    pub model: Option<String>,
    pub provider: String,
    pub recording_id: String,
    pub region_height: i64,
    pub region_width: i64,
    pub region_x: i64,
    pub region_y: i64,
    pub source_frame_ref: String,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VlmObservationRow {
    pub chunk_id: String,
    pub confidence: Option<f64>,
    pub frame_timestamp_ms: i64,
    pub id: String,
    pub job_id: String,
    pub model: Option<String>,
    pub observation_kind: String,
    pub prompt_version: String,
    pub provider: String,
    pub recording_id: String,
    pub region_height: i64,
    pub region_width: i64,
    pub region_x: i64,
    pub region_y: i64,
    pub source_frame_ref: String,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CreateFrameSimilarityEntryInput {
    pub chunk_id: Option<String>,
    pub id: String,
    pub job_id: String,
    pub last_processed_timestamp_ms: i64,
    pub recording_id: String,
    pub region_height: i64,
    pub region_width: i64,
    pub region_x: i64,
    pub region_y: i64,
    pub signature: String,
    pub source_frame_ref: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CreateVisualFrameProcessingStateInput {
    pub chunk_id: String,
    pub created_at_ms: i64,
    pub error_code: Option<String>,
    pub frame_timestamp_ms: i64,
    pub id: String,
    pub job_id: String,
    pub output_count: i64,
    pub progress: f64,
    pub recording_id: String,
    pub region_height: i64,
    pub region_width: i64,
    pub region_x: i64,
    pub region_y: i64,
    pub segment_id: Option<String>,
    pub source_frame_ref: String,
    pub source_kind: String,
    pub status: String,
    pub updated_at_ms: i64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VisualFrameProcessingStateRow {
    pub chunk_id: String,
    pub created_at_ms: i64,
    pub error_code: Option<String>,
    pub frame_timestamp_ms: i64,
    pub id: String,
    pub job_id: String,
    pub output_count: i64,
    pub progress: f64,
    pub recording_id: String,
    pub region_height: i64,
    pub region_width: i64,
    pub region_x: i64,
    pub region_y: i64,
    pub segment_id: Option<String>,
    pub source_frame_ref: String,
    pub source_kind: String,
    pub status: String,
    pub updated_at_ms: i64,
}
