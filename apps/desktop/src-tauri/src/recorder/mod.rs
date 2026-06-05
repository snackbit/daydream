mod commands;
mod errors;
mod manager;
mod platform;
mod storage_mapping;
#[cfg(test)]
mod tests;
mod types;

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
    storage: State<'_, AppStorage>,
    manager: State<'_, RecorderManager>,
) -> Result<CaptureSourceList, RecorderError> {
    commands::select_capture_sources(input, storage, manager).await
}

#[tauri::command(rename = "getAudioSources")]
pub async fn get_audio_sources() -> Result<AudioSourceList, RecorderError> {
    commands::get_audio_sources().await
}

#[tauri::command(rename = "saveRecorderScene")]
pub fn save_recorder_scene(
    input: SaveRecorderSceneInput,
    storage: State<'_, AppStorage>,
) -> Result<RecorderScene, RecorderError> {
    commands::save_recorder_scene(input, storage)
}

#[tauri::command(rename = "startRecording")]
pub fn start_recording(
    input: StartRecordingInput,
    storage: State<'_, AppStorage>,
    manager: State<'_, RecorderManager>,
) -> Result<RecorderStatus, RecorderError> {
    commands::start_recording(input, storage, manager)
}

#[tauri::command(rename = "pauseRecording")]
pub fn pause_recording(
    input: PauseRecordingInput,
    storage: State<'_, AppStorage>,
    manager: State<'_, RecorderManager>,
) -> Result<RecorderStatus, RecorderError> {
    commands::pause_recording(input, storage, manager)
}

#[tauri::command(rename = "resumeRecording")]
pub fn resume_recording(
    input: ResumeRecordingInput,
    storage: State<'_, AppStorage>,
    manager: State<'_, RecorderManager>,
) -> Result<RecorderStatus, RecorderError> {
    commands::resume_recording(input, storage, manager)
}

#[tauri::command(rename = "stopRecording")]
pub fn stop_recording(
    input: StopRecordingInput,
    storage: State<'_, AppStorage>,
    manager: State<'_, RecorderManager>,
) -> Result<RecorderStatus, RecorderError> {
    commands::stop_recording(input, storage, manager)
}
