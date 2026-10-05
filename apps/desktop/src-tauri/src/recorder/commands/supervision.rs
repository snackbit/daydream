use super::super::{
    errors::dependency_error,
    manager::{now_ms, RecorderManager},
    types::RecorderError,
};
use crate::storage::AppStorage;
use std::{thread, time::Duration};
use tauri::{AppHandle, Emitter, Manager, Runtime};

pub(crate) fn report_recording_failure<R: Runtime>(
    app: AppHandle<R>,
    recording_id: String,
    error: RecorderError,
) {
    // Publish synchronously so joining a failed monitor also joins its failure
    // decision. Stop cannot publish Completed before an already-observed error.
    let marked =
        app.state::<RecorderManager>()
            .mark_recording_failure(&recording_id, &error, now_ms());
    let Ok((status, abort)) = marked else {
        return;
    };
    let _ = app.emit(super::RECORDER_STATUS_CHANGED_EVENT, status);
    if abort {
        tauri::async_runtime::spawn_blocking(move || {
            let manager = app.state::<RecorderManager>();
            let storage = app.state::<AppStorage>();
            let result = manager.abort_recording(&storage, &recording_id, error, now_ms());
            if let Ok(status) = result.or_else(|_| manager.status()) {
                let _ = app.emit(super::RECORDER_STATUS_CHANGED_EVENT, status);
            }
        });
    }
}

pub(super) fn start_runtime_supervision<R: Runtime>(
    app: AppHandle<R>,
    recording_id: String,
) -> Result<(), RecorderError> {
    thread::Builder::new()
        .name("daydream-recorder-supervision".into())
        .spawn(move || loop {
            let manager = app.state::<RecorderManager>();
            let Ok(status) = manager.status() else {
                return;
            };
            if status.active_recording_id.as_deref() != Some(recording_id.as_str()) {
                return;
            }
            match manager.active_runtime_error(&recording_id) {
                Ok(None) => (),
                Ok(Some(error)) | Err(error) => {
                    report_recording_failure(app, recording_id, error);
                    return;
                }
            }
            thread::sleep(Duration::from_millis(250));
        })
        .map(|_| ())
        .map_err(|_| {
            dependency_error(
                "recorder_supervision_failed",
                "Recording supervision could not start.",
            )
        })
}
