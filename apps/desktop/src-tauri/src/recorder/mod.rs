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
mod operation_timing;
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

use operation_timing::run_blocking;
use tauri::{Manager, State};

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
pub async fn get_recorder_recording_stats<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
) -> Result<Option<RecorderRecordingStats>, RecorderError> {
    run_blocking("recording_stats", move || {
        commands::get_recorder_recording_stats(
            app.state::<AppStorage>(),
            app.state::<RecorderManager>(),
        )
    })
    .await
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
pub async fn get_capture_sources<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
) -> Result<CaptureSourceList, RecorderError> {
    run_blocking("capture_sources", move || {
        commands::get_capture_sources(app.state::<RecorderManager>())
    })
    .await
}

#[tauri::command(rename = "getRecorderScene")]
pub async fn get_recorder_scene<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
) -> Result<Option<RecorderScene>, RecorderError> {
    run_blocking("capture_scene_read", move || {
        commands::get_recorder_scene(app.state::<AppStorage>())
    })
    .await
}

#[tauri::command(rename = "selectCaptureSources")]
pub async fn select_capture_sources<R: tauri::Runtime>(
    input: Option<SelectCaptureSourcesInput>,
    app: tauri::AppHandle<R>,
) -> Result<CaptureSourceList, RecorderError> {
    run_blocking("capture_selection", move || {
        let _activity = crate::maintenance::runtime::admit(&app, "capture-selection")
            .map_err(|message| errors::lifecycle_error("recorder_maintenance_active", &message))?;
        tauri::async_runtime::block_on(commands::select_capture_sources(
            input,
            app.state::<AppStorage>(),
            app.state::<RecorderManager>(),
        ))
    })
    .await
}

#[tauri::command(rename = "getAudioSources")]
pub async fn get_audio_sources() -> Result<AudioSourceList, RecorderError> {
    commands::get_audio_sources().await
}

#[tauri::command(rename = "saveRecorderScene")]
pub async fn save_recorder_scene<R: tauri::Runtime>(
    input: SaveRecorderSceneInput,
    app: tauri::AppHandle<R>,
) -> Result<RecorderScene, RecorderError> {
    run_blocking("capture_scene_save", move || {
        let _activity = crate::maintenance::runtime::admit(&app, "capture-configure")
            .map_err(|message| errors::lifecycle_error("recorder_maintenance_active", &message))?;
        commands::save_recorder_scene(
            input,
            app.state::<AppStorage>(),
            app.state::<RecorderManager>(),
        )
    })
    .await
}

#[tauri::command(rename = "getRecorderPreviewStatus")]
pub async fn get_recorder_preview_status<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
) -> Result<RecorderPreviewStatus, RecorderError> {
    run_blocking("preview_status", move || {
        commands::get_recorder_preview_status(app.state::<RecorderManager>())
    })
    .await
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
pub async fn stop_recorder_preview<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
) -> Result<RecorderPreviewStatus, RecorderError> {
    run_blocking("preview_stop", move || {
        commands::stop_recorder_preview(app.state::<RecorderManager>())
    })
    .await
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
