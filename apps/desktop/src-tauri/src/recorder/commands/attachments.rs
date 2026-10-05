use super::super::{
    activity_input,
    errors::dependency_error,
    manager::{now_ms, RecorderManager, RecordingSegmentRuntime},
    types::{RecorderError, RecorderStatus},
    window_protection,
};
use crate::{
    settings,
    storage::{self, AppStorage},
};
use tauri::{AppHandle, Manager, Runtime};
use tracing::warn;

pub(super) fn attach_activity_monitor(
    app: &AppHandle<impl Runtime>,
    storage: &AppStorage,
    recording_id: &str,
    media: &mut RecordingSegmentRuntime,
) {
    let Ok(settings) = settings::load_app_settings(storage) else {
        warn!("Input activity monitor skipped because settings failed to load.");
        return;
    };
    let started = recording_start(storage, recording_id).unwrap_or_else(|_| now_ms());
    if let Some(monitor) = activity_input::start_activity_monitor(
        app.clone(),
        recording_id.into(),
        started,
        &settings.ai.activity,
    ) {
        media.attach_activity_monitor(monitor);
    }
}

pub(super) fn attach_window_protection_monitor<R: Runtime>(
    app: &AppHandle<R>,
    storage: &AppStorage,
    status: &RecorderStatus,
) -> Result<(), RecorderError> {
    let Some(id) = status.active_recording_id.clone() else {
        return Ok(());
    };
    let settings = settings::load_app_settings(storage).map_err(|_| {
        dependency_error(
            "recorder_settings_unavailable",
            "Protected-window settings could not be loaded.",
        )
    })?;
    let started = recording_start(storage, &id)?;
    if let Some(monitor) = window_protection::start_window_protection_monitor(
        app.clone(),
        id.clone(),
        started,
        settings.record.window_protection,
    )? {
        app.state::<RecorderManager>()
            .attach_protection_monitor(&id, monitor)?;
    }
    Ok(())
}

fn recording_start(storage: &AppStorage, id: &str) -> Result<u64, RecorderError> {
    storage
        .with_connection(|connection| storage::get_recording(connection, id))
        .map_err(|_| {
            dependency_error(
                "recorder_storage_error",
                "Recording timestamps could not be loaded.",
            )
        })?
        .started_at_ms
        .and_then(|start| u64::try_from(start).ok())
        .ok_or_else(|| {
            dependency_error(
                "recorder_storage_error",
                "Recording start timestamp is unavailable.",
            )
        })
}
