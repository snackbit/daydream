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
    let app_settings = crate::settings::load_app_settings(storage).ok();
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
                    "AI processing completed for this recording.".to_string(),
                    Vec::new(),
                ),
                "cancelled" => (
                    "cancelled",
                    "AI processing cancelled",
                    "AI processing was cancelled.".to_string(),
                    processing_retry_actions(job_id),
                ),
                _ => (
                    "failed",
                    "AI processing failed",
                    error
                        .map(|error_code| {
                            processing_failure_message(app_settings.as_ref(), error_code)
                        })
                        .unwrap_or_else(|| "AI processing failed.".to_string()),
                    processing_failed_actions(job_id),
                ),
            };
            save_processing_notification(
                connection,
                ProcessingNotificationInput {
                    actions: &actions,
                    job_id: Some(job_id),
                    message: &message,
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
    let app_settings = crate::settings::load_app_settings(storage).ok();
    let message = processing_failure_message(app_settings.as_ref(), error_code);
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
                    message: &message,
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

fn processing_failure_message(
    app_settings: Option<&crate::settings::AppSettings>,
    error_code: &str,
) -> String {
    if let Some(backend) = local_transcription_backend(app_settings) {
        if error_code == "ai_transcription_provider_failed" && backend != "cpu" {
            return format!(
                "Local GPU transcription failed on {}. Switch local transcription backend to CPU in Settings if this keeps happening. ({error_code})",
                transcription_backend_label(backend)
            );
        }
        if error_code == "ai_transcription_local_worker_crashed" {
            return format!(
                "Local Whisper worker crashed while using {}. Retry the job; if it keeps crashing, switch local transcription backend to CPU in Settings. ({error_code})",
                transcription_backend_label(backend)
            );
        }
    }

    format!("AI processing failed: {error_code}.")
}

fn local_transcription_backend(
    app_settings: Option<&crate::settings::AppSettings>,
) -> Option<&str> {
    let app_settings = app_settings?;
    (app_settings.ai.transcription.provider == "local-whisper-cpp")
        .then_some(app_settings.ai.transcription.local_backend.as_str())
}

fn transcription_backend_label(backend: &str) -> &str {
    match backend {
        "cuda" => "CUDA",
        "vulkan" => "Vulkan",
        _ => "CPU",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn processing_failure_message_suggests_cpu_for_local_gpu_transcription_failure() {
        let mut settings = crate::settings::default_settings();
        settings.ai.transcription.local_backend = "vulkan".to_string();

        let message =
            processing_failure_message(Some(&settings), "ai_transcription_provider_failed");

        assert!(message.contains("Local GPU transcription failed on Vulkan"));
        assert!(message.contains("Switch local transcription backend to CPU"));
        assert!(message.contains("ai_transcription_provider_failed"));
    }

    #[test]
    fn processing_failure_message_preserves_raw_code_for_cpu_failure() {
        let settings = crate::settings::default_settings();

        let message =
            processing_failure_message(Some(&settings), "ai_transcription_provider_failed");

        assert_eq!(
            message,
            "AI processing failed: ai_transcription_provider_failed."
        );
    }

    #[test]
    fn processing_failure_message_explains_worker_crash() {
        let settings = crate::settings::default_settings();

        let message =
            processing_failure_message(Some(&settings), "ai_transcription_local_worker_crashed");

        assert!(message.contains("Local Whisper worker crashed"));
        assert!(message.contains("ai_transcription_local_worker_crashed"));
    }
}
