use tauri::State;

use crate::storage::AppStorage;

use super::errors::unavailable_error;
use super::manager::{now_ms, RecorderManager};
use super::platform;
use super::storage_mapping::{latest_recorder_scene_for_storage, save_recorder_scene_for_storage};
use super::types::{
    AudioSourceList, CaptureSourceList, PauseRecordingInput, RecorderError, RecorderScene,
    RecorderStatus, ResumeRecordingInput, SaveRecorderSceneInput, SelectCaptureSourcesInput,
    StartRecordingInput, StopRecordingInput,
};

pub async fn get_recorder_status(
    manager: State<'_, RecorderManager>,
) -> Result<RecorderStatus, RecorderError> {
    manager.status_with_backend_health(platform::collect_backend_health().await, now_ms())
}

pub fn get_capture_sources(
    manager: State<'_, RecorderManager>,
) -> Result<CaptureSourceList, RecorderError> {
    platform::get_capture_sources(&manager)
}

pub fn get_recorder_scene(
    storage: State<'_, AppStorage>,
) -> Result<Option<RecorderScene>, RecorderError> {
    latest_recorder_scene_for_storage(&storage)
}

pub async fn select_capture_sources(
    input: Option<SelectCaptureSourcesInput>,
    storage: State<'_, AppStorage>,
    manager: State<'_, RecorderManager>,
) -> Result<CaptureSourceList, RecorderError> {
    platform::select_capture_sources(
        &storage,
        &manager,
        input.is_some_and(|input| input.force_chooser),
    )
    .await
}

pub async fn get_audio_sources() -> Result<AudioSourceList, RecorderError> {
    platform::get_audio_sources().await
}

pub fn save_recorder_scene(
    input: SaveRecorderSceneInput,
    storage: State<'_, AppStorage>,
) -> Result<RecorderScene, RecorderError> {
    save_recorder_scene_for_storage(&storage, input, now_ms())
}

pub fn start_recording(
    _input: StartRecordingInput,
    _storage: State<'_, AppStorage>,
    _manager: State<'_, RecorderManager>,
) -> Result<RecorderStatus, RecorderError> {
    Err(unavailable_error())
}

pub fn pause_recording(
    _input: PauseRecordingInput,
    _storage: State<'_, AppStorage>,
    _manager: State<'_, RecorderManager>,
) -> Result<RecorderStatus, RecorderError> {
    Err(unavailable_error())
}

pub fn resume_recording(
    _input: ResumeRecordingInput,
    _storage: State<'_, AppStorage>,
    _manager: State<'_, RecorderManager>,
) -> Result<RecorderStatus, RecorderError> {
    Err(unavailable_error())
}

pub fn stop_recording(
    _input: StopRecordingInput,
    _storage: State<'_, AppStorage>,
    _manager: State<'_, RecorderManager>,
) -> Result<RecorderStatus, RecorderError> {
    Err(unavailable_error())
}
