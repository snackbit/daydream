use super::super::super::*;

pub(super) struct ProcessingChunkContext<'a> {
    app: &'a AppHandle,
    chunk: &'a ProcessingChunkRow,
    job_id: &'a str,
    storage: State<'a, AppStorage>,
}

impl<'a> ProcessingChunkContext<'a> {
    pub(super) fn new(app: &'a AppHandle, job_id: &'a str, chunk: &'a ProcessingChunkRow) -> Self {
        Self {
            app,
            chunk,
            job_id,
            storage: app.state::<AppStorage>(),
        }
    }

    pub(super) fn emit_processing_state(&self) {
        emit_processing_state(self.app, &self.chunk.recording_id);
    }

    pub(super) fn current_job(&self) -> Result<ProcessingJobRow, AiError> {
        self.storage
            .with_connection(|connection| storage::get_processing_job(connection, self.job_id))
            .map_err(storage_error)?
            .ok_or_else(|| ai_error("ai_processing_job_missing", "Processing job is missing."))
    }

    pub(super) fn mark_succeeded(&self, result_code: Option<&str>) -> Result<(), AiError> {
        let updated = self
            .storage
            .with_connection(|connection| {
                let current = storage::get_processing_chunk(connection, &self.chunk.id)?
                    .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
                if current.attempt_count != self.chunk.attempt_count {
                    return Ok(current);
                }
                let error = storage::first_processing_chunk_failure(connection, &self.chunk.id)?;
                let updated = storage::update_processing_chunk_status_if_active(
                    connection,
                    &self.chunk.id,
                    if error.is_some() {
                        "completed-with-errors"
                    } else {
                        "succeeded"
                    },
                    1.0,
                    error.as_deref(),
                    now_ms(),
                )?;
                if matches!(
                    updated.status.as_str(),
                    "succeeded" | "completed-with-errors"
                ) {
                    if let Some(result_code) = result_code {
                        storage::update_processing_chunk_result_code(
                            connection,
                            &self.chunk.id,
                            Some(result_code),
                            now_ms(),
                        )?;
                    }
                }
                Ok(updated)
            })
            .map_err(storage_error)?;
        if updated.status == "succeeded" {
            save_job_progress_notification(&self.storage, self.job_id)?;
        }
        Ok(())
    }

    pub(super) fn requeue_paused(&self) -> Result<(), AiError> {
        self.update_status("queued", 0.0, None).map(|_updated| ())
    }

    pub(super) fn mark_cancelled(&self) -> Result<(), AiError> {
        self.update_status("cancelled", 0.0, Some("ai_processing_cancelled"))
            .map(|_updated| ())
    }

    pub(super) fn mark_failed(&self, error_code: &str) -> Result<(), AiError> {
        super::chunk_failure::fail_chunk(&self.storage, self.chunk, error_code, now_ms())
    }

    fn update_status(
        &self,
        status: &str,
        progress: f64,
        error_code: Option<&str>,
    ) -> Result<ProcessingChunkRow, AiError> {
        self.storage
            .with_connection(|connection| {
                let current = storage::get_processing_chunk(connection, &self.chunk.id)?
                    .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
                if current.attempt_count != self.chunk.attempt_count {
                    return Ok(current);
                }
                if matches!(status, "cancelled" | "failed") {
                    storage::cancel_visual_frame_processing_states_for_chunk(
                        connection,
                        &self.chunk.id,
                        now_ms(),
                    )?;
                }
                storage::update_processing_chunk_status_if_active(
                    connection,
                    &self.chunk.id,
                    status,
                    progress,
                    error_code,
                    now_ms(),
                )
            })
            .map_err(storage_error)
    }
}
