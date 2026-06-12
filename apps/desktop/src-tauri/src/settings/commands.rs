use tauri::State;

use crate::{
    storage::{self, AppStorage},
    storage_limits::{self, StorageCleanupPreview},
};

use super::{
    default_settings,
    persistence::{settings_from_storage_rows, settings_to_storage_inputs},
    service::{
        cleanup_error, enforce_storage_cleanup_for_settings, load_app_settings, now_ms,
        recordings_root, settings_state, settings_state_with_cleanup, storage_error,
    },
    types::*,
    validation::validate_settings,
};

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
