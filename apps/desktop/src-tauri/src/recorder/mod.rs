mod commands;
mod errors;
mod manager;
mod storage_mapping;
#[cfg(test)]
mod tests;
mod types;

pub use manager::RecorderManager;
pub use types::*;

use tauri::State;

use crate::storage::AppStorage;

#[tauri::command(rename = "getRecorderStatus")]
pub fn get_recorder_status(
    manager: State<'_, RecorderManager>,
) -> Result<RecorderStatus, RecorderError> {
    commands::get_recorder_status(manager)
}

#[tauri::command(rename = "getCaptureSources")]
pub fn get_capture_sources() -> Result<CaptureSourceList, RecorderError> {
    commands::get_capture_sources()
}

#[tauri::command(rename = "getAudioSources")]
pub fn get_audio_sources() -> Result<AudioSourceList, RecorderError> {
    commands::get_audio_sources()
}

#[tauri::command(rename = "saveRecorderScene")]
pub fn save_recorder_scene(
    input: SaveRecorderSceneInput,
    storage: State<'_, AppStorage>,
) -> Result<RecorderScene, RecorderError> {
    commands::save_recorder_scene(input, storage)
}

#[tauri::command(rename = "startRecording")]
pub fn start_recording(input: StartRecordingInput) -> Result<RecorderStatus, RecorderError> {
    commands::start_recording(input)
}

#[tauri::command(rename = "pauseRecording")]
pub fn pause_recording(input: PauseRecordingInput) -> Result<RecorderStatus, RecorderError> {
    commands::pause_recording(input)
}

#[tauri::command(rename = "resumeRecording")]
pub fn resume_recording(input: ResumeRecordingInput) -> Result<RecorderStatus, RecorderError> {
    commands::resume_recording(input)
}

#[tauri::command(rename = "stopRecording")]
pub fn stop_recording(input: StopRecordingInput) -> Result<RecorderStatus, RecorderError> {
    commands::stop_recording(input)
}
