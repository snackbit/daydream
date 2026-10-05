use super::super::{
    manager::{now_ms, PauseOwner, RecorderManager},
    platform,
};
use super::{
    attachments::attach_activity_monitor,
    completion::{
        restore_paused_after_resume_failure, spawn_lifecycle_completion,
        RecorderLifecycleCompletion,
    },
};
use crate::storage::AppStorage;
use tauri::{AppHandle, Emitter, Manager};

pub(crate) fn request_auto_pause<R: tauri::Runtime>(
    app: &AppHandle<R>,
    recording_id: &str,
) -> bool {
    let manager = app.state::<RecorderManager>();
    let Ok((_status, complete)) =
        manager.begin_pause_with_owner(recording_id, now_ms(), PauseOwner::Automatic)
    else {
        return false;
    };
    if complete {
        spawn_lifecycle_completion(
            app.clone(),
            recording_id.to_string(),
            RecorderLifecycleCompletion::Pause,
        );
    }
    true
}

pub(crate) fn request_auto_resume<R: tauri::Runtime>(app: &AppHandle<R>, recording_id: &str) {
    let Ok(Some(generation)) = app
        .state::<RecorderManager>()
        .automatic_resume_authority(recording_id)
    else {
        return;
    };
    let app = app.clone();
    let recording_id = recording_id.to_string();
    tauri::async_runtime::spawn(async move {
        let storage = app.state::<AppStorage>();
        let manager = app.state::<RecorderManager>();
        let active = loop {
            match manager.begin_auto_resume_with_authority(&recording_id, now_ms(), generation) {
                Ok(active) => break active,
                Err(error)
                    if error.code == "recorder_already_active"
                        && manager.status().is_ok_and(|status| {
                            status.finalization.as_ref().is_some_and(|finalization| {
                                finalization.action
                                    == super::super::types::RecorderFinalizationAction::Pause
                            })
                        }) =>
                {
                    tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                }
                Err(_) => return,
            }
        };
        let timestamp_ms = now_ms();
        let pending =
            match manager.prepare_resume_segment_start(&storage, active.clone(), timestamp_ms) {
                Ok(pending) => pending,
                Err(_error) => {
                    restore_paused_after_resume_failure(&app, &manager, &storage, active);
                    return;
                }
            };
        let prepared = match platform::prepare_live_capture(&storage, &pending.scene).await {
            Ok(prepared) => prepared,
            Err(_error) => {
                restore_paused_after_resume_failure(
                    &app,
                    &manager,
                    &storage,
                    pending.active.clone(),
                );
                return;
            }
        };
        if manager
            .create_pending_segment_rows(&storage, &pending)
            .is_err()
        {
            restore_paused_after_resume_failure(&app, &manager, &storage, pending.active.clone());
            return;
        }
        let mut media = match platform::start_live_segment(&pending.segment_input, prepared) {
            Ok(media) => media,
            Err(error) => {
                let _ = manager.fail_pending_segment(&storage, &pending, now_ms(), &error);
                return;
            }
        };
        attach_activity_monitor(&app, &storage, &recording_id, &mut media);
        if let Ok(status) = manager.activate_pending_segment(&storage, pending, media, now_ms()) {
            let _ = app.emit(super::RECORDER_STATUS_CHANGED_EVENT, status);
        }
    });
}
