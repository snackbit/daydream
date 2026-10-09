//! Stop resource owners before maintenance touches their files.
use crate::{
    recorder::{RecorderCleanupDisposition, RecorderManager, RecorderState, RecorderStatus},
    storage::AppStorage,
};
use std::time::Duration;
use tauri::{AppHandle, Manager};

pub(crate) fn ensure_capture_stopped(app: &AppHandle) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    if crate::recorder::media_worker::is_quarantined() {
        return Err(
            "The recording worker could not stop safely. Restart Daydream before maintenance."
                .into(),
        );
    }
    #[cfg(target_os = "linux")]
    if !crate::recorder::media_worker::is_quiescent() {
        return Err("Stop the recording and wait for its worker to finish. Restart Daydream if it cannot stop.".into());
    }
    let manager = app.state::<RecorderManager>();
    let status = manager
        .status()
        .map_err(|_| "Recording status is unavailable.")?;
    if status.cleanup_disposition == RecorderCleanupDisposition::Quarantined {
        return Err(
            "Recording cleanup is quarantined. Restart Daydream before maintenance.".into(),
        );
    }
    let capture_owned = manager
        .configuration_is_locked()
        .map_err(|_| "Recording ownership is unavailable.")?;
    #[cfg(target_os = "linux")]
    let worker_quiescent = crate::recorder::media_worker::is_quiescent();
    #[cfg(not(target_os = "linux"))]
    let worker_quiescent = true;
    validate_capture_stopped(&status, capture_owned, worker_quiescent)
}
fn validate_capture_stopped(
    status: &RecorderStatus,
    capture_owned: bool,
    worker_quiescent: bool,
) -> Result<(), String> {
    if status.cleanup_disposition == RecorderCleanupDisposition::Quarantined {
        return Err(
            "Recording cleanup is quarantined. Restart Daydream before maintenance.".into(),
        );
    }
    if capture_owned
        || status.finalization.is_some()
        || !matches!(
            status.state,
            RecorderState::Idle | RecorderState::Error | RecorderState::Unavailable
        )
    {
        return Err("Stop the recording and wait for it to finish before maintenance.".into());
    }
    if !worker_quiescent {
        return Err(
            "The recording worker has not stopped. Restart Daydream before maintenance.".into(),
        );
    }
    Ok(())
}

pub(crate) async fn quiesce(app: &AppHandle) -> Result<(), String> {
    let manager = app.state::<RecorderManager>();
    if manager
        .preview_status()
        .map_err(|_| "Preview status is unavailable.")?
        .scene_id
        .is_some()
    {
        manager
            .stop_preview(chrono::Utc::now().timestamp_millis().max(0) as u64)
            .map_err(|_| "Unable to stop the capture preview.")?;
    }
    app.state::<crate::recall::search_service::SearchControl>()
        .cancel()?;
    let storage = app.state::<AppStorage>();
    storage.with_connection(|connection| {
        let timestamp = chrono::Utc::now().timestamp_millis();
        connection.execute("UPDATE processing_jobs SET status='cancelled',updated_at_ms=?1 WHERE status IN ('queued','running','paused')", [timestamp])?;
        connection.execute("UPDATE recall_jobs SET status='cancelled',updated_at_ms=?1 WHERE status IN ('pending','running','paused')", [timestamp])?;
        Ok(())
    }).map_err(|_| "Unable to stop background processing safely.".to_string())?;
    tokio::time::timeout(
        Duration::from_secs(20),
        crate::ai::cancel_transfers_for_maintenance(app),
    )
    .await
    .map_err(|_| {
        "Model transfers did not stop. Restart Daydream before maintenance.".to_string()
    })??;
    let playback = app
        .state::<crate::playback_server::PlaybackServer>()
        .inner()
        .clone();
    tauri::async_runtime::spawn_blocking(move || {
        playback.close_all_and_drain(Duration::from_secs(20))
    })
    .await
    .map_err(|_| "Playback cleanup failed.".to_string())??;
    Ok(())
}

pub(crate) async fn release_models(app: &AppHandle) -> Result<(), String> {
    crate::ai::cancel_transfers_for_maintenance(app).await?;
    app.state::<crate::transcription::TranscriptionProviderManager>()
        .shutdown_for_maintenance()
        .await?;
    crate::recall::embedding::shutdown_for_maintenance().await
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::recorder::{
        RecorderFinalizationAction, RecorderFinalizationPhase, RecorderFinalizationStatus,
    };
    #[test]
    fn maintenance_rejects_paused_pending_finalization_and_unreaped_capture_owners() {
        let mut status = RecorderManager::new("/tmp/synthetic-maintenance-recorder".into())
            .status()
            .unwrap();
        for state in [
            RecorderState::Recording,
            RecorderState::Paused,
            RecorderState::Pausing,
            RecorderState::Selecting,
            RecorderState::Stopping,
        ] {
            status.state = state;
            assert!(validate_capture_stopped(&status, false, true).is_err());
        }
        status.state = RecorderState::Idle;
        status.finalization = Some(RecorderFinalizationStatus {
            action: RecorderFinalizationAction::Stop,
            phase: RecorderFinalizationPhase::SavingSegment,
            recording_id: "synthetic-recording".into(),
            segment_id: None,
            started_at_ms: 1,
        });
        assert!(validate_capture_stopped(&status, false, true).is_err());
        status.finalization = None;
        assert!(validate_capture_stopped(&status, false, false)
            .unwrap_err()
            .contains("Restart Daydream"));
        assert!(validate_capture_stopped(&status, true, true).is_err());
        assert!(validate_capture_stopped(&status, false, true).is_ok());
        status.cleanup_disposition = RecorderCleanupDisposition::Quarantined;
        assert!(validate_capture_stopped(&status, false, true)
            .unwrap_err()
            .contains("Restart Daydream"));
    }
}
