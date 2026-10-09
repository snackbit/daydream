use tauri::{AppHandle, Emitter, State};
use tracing::{error, info, warn};

use crate::{settings, storage::AppStorage};

use super::super::{
    errors::{dependency_error, window_protection_unavailable_error},
    manager::{now_ms, PauseOwner, RecorderManager},
    platform,
    types::{
        PauseRecordingInput, ProtectableWindowProviderStatus, RecorderError, RecorderStatus,
        ResumeRecordingInput, StartRecordingInput, StopRecordingInput,
    },
};
use super::{
    attachments::{attach_activity_monitor, attach_window_protection_monitor},
    completion::{
        restore_paused_after_resume_failure, spawn_lifecycle_completion,
        RecorderLifecycleCompletion,
    },
    supervision::start_runtime_supervision,
};

pub async fn start_recording<R: tauri::Runtime>(
    mut input: StartRecordingInput,
    app: AppHandle<R>,
    storage: State<'_, AppStorage>,
    manager: State<'_, RecorderManager>,
) -> Result<RecorderStatus, RecorderError> {
    let _activity =
        crate::maintenance::runtime::admit(&app, "capture-start").map_err(|message| {
            super::super::errors::lifecycle_error("recorder_maintenance_active", &message)
        })?;
    let timestamp_ms = now_ms();
    info!(scene_id = %input.scene_id, "recorder_start_requested");
    manager.begin_selecting(&input.scene_id, timestamp_ms)?;
    let configuration = (|| {
        if let Some(setup) = input.setup.take() {
            if setup.scene.id != input.scene_id {
                return Err(super::super::errors::invalid_scene_error(
                    "The reviewed setup belongs to another scene.",
                ));
            }
            if setup.scene.desktop_audio_source_id != input.desktop_audio_source_id
                || setup.scene.microphone_audio_source_id != input.microphone_audio_source_id
            {
                return Err(super::super::errors::invalid_scene_error(
                    "Review the selected audio sources before starting.",
                ));
            }
            let saved = super::super::source_recording::persist_capture_setup(setup, &storage)?;
            let _ = app.emit("capture-setup-saved", saved);
        }
        ensure_window_protection_provider_available(&storage)?;
        manager.stop_preview(timestamp_ms)?;
        Ok(())
    })();
    if let Err(error) = configuration {
        let _ = manager.clear_selecting_to_idle(now_ms());
        return Err(error);
    }

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
    let mut media = match platform::start_live_segment(&pending.segment_input, prepared).await {
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

    let status = manager.activate_pending_recording(pending, media, now_ms())?;
    if let Err(error) = attach_window_protection_monitor(&app, &storage, &status).and_then(|_| {
        start_runtime_supervision(
            app.clone(),
            status
                .active_recording_id
                .clone()
                .expect("recording activated"),
        )
    }) {
        let id = status
            .active_recording_id
            .as_deref()
            .expect("recording activated");
        let _ = manager.abort_recording(&storage, id, error.clone(), now_ms());
        return Err(error);
    }

    Ok(status)
}

fn ensure_window_protection_provider_available(storage: &AppStorage) -> Result<(), RecorderError> {
    let app_settings = settings::load_app_settings(storage).map_err(|error| {
        dependency_error(
            "recorder_settings_unavailable",
            format!("Recorder settings failed: {}", error.message),
        )
    })?;
    let protection = &app_settings.record.window_protection;
    if !protection.enabled || protection.rules.iter().all(|rule| !rule.enabled) {
        return Ok(());
    }

    let status = platform::window_protection_status();
    if status.status == ProtectableWindowProviderStatus::Available {
        Ok(())
    } else {
        Err(window_protection_unavailable_error(&status))
    }
}

pub fn pause_recording<R: tauri::Runtime>(
    input: PauseRecordingInput,
    app: AppHandle<R>,
    manager: State<'_, RecorderManager>,
) -> Result<RecorderStatus, RecorderError> {
    info!(recording_id = %input.recording_id, "recorder_pause_requested");
    let (status, complete) =
        manager.begin_pause_with_owner(&input.recording_id, now_ms(), PauseOwner::Manual)?;
    if complete {
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
    let _activity =
        crate::maintenance::runtime::admit(&app, "capture-resume").map_err(|message| {
            super::super::errors::lifecycle_error("recorder_maintenance_active", &message)
        })?;
    let timestamp_ms = now_ms();
    info!(recording_id = %input.recording_id, "recorder_resume_requested");
    ensure_window_protection_provider_available(&storage)?;
    let active = manager.begin_resume_selecting(&input.recording_id, timestamp_ms)?;
    let pending = match manager.prepare_resume_segment_start(&storage, active.clone(), timestamp_ms)
    {
        Ok(pending) => pending,
        Err(error) => {
            restore_paused_after_resume_failure(&app, &manager, &storage, active);
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
            restore_paused_after_resume_failure(&app, &manager, &storage, pending.active.clone());
            return Err(error);
        }
    };
    if let Err(error) = manager.create_pending_segment_rows(&storage, &pending) {
        restore_paused_after_resume_failure(&app, &manager, &storage, pending.active.clone());
        return Err(error);
    }
    let mut media = match platform::start_live_segment(&pending.segment_input, prepared).await {
        Ok(media) => media,
        Err(error) => {
            let _ = manager.fail_pending_segment(&storage, &pending, now_ms(), &error);
            return Err(error);
        }
    };
    attach_activity_monitor(&app, &storage, &input.recording_id, &mut media);

    manager.activate_pending_segment(&storage, pending, media, now_ms())
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
