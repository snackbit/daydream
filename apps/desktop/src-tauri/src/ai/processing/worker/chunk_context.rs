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
        self.storage
            .with_connection(|connection| {
                storage::update_processing_chunk_status(
                    connection,
                    &self.chunk.id,
                    "succeeded",
                    1.0,
                    None,
                    now_ms(),
                )?;
                if let Some(result_code) = result_code {
                    storage::update_processing_chunk_result_code(
                        connection,
                        &self.chunk.id,
                        Some(result_code),
                        now_ms(),
                    )?;
                }
                Ok(())
            })
            .map_err(storage_error)?;
        save_job_progress_notification(&self.storage, self.job_id)
    }

    pub(super) fn requeue_paused(&self) -> Result<(), AiError> {
        self.update_status("queued", 0.0, None)
    }

    pub(super) fn mark_cancelled(&self) -> Result<(), AiError> {
        self.update_status("cancelled", 0.0, Some("ai_processing_cancelled"))
    }

    pub(super) fn mark_failed(&self, error_code: &str) -> Result<(), AiError> {
        self.update_status("failed", 0.0, Some(error_code))
    }

    fn update_status(
        &self,
        status: &str,
        progress: f64,
        error_code: Option<&str>,
    ) -> Result<(), AiError> {
        self.storage
            .with_connection(|connection| {
                storage::update_processing_chunk_status(
                    connection,
                    &self.chunk.id,
                    status,
                    progress,
                    error_code,
                    now_ms(),
                )
                .map(|_row| ())
            })
            .map_err(storage_error)
    }
}
