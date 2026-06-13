use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::Value;

use crate::recorder::{
    CaptureTimingDiagnostics, RecorderBackendHealth, RecorderMediaWriterCapabilities,
    RecorderPreviewStatus,
};
use crate::storage::DatabaseDiagnostics;
use crate::telemetry::DebugLoggingDiagnostics;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppDiagnostics {
    pub identifier: String,
    pub name: String,
    pub version: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DaydreamDiagnostics {
    pub app: AppDiagnostics,
    pub ai_processing: AiProcessingDiagnostics,
    pub database: DatabaseDiagnostics,
    pub debug_logging: DebugLoggingDiagnostics,
    pub latest_recording: LatestRecordingDiagnostics,
    pub media_writer: MediaWriterDiagnostics,
    pub ocr: OcrDiagnostics,
    pub portal_restore_token: PortalRestoreTokenDiagnostics,
    pub recorder: RecorderBackendHealth,
    pub storage: StorageDiagnostics,
    pub system: SystemDiagnostics,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DebugLogBundle {
    pub diagnostics: DaydreamDiagnostics,
    pub log_tail_lines: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaWriterDiagnostics {
    pub available: bool,
    pub capabilities: RecorderMediaWriterCapabilities,
    pub error: Option<String>,
    pub version: Option<String>,
    pub writer: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiProcessingDiagnostics {
    pub chunk_count: usize,
    pub chunk_status_counts: BTreeMap<String, usize>,
    pub chunks: Vec<AiProcessingChunkDiagnostics>,
    pub error: Option<String>,
    pub latest_job: Option<AiProcessingJobDiagnostics>,
    pub recording_id: Option<String>,
    pub settings: Option<AiProcessingSettingsDiagnostics>,
    pub transcript_tracks: Vec<AiTranscriptTrackDiagnostics>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiProcessingSettingsDiagnostics {
    pub auto_start_after_recording: bool,
    pub max_parallel_chunks_per_job: u32,
    pub max_parallel_jobs: u32,
    pub transcription_max_initial_timestamp_seconds: f64,
    pub transcription_merge_gap_ms: u32,
    pub transcription_backend: String,
    pub transcription_language: String,
    pub transcription_min_speech_ms: u32,
    pub transcription_model: String,
    pub transcription_provider: String,
    pub transcription_silence_gate_enabled: bool,
    pub transcription_silence_threshold_db: f64,
    pub word_timestamps_enabled: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiProcessingJobDiagnostics {
    pub created_at_ms: i64,
    pub error: Option<String>,
    pub id: String,
    pub kind: String,
    pub status: String,
    pub updated_at_ms: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiProcessingChunkDiagnostics {
    pub attempt_count: i64,
    pub end_offset_ms: Option<i64>,
    pub error_code: Option<String>,
    pub id: String,
    pub progress: f64,
    pub result_code: Option<String>,
    pub segment_id: Option<String>,
    pub source_kind: String,
    pub start_offset_ms: Option<i64>,
    pub status: String,
    pub track_id: Option<String>,
    pub track_kind: Option<String>,
    pub track_label: Option<String>,
    pub updated_at_ms: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiTranscriptTrackDiagnostics {
    pub first_start_offset_ms: Option<i64>,
    pub last_end_offset_ms: Option<i64>,
    pub segment_count: usize,
    pub source_role: String,
    pub text_char_count: usize,
    pub track_id: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OcrDiagnostics {
    pub data_download_status: String,
    pub data_path_present: bool,
    pub data_source: String,
    pub enabled: bool,
    pub engine: String,
    pub languages: Vec<String>,
    pub runtime_download_status: String,
    pub runtime_path: Option<String>,
    pub runtime_path_present: bool,
    pub runtime_source: String,
    pub runtime_version: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PortalRestoreTokenDiagnostics {
    pub created_at_ms: Option<i64>,
    pub error: Option<String>,
    pub expected_id: String,
    pub last_used_at_ms: Option<i64>,
    pub portal: Option<String>,
    pub present: bool,
    pub token_length: Option<usize>,
    pub updated_at_ms: Option<i64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LatestRecordingDiagnostics {
    pub active_capture_timing: Option<CaptureTimingDiagnostics>,
    pub active_recording_id: Option<String>,
    pub active_segment_id: Option<String>,
    pub error: Option<String>,
    pub latest_segment: Option<LatestSegmentDiagnostics>,
    pub preview_status: Option<RecorderPreviewStatus>,
    pub recording_id: Option<String>,
    pub segment_count: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LatestSegmentDiagnostics {
    pub desktop_audio_stream_index: Option<i64>,
    pub duration_ms: Option<i64>,
    pub ended_at_ms: Option<i64>,
    pub error_code: Option<String>,
    pub writer_exit_code: Option<i64>,
    pub file_path: String,
    pub id: String,
    pub microphone_audio_stream_index: Option<i64>,
    pub probe: Option<Value>,
    pub recording_id: String,
    pub segment_index: i64,
    pub started_at_ms: i64,
    pub state: String,
    pub video_stream_index: Option<i64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageDiagnostics {
    pub app_data_dir: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemDiagnostics {
    pub arch: String,
    pub family: String,
    pub platform: String,
}
