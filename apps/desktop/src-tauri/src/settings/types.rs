use serde::{Deserialize, Serialize};

use crate::storage_limits::StorageCleanupResult;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AppSettingsState {
    pub settings: AppSettings,
    pub defaults: AppSettings,
    pub limits: AppSettingsLimits,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage_cleanup: Option<StorageCleanupResult>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub ai: AiSettings,
    pub debug: DebugSettings,
    pub expert: ExpertSettings,
    pub record: RecordSettings,
    pub storage: StorageSettings,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DebugSettings {
    pub enabled: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExpertSettings {
    pub enabled: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AiSettings {
    pub activity: AiActivitySettings,
    pub ocr: AiOcrSettings,
    pub processing: AiProcessingSettings,
    pub transcription: AiTranscriptionProviderSettings,
    pub vision: AiVisionProviderSettings,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AiTranscriptionProviderSettings {
    pub language: String,
    pub local_backend: String,
    pub local_model_id: String,
    pub max_initial_timestamp_seconds: f64,
    pub merge_gap_ms: u32,
    pub min_speech_ms: u32,
    pub provider: String,
    pub remote_api_key_ref: Option<String>,
    pub remote_base_url: Option<String>,
    pub remote_model: Option<String>,
    pub silence_gate_enabled: bool,
    pub silence_threshold_db: f64,
    pub word_timestamps_enabled: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiVisionProviderSettings {
    pub api_key_ref: Option<String>,
    pub base_url: Option<String>,
    pub model: Option<String>,
    pub provider: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiOcrSettings {
    pub enabled: bool,
    pub engine: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AiProcessingSettings {
    pub auto_start_after_recording: bool,
    pub frame_recognition_fps: f64,
    pub frame_recognition_sync_with_recording: bool,
    pub frame_similarity_enabled: bool,
    pub max_full_frame_checkpoints_per_minute: u32,
    pub max_parallel_chunks_per_job: u32,
    pub max_parallel_jobs: u32,
    pub periodic_full_frame_interval_ms: u32,
    pub visual_delta_tile_size_px: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiActivitySettings {
    pub frame_deltas: AiActivitySourceSettings,
    pub keyboard: AiActivityWindowSettings,
    pub mouse: AiActivityWindowSettings,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiActivitySourceSettings {
    pub enabled: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiActivityWindowSettings {
    pub enabled: bool,
    pub window_ms: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecordSettings {
    pub fps: f64,
    pub video_bitrate_kbps: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StorageSettings {
    pub retention_days: u32,
    pub size_limit_bytes: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AppSettingsLimits {
    pub ai: AiSettingsLimits,
    pub record: RecordSettingsLimits,
    pub storage: StorageSettingsLimits,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AiSettingsLimits {
    pub activity: AiActivityLimits,
    pub processing: AiProcessingLimits,
    pub transcription: AiTranscriptionLimits,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AiTranscriptionLimits {
    pub max_initial_timestamp_seconds_min: f64,
    pub max_initial_timestamp_seconds_max: f64,
    pub merge_gap_ms_min: u32,
    pub merge_gap_ms_max: u32,
    pub min_speech_ms_min: u32,
    pub min_speech_ms_max: u32,
    pub silence_threshold_db_min: f64,
    pub silence_threshold_db_max: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AiProcessingLimits {
    pub frame_recognition_fps_min: f64,
    pub frame_recognition_fps_max: f64,
    pub max_full_frame_checkpoints_per_minute_min: u32,
    pub max_full_frame_checkpoints_per_minute_max: u32,
    pub max_parallel_chunks_per_job_min: u32,
    pub max_parallel_chunks_per_job_max: u32,
    pub max_parallel_jobs_min: u32,
    pub max_parallel_jobs_max: u32,
    pub periodic_full_frame_interval_ms_min: u32,
    pub periodic_full_frame_interval_ms_max: u32,
    pub visual_delta_tile_size_px_min: u32,
    pub visual_delta_tile_size_px_max: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiActivityLimits {
    pub keyboard_window_ms_min: u32,
    pub keyboard_window_ms_max: u32,
    pub mouse_window_ms_min: u32,
    pub mouse_window_ms_max: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecordSettingsLimits {
    pub fps_min: f64,
    pub fps_max: f64,
    pub video_bitrate_kbps_min: u32,
    pub video_bitrate_kbps_max: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StorageSettingsLimits {
    pub retention_days_min: u32,
    pub retention_days_max: u32,
    pub size_limit_bytes_min: u64,
    pub size_limit_bytes_max: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SaveAppSettingsInput {
    pub settings: AppSettings,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PreviewAppSettingsStorageCleanupInput {
    pub settings: AppSettings,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppSettingsError {
    pub code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    pub message: String,
}
