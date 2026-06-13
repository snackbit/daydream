use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiError {
    pub code: String,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiModelAsset {
    pub checksum: Option<String>,
    pub checksum_algorithm: Option<String>,
    pub created_at_ms: i64,
    pub downloaded_bytes: i64,
    pub download_status: String,
    pub etag: Option<String>,
    pub error_code: Option<String>,
    pub id: String,
    pub label: String,
    pub local_path: Option<String>,
    pub provider: String,
    pub self_test_status: String,
    pub size_bytes: Option<i64>,
    pub source_url: Option<String>,
    pub total_bytes: Option<i64>,
    pub updated_at_ms: i64,
    pub verification_status: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiModelAssetList {
    pub models: Vec<AiModelAsset>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DownloadAiModelInput {
    pub model_id: String,
    pub notification_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CancelAiModelDownloadInput {
    pub model_id: String,
    pub notification_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeleteAiModelInput {
    pub model_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SelfTestAiModelInput {
    pub model_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SaveAiProviderSecretInput {
    pub plaintext: String,
    pub role: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeleteAiProviderSecretInput {
    pub secret_ref: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SaveAiProviderSecretResult {
    pub secret_ref: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiProviderStatus {
    pub local_transcription: AiLocalTranscriptionStatus,
    pub ocr: AiOcrStatus,
    pub remote_transcription: String,
    pub vision: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiLocalTranscriptionStatus {
    pub available_languages: Vec<AiTranscriptionLanguageStatus>,
    pub available_backends: Vec<AiBackendStatus>,
    pub checksum_status: String,
    pub local_model_id: String,
    pub model_download_status: String,
    pub model_path: Option<String>,
    pub selected_backend: String,
    pub self_test_status: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiBackendStatus {
    pub available: bool,
    pub id: String,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiTranscriptionLanguageStatus {
    pub id: String,
    pub label: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiOcrStatus {
    pub data_source: String,
    pub data_download_status: String,
    pub languages: Vec<String>,
    pub data_path: Option<String>,
    pub enabled: bool,
    pub engine: String,
    pub runtime_download_status: String,
    pub runtime_path: Option<String>,
    pub runtime_source: String,
    pub runtime_version: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProcessingJob {
    pub created_at_ms: i64,
    pub error_code: Option<String>,
    pub id: String,
    pub kind: String,
    pub progress: f64,
    pub recording_id: String,
    pub status: String,
    pub updated_at_ms: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProcessingJobList {
    pub jobs: Vec<ProcessingJob>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProcessingChunk {
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListProcessingJobsInput {
    pub recording_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProcessingJobActionInput {
    pub job_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StartRecordingAiProcessingInput {
    pub recording_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GetRecordingTranscriptInput {
    pub recording_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecordingAiResultInput {
    pub recording_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecordingAiProcessingState {
    pub chunks: Vec<ProcessingChunk>,
    pub job: Option<ProcessingJob>,
    pub recording_id: String,
    pub transcript: RecordingTranscript,
    pub transcription_tracks: Vec<RecordingTranscriptionTrackState>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecordingTranscriptionTrackState {
    pub chunks: Vec<RecordingTranscriptionChunkState>,
    pub description: String,
    pub label: String,
    pub library_track_id: Option<String>,
    pub source_role: String,
    pub status: String,
    pub status_label: String,
    pub text_segment_count: usize,
    pub track_id: Option<String>,
    pub track_key: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecordingTranscriptionChunkState {
    pub attempt_count: i64,
    pub end_offset_ms: Option<i64>,
    pub error_code: Option<String>,
    pub id: String,
    pub progress: f64,
    pub result: String,
    pub result_code: Option<String>,
    pub segment_id: Option<String>,
    pub start_offset_ms: Option<i64>,
    pub status: String,
    pub text: Option<String>,
    pub transcript_segments: Vec<RecordingTranscriptSegment>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecordingTranscript {
    pub recording_id: String,
    pub tracks: Vec<RecordingTranscriptTrack>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecordingTranscriptTrack {
    pub source_role: String,
    pub track_id: String,
    pub segments: Vec<RecordingTranscriptSegment>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecordingTranscriptSegment {
    pub confidence: Option<f64>,
    pub end_offset_ms: i64,
    pub id: String,
    pub model: Option<String>,
    pub provider: String,
    pub segment_id: String,
    pub start_offset_ms: i64,
    pub text: String,
    pub words: Vec<RecordingTranscriptWord>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecordingTranscriptWord {
    pub end_offset_ms: i64,
    pub id: String,
    pub probability: Option<f64>,
    pub start_offset_ms: i64,
    pub text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecordingActivity {
    pub intervals: Vec<ActivityInterval>,
    pub recording_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ActivityInterval {
    pub end_offset_ms: i64,
    pub id: String,
    pub job_id: String,
    pub recording_id: String,
    pub score: Option<f64>,
    pub source_kind: String,
    pub start_offset_ms: i64,
    pub state: String,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VisualRegion {
    pub height: i64,
    pub width: i64,
    pub x: i64,
    pub y: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OcrRegion {
    pub chunk_id: String,
    pub confidence: Option<f64>,
    pub frame_timestamp_ms: i64,
    pub id: String,
    pub job_id: String,
    pub model: Option<String>,
    pub provider: String,
    pub recording_id: String,
    pub region: VisualRegion,
    pub source_frame_ref: String,
    pub text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VlmObservation {
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
    pub region: VisualRegion,
    pub source_frame_ref: String,
    pub text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VlmState {
    pub error_code: Option<String>,
    pub provider_status: String,
    pub status: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecordingVisualObservations {
    pub ocr_regions: Vec<OcrRegion>,
    pub recording_id: String,
    pub vlm_state: VlmState,
    pub vlm_observations: Vec<VlmObservation>,
}
