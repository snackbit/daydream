use super::super::super::*;

pub(in crate::ai) enum ProcessingCompletion<'a> {
    Active,
    Terminal {
        error: Option<&'a str>,
        status: &'static str,
    },
}

pub(in crate::ai) fn summarize_processing_completion(
    chunks: &[ProcessingChunkRow],
) -> ProcessingCompletion<'_> {
    if chunks
        .iter()
        .any(|chunk| matches!(chunk.status.as_str(), "queued" | "running"))
    {
        return ProcessingCompletion::Active;
    }

    if let Some(chunk) = chunks.iter().find(|chunk| chunk.status == "failed") {
        return ProcessingCompletion::Terminal {
            error: chunk.error_code.as_deref(),
            status: "failed",
        };
    }

    if let Some(chunk) = chunks.iter().find(|chunk| chunk.status == "cancelled") {
        return ProcessingCompletion::Terminal {
            error: chunk.error_code.as_deref(),
            status: "cancelled",
        };
    }

    ProcessingCompletion::Terminal {
        error: None,
        status: "succeeded",
    }
}

pub(super) fn finish_processing_job(
    app: &AppHandle,
    storage: &AppStorage,
    job_id: &str,
) -> Result<(), AiError> {
    let mut recording_id = None;
    storage
        .with_connection(|connection| {
            let chunks = storage::list_processing_chunks(connection, job_id)?;
            let chunk_status_counts = processing_chunk_status_counts(&chunks);
            let ProcessingCompletion::Terminal { error, status } =
                summarize_processing_completion(&chunks)
            else {
                tracing::info!(
                    job_id = %job_id,
                    chunk_status_counts = %chunk_status_counts,
                    "ai_processing_job_completion_deferred_active_chunks"
                );
                return Ok(());
            };
            let job =
                storage::update_processing_job_status(connection, job_id, status, error, now_ms())?;
            tracing::info!(
                recording_id = %job.recording_id,
                job_id = %job_id,
                job_status = %status,
                error_code = error.unwrap_or(""),
                chunk_status_counts = %chunk_status_counts,
                "ai_processing_job_completion_decided"
            );
            recording_id = Some(job.recording_id.clone());
            let (notification_status, title, message, actions) = match status {
                "succeeded" => (
                    "succeeded",
                    "AI processing complete",
                    "Audio transcription completed for this recording.",
                    Vec::new(),
                ),
                "cancelled" => (
                    "cancelled",
                    "AI processing cancelled",
                    "Audio transcription was cancelled.",
                    processing_retry_actions(job_id),
                ),
                _ => (
                    "failed",
                    "AI processing failed",
                    "Audio transcription failed.",
                    processing_failed_actions(job_id),
                ),
            };
            save_processing_notification(
                connection,
                ProcessingNotificationInput {
                    actions: &actions,
                    job_id: Some(job_id),
                    message,
                    recording_id: &job.recording_id,
                    status: notification_status,
                    timestamp_ms: now_ms(),
                    title,
                },
            )
            .map(|_row| ())
        })
        .map_err(storage_error)?;
    if let Some(recording_id) = recording_id {
        emit_processing_state(app, &recording_id);
    }
    Ok(())
}

pub(super) fn fail_processing_job(
    app: &AppHandle,
    storage: &AppStorage,
    job_id: &str,
    error_code: &str,
) -> Result<(), AiError> {
    let mut recording_id = None;
    storage
        .with_connection(|connection| {
            let job = storage::get_processing_job(connection, job_id)?
                .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
            recording_id = Some(job.recording_id.clone());
            storage::cancel_processing_chunks(
                connection,
                job_id,
                &["queued", "running"],
                now_ms(),
            )?;
            storage::update_processing_job_status(
                connection,
                job_id,
                "failed",
                Some(error_code),
                now_ms(),
            )?;
            save_processing_notification(
                connection,
                ProcessingNotificationInput {
                    actions: &processing_failed_actions(job_id),
                    job_id: Some(job_id),
                    message: &format!("Audio transcription failed: {error_code}."),
                    recording_id: &job.recording_id,
                    status: "failed",
                    timestamp_ms: now_ms(),
                    title: "AI processing failed",
                },
            )
            .map(|_row| ())
        })
        .map_err(storage_error)?;
    if let Some(recording_id) = recording_id {
        emit_processing_state(app, &recording_id);
    }
    Ok(())
}
