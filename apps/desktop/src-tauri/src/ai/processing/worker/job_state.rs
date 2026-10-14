use super::super::super::*;

pub(super) enum JobStatusAction {
    Continue,
    Stop,
}

pub(super) fn handle_current_job_status(
    app: &AppHandle,
    storage: &AppStorage,
    job_id: &str,
) -> Result<JobStatusAction, AiError> {
    let current = storage
        .with_connection(|connection| storage::get_processing_job(connection, job_id))
        .map_err(storage_error)?
        .ok_or_else(|| ai_error("ai_processing_job_missing", "Processing job is missing."))?;

    match current.status.as_str() {
        "paused" => {
            pause_processing_job(app, storage, job_id, &current.recording_id)?;
            Ok(JobStatusAction::Stop)
        }
        "cancelled" => {
            cancel_processing_job(app, storage, job_id, &current.recording_id)?;
            Ok(JobStatusAction::Stop)
        }
        "running" => Ok(JobStatusAction::Continue),
        _ => Ok(JobStatusAction::Stop),
    }
}

fn pause_processing_job(
    app: &AppHandle,
    storage: &AppStorage,
    job_id: &str,
    recording_id: &str,
) -> Result<(), AiError> {
    storage
        .with_connection(|connection| {
            save_processing_notification(
                connection,
                ProcessingNotificationInput {
                    actions: &processing_paused_actions(job_id),
                    job_id: Some(job_id),
                    message: "Recording processing is paused.",
                    recording_id,
                    status: "info",
                    timestamp_ms: now_ms(),
                    title: "AI processing paused",
                },
            )
            .map(|_row| ())
        })
        .map_err(storage_error)?;
    emit_processing_state(app, recording_id);
    Ok(())
}

fn cancel_processing_job(
    app: &AppHandle,
    storage: &AppStorage,
    job_id: &str,
    recording_id: &str,
) -> Result<(), AiError> {
    storage
        .with_connection(|connection| {
            storage::cancel_processing_chunks(
                connection,
                job_id,
                &["queued", "running"],
                now_ms(),
            )?;
            save_processing_notification(
                connection,
                ProcessingNotificationInput {
                    actions: &processing_retry_actions(job_id),
                    job_id: Some(job_id),
                    message: "Recording processing was cancelled.",
                    recording_id,
                    status: "cancelled",
                    timestamp_ms: now_ms(),
                    title: "AI processing cancelled",
                },
            )
            .map(|_row| ())
        })
        .map_err(storage_error)?;
    emit_processing_state(app, recording_id);
    Ok(())
}
