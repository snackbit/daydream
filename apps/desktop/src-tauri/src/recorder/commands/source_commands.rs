use tauri::State;

use crate::storage::AppStorage;

use super::super::{
    errors::lifecycle_error,
    manager::{now_ms, RecorderManager},
    platform,
    storage_mapping::{latest_recorder_scene_for_storage, save_recorder_scene_for_storage},
    types::{
        AudioSourceList, CaptureSourceList, RecorderError, RecorderScene, SaveRecorderSceneInput,
        SelectCaptureSourcesInput,
    },
};

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
    ensure_configuration_unlocked(&manager)?;

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
    manager: State<'_, RecorderManager>,
) -> Result<RecorderScene, RecorderError> {
    ensure_configuration_unlocked(&manager)?;

    save_recorder_scene_for_storage(&storage, input, now_ms())
}

fn ensure_configuration_unlocked(manager: &RecorderManager) -> Result<(), RecorderError> {
    if manager.configuration_is_locked()? {
        return Err(lifecycle_error(
            "recorder_configuration_locked",
            "Recorder scene configuration is read-only while recording is active.",
        ));
    }

    Ok(())
}
