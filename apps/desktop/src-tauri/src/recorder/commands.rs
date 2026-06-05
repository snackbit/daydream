use tauri::{AppHandle, Emitter, Manager, State};

use crate::storage::AppStorage;
use std::path::Path;

use super::manager::{now_ms, RecorderManager};
use super::platform;
use super::storage_mapping::{latest_recorder_scene_for_storage, save_recorder_scene_for_storage};
use super::types::{
    AudioSourceList, CaptureSourceList, PauseRecordingInput, RecorderError, RecorderScene,
    RecorderStatus, ResumeRecordingInput, SaveRecorderSceneInput, SelectCaptureSourcesInput,
    StartRecordingInput, StopRecordingInput,
};

pub(crate) const RECORDER_STATUS_CHANGED_EVENT: &str = "recorder-status-changed";

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

pub async fn start_recording(
    input: StartRecordingInput,
    storage: State<'_, AppStorage>,
    manager: State<'_, RecorderManager>,
) -> Result<RecorderStatus, RecorderError> {
    let timestamp_ms = now_ms();
    manager.begin_selecting(&input.scene_id, timestamp_ms)?;

    let pending = match manager.prepare_recording_start(&storage, input, timestamp_ms) {
        Ok(pending) => pending,
        Err(error) => {
            let _ = manager.clear_selecting_to_idle(now_ms());
            return Err(error);
        }
    };
    let prepared = match platform::prepare_live_capture(&storage, &pending.scene).await {
        Ok(prepared) => prepared,
        Err(error) => {
            let _ = manager.clear_selecting_to_idle(now_ms());
            return Err(error);
        }
    };
    if let Err(error) = manager.create_pending_recording_rows(&storage, &pending) {
        let _ = manager.clear_selecting_to_idle(now_ms());
        return Err(error);
    }
    let media = match platform::start_live_segment(
        Path::new(&pending.ffmpeg_path),
        &pending.segment_input,
        prepared,
    ) {
        Ok(media) => media,
        Err(error) => {
            let _ = manager.fail_pending_recording(&storage, &pending, now_ms(), &error);
            return Err(error);
        }
    };

    manager.activate_pending_recording(pending, media, now_ms())
}

pub fn pause_recording<R: tauri::Runtime>(
    input: PauseRecordingInput,
    app: AppHandle<R>,
    manager: State<'_, RecorderManager>,
) -> Result<RecorderStatus, RecorderError> {
    let status = manager.begin_pause_recording(&input.recording_id, now_ms())?;
    if status.state == super::types::RecorderState::Pausing {
        spawn_lifecycle_completion(app, input.recording_id, RecorderLifecycleCompletion::Pause);
    }

    Ok(status)
}

pub async fn resume_recording(
    input: ResumeRecordingInput,
    storage: State<'_, AppStorage>,
    manager: State<'_, RecorderManager>,
) -> Result<RecorderStatus, RecorderError> {
    let timestamp_ms = now_ms();
    let active = manager.begin_resume_selecting(&input.recording_id, timestamp_ms)?;
    let pending = match manager.prepare_resume_segment_start(&storage, active.clone(), timestamp_ms)
    {
        Ok(pending) => pending,
        Err(error) => {
            let _ = manager.restore_paused_after_selecting(active, now_ms());
            return Err(error);
        }
    };
    let prepared = match platform::prepare_live_capture(&storage, &pending.scene).await {
        Ok(prepared) => prepared,
        Err(error) => {
            let _ = manager.restore_paused_after_selecting(pending.active.clone(), now_ms());
            return Err(error);
        }
    };
    if let Err(error) = manager.create_pending_segment_rows(&storage, &pending) {
        let _ = manager.restore_paused_after_selecting(pending.active.clone(), now_ms());
        return Err(error);
    }
    let media = match platform::start_live_segment(
        Path::new(&pending.ffmpeg_path),
        &pending.segment_input,
        prepared,
    ) {
        Ok(media) => media,
        Err(error) => {
            let _ = manager.fail_pending_segment(&storage, &pending, now_ms(), &error);
            return Err(error);
        }
    };

    manager.activate_pending_segment(pending, media, now_ms())
}

pub fn stop_recording<R: tauri::Runtime>(
    input: StopRecordingInput,
    app: AppHandle<R>,
    manager: State<'_, RecorderManager>,
) -> Result<RecorderStatus, RecorderError> {
    let status = manager.begin_stop_recording(&input.recording_id, now_ms())?;
    if status.state == super::types::RecorderState::Stopping {
        spawn_lifecycle_completion(app, input.recording_id, RecorderLifecycleCompletion::Stop);
    }

    Ok(status)
}

#[derive(Clone, Copy)]
enum RecorderLifecycleCompletion {
    Pause,
    Stop,
}

fn spawn_lifecycle_completion<R: tauri::Runtime>(
    app: AppHandle<R>,
    recording_id: String,
    completion: RecorderLifecycleCompletion,
) {
    tauri::async_runtime::spawn_blocking(move || {
        let manager = app.state::<RecorderManager>();
        let storage = app.state::<AppStorage>();
        let result = match completion {
            RecorderLifecycleCompletion::Pause => {
                manager.complete_pause_recording(&storage, &recording_id, now_ms())
            }
            RecorderLifecycleCompletion::Stop => {
                manager.complete_stop_recording(&storage, &recording_id, now_ms())
            }
        };
        let status = result.or_else(|_error| manager.status());

        if let Ok(status) = status {
            let _ = app.emit(RECORDER_STATUS_CHANGED_EVENT, status);
        }
    });
}
