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
    let manager = app.state::<RecorderManager>();
    let Ok(initial) = manager.status() else {
        return;
    };
    let deadline = manager.lifecycle_deadline();
    let cutoff = initial
        .finalization
        .as_ref()
        .map_or_else(now_ms, |pending| pending.started_at_ms);
    manager.set_lifecycle_background_work(true);
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let worker_app = app.clone();
    let worker_id = recording_id.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let manager = worker_app.state::<RecorderManager>();
        let storage = worker_app.state::<AppStorage>();
        let result = match completion {
            RecorderLifecycleCompletion::Pause => {
                manager.complete_pause_recording(&storage, &worker_id, cutoff)
            }
            RecorderLifecycleCompletion::Stop => {
                manager.complete_stop_recording(&storage, &worker_id, cutoff)
            }
        };
        let _ = sender.send(result);
    });
    tauri::async_runtime::spawn(async move {
        let outcome = tokio::time::timeout(
            deadline.saturating_duration_since(std::time::Instant::now()),
            receiver,
        )
        .await;
        let manager = app.state::<RecorderManager>();
        match outcome {
            Ok(Ok(result)) => {
                manager.set_lifecycle_background_work(false);
                let completed = result.as_ref().is_ok_and(|status| {
                    status.state == super::super::types::RecorderState::Idle
                        && status.last_error.is_none()
                }) && manager.forced_lifecycle_error(&recording_id).is_none();
                if let Ok(status) = result.or_else(|_| manager.status()) {
                    let _ = app.emit(super::RECORDER_STATUS_CHANGED_EVENT, status);
                }
                if completed && matches!(completion, RecorderLifecycleCompletion::Stop) {
                    let _ = app.emit(
                        "recording-saved",
                        super::super::types::RecordingSaved {
                            recording_id: recording_id.clone(),
                            saved_at_ms: now_ms(),
                        },
                    );
                    let processing_app = app.clone();
                    tauri::async_runtime::spawn_blocking(move || {
                        maybe_auto_start_ai_processing(
                            &processing_app,
                            &processing_app.state::<AppStorage>(),
                            &recording_id,
                        );
                        if let Some(registry) = processing_app
                            .try_state::<crate::maintenance::RuntimeActivityRegistry>(
                        ) {
                            if let Ok(lease) = registry.begin("recording-retention") {
                                if !lease.cancellation().is_cancelled() {
                                    processing_app
                                        .state::<RecorderManager>()
                                        .cleanup_after_stop(
                                            &processing_app.state::<AppStorage>(),
                                            now_ms(),
                                        );
                                }
                            }
                        }
                    });
                }
            }
            _ => {
                let error=super::super::errors::lifecycle_error("recorder_stop_deadline_exceeded","Recording cleanup exceeded its deadline. Saved files have been preserved; restart Daydream before recording again.");
                let status = manager.force_lifecycle_failure(
                    &recording_id,
                    initial.backend_health,
                    error.clone(),
                    now_ms(),
                );
                let _ = app.emit(super::RECORDER_STATUS_CHANGED_EVENT, status);
                let cleanup_app = app.clone();
                tauri::async_runtime::spawn_blocking(move || {
                    let _ = cleanup_app.state::<RecorderManager>().abort_recording(
                        &cleanup_app.state::<AppStorage>(),
                        &recording_id,
                        error,
                        now_ms(),
                    );
                });
            }
        }
    });
}

fn maybe_auto_start_ai_processing<R: Runtime>(app: &AppHandle<R>, storage: &AppStorage, id: &str) {
    match ai::finish_live_recording(app, storage, id) {
        Ok(true) => return,
        Ok(false) => (),
        Err(error) => {
            warn!(recording_id=id,error_code=%error.code,"ai_live_final_admission_failed");
            return;
        }
    }
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
    if !settings.ai.processing.auto_start_after_recording
        || settings.ai.processing.timing == "manual"
    {
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
