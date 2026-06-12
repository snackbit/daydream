use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use tauri::{Manager, State};

use crate::{
    storage::{self, AppSettingRow, AppStorage, SaveAppSettingInput},
    storage_limits::{self, StorageCleanupPreview, StorageCleanupResult},
    whisper_backend,
};

pub const DEFAULT_RECORD_FPS: f64 = 0.2;
pub const MIN_RECORD_FPS: f64 = 1.0 / 60.0;
pub const MAX_RECORD_FPS: f64 = 5.0;
pub const DEFAULT_VIDEO_BITRATE_KBPS: u32 = 1_500;
pub const MIN_VIDEO_BITRATE_KBPS: u32 = 250;
pub const MAX_VIDEO_BITRATE_KBPS: u32 = 10_000;
pub const DEFAULT_STORAGE_RETENTION_DAYS: u32 = 7;
pub const MIN_STORAGE_RETENTION_DAYS: u32 = 1;
pub const MAX_STORAGE_RETENTION_DAYS: u32 = 365;
pub const DEFAULT_STORAGE_SIZE_LIMIT_BYTES: u64 = 50 * 1024 * 1024 * 1024;
pub const MIN_STORAGE_SIZE_LIMIT_BYTES: u64 = 1024 * 1024 * 1024;
pub const DEFAULT_AI_TRANSCRIPTION_MODEL_ID: &str = "small";
pub const DEFAULT_AI_TRANSCRIPTION_BACKEND: &str = "cpu";
pub const DEFAULT_AI_TRANSCRIPTION_LANGUAGE: &str = "auto";
pub const DEFAULT_AI_KEYBOARD_WINDOW_MS: u32 = 30_000;
pub const DEFAULT_AI_MOUSE_WINDOW_MS: u32 = 5_000;
pub const MIN_AI_ACTIVITY_WINDOW_MS: u32 = 1_000;
pub const MAX_AI_ACTIVITY_WINDOW_MS: u32 = 300_000;
pub const DEFAULT_AI_MAX_PARALLEL_JOBS: u32 = 1;
pub const MIN_AI_MAX_PARALLEL_JOBS: u32 = 1;
pub const MAX_AI_MAX_PARALLEL_JOBS: u32 = 4;
pub const DEFAULT_AI_MAX_PARALLEL_CHUNKS_PER_JOB: u32 = 1;
pub const MIN_AI_MAX_PARALLEL_CHUNKS_PER_JOB: u32 = 1;
pub const MAX_AI_MAX_PARALLEL_CHUNKS_PER_JOB: u32 = 8;
pub const DEFAULT_AI_FRAME_RECOGNITION_FPS: f64 = DEFAULT_RECORD_FPS;
pub const MIN_AI_FRAME_RECOGNITION_FPS: f64 = MIN_RECORD_FPS;
pub const MAX_AI_FRAME_RECOGNITION_FPS: f64 = MAX_RECORD_FPS;
pub const DEFAULT_AI_PERIODIC_FULL_FRAME_INTERVAL_MS: u32 = 60_000;
pub const MIN_AI_PERIODIC_FULL_FRAME_INTERVAL_MS: u32 = 5_000;
pub const MAX_AI_PERIODIC_FULL_FRAME_INTERVAL_MS: u32 = 600_000;
pub const DEFAULT_AI_MAX_FULL_FRAME_CHECKPOINTS_PER_MINUTE: u32 = 2;
pub const MIN_AI_MAX_FULL_FRAME_CHECKPOINTS_PER_MINUTE: u32 = 1;
pub const MAX_AI_MAX_FULL_FRAME_CHECKPOINTS_PER_MINUTE: u32 = 60;
pub const DEFAULT_AI_VISUAL_DELTA_TILE_SIZE_PX: u32 = 64;
pub const MIN_AI_VISUAL_DELTA_TILE_SIZE_PX: u32 = 32;
pub const MAX_AI_VISUAL_DELTA_TILE_SIZE_PX: u32 = 256;
pub const DEFAULT_AI_TRANSCRIPTION_SILENCE_THRESHOLD_DB: f64 = -45.0;
pub const MIN_AI_TRANSCRIPTION_SILENCE_THRESHOLD_DB: f64 = -80.0;
pub const MAX_AI_TRANSCRIPTION_SILENCE_THRESHOLD_DB: f64 = -20.0;
pub const DEFAULT_AI_TRANSCRIPTION_MIN_SPEECH_MS: u32 = 250;
pub const MIN_AI_TRANSCRIPTION_MIN_SPEECH_MS: u32 = 50;
pub const MAX_AI_TRANSCRIPTION_MIN_SPEECH_MS: u32 = 2_000;
pub const DEFAULT_AI_TRANSCRIPTION_MERGE_GAP_MS: u32 = 700;
pub const MIN_AI_TRANSCRIPTION_MERGE_GAP_MS: u32 = 0;
pub const MAX_AI_TRANSCRIPTION_MERGE_GAP_MS: u32 = 5_000;
pub const DEFAULT_AI_TRANSCRIPTION_MAX_INITIAL_TIMESTAMP_SECONDS: f64 = 3.0;
pub const MIN_AI_TRANSCRIPTION_MAX_INITIAL_TIMESTAMP_SECONDS: f64 = 1.0;
pub const MAX_AI_TRANSCRIPTION_MAX_INITIAL_TIMESTAMP_SECONDS: f64 = 10.0;

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

#[tauri::command(rename = "getAppSettings")]
pub fn get_app_settings(
    storage: State<'_, AppStorage>,
) -> Result<AppSettingsState, AppSettingsError> {
    load_app_settings(&storage).map(settings_state)
}

#[tauri::command(rename = "previewAppSettingsStorageCleanup")]
pub fn preview_app_settings_storage_cleanup(
    app: tauri::AppHandle,
    input: PreviewAppSettingsStorageCleanupInput,
    storage: State<'_, AppStorage>,
) -> Result<StorageCleanupPreview, AppSettingsError> {
    let settings = validate_settings(input.settings)?;
    let recordings_root = recordings_root(&app)?;

    storage_limits::preview_storage_cleanup(&storage, &recordings_root, &settings.storage)
        .map_err(cleanup_error)
}

#[tauri::command(rename = "saveAppSettings")]
pub fn save_app_settings(
    app: tauri::AppHandle,
    input: SaveAppSettingsInput,
    storage: State<'_, AppStorage>,
) -> Result<AppSettingsState, AppSettingsError> {
    let settings = validate_settings(input.settings)?;
    let storage_input = settings_to_storage_inputs(&settings, now_ms() as i64)?;

    storage
        .with_connection_mut(|connection| storage::save_app_settings(connection, &storage_input))
        .map_err(storage_error)
        .and_then(settings_from_storage_rows)
        .map(|saved_settings| {
            let cleanup = enforce_storage_cleanup_for_settings(&app, &storage, &saved_settings);
            settings_state_with_cleanup(saved_settings, Some(cleanup))
        })
}

#[tauri::command(rename = "resetAppSettings")]
pub fn reset_app_settings(
    app: tauri::AppHandle,
    storage: State<'_, AppStorage>,
) -> Result<AppSettingsState, AppSettingsError> {
    storage
        .with_connection(storage::reset_app_settings)
        .map_err(storage_error)?;

    let settings = default_settings();
    let cleanup = enforce_storage_cleanup_for_settings(&app, &storage, &settings);

    Ok(settings_state_with_cleanup(settings, Some(cleanup)))
}

pub fn load_app_settings(storage: &AppStorage) -> Result<AppSettings, AppSettingsError> {
    let rows = storage
        .with_connection(storage::list_app_settings)
        .map_err(storage_error)?;

    settings_from_storage_rows(rows)
}

pub fn default_settings() -> AppSettings {
    AppSettings {
        ai: default_ai_settings(),
        debug: DebugSettings { enabled: false },
        expert: ExpertSettings { enabled: false },
        record: RecordSettings {
            fps: DEFAULT_RECORD_FPS,
            video_bitrate_kbps: DEFAULT_VIDEO_BITRATE_KBPS,
        },
        storage: StorageSettings {
            retention_days: DEFAULT_STORAGE_RETENTION_DAYS,
            size_limit_bytes: DEFAULT_STORAGE_SIZE_LIMIT_BYTES,
        },
    }
}

pub fn default_ai_settings() -> AiSettings {
    AiSettings {
        activity: AiActivitySettings {
            frame_deltas: AiActivitySourceSettings { enabled: true },
            keyboard: AiActivityWindowSettings {
                enabled: false,
                window_ms: DEFAULT_AI_KEYBOARD_WINDOW_MS,
            },
            mouse: AiActivityWindowSettings {
                enabled: false,
                window_ms: DEFAULT_AI_MOUSE_WINDOW_MS,
            },
        },
        ocr: AiOcrSettings {
            enabled: true,
            engine: "tesseract".to_string(),
        },
        processing: AiProcessingSettings {
            auto_start_after_recording: true,
            frame_recognition_fps: DEFAULT_AI_FRAME_RECOGNITION_FPS,
            frame_recognition_sync_with_recording: true,
            frame_similarity_enabled: true,
            max_full_frame_checkpoints_per_minute: DEFAULT_AI_MAX_FULL_FRAME_CHECKPOINTS_PER_MINUTE,
            max_parallel_chunks_per_job: DEFAULT_AI_MAX_PARALLEL_CHUNKS_PER_JOB,
            max_parallel_jobs: DEFAULT_AI_MAX_PARALLEL_JOBS,
            periodic_full_frame_interval_ms: DEFAULT_AI_PERIODIC_FULL_FRAME_INTERVAL_MS,
            visual_delta_tile_size_px: DEFAULT_AI_VISUAL_DELTA_TILE_SIZE_PX,
        },
        transcription: AiTranscriptionProviderSettings {
            language: DEFAULT_AI_TRANSCRIPTION_LANGUAGE.to_string(),
            local_backend: DEFAULT_AI_TRANSCRIPTION_BACKEND.to_string(),
            local_model_id: DEFAULT_AI_TRANSCRIPTION_MODEL_ID.to_string(),
            max_initial_timestamp_seconds: DEFAULT_AI_TRANSCRIPTION_MAX_INITIAL_TIMESTAMP_SECONDS,
            merge_gap_ms: DEFAULT_AI_TRANSCRIPTION_MERGE_GAP_MS,
            min_speech_ms: DEFAULT_AI_TRANSCRIPTION_MIN_SPEECH_MS,
            provider: "local-whisper-cpp".to_string(),
            remote_api_key_ref: None,
            remote_base_url: None,
            remote_model: None,
            silence_gate_enabled: true,
            silence_threshold_db: DEFAULT_AI_TRANSCRIPTION_SILENCE_THRESHOLD_DB,
            word_timestamps_enabled: true,
        },
        vision: AiVisionProviderSettings {
            api_key_ref: None,
            base_url: None,
            model: None,
            provider: "disabled".to_string(),
        },
    }
}

pub fn settings_limits() -> AppSettingsLimits {
    AppSettingsLimits {
        ai: AiSettingsLimits {
            activity: AiActivityLimits {
                keyboard_window_ms_min: MIN_AI_ACTIVITY_WINDOW_MS,
                keyboard_window_ms_max: MAX_AI_ACTIVITY_WINDOW_MS,
                mouse_window_ms_min: MIN_AI_ACTIVITY_WINDOW_MS,
                mouse_window_ms_max: MAX_AI_ACTIVITY_WINDOW_MS,
            },
            processing: AiProcessingLimits {
                frame_recognition_fps_min: MIN_AI_FRAME_RECOGNITION_FPS,
                frame_recognition_fps_max: MAX_AI_FRAME_RECOGNITION_FPS,
                max_full_frame_checkpoints_per_minute_min:
                    MIN_AI_MAX_FULL_FRAME_CHECKPOINTS_PER_MINUTE,
                max_full_frame_checkpoints_per_minute_max:
                    MAX_AI_MAX_FULL_FRAME_CHECKPOINTS_PER_MINUTE,
                max_parallel_chunks_per_job_min: MIN_AI_MAX_PARALLEL_CHUNKS_PER_JOB,
                max_parallel_chunks_per_job_max: MAX_AI_MAX_PARALLEL_CHUNKS_PER_JOB,
                max_parallel_jobs_min: MIN_AI_MAX_PARALLEL_JOBS,
                max_parallel_jobs_max: MAX_AI_MAX_PARALLEL_JOBS,
                periodic_full_frame_interval_ms_min: MIN_AI_PERIODIC_FULL_FRAME_INTERVAL_MS,
                periodic_full_frame_interval_ms_max: MAX_AI_PERIODIC_FULL_FRAME_INTERVAL_MS,
                visual_delta_tile_size_px_min: MIN_AI_VISUAL_DELTA_TILE_SIZE_PX,
                visual_delta_tile_size_px_max: MAX_AI_VISUAL_DELTA_TILE_SIZE_PX,
            },
            transcription: AiTranscriptionLimits {
                max_initial_timestamp_seconds_min:
                    MIN_AI_TRANSCRIPTION_MAX_INITIAL_TIMESTAMP_SECONDS,
                max_initial_timestamp_seconds_max:
                    MAX_AI_TRANSCRIPTION_MAX_INITIAL_TIMESTAMP_SECONDS,
                merge_gap_ms_min: MIN_AI_TRANSCRIPTION_MERGE_GAP_MS,
                merge_gap_ms_max: MAX_AI_TRANSCRIPTION_MERGE_GAP_MS,
                min_speech_ms_min: MIN_AI_TRANSCRIPTION_MIN_SPEECH_MS,
                min_speech_ms_max: MAX_AI_TRANSCRIPTION_MIN_SPEECH_MS,
                silence_threshold_db_min: MIN_AI_TRANSCRIPTION_SILENCE_THRESHOLD_DB,
                silence_threshold_db_max: MAX_AI_TRANSCRIPTION_SILENCE_THRESHOLD_DB,
            },
        },
        record: RecordSettingsLimits {
            fps_min: MIN_RECORD_FPS,
            fps_max: MAX_RECORD_FPS,
            video_bitrate_kbps_min: MIN_VIDEO_BITRATE_KBPS,
            video_bitrate_kbps_max: MAX_VIDEO_BITRATE_KBPS,
        },
        storage: StorageSettingsLimits {
            retention_days_min: MIN_STORAGE_RETENTION_DAYS,
            retention_days_max: MAX_STORAGE_RETENTION_DAYS,
            size_limit_bytes_min: MIN_STORAGE_SIZE_LIMIT_BYTES,
            size_limit_bytes_max: None,
        },
    }
}

fn settings_state(settings: AppSettings) -> AppSettingsState {
    settings_state_with_cleanup(settings, None)
}

fn settings_state_with_cleanup(
    settings: AppSettings,
    storage_cleanup: Option<StorageCleanupResult>,
) -> AppSettingsState {
    AppSettingsState {
        settings,
        defaults: default_settings(),
        limits: settings_limits(),
        storage_cleanup,
    }
}

fn enforce_storage_cleanup_for_settings(
    app: &tauri::AppHandle,
    storage: &AppStorage,
    settings: &AppSettings,
) -> StorageCleanupResult {
    match recordings_root(app) {
        Ok(recordings_root) => {
            storage_limits::enforce_storage_cleanup(storage, &recordings_root, &settings.storage)
        }
        Err(_error) => StorageCleanupResult {
            failed_recording_count: 1,
            ..StorageCleanupResult::default()
        },
    }
}

fn settings_from_storage_rows(rows: Vec<AppSettingRow>) -> Result<AppSettings, AppSettingsError> {
    let mut settings = default_settings();

    for row in rows {
        apply_setting_row(&mut settings, &row)?;
    }

    validate_settings(settings)
}

fn apply_setting_row(
    settings: &mut AppSettings,
    row: &AppSettingRow,
) -> Result<(), AppSettingsError> {
    match row.key.as_str() {
        "debug.enabled" => settings.debug.enabled = setting_value(row)?,
        "expert.enabled" => settings.expert.enabled = setting_value(row)?,
        "record.fps" => settings.record.fps = setting_value(row)?,
        "record.videoBitrateKbps" => settings.record.video_bitrate_kbps = setting_value(row)?,
        "storage.retentionDays" => settings.storage.retention_days = setting_value(row)?,
        "storage.sizeLimitBytes" => settings.storage.size_limit_bytes = setting_value(row)?,
        "ai.transcription.provider" => settings.ai.transcription.provider = setting_value(row)?,
        "ai.transcription.localModelId" => {
            settings.ai.transcription.local_model_id = setting_value(row)?;
        }
        "ai.transcription.localBackend" => {
            settings.ai.transcription.local_backend =
                coerce_stored_transcription_backend(setting_value(row)?);
        }
        "ai.transcription.language" => {
            settings.ai.transcription.language = setting_value(row)?;
        }
        "ai.transcription.maxInitialTimestampSeconds" => {
            settings.ai.transcription.max_initial_timestamp_seconds = setting_value(row)?;
        }
        "ai.transcription.mergeGapMs" => {
            settings.ai.transcription.merge_gap_ms = setting_value(row)?;
        }
        "ai.transcription.minSpeechMs" => {
            settings.ai.transcription.min_speech_ms = setting_value(row)?;
        }
        "ai.transcription.remoteBaseUrl" => {
            settings.ai.transcription.remote_base_url = setting_value(row)?;
        }
        "ai.transcription.remoteModel" => {
            settings.ai.transcription.remote_model = setting_value(row)?;
        }
        "ai.transcription.remoteApiKeyRef" => {
            settings.ai.transcription.remote_api_key_ref = setting_value(row)?;
        }
        "ai.transcription.silenceGateEnabled" => {
            settings.ai.transcription.silence_gate_enabled = setting_value(row)?;
        }
        "ai.transcription.silenceThresholdDb" => {
            settings.ai.transcription.silence_threshold_db = setting_value(row)?;
        }
        "ai.transcription.wordTimestampsEnabled" => {
            settings.ai.transcription.word_timestamps_enabled = setting_value(row)?;
        }
        "ai.vision.provider" => settings.ai.vision.provider = setting_value(row)?,
        "ai.vision.baseUrl" => settings.ai.vision.base_url = setting_value(row)?,
        "ai.vision.apiKeyRef" => settings.ai.vision.api_key_ref = setting_value(row)?,
        "ai.vision.model" => settings.ai.vision.model = setting_value(row)?,
        "ai.ocr.enabled" => settings.ai.ocr.enabled = setting_value(row)?,
        "ai.ocr.engine" => settings.ai.ocr.engine = setting_value(row)?,
        "ai.processing.autoStartAfterRecording" => {
            settings.ai.processing.auto_start_after_recording = setting_value(row)?;
        }
        "ai.processing.maxParallelJobs" => {
            settings.ai.processing.max_parallel_jobs = setting_value(row)?;
        }
        "ai.processing.maxParallelChunksPerJob" => {
            settings.ai.processing.max_parallel_chunks_per_job = setting_value(row)?;
        }
        "ai.processing.frameRecognitionFps" => {
            settings.ai.processing.frame_recognition_fps = setting_value(row)?;
        }
        "ai.processing.frameRecognitionSyncWithRecording" => {
            settings.ai.processing.frame_recognition_sync_with_recording = setting_value(row)?;
        }
        "ai.processing.periodicFullFrameIntervalMs" => {
            settings.ai.processing.periodic_full_frame_interval_ms = setting_value(row)?;
        }
        "ai.processing.frameSimilarityEnabled" => {
            settings.ai.processing.frame_similarity_enabled = setting_value(row)?;
        }
        "ai.processing.maxFullFrameCheckpointsPerMinute" => {
            settings.ai.processing.max_full_frame_checkpoints_per_minute = setting_value(row)?;
        }
        "ai.processing.visualDeltaTileSizePx" => {
            settings.ai.processing.visual_delta_tile_size_px = setting_value(row)?;
        }
        "ai.activity.keyboard.enabled" => {
            settings.ai.activity.keyboard.enabled = setting_value(row)?
        }
        "ai.activity.keyboard.windowMs" => {
            settings.ai.activity.keyboard.window_ms = setting_value(row)?;
        }
        "ai.activity.mouse.enabled" => settings.ai.activity.mouse.enabled = setting_value(row)?,
        "ai.activity.mouse.windowMs" => settings.ai.activity.mouse.window_ms = setting_value(row)?,
        "ai.activity.frameDeltas.enabled" => {
            settings.ai.activity.frame_deltas.enabled = setting_value(row)?;
        }
        _ => {}
    }

    Ok(())
}

fn setting_value<T: serde::de::DeserializeOwned>(
    row: &AppSettingRow,
) -> Result<T, AppSettingsError> {
    serde_json::from_str(&row.value_json).map_err(|error| AppSettingsError {
        code: "settings_invalid_value".to_string(),
        field: Some(row.key.clone()),
        message: format!("Stored setting value is invalid: {error}"),
    })
}

fn settings_to_storage_inputs(
    settings: &AppSettings,
    timestamp_ms: i64,
) -> Result<Vec<SaveAppSettingInput>, AppSettingsError> {
    Ok(vec![
        setting_input(
            "debug.enabled",
            &settings.debug.enabled,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "expert.enabled",
            &settings.expert.enabled,
            false,
            timestamp_ms,
        )?,
        setting_input("record.fps", &settings.record.fps, false, timestamp_ms)?,
        setting_input(
            "record.videoBitrateKbps",
            &settings.record.video_bitrate_kbps,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "storage.retentionDays",
            &settings.storage.retention_days,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "storage.sizeLimitBytes",
            &settings.storage.size_limit_bytes,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.transcription.provider",
            &settings.ai.transcription.provider,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.transcription.localModelId",
            &settings.ai.transcription.local_model_id,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.transcription.localBackend",
            &settings.ai.transcription.local_backend,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.transcription.language",
            &settings.ai.transcription.language,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.transcription.maxInitialTimestampSeconds",
            &settings.ai.transcription.max_initial_timestamp_seconds,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.transcription.mergeGapMs",
            &settings.ai.transcription.merge_gap_ms,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.transcription.minSpeechMs",
            &settings.ai.transcription.min_speech_ms,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.transcription.remoteBaseUrl",
            &settings.ai.transcription.remote_base_url,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.transcription.remoteModel",
            &settings.ai.transcription.remote_model,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.transcription.remoteApiKeyRef",
            &settings.ai.transcription.remote_api_key_ref,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.transcription.silenceGateEnabled",
            &settings.ai.transcription.silence_gate_enabled,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.transcription.silenceThresholdDb",
            &settings.ai.transcription.silence_threshold_db,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.transcription.wordTimestampsEnabled",
            &settings.ai.transcription.word_timestamps_enabled,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.vision.provider",
            &settings.ai.vision.provider,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.vision.baseUrl",
            &settings.ai.vision.base_url,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.vision.apiKeyRef",
            &settings.ai.vision.api_key_ref,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.vision.model",
            &settings.ai.vision.model,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.ocr.enabled",
            &settings.ai.ocr.enabled,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.ocr.engine",
            &settings.ai.ocr.engine,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.processing.autoStartAfterRecording",
            &settings.ai.processing.auto_start_after_recording,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.processing.maxParallelJobs",
            &settings.ai.processing.max_parallel_jobs,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.processing.maxParallelChunksPerJob",
            &settings.ai.processing.max_parallel_chunks_per_job,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.processing.frameRecognitionFps",
            &settings.ai.processing.frame_recognition_fps,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.processing.frameRecognitionSyncWithRecording",
            &settings.ai.processing.frame_recognition_sync_with_recording,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.processing.periodicFullFrameIntervalMs",
            &settings.ai.processing.periodic_full_frame_interval_ms,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.processing.frameSimilarityEnabled",
            &settings.ai.processing.frame_similarity_enabled,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.processing.maxFullFrameCheckpointsPerMinute",
            &settings.ai.processing.max_full_frame_checkpoints_per_minute,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.processing.visualDeltaTileSizePx",
            &settings.ai.processing.visual_delta_tile_size_px,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.activity.keyboard.enabled",
            &settings.ai.activity.keyboard.enabled,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.activity.keyboard.windowMs",
            &settings.ai.activity.keyboard.window_ms,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.activity.mouse.enabled",
            &settings.ai.activity.mouse.enabled,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.activity.mouse.windowMs",
            &settings.ai.activity.mouse.window_ms,
            false,
            timestamp_ms,
        )?,
        setting_input(
            "ai.activity.frameDeltas.enabled",
            &settings.ai.activity.frame_deltas.enabled,
            false,
            timestamp_ms,
        )?,
    ])
}

fn setting_input(
    key: &str,
    value: &impl Serialize,
    is_sensitive: bool,
    timestamp_ms: i64,
) -> Result<SaveAppSettingInput, AppSettingsError> {
    Ok(SaveAppSettingInput {
        key: key.to_string(),
        value_json: serde_json::to_string(value).map_err(|error| AppSettingsError {
            code: "settings_invalid_value".to_string(),
            field: Some(key.to_string()),
            message: format!("Setting value could not be serialized: {error}"),
        })?,
        is_sensitive,
        timestamp_ms,
    })
}

fn validate_settings(settings: AppSettings) -> Result<AppSettings, AppSettingsError> {
    validate_f64_range(
        "record.fps",
        settings.record.fps,
        MIN_RECORD_FPS,
        MAX_RECORD_FPS,
        "Frame rate must be between one frame every 60 seconds and 5 frames per second.",
    )?;
    validate_u32_range(
        "record.videoBitrateKbps",
        settings.record.video_bitrate_kbps,
        MIN_VIDEO_BITRATE_KBPS,
        MAX_VIDEO_BITRATE_KBPS,
        "Video bitrate must be between 250 and 10000 kbps.",
    )?;
    validate_u32_range(
        "storage.retentionDays",
        settings.storage.retention_days,
        MIN_STORAGE_RETENTION_DAYS,
        MAX_STORAGE_RETENTION_DAYS,
        "Retention must be between 1 and 365 days.",
    )?;
    if settings.storage.size_limit_bytes < MIN_STORAGE_SIZE_LIMIT_BYTES {
        return Err(validation_error(
            "storage.sizeLimitBytes",
            "Storage size limit must be at least 1 GiB.",
        ));
    }
    validate_ai_settings(&settings.ai)?;

    Ok(settings)
}

fn validate_ai_settings(settings: &AiSettings) -> Result<(), AppSettingsError> {
    validate_one_of(
        "ai.transcription.provider",
        &settings.transcription.provider,
        &["local-whisper-cpp", "openai-compatible"],
        "Transcription provider must be local Whisper or OpenAI-compatible.",
    )?;
    validate_one_of(
        "ai.transcription.localModelId",
        &settings.transcription.local_model_id,
        &[
            "tiny",
            "base",
            "small",
            "medium",
            "large-v3-turbo",
            "large-v3",
        ],
        "Local transcription model must be one of the supported multilingual models.",
    )?;
    validate_transcription_backend(&settings.transcription.local_backend)?;
    validate_transcription_language(&settings.transcription.language)?;
    validate_f64_range(
        "ai.transcription.maxInitialTimestampSeconds",
        settings.transcription.max_initial_timestamp_seconds,
        MIN_AI_TRANSCRIPTION_MAX_INITIAL_TIMESTAMP_SECONDS,
        MAX_AI_TRANSCRIPTION_MAX_INITIAL_TIMESTAMP_SECONDS,
        "Max initial timestamp must be between 1 and 10 seconds.",
    )?;
    validate_u32_range(
        "ai.transcription.mergeGapMs",
        settings.transcription.merge_gap_ms,
        MIN_AI_TRANSCRIPTION_MERGE_GAP_MS,
        MAX_AI_TRANSCRIPTION_MERGE_GAP_MS,
        "Transcription merge gap must be between 0 and 5000 ms.",
    )?;
    validate_u32_range(
        "ai.transcription.minSpeechMs",
        settings.transcription.min_speech_ms,
        MIN_AI_TRANSCRIPTION_MIN_SPEECH_MS,
        MAX_AI_TRANSCRIPTION_MIN_SPEECH_MS,
        "Minimum speech duration must be between 50 and 2000 ms.",
    )?;
    validate_optional_url(
        "ai.transcription.remoteBaseUrl",
        settings.transcription.remote_base_url.as_deref(),
    )?;
    validate_optional_text(
        "ai.transcription.remoteModel",
        settings.transcription.remote_model.as_deref(),
        120,
    )?;
    validate_f64_range(
        "ai.transcription.silenceThresholdDb",
        settings.transcription.silence_threshold_db,
        MIN_AI_TRANSCRIPTION_SILENCE_THRESHOLD_DB,
        MAX_AI_TRANSCRIPTION_SILENCE_THRESHOLD_DB,
        "Silence threshold must be between -80 and -20 dBFS.",
    )?;
    validate_one_of(
        "ai.vision.provider",
        &settings.vision.provider,
        &["disabled", "openai-compatible"],
        "Vision provider must be disabled or OpenAI-compatible.",
    )?;
    validate_optional_url("ai.vision.baseUrl", settings.vision.base_url.as_deref())?;
    validate_one_of(
        "ai.ocr.engine",
        &settings.ocr.engine,
        &["tesseract"],
        "OCR engine must be Tesseract.",
    )?;
    validate_u32_range(
        "ai.processing.maxParallelJobs",
        settings.processing.max_parallel_jobs,
        MIN_AI_MAX_PARALLEL_JOBS,
        MAX_AI_MAX_PARALLEL_JOBS,
        "Max parallel jobs must be between 1 and 4.",
    )?;
    validate_u32_range(
        "ai.processing.maxParallelChunksPerJob",
        settings.processing.max_parallel_chunks_per_job,
        MIN_AI_MAX_PARALLEL_CHUNKS_PER_JOB,
        MAX_AI_MAX_PARALLEL_CHUNKS_PER_JOB,
        "Max parallel chunks per job must be between 1 and 8.",
    )?;
    validate_f64_range(
        "ai.processing.frameRecognitionFps",
        settings.processing.frame_recognition_fps,
        MIN_AI_FRAME_RECOGNITION_FPS,
        MAX_AI_FRAME_RECOGNITION_FPS,
        "Frame recognition FPS must be between 1/60 and 5.",
    )?;
    validate_u32_range(
        "ai.processing.periodicFullFrameIntervalMs",
        settings.processing.periodic_full_frame_interval_ms,
        MIN_AI_PERIODIC_FULL_FRAME_INTERVAL_MS,
        MAX_AI_PERIODIC_FULL_FRAME_INTERVAL_MS,
        "Periodic full-frame interval must be between 5 seconds and 10 minutes.",
    )?;
    validate_u32_range(
        "ai.processing.maxFullFrameCheckpointsPerMinute",
        settings.processing.max_full_frame_checkpoints_per_minute,
        MIN_AI_MAX_FULL_FRAME_CHECKPOINTS_PER_MINUTE,
        MAX_AI_MAX_FULL_FRAME_CHECKPOINTS_PER_MINUTE,
        "Max full-frame checkpoints must be between 1 and 60 per minute.",
    )?;
    validate_u32_range(
        "ai.processing.visualDeltaTileSizePx",
        settings.processing.visual_delta_tile_size_px,
        MIN_AI_VISUAL_DELTA_TILE_SIZE_PX,
        MAX_AI_VISUAL_DELTA_TILE_SIZE_PX,
        "Visual delta tile size must be between 32 and 256 px.",
    )?;
    validate_u32_range(
        "ai.activity.keyboard.windowMs",
        settings.activity.keyboard.window_ms,
        MIN_AI_ACTIVITY_WINDOW_MS,
        MAX_AI_ACTIVITY_WINDOW_MS,
        "Keyboard activity window must be between 1 second and 5 minutes.",
    )?;
    validate_u32_range(
        "ai.activity.mouse.windowMs",
        settings.activity.mouse.window_ms,
        MIN_AI_ACTIVITY_WINDOW_MS,
        MAX_AI_ACTIVITY_WINDOW_MS,
        "Mouse activity window must be between 1 second and 5 minutes.",
    )?;

    Ok(())
}

fn coerce_stored_transcription_backend(value: String) -> String {
    if transcription_backend_available(&value) {
        value
    } else {
        DEFAULT_AI_TRANSCRIPTION_BACKEND.to_string()
    }
}

fn validate_transcription_backend(value: &str) -> Result<(), AppSettingsError> {
    validate_one_of(
        "ai.transcription.localBackend",
        value,
        &["cpu", "vulkan", "cuda"],
        "Local transcription backend must be CPU, Vulkan, or CUDA.",
    )?;
    if transcription_backend_available(value) {
        return Ok(());
    }

    Err(validation_error(
        "ai.transcription.localBackend",
        "Selected local transcription backend is not available in this build.",
    ))
}

fn validate_transcription_language(value: &str) -> Result<(), AppSettingsError> {
    if value == DEFAULT_AI_TRANSCRIPTION_LANGUAGE || whisper_backend::language_supported(value) {
        return Ok(());
    }

    Err(validation_error(
        "ai.transcription.language",
        "Transcription language must be auto or one of the supported Whisper languages.",
    ))
}

fn transcription_backend_available(value: &str) -> bool {
    match value {
        "cpu" => true,
        "vulkan" => cfg!(feature = "whisper-vulkan"),
        "cuda" => cfg!(feature = "whisper-cuda"),
        _ => false,
    }
}

fn validate_one_of(
    field: &str,
    value: &str,
    allowed: &[&str],
    message: &str,
) -> Result<(), AppSettingsError> {
    if !allowed.contains(&value) {
        return Err(validation_error(field, message));
    }

    Ok(())
}

fn validate_optional_url(field: &str, value: Option<&str>) -> Result<(), AppSettingsError> {
    let Some(value) = value else {
        return Ok(());
    };

    if value.starts_with("http://") || value.starts_with("https://") {
        return Ok(());
    }

    Err(validation_error(
        field,
        "Remote endpoint URL must start with http:// or https://.",
    ))
}

fn validate_optional_text(
    field: &str,
    value: Option<&str>,
    max_chars: usize,
) -> Result<(), AppSettingsError> {
    let Some(value) = value else {
        return Ok(());
    };
    if value.chars().count() <= max_chars {
        return Ok(());
    }

    Err(validation_error(field, "Setting value is too long."))
}

fn validate_f64_range(
    field: &str,
    value: f64,
    min: f64,
    max: f64,
    message: &str,
) -> Result<(), AppSettingsError> {
    if !value.is_finite() || value < min || value > max {
        return Err(validation_error(field, message));
    }

    Ok(())
}

fn validate_u32_range(
    field: &str,
    value: u32,
    min: u32,
    max: u32,
    message: &str,
) -> Result<(), AppSettingsError> {
    if value < min || value > max {
        return Err(validation_error(field, message));
    }

    Ok(())
}

fn validation_error(field: &str, message: &str) -> AppSettingsError {
    AppSettingsError {
        code: "settings_invalid_value".to_string(),
        field: Some(field.to_string()),
        message: message.to_string(),
    }
}

fn storage_error(error: rusqlite::Error) -> AppSettingsError {
    AppSettingsError {
        code: "settings_storage_error".to_string(),
        field: None,
        message: format!("Settings storage failed: {error}"),
    }
}

fn cleanup_error(message: String) -> AppSettingsError {
    AppSettingsError {
        code: "settings_cleanup_preview_failed".to_string(),
        field: None,
        message,
    }
}

fn recordings_root(app: &tauri::AppHandle) -> Result<PathBuf, AppSettingsError> {
    app.path()
        .app_data_dir()
        .map(|path| path.join("recordings"))
        .map_err(|error| AppSettingsError {
            code: "settings_storage_error".to_string(),
            field: None,
            message: format!("Settings storage failed: {error}"),
        })
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::storage::{self, SaveAppSettingInput};

    use super::*;

    #[test]
    fn load_app_settings_returns_defaults_without_saved_row() {
        let storage = storage::test_storage(PathBuf::from("/tmp/daydream.sqlite3"));
        let settings = load_app_settings(&storage).expect("load settings");

        assert_eq!(settings, default_settings());
    }

    #[test]
    fn load_app_settings_returns_saved_values() {
        let storage = storage::test_storage(PathBuf::from("/tmp/daydream.sqlite3"));
        storage
            .with_connection_mut(|connection| {
                storage::save_app_settings(
                    connection,
                    &[
                        SaveAppSettingInput {
                            key: "record.fps".to_string(),
                            value_json: "5.0".to_string(),
                            is_sensitive: false,
                            timestamp_ms: 1,
                        },
                        SaveAppSettingInput {
                            key: "record.videoBitrateKbps".to_string(),
                            value_json: "2500".to_string(),
                            is_sensitive: false,
                            timestamp_ms: 1,
                        },
                        SaveAppSettingInput {
                            key: "storage.retentionDays".to_string(),
                            value_json: "30".to_string(),
                            is_sensitive: false,
                            timestamp_ms: 1,
                        },
                        SaveAppSettingInput {
                            key: "storage.sizeLimitBytes".to_string(),
                            value_json: (75_u64 * 1024 * 1024 * 1024).to_string(),
                            is_sensitive: false,
                            timestamp_ms: 1,
                        },
                        SaveAppSettingInput {
                            key: "ai.transcription.language".to_string(),
                            value_json: "\"de\"".to_string(),
                            is_sensitive: false,
                            timestamp_ms: 1,
                        },
                    ],
                )
            })
            .expect("save settings");

        let settings = load_app_settings(&storage).expect("load settings");

        assert_eq!(settings.record.fps, 5.0);
        assert_eq!(settings.record.video_bitrate_kbps, 2_500);
        assert_eq!(settings.storage.retention_days, 30);
        assert_eq!(settings.storage.size_limit_bytes, 75 * 1024 * 1024 * 1024);
        assert_eq!(settings.ai.transcription.language, "de");
    }

    #[test]
    fn load_app_settings_handles_saved_vulkan_by_build_feature() {
        let storage = storage::test_storage(PathBuf::from("/tmp/daydream.sqlite3"));
        storage
            .with_connection_mut(|connection| {
                storage::save_app_setting(
                    connection,
                    &SaveAppSettingInput {
                        key: "ai.transcription.localBackend".to_string(),
                        value_json: "\"vulkan\"".to_string(),
                        is_sensitive: false,
                        timestamp_ms: 1,
                    },
                )
            })
            .expect("save settings");

        let settings = load_app_settings(&storage).expect("load settings");

        if cfg!(feature = "whisper-vulkan") {
            assert_eq!(settings.ai.transcription.local_backend, "vulkan");
        } else {
            assert_eq!(
                settings.ai.transcription.local_backend,
                DEFAULT_AI_TRANSCRIPTION_BACKEND
            );
        }
    }

    #[test]
    fn validate_settings_rejects_out_of_range_values() {
        for (settings, field) in [
            (
                AppSettings {
                    record: RecordSettings {
                        fps: 0.0,
                        ..default_settings().record
                    },
                    ..default_settings()
                },
                "record.fps",
            ),
            (
                AppSettings {
                    record: RecordSettings {
                        video_bitrate_kbps: 10_001,
                        ..default_settings().record
                    },
                    ..default_settings()
                },
                "record.videoBitrateKbps",
            ),
            (
                AppSettings {
                    storage: StorageSettings {
                        retention_days: 366,
                        ..default_settings().storage
                    },
                    ..default_settings()
                },
                "storage.retentionDays",
            ),
            (
                AppSettings {
                    storage: StorageSettings {
                        size_limit_bytes: MIN_STORAGE_SIZE_LIMIT_BYTES - 1,
                        ..default_settings().storage
                    },
                    ..default_settings()
                },
                "storage.sizeLimitBytes",
            ),
        ] {
            let error =
                validate_settings(settings).expect_err("invalid settings should fail validation");

            assert_eq!(error.field, Some(field.to_string()));
        }
    }

    #[test]
    fn validate_settings_rejects_invalid_ai_values() {
        for (settings, field) in [
            (
                AppSettings {
                    ai: AiSettings {
                        transcription: AiTranscriptionProviderSettings {
                            provider: "remote".to_string(),
                            ..default_settings().ai.transcription
                        },
                        ..default_settings().ai
                    },
                    ..default_settings()
                },
                "ai.transcription.provider",
            ),
            (
                AppSettings {
                    ai: AiSettings {
                        transcription: AiTranscriptionProviderSettings {
                            local_model_id: "small.en".to_string(),
                            ..default_settings().ai.transcription
                        },
                        ..default_settings().ai
                    },
                    ..default_settings()
                },
                "ai.transcription.localModelId",
            ),
            (
                AppSettings {
                    ai: AiSettings {
                        transcription: AiTranscriptionProviderSettings {
                            local_backend: "metal".to_string(),
                            ..default_settings().ai.transcription
                        },
                        ..default_settings().ai
                    },
                    ..default_settings()
                },
                "ai.transcription.localBackend",
            ),
            (
                AppSettings {
                    ai: AiSettings {
                        transcription: AiTranscriptionProviderSettings {
                            language: "klingon".to_string(),
                            ..default_settings().ai.transcription
                        },
                        ..default_settings().ai
                    },
                    ..default_settings()
                },
                "ai.transcription.language",
            ),
            (
                AppSettings {
                    ai: AiSettings {
                        transcription: AiTranscriptionProviderSettings {
                            remote_base_url: Some("file:///tmp/model".to_string()),
                            ..default_settings().ai.transcription
                        },
                        ..default_settings().ai
                    },
                    ..default_settings()
                },
                "ai.transcription.remoteBaseUrl",
            ),
            (
                AppSettings {
                    ai: AiSettings {
                        transcription: AiTranscriptionProviderSettings {
                            max_initial_timestamp_seconds:
                                MAX_AI_TRANSCRIPTION_MAX_INITIAL_TIMESTAMP_SECONDS + 1.0,
                            ..default_settings().ai.transcription
                        },
                        ..default_settings().ai
                    },
                    ..default_settings()
                },
                "ai.transcription.maxInitialTimestampSeconds",
            ),
            (
                AppSettings {
                    ai: AiSettings {
                        transcription: AiTranscriptionProviderSettings {
                            silence_threshold_db: MIN_AI_TRANSCRIPTION_SILENCE_THRESHOLD_DB - 1.0,
                            ..default_settings().ai.transcription
                        },
                        ..default_settings().ai
                    },
                    ..default_settings()
                },
                "ai.transcription.silenceThresholdDb",
            ),
            (
                AppSettings {
                    ai: AiSettings {
                        transcription: AiTranscriptionProviderSettings {
                            min_speech_ms: MIN_AI_TRANSCRIPTION_MIN_SPEECH_MS - 1,
                            ..default_settings().ai.transcription
                        },
                        ..default_settings().ai
                    },
                    ..default_settings()
                },
                "ai.transcription.minSpeechMs",
            ),
            (
                AppSettings {
                    ai: AiSettings {
                        transcription: AiTranscriptionProviderSettings {
                            merge_gap_ms: MAX_AI_TRANSCRIPTION_MERGE_GAP_MS + 1,
                            ..default_settings().ai.transcription
                        },
                        ..default_settings().ai
                    },
                    ..default_settings()
                },
                "ai.transcription.mergeGapMs",
            ),
            (
                AppSettings {
                    ai: AiSettings {
                        processing: AiProcessingSettings {
                            max_parallel_jobs: MAX_AI_MAX_PARALLEL_JOBS + 1,
                            ..default_settings().ai.processing
                        },
                        ..default_settings().ai
                    },
                    ..default_settings()
                },
                "ai.processing.maxParallelJobs",
            ),
            (
                AppSettings {
                    ai: AiSettings {
                        processing: AiProcessingSettings {
                            frame_recognition_fps: MIN_AI_FRAME_RECOGNITION_FPS / 2.0,
                            ..default_settings().ai.processing
                        },
                        ..default_settings().ai
                    },
                    ..default_settings()
                },
                "ai.processing.frameRecognitionFps",
            ),
            (
                AppSettings {
                    ai: AiSettings {
                        processing: AiProcessingSettings {
                            periodic_full_frame_interval_ms: MIN_AI_PERIODIC_FULL_FRAME_INTERVAL_MS
                                - 1,
                            ..default_settings().ai.processing
                        },
                        ..default_settings().ai
                    },
                    ..default_settings()
                },
                "ai.processing.periodicFullFrameIntervalMs",
            ),
            (
                AppSettings {
                    ai: AiSettings {
                        processing: AiProcessingSettings {
                            max_full_frame_checkpoints_per_minute:
                                MAX_AI_MAX_FULL_FRAME_CHECKPOINTS_PER_MINUTE + 1,
                            ..default_settings().ai.processing
                        },
                        ..default_settings().ai
                    },
                    ..default_settings()
                },
                "ai.processing.maxFullFrameCheckpointsPerMinute",
            ),
            (
                AppSettings {
                    ai: AiSettings {
                        processing: AiProcessingSettings {
                            visual_delta_tile_size_px: MIN_AI_VISUAL_DELTA_TILE_SIZE_PX - 1,
                            ..default_settings().ai.processing
                        },
                        ..default_settings().ai
                    },
                    ..default_settings()
                },
                "ai.processing.visualDeltaTileSizePx",
            ),
        ] {
            let error = validate_settings(settings)
                .expect_err("invalid AI settings should fail validation");

            assert_eq!(error.field, Some(field.to_string()));
        }
    }
}
