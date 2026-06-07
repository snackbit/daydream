use tauri::{AppHandle, Emitter, Manager, State};
use tracing::{debug, error, info, warn};

use crate::storage::AppStorage;

use super::errors::lifecycle_error;
use super::manager::{now_ms, PreviewStartDecision, RecorderManager};
use super::platform;
use super::storage_mapping::{
    latest_recorder_scene_for_storage, recorder_scene_for_storage, save_recorder_scene_for_storage,
};
use super::types::{
    AudioSourceList, CaptureSourceList, PauseRecordingInput, RecorderError,
    RecorderPerformanceStats, RecorderPreviewStatus, RecorderScene, RecorderStatus,
    ResumeRecordingInput, SaveRecorderSceneInput, SelectCaptureSourcesInput,
    StartRecorderPreviewInput, StartRecordingInput, StopRecordingInput,
};

pub(crate) const RECORDER_STATUS_CHANGED_EVENT: &str = "recorder-status-changed";
pub(crate) const RECORDER_PREVIEW_FRAME_EVENT: &str = "recorder-preview-frame";

fn ensure_configuration_unlocked(manager: &RecorderManager) -> Result<(), RecorderError> {
    if manager.configuration_is_locked()? {
        return Err(lifecycle_error(
            "recorder_configuration_locked",
            "Recorder scene configuration is read-only while recording is active.",
        ));
    }

    Ok(())
}

pub async fn get_recorder_status(
    manager: State<'_, RecorderManager>,
) -> Result<RecorderStatus, RecorderError> {
    manager.status_with_backend_health(platform::collect_backend_health().await, now_ms())
}

pub fn get_recorder_performance_stats(
    manager: State<'_, RecorderManager>,
) -> Result<RecorderPerformanceStats, RecorderError> {
    manager.performance_stats(now_ms())
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

pub fn get_recorder_preview_status(
    manager: State<'_, RecorderManager>,
) -> Result<RecorderPreviewStatus, RecorderError> {
    manager.preview_status()
}

pub async fn start_recorder_preview(
    input: StartRecorderPreviewInput,
    app: AppHandle,
    storage: State<'_, AppStorage>,
    manager: State<'_, RecorderManager>,
) -> Result<RecorderPreviewStatus, RecorderError> {
    let timestamp_ms = now_ms();
    info!(scene_id = %input.scene_id, "recorder_preview_start_requested");
    let (scene_id, generation) = match manager.begin_preview_start(&input.scene_id, timestamp_ms)? {
        PreviewStartDecision::AlreadyCurrent(status) => {
            debug!(
                scene_id = %input.scene_id,
                preview_state = ?status.state,
                "recorder_preview_start_reused"
            );
            return Ok(status);
        }
        PreviewStartDecision::Pending { generation, .. } => (input.scene_id, generation),
    };

    let scene = match recorder_scene_for_storage(&storage, &scene_id) {
        Ok(scene) => scene,
        Err(error) => {
            warn!(
                scene_id = %scene_id,
                error_code = %error.code,
                error_category = ?error.category,
                "recorder_preview_scene_load_failed"
            );
            let _ = manager.fail_preview_start(scene_id, generation, error.clone(), now_ms());
            return Err(error);
        }
    };
    debug!(
        scene_id = %scene.id,
        scene_source_count = scene.sources.len(),
        scene_source_ids = %scene
            .sources
            .iter()
            .map(|source| source.source_id.as_str())
            .collect::<Vec<_>>()
            .join(","),
        "recorder_preview_scene_loaded"
    );
    if !manager.preview_start_is_current(&scene_id, generation) {
        debug!(
            scene_id = %scene_id,
            generation,
            "recorder_preview_start_stale_before_platform_start"
        );
        return manager.preview_status();
    }
    if let Some(status) = stop_empty_preview_start(&manager, &scene, &scene_id, generation)? {
        return Ok(status);
    }
    let preview = match platform::start_preview_session(app, &storage, &scene).await {
        Ok(preview) => preview,
        Err(error) => {
            warn!(
                scene_id = %scene_id,
                scene_source_count = scene.sources.len(),
                scene_source_ids = %scene
                    .sources
                    .iter()
                    .map(|source| source.source_id.as_str())
                    .collect::<Vec<_>>()
                    .join(","),
                error_code = %error.code,
                error_category = ?error.category,
                "recorder_preview_start_failed"
            );
            let _ = manager.fail_preview_start(scene_id, generation, error.clone(), now_ms());
            return Err(error);
        }
    };
    info!(
        scene_id = %scene_id,
        scene_source_ids = %preview.source_ids.join(","),
        "recorder_preview_start_succeeded"
    );

    manager.activate_preview(
        scene_id,
        generation,
        preview.source_ids,
        preview.session,
        now_ms(),
    )
}

pub fn stop_recorder_preview(
    manager: State<'_, RecorderManager>,
) -> Result<RecorderPreviewStatus, RecorderError> {
    info!("recorder_preview_stop_requested");
    manager.stop_preview(now_ms())
}

pub(crate) fn stop_empty_preview_start(
    manager: &RecorderManager,
    scene: &RecorderScene,
    scene_id: &str,
    generation: u64,
) -> Result<Option<RecorderPreviewStatus>, RecorderError> {
    if !scene.sources.is_empty() {
        return Ok(None);
    }

    info!(
        scene_id = %scene_id,
        generation,
        "recorder_preview_start_skipped_empty_scene"
    );

    manager.stop_preview(now_ms()).map(Some)
}

pub async fn start_recording(
    input: StartRecordingInput,
    storage: State<'_, AppStorage>,
    manager: State<'_, RecorderManager>,
) -> Result<RecorderStatus, RecorderError> {
    let timestamp_ms = now_ms();
    info!(scene_id = %input.scene_id, "recorder_start_requested");
    manager.stop_preview(timestamp_ms)?;
    manager.begin_selecting(&input.scene_id, timestamp_ms)?;

    let pending = match manager.prepare_recording_start(&storage, input, timestamp_ms) {
        Ok(pending) => pending,
        Err(error) => {
            warn!(
                error_code = %error.code,
                error_category = ?error.category,
                "recorder_prepare_start_failed"
            );
            let _ = manager.clear_selecting_to_idle(now_ms());
            return Err(error);
        }
    };
    let prepared = match platform::prepare_live_capture(&storage, &pending.scene).await {
        Ok(prepared) => prepared,
        Err(error) => {
            warn!(
                scene_id = %pending.scene.id,
                recording_id = %pending.recording_id,
                error_code = %error.code,
                error_category = ?error.category,
                "recorder_prepare_capture_failed"
            );
            let _ = manager.clear_selecting_to_idle(now_ms());
            return Err(error);
        }
    };
    if let Err(error) = manager.create_pending_recording_rows(&storage, &pending) {
        error!(
            recording_id = %pending.recording_id,
            error_code = %error.code,
            error_category = ?error.category,
            "recorder_create_rows_failed"
        );
        let _ = manager.clear_selecting_to_idle(now_ms());
        return Err(error);
    }
    let media = match platform::start_live_segment(&pending.segment_input, prepared) {
        Ok(media) => media,
        Err(error) => {
            error!(
                recording_id = %pending.recording_id,
                segment_index = pending.segment_index,
                error_code = %error.code,
                error_category = ?error.category,
                "recorder_start_segment_failed"
            );
            let _ = manager.fail_pending_recording(&storage, &pending, now_ms(), &error);
            return Err(error);
        }
    };
    info!(
        recording_id = %pending.recording_id,
        segment_index = pending.segment_index,
        output_path = %pending.segment_path.to_string_lossy(),
        "recorder_start_succeeded"
    );

    manager.activate_pending_recording(pending, media, now_ms())
}

pub fn pause_recording<R: tauri::Runtime>(
    input: PauseRecordingInput,
    app: AppHandle<R>,
    manager: State<'_, RecorderManager>,
) -> Result<RecorderStatus, RecorderError> {
    info!(recording_id = %input.recording_id, "recorder_pause_requested");
    let status = manager.begin_pause_recording(&input.recording_id, now_ms())?;
    if status.finalization.is_some() {
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
    info!(recording_id = %input.recording_id, "recorder_resume_requested");
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
            warn!(
                recording_id = %input.recording_id,
                error_code = %error.code,
                error_category = ?error.category,
                "recorder_resume_prepare_capture_failed"
            );
            let _ = manager.restore_paused_after_selecting(pending.active.clone(), now_ms());
            return Err(error);
        }
    };
    if let Err(error) = manager.create_pending_segment_rows(&storage, &pending) {
        let _ = manager.restore_paused_after_selecting(pending.active.clone(), now_ms());
        return Err(error);
    }
    let media = match platform::start_live_segment(&pending.segment_input, prepared) {
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
    info!(recording_id = %input.recording_id, "recorder_stop_requested");
    let status = manager.begin_stop_recording(&input.recording_id, now_ms())?;
    if status.finalization.is_some() {
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
        match &status {
            Ok(status) => info!(
                recording_id = %recording_id,
                recorder_state = ?status.state,
                "recorder_lifecycle_completion_finished"
            ),
            Err(error) => error!(
                recording_id = %recording_id,
                error_code = %error.code,
                error_category = ?error.category,
                "recorder_lifecycle_completion_failed"
            ),
        }

        if let Ok(status) = status {
            let _ = app.emit(RECORDER_STATUS_CHANGED_EVENT, status);
        }
    });
}
