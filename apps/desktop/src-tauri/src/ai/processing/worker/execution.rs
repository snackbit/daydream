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
    let app_settings = settings::load_app_settings(&storage).map_err(settings_error)?;
    let max_parallel_chunks = app_settings
        .ai
        .processing
        .max_parallel_chunks_per_job
        .max(1) as usize;
    let mut active_chunks = futures_util::stream::FuturesUnordered::new();

    loop {
        match handle_current_job_status(&app, &storage, &job_id)? {
            JobStatusAction::Continue => {}
            JobStatusAction::Stop => return Ok(()),
        }

        while active_chunks.len() < max_parallel_chunks {
            let Some(chunk) = claim_next_processing_chunk(&storage, &job_id)? else {
                break;
            };

            let app = app.clone();
            let job_id = job_id.clone();
            active_chunks.push(tauri::async_runtime::spawn(async move {
                process_processing_chunk(&app, &job_id, chunk).await
            }));
        }

        if active_chunks.is_empty() {
            finish_processing_job(&app, &storage, &job_id)?;
            let current = storage
                .with_connection(|connection| storage::get_processing_job(connection, &job_id))
                .map_err(storage_error)?
                .ok_or_else(|| {
                    ai_error("ai_processing_job_missing", "Processing job is missing.")
                })?;
            if current.status == "running" {
                tokio::time::sleep(Duration::from_millis(200)).await;
                continue;
            }
            tracing::info!(
                job_id = %job.id,
                recording_id = %job.recording_id,
                elapsed_ms = elapsed_ms(started),
                "ai_processing_job_finished"
            );
            return Ok(());
        }

        if let Some(result) = active_chunks.next().await {
            let result = result.map_err(|error| {
                ai_error(
                    "ai_processing_chunk_worker_failed",
                    &format!("AI processing chunk worker failed: {error}"),
                )
            })?;
            match result {
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
                    while let Some(_remaining) = active_chunks.next().await {}
                    return Err(error);
                }
            }
        }
    }
}
