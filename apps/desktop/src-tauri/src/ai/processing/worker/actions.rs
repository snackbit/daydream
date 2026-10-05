use super::super::super::*;

pub(crate) fn cancel_recording_processing_for_privacy(
    storage: &AppStorage,
    recording_id: &str,
    timestamp_ms: i64,
) -> rusqlite::Result<()> {
    crate::recording_privacy::cancel_recording_processing(storage, recording_id, timestamp_ms)?;
    storage.with_connection(|connection| {
        for job in storage::list_processing_jobs(connection, Some(recording_id))? {
            if job.status == "cancelled" && job.error.as_deref() == Some("ai_processing_privacy_changed") {
                save_processing_notification(connection, ProcessingNotificationInput {
                    actions: &processing_retry_actions(&job.id), job_id: Some(&job.id),
                    message: "Processing stopped because recording privacy changed. Retry processes only eligible sources.",
                    recording_id, status: "cancelled", timestamp_ms, title: "AI processing stopped",
                })?;
            }
        }
        Ok(())
    })
}

pub(in crate::ai) fn update_processing_job_for_action(
    storage: &AppStorage,
    job_id: &str,
    status: &str,
) -> Result<ProcessingJob, AiError> {
    if status == "queued" {
        ensure_no_running_processing_chunks(
            storage,
            job_id,
            "ai_processing_resume_pending",
            "AI processing cannot resume until the active chunk has paused.",
        )?;
    }

    storage
        .with_connection(|connection| {
            match status {
                "paused" => {}
                "cancelled" => {
                    storage::cancel_processing_chunks(connection, job_id, &["queued"], now_ms())?;
                }
                "queued" => {
                    storage::requeue_processing_chunks(
                        connection,
                        job_id,
                        &["cancelled"],
                        now_ms(),
                    )?;
                }
                _ => {}
            }
            storage::update_processing_job_status(connection, job_id, status, None, now_ms())
        })
        .map_err(storage_error)
        .map(|row| processing_job_from_row_with_storage(storage, row))
}

pub(in crate::ai) fn retry_processing_job_for_action(
    storage: &AppStorage,
    job_id: &str,
) -> Result<ProcessingJob, AiError> {
    ensure_no_running_processing_chunks(
        storage,
        job_id,
        "ai_processing_retry_pending",
        "AI processing cannot retry until active chunks have stopped.",
    )?;

    storage
        .with_connection(|connection| {
            storage::retry_processing_job_chunks(connection, job_id, now_ms())
        })
        .map_err(storage_error)
        .map(|row| processing_job_from_row_with_storage(storage, row))
}

fn ensure_no_running_processing_chunks(
    storage: &AppStorage,
    job_id: &str,
    code: &str,
    message: &str,
) -> Result<(), AiError> {
    let running = storage
        .with_connection(|connection| {
            storage::list_processing_chunks_by_status(connection, job_id, &["running"], Some(1))
        })
        .map_err(storage_error)?;
    if running.is_empty() {
        Ok(())
    } else {
        Err(ai_error(code, message))
    }
}
