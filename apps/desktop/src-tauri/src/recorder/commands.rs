use tauri::State;

use crate::storage::AppStorage;

use super::errors::unavailable_error;
use super::manager::{now_ms, RecorderManager};
use super::storage_mapping::save_recorder_scene_for_storage;
use super::types::{
    AudioSourceList, CaptureSourceList, PauseRecordingInput, RecorderError, RecorderScene,
    RecorderStatus, ResumeRecordingInput, SaveRecorderSceneInput, StartRecordingInput,
    StopRecordingInput,
};

pub fn get_recorder_status(
    manager: State<'_, RecorderManager>,
) -> Result<RecorderStatus, RecorderError> {
    manager.status()
}

pub fn get_capture_sources() -> Result<CaptureSourceList, RecorderError> {
    Err(unavailable_error())
}

pub fn get_audio_sources() -> Result<AudioSourceList, RecorderError> {
    Err(unavailable_error())
}

pub fn save_recorder_scene(
    input: SaveRecorderSceneInput,
    storage: State<'_, AppStorage>,
) -> Result<RecorderScene, RecorderError> {
    save_recorder_scene_for_storage(&storage, input, now_ms())
}

pub fn start_recording(_input: StartRecordingInput) -> Result<RecorderStatus, RecorderError> {
    Err(unavailable_error())
}

pub fn pause_recording(_input: PauseRecordingInput) -> Result<RecorderStatus, RecorderError> {
    Err(unavailable_error())
}

pub fn resume_recording(_input: ResumeRecordingInput) -> Result<RecorderStatus, RecorderError> {
    Err(unavailable_error())
}

pub fn stop_recording(_input: StopRecordingInput) -> Result<RecorderStatus, RecorderError> {
    Err(unavailable_error())
}
