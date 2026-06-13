use super::super::super::*;
use super::{
    chunk_runner::process_processing_chunk,
    completion::{fail_processing_job, finish_processing_job},
    dispatch::claim_next_processing_chunk,
    job_state::{handle_current_job_status, JobStatusAction},
};

pub(super) async fn process_recording_ai_job(
    app: AppHandle,
    job_id: String,
) -> Result<(), AiError> {
    let started = Instant::now();
    let storage = app.state::<AppStorage>();
    let job = storage
        .with_connection(|connection| storage::get_processing_job(connection, &job_id))
        .map_err(storage_error)?
        .ok_or_else(|| ai_error("ai_processing_job_missing", "Processing job is missing."))?;
    tracing::info!(
        job_id = %job.id,
        recording_id = %job.recording_id,
        "AI processing job started."
    );
    save_job_running_notification(&storage, &job)?;

    loop {
        match handle_current_job_status(&app, &storage, &job_id)? {
            JobStatusAction::Continue => {}
            JobStatusAction::Stop => return Ok(()),
        }

        let Some(chunk) = claim_next_processing_chunk(&storage, &job_id)? else {
            finish_processing_job(&app, &storage, &job_id)?;
            tracing::info!(
                job_id = %job.id,
                recording_id = %job.recording_id,
                elapsed_ms = elapsed_ms(started),
                "ai_processing_job_finished"
            );
            return Ok(());
        };

        match process_processing_chunk(&app, &job_id, chunk).await {
            Ok(()) => {}
            Err(error) if error.code == "ai_processing_paused" => return Ok(()),
            Err(error) if error.code == "ai_processing_cancelled" => return Ok(()),
            Err(error) => {
                tracing::warn!(
                    job_id = %job.id,
                    recording_id = %job.recording_id,
                    error_code = %error.code,
                    elapsed_ms = elapsed_ms(started),
                    "ai_processing_job_chunk_failed"
                );
                fail_processing_job(&app, &storage, &job_id, &error.code)?;
                return Err(error);
            }
        }
    }
}
