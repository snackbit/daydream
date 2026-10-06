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
    #[serde(
        default = "default_vision_auth_mode",
        skip_serializing_if = "default_key_auth"
    )]
    pub auth_mode: String,
    #[serde(default)]
    pub observations_enabled: bool,
    pub api_key_ref: Option<String>,
    pub base_url: Option<String>,
    pub model: Option<String>,
    pub provider: String,
}

fn default_vision_auth_mode() -> String {
    "api-key".into()
}
fn default_key_auth(value: &String) -> bool {
    value == "api-key"
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
    #[serde(default = "default_processing_timing")]
    pub timing: String,
    #[serde(default = "default_processing_resource_mode")]
    pub resource_mode: String,
    pub frame_recognition_fps: f64,
    pub frame_recognition_sync_with_recording: bool,
    pub frame_similarity_enabled: bool,
    pub max_full_frame_checkpoints_per_minute: u32,
    pub max_parallel_chunks_per_job: u32,
    pub max_parallel_ocr_jobs: u32,
    pub max_parallel_jobs: u32,
    pub max_parallel_vlm_jobs: u32,
    pub periodic_full_frame_interval_ms: u32,
    pub visual_delta_tile_size_px: u32,
}

fn default_processing_timing() -> String {
    "during-recording".into()
}
fn default_processing_resource_mode() -> String {
    "background".into()
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiActivitySettings {
    pub frame_deltas: AiActivitySourceSettings,
    pub keyboard: AiActivityWindowSettings,
    pub mouse: AiActivityWindowSettings,
    pub windows: AiActivitySourceSettings,
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
    pub window_protection: RecordWindowProtectionSettings,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ProtectedWindowAction {
    Pause,
    Redact,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ProtectedWindowRuleAction {
    Inherit,
    Pause,
    Redact,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ProtectedWindowTitleMatcherKind {
    Contains,
    Regex,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProtectedWindowTitleMatcher {
    pub case_sensitive: bool,
    pub kind: ProtectedWindowTitleMatcherKind,
    pub value: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProtectedWindowRule {
    pub action: ProtectedWindowRuleAction,
    pub app_identity: Option<String>,
    pub enabled: bool,
    pub id: String,
    pub label: String,
    pub process_name: Option<String>,
    pub title_matcher: Option<ProtectedWindowTitleMatcher>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecordWindowProtectionSettings {
    pub default_action: ProtectedWindowAction,
    pub dwell_ms: u32,
    pub enabled: bool,
    pub merge_gap_ms: u32,
    pub rules: Vec<ProtectedWindowRule>,
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
    pub max_parallel_ocr_jobs_min: u32,
    pub max_parallel_ocr_jobs_max: u32,
    pub max_parallel_vlm_jobs_min: u32,
    pub max_parallel_vlm_jobs_max: u32,
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
    pub window_protection_dwell_ms_min: u32,
    pub window_protection_dwell_ms_max: u32,
    pub window_protection_merge_gap_ms_min: u32,
    pub window_protection_merge_gap_ms_max: u32,
    pub window_protection_rule_count_max: usize,
    pub window_protection_text_max_chars: usize,
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
    #[serde(default)]
    pub credentials: SettingsCredentialDrafts,
}

#[derive(Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SettingsCredentialDrafts {
    pub transcription_key: Option<String>,
    pub vision_key: Option<String>,
}

impl std::fmt::Debug for SettingsCredentialDrafts {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("SettingsCredentialDrafts([redacted])")
    }
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum InputActivityStatusReason {
    Available,
    DevInputMissing,
    NoSupportedDevices,
    PermissionDenied,
    Unknown,
    UnsupportedPlatform,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InputActivityStatus {
    pub available: bool,
    pub candidate_device_count: usize,
    pub current_user: Option<String>,
    pub message: String,
    pub permission_denied_device_count: usize,
    pub platform: String,
    pub readable_device_count: usize,
    pub reason: InputActivityStatusReason,
    pub repair_available: bool,
    pub user_in_input_group: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InputActivityRepairResult {
    pub message: String,
    pub status: InputActivityStatus,
    pub success: bool,
}
