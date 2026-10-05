use super::super::manager::{now_ms, ActiveRecording, RecorderManager};
use crate::{ai, settings, storage::AppStorage};
use tauri::{AppHandle, Emitter, Manager, Runtime};
use tracing::{info, warn};

#[derive(Clone, Copy)]
pub(super) enum RecorderLifecycleCompletion {
    Pause,
    Stop,
}

pub(super) fn restore_paused_after_resume_failure<R: Runtime>(
    app: &AppHandle<R>,
    manager: &RecorderManager,
    storage: &AppStorage,
    active: ActiveRecording,
) {
    let result = manager.restore_paused_after_selecting(storage, active, now_ms());
    if let Ok(status) = result.or_else(|_| manager.status()) {
        let _ = app.emit(super::RECORDER_STATUS_CHANGED_EVENT, status);
    }
}

pub(super) fn spawn_lifecycle_completion<R: Runtime>(
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
        let completed = result.as_ref().is_ok_and(|status| {
            status.state == super::super::types::RecorderState::Idle && status.last_error.is_none()
        });
        if let Ok(status) = result.or_else(|_| manager.status()) {
            let _ = app.emit(super::RECORDER_STATUS_CHANGED_EVENT, status);
        }
        if completed && matches!(completion, RecorderLifecycleCompletion::Stop) {
            maybe_auto_start_ai_processing(&app, &storage, &recording_id);
        }
    });
}

fn maybe_auto_start_ai_processing<R: Runtime>(app: &AppHandle<R>, storage: &AppStorage, id: &str) {
    let settings = match settings::load_app_settings(storage) {
        Ok(settings) => settings,
        Err(error) => {
            warn!(
                recording_id = id,
                error_code = error.code,
                "ai_processing_auto_start_settings_failed"
            );
            return;
        }
    };
    if !settings.ai.processing.auto_start_after_recording {
        return;
    }
    match ai::enqueue_recording_ai_processing(app, storage, id) {
        Ok(job) => info!(
            recording_id = id,
            job_id = job.id,
            "ai_processing_auto_start_enqueued"
        ),
        Err(error) => warn!(
            recording_id = id,
            error_code = error.code,
            "ai_processing_auto_start_failed"
        ),
    }
}
