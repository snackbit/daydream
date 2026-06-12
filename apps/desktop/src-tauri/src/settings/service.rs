use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use tauri::Manager;

use crate::{
    storage::{self, AppStorage},
    storage_limits::{self, StorageCleanupResult},
};

use super::{
    defaults::{default_settings, settings_limits},
    persistence::settings_from_storage_rows,
    types::*,
};

pub fn load_app_settings(storage: &AppStorage) -> Result<AppSettings, AppSettingsError> {
    let rows = storage
        .with_connection(storage::list_app_settings)
        .map_err(storage_error)?;

    settings_from_storage_rows(rows)
}

pub(super) fn settings_state(settings: AppSettings) -> AppSettingsState {
    settings_state_with_cleanup(settings, None)
}

pub(super) fn settings_state_with_cleanup(
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

pub(super) fn enforce_storage_cleanup_for_settings(
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

pub(super) fn storage_error(error: rusqlite::Error) -> AppSettingsError {
    AppSettingsError {
        code: "settings_storage_error".to_string(),
        field: None,
        message: format!("Settings storage failed: {error}"),
    }
}

pub(super) fn cleanup_error(message: String) -> AppSettingsError {
    AppSettingsError {
        code: "settings_cleanup_preview_failed".to_string(),
        field: None,
        message,
    }
}

pub(super) fn recordings_root(app: &tauri::AppHandle) -> Result<PathBuf, AppSettingsError> {
    app.path()
        .app_data_dir()
        .map(|path| path.join("recordings"))
        .map_err(|error| AppSettingsError {
            code: "settings_storage_error".to_string(),
            field: None,
            message: format!("Settings storage failed: {error}"),
        })
}

pub(super) fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}
