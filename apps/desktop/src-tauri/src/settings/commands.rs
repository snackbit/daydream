use tauri::State;

use crate::{
    storage::{self, AppStorage},
    storage_limits::{self, StorageCleanupPreview},
};

use super::{
    default_settings, input_activity,
    service::{
        cleanup_error, enforce_storage_cleanup_for_settings, load_app_settings, recordings_root,
        settings_state, settings_state_with_cleanup, storage_error,
    },
    types::*,
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
    super::validation::validate_storage_settings(&input.settings.storage)?;
    let recordings_root = recordings_root(&app)?;

    storage_limits::preview_storage_cleanup(&storage, &recordings_root, &input.settings.storage)
        .map_err(cleanup_error)
}

#[tauri::command(rename = "saveAppSettings")]
pub fn save_app_settings(
    app: tauri::AppHandle,
    input: SaveAppSettingsInput,
    storage: State<'_, AppStorage>,
) -> Result<AppSettingsState, AppSettingsError> {
    super::credentials::save(&storage, input).map(|saved_settings| {
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

#[tauri::command(rename = "getInputActivityStatus")]
pub fn get_input_activity_status() -> InputActivityStatus {
    input_activity::input_activity_status()
}

#[tauri::command(rename = "repairInputActivityPermissions")]
pub fn repair_input_activity_permissions() -> InputActivityRepairResult {
    input_activity::repair_input_activity_permissions()
}
