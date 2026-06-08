use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use tauri::{Manager, State};

use crate::{
    storage::{
        self, AppSettingsRow, AppStorage, SaveAppSettingsInput as StorageSaveAppSettingsInput,
    },
    storage_limits::{self, StorageCleanupPreview, StorageCleanupResult},
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
    pub record: RecordSettings,
    pub storage: StorageSettings,
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
    pub record: RecordSettingsLimits,
    pub storage: StorageSettingsLimits,
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
    let storage_input = StorageSaveAppSettingsInput {
        record_fps: settings.record.fps,
        record_video_bitrate_kbps: settings.record.video_bitrate_kbps.into(),
        storage_retention_days: settings.storage.retention_days.into(),
        storage_size_limit_bytes: settings.storage.size_limit_bytes.try_into().map_err(
            |_error| validation_error("storage.sizeLimitBytes", "Storage size limit is too large."),
        )?,
        updated_at_ms: now_ms() as i64,
    };

    storage
        .with_connection(|connection| storage::save_app_settings(connection, &storage_input))
        .map_err(storage_error)
        .and_then(settings_from_storage)
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
    let row = storage
        .with_connection(storage::get_app_settings)
        .map_err(storage_error)?;

    row.map(settings_from_storage)
        .transpose()
        .map(|settings| settings.unwrap_or_else(default_settings))
}

pub fn default_settings() -> AppSettings {
    AppSettings {
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

pub fn settings_limits() -> AppSettingsLimits {
    AppSettingsLimits {
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

fn settings_from_storage(row: AppSettingsRow) -> Result<AppSettings, AppSettingsError> {
    validate_settings(AppSettings {
        record: RecordSettings {
            fps: row.record_fps,
            video_bitrate_kbps: row.record_video_bitrate_kbps.try_into().map_err(|_error| {
                validation_error("record.videoBitrateKbps", "Video bitrate must be positive.")
            })?,
        },
        storage: StorageSettings {
            retention_days: row.storage_retention_days.try_into().map_err(|_error| {
                validation_error("storage.retentionDays", "Retention must be positive.")
            })?,
            size_limit_bytes: row.storage_size_limit_bytes.try_into().map_err(|_error| {
                validation_error(
                    "storage.sizeLimitBytes",
                    "Storage size limit must be positive.",
                )
            })?,
        },
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

    Ok(settings)
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

    use crate::storage::{self, SaveAppSettingsInput as StorageSaveAppSettingsInput};

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
            .with_connection(|connection| {
                storage::save_app_settings(
                    connection,
                    &StorageSaveAppSettingsInput {
                        record_fps: 5.0,
                        record_video_bitrate_kbps: 2_500,
                        storage_retention_days: 30,
                        storage_size_limit_bytes: 75 * 1024 * 1024 * 1024,
                        updated_at_ms: 1,
                    },
                )
            })
            .expect("save settings");

        let settings = load_app_settings(&storage).expect("load settings");

        assert_eq!(settings.record.fps, 5.0);
        assert_eq!(settings.record.video_bitrate_kbps, 2_500);
        assert_eq!(settings.storage.retention_days, 30);
        assert_eq!(settings.storage.size_limit_bytes, 75 * 1024 * 1024 * 1024);
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
}
