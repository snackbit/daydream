use tauri::{AppHandle, Emitter, Manager, State};
use tracing::{error, info, warn};

use crate::{
    ai, settings,
    storage::{self, AppStorage},
};

use super::super::{
    activity_input,
    manager::{now_ms, RecorderManager, RecordingSegmentRuntime},
    platform,
    types::{
        PauseRecordingInput, RecorderError, RecorderStatus, ResumeRecordingInput,
        StartRecordingInput, StopRecordingInput,
    },
};

pub async fn start_recording<R: tauri::Runtime>(
    input: StartRecordingInput,
    app: AppHandle<R>,
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
    let mut media = match platform::start_live_segment(&pending.segment_input, prepared) {
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
    attach_activity_monitor(&app, &storage, &pending.recording_id, &mut media);
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

pub async fn resume_recording<R: tauri::Runtime>(
    input: ResumeRecordingInput,
    app: AppHandle<R>,
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
    let mut media = match platform::start_live_segment(&pending.segment_input, prepared) {
        Ok(media) => media,
        Err(error) => {
            let _ = manager.fail_pending_segment(&storage, &pending, now_ms(), &error);
            return Err(error);
        }
    };
    attach_activity_monitor(&app, &storage, &input.recording_id, &mut media);

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

fn attach_activity_monitor(
    app: &AppHandle<impl tauri::Runtime>,
    storage: &AppStorage,
    recording_id: &str,
    media: &mut RecordingSegmentRuntime,
) {
    let Ok(app_settings) = settings::load_app_settings(storage) else {
        warn!("Input activity monitor skipped because settings failed to load.");
        return;
    };
    let recording_started_at_ms = storage
        .with_connection(|connection| storage::get_recording(connection, recording_id))
        .ok()
        .and_then(|recording| recording.started_at_ms)
        .and_then(|started_at_ms| u64::try_from(started_at_ms).ok())
        .unwrap_or_else(now_ms);
    let Some(monitor) = activity_input::start_activity_monitor(
        app.clone(),
        recording_id.to_string(),
        recording_started_at_ms,
        &app_settings.ai.activity,
    ) else {
        return;
    };
    media.attach_activity_monitor(monitor);
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
            let _ = app.emit(super::RECORDER_STATUS_CHANGED_EVENT, status);
        }
        if matches!(completion, RecorderLifecycleCompletion::Stop) {
            maybe_auto_start_ai_processing(&app, &storage, &recording_id);
        }
    });
}

fn maybe_auto_start_ai_processing<R: tauri::Runtime>(
    app: &AppHandle<R>,
    storage: &AppStorage,
    recording_id: &str,
) {
    let app_settings = match settings::load_app_settings(storage) {
        Ok(app_settings) => app_settings,
        Err(error) => {
            warn!(
                recording_id = %recording_id,
                error_code = %error.code,
                "ai_processing_auto_start_settings_failed"
            );
            return;
        }
    };
    if !app_settings.ai.processing.auto_start_after_recording {
        info!(
            recording_id = %recording_id,
            "ai_processing_auto_start_skipped_disabled"
        );
        return;
    }

    info!(recording_id = %recording_id, "ai_processing_auto_start_requested");
    match ai::enqueue_recording_ai_processing(app, storage, recording_id) {
        Ok(job) => info!(
            recording_id = %recording_id,
            job_id = %job.id,
            job_status = %job.status,
            "ai_processing_auto_start_enqueued"
        ),
        Err(error) => warn!(
            recording_id = %recording_id,
            error_code = %error.code,
            "ai_processing_auto_start_failed"
        ),
    }
}
