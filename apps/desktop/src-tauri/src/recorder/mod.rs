mod activity_input;
mod benchmark;
mod capture_runtime;
mod commands;
mod compositor;
mod errors;
mod manager;
#[cfg(target_os = "linux")]
pub(crate) mod media_worker;
#[cfg(target_os = "linux")]
pub(crate) mod media_writer;
#[cfg(not(target_os = "linux"))]
#[path = "media_writer_unsupported.rs"]
pub(crate) mod media_writer;
mod platform;
mod preview;
#[cfg(target_os = "linux")]
pub(crate) mod recovery;
#[cfg(not(target_os = "linux"))]
#[path = "recovery_unsupported.rs"]
pub(crate) mod recovery;
pub(crate) mod source_recording;
mod storage_mapping;
#[cfg(test)]
mod tests;
mod types;
mod window_application_icons;
mod window_protection;

pub use manager::RecorderManager;
pub use types::*;

use tauri::State;

use crate::storage::AppStorage;

pub async fn collect_backend_health() -> RecorderBackendHealth {
    platform::collect_backend_health().await
}

#[tauri::command(rename = "getRecorderStatus")]
pub async fn get_recorder_status(
    manager: State<'_, RecorderManager>,
) -> Result<RecorderStatus, RecorderError> {
    commands::get_recorder_status(manager).await
}

#[tauri::command(rename = "getRecorderPerformanceStats")]
pub fn get_recorder_performance_stats(
    manager: State<'_, RecorderManager>,
) -> Result<RecorderPerformanceStats, RecorderError> {
    commands::get_recorder_performance_stats(manager)
}

#[tauri::command(rename = "getRecorderRecordingStats")]
pub fn get_recorder_recording_stats(
    storage: State<'_, AppStorage>,
    manager: State<'_, RecorderManager>,
) -> Result<Option<RecorderRecordingStats>, RecorderError> {
    commands::get_recorder_recording_stats(storage, manager)
}

#[tauri::command(rename = "listProtectableWindows")]
pub async fn list_protectable_windows(
    app: tauri::AppHandle,
) -> window_application_icons::WindowPickerList {
    commands::list_protectable_windows(app).await
}

#[tauri::command(rename = "getWindowApplicationIcon")]
pub async fn get_window_application_icon(
    app: tauri::AppHandle,
    input: window_application_icons::GetWindowApplicationIconInput,
) -> window_application_icons::WindowApplicationIcon {
    window_application_icons::get_window_application_icon(app, input).await
}

#[tauri::command(rename = "getWindowProtectionStatus")]
pub fn get_window_protection_status() -> WindowProtectionStatus {
    commands::get_window_protection_status()
}

#[tauri::command(rename = "getCaptureSources")]
pub fn get_capture_sources(
    manager: State<'_, RecorderManager>,
) -> Result<CaptureSourceList, RecorderError> {
    commands::get_capture_sources(manager)
}

#[tauri::command(rename = "getRecorderScene")]
pub fn get_recorder_scene(
    storage: State<'_, AppStorage>,
) -> Result<Option<RecorderScene>, RecorderError> {
    commands::get_recorder_scene(storage)
}

#[tauri::command(rename = "selectCaptureSources")]
pub async fn select_capture_sources(
    input: Option<SelectCaptureSourcesInput>,
    app: tauri::AppHandle,
    storage: State<'_, AppStorage>,
    manager: State<'_, RecorderManager>,
) -> Result<CaptureSourceList, RecorderError> {
    let _activity = crate::maintenance::runtime::admit(&app, "capture-selection")
        .map_err(|message| errors::lifecycle_error("recorder_maintenance_active", &message))?;
    commands::select_capture_sources(input, storage, manager).await
}

#[tauri::command(rename = "getAudioSources")]
pub async fn get_audio_sources() -> Result<AudioSourceList, RecorderError> {
    commands::get_audio_sources().await
}

#[tauri::command(rename = "saveRecorderScene")]
pub fn save_recorder_scene(
    input: SaveRecorderSceneInput,
    app: tauri::AppHandle,
    storage: State<'_, AppStorage>,
    manager: State<'_, RecorderManager>,
) -> Result<RecorderScene, RecorderError> {
    let _activity = crate::maintenance::runtime::admit(&app, "capture-configure")
        .map_err(|message| errors::lifecycle_error("recorder_maintenance_active", &message))?;
    commands::save_recorder_scene(input, storage, manager)
}

#[tauri::command(rename = "getRecorderPreviewStatus")]
pub fn get_recorder_preview_status(
    manager: State<'_, RecorderManager>,
) -> Result<RecorderPreviewStatus, RecorderError> {
    commands::get_recorder_preview_status(manager)
}

#[tauri::command(rename = "startRecorderPreview")]
pub async fn start_recorder_preview(
    input: StartRecorderPreviewInput,
    app: tauri::AppHandle,
    storage: State<'_, AppStorage>,
    manager: State<'_, RecorderManager>,
) -> Result<RecorderPreviewStatus, RecorderError> {
    commands::start_recorder_preview(input, app, storage, manager).await
}

#[tauri::command(rename = "stopRecorderPreview")]
pub fn stop_recorder_preview(
    manager: State<'_, RecorderManager>,
) -> Result<RecorderPreviewStatus, RecorderError> {
    commands::stop_recorder_preview(manager)
}

#[tauri::command(rename = "updateRecorderPreviewDemand")]
pub fn update_recorder_preview_demand(
    input: RecorderPreviewDemand,
    manager: State<'_, RecorderManager>,
) -> Result<(), RecorderError> {
    commands::update_recorder_preview_demand(input, manager)
}

#[tauri::command(rename = "startRecording")]
pub async fn start_recording<R: tauri::Runtime>(
    input: StartRecordingInput,
    app: tauri::AppHandle<R>,
    storage: State<'_, AppStorage>,
    manager: State<'_, RecorderManager>,
) -> Result<RecorderStatus, RecorderError> {
    commands::start_recording(input, app, storage, manager).await
}

#[tauri::command(rename = "pauseRecording")]
pub fn pause_recording(
    input: PauseRecordingInput,
    app: tauri::AppHandle,
    manager: State<'_, RecorderManager>,
) -> Result<RecorderStatus, RecorderError> {
    commands::pause_recording(input, app, manager)
}

#[tauri::command(rename = "resumeRecording")]
pub async fn resume_recording<R: tauri::Runtime>(
    input: ResumeRecordingInput,
    app: tauri::AppHandle<R>,
    storage: State<'_, AppStorage>,
    manager: State<'_, RecorderManager>,
) -> Result<RecorderStatus, RecorderError> {
    commands::resume_recording(input, app, storage, manager).await
}

#[tauri::command(rename = "stopRecording")]
pub fn stop_recording(
    input: StopRecordingInput,
    app: tauri::AppHandle,
    manager: State<'_, RecorderManager>,
) -> Result<RecorderStatus, RecorderError> {
    commands::stop_recording(input, app, manager)
}
