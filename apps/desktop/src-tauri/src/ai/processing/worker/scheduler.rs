use super::super::super::*;
use super::execution::process_recording_ai_job;

pub(in crate::ai) fn recover_stale_processing(storage: &AppStorage) -> Result<(), AiError> {
    storage
        .with_connection(|connection| {
            let timestamp_ms = now_ms();
            let jobs = storage::list_processing_jobs_by_status(
                connection,
                AI_PROCESSING_JOB_KIND,
                &["running"],
                None,
            )?;
            for job in jobs {
                storage::update_processing_job_status(
                    connection,
                    &job.id,
                    "queued",
                    None,
                    timestamp_ms,
                )?;
                storage::requeue_processing_chunks(
                    connection,
                    &job.id,
                    &["running"],
                    timestamp_ms,
                )?;
            }
            Ok(())
        })
        .map_err(storage_error)
}

pub(in crate::ai) async fn run_processing_worker_tick(app: &AppHandle) -> Result<(), AiError> {
    let storage = app.state::<AppStorage>();
    let app_settings = settings::load_app_settings(&storage).map_err(settings_error)?;
    let running = storage
        .with_connection(|connection| {
            storage::list_processing_jobs_by_status(
                connection,
                AI_PROCESSING_JOB_KIND,
                &["running"],
                None,
            )
        })
        .map_err(storage_error)?;
    let available = app_settings
        .ai
        .processing
        .max_parallel_jobs
        .saturating_sub(running.len() as u32) as usize;
    if available == 0 {
        tracing::debug!(
            running_job_count = running.len(),
            max_parallel_jobs = app_settings.ai.processing.max_parallel_jobs,
            "ai_processing_worker_capacity_full"
        );
        return Ok(());
    }
    let queued = storage
        .with_connection(|connection| {
            storage::list_processing_jobs_by_status(
                connection,
                AI_PROCESSING_JOB_KIND,
                &["queued"],
                Some(available),
            )
        })
        .map_err(storage_error)?;
    tracing::debug!(
        running_job_count = running.len(),
        queued_job_count = queued.len(),
        available_job_slots = available,
        "ai_processing_worker_tick"
    );

    for job in queued {
        let claimed = storage
            .with_connection(|connection| {
                let current = storage::get_processing_job(connection, &job.id)?
                    .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
                if current.status != "queued" {
                    return Ok(None);
                }
                storage::update_processing_job_status(
                    connection,
                    &job.id,
                    "running",
                    None,
                    now_ms(),
                )
                .map(Some)
            })
            .map_err(storage_error)?;
        if let Some(claimed) = claimed {
            tracing::info!(
                recording_id = %claimed.recording_id,
                job_id = %claimed.id,
                "ai_processing_job_claimed"
            );
            emit_processing_state(app, &claimed.recording_id);
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(error) = process_recording_ai_job(app.clone(), claimed.id.clone()).await
                {
                    let storage = app.state::<AppStorage>();
                    let _ = storage.with_connection(|connection| {
                        storage::update_processing_job_status(
                            connection,
                            &claimed.id,
                            "failed",
                            Some(&error.code),
                            now_ms(),
                        )?;
                        save_processing_notification(
                            connection,
                            ProcessingNotificationInput {
                                actions: &processing_failed_actions(&claimed.id),
                                job_id: Some(&claimed.id),
                                message: &format!("AI processing failed: {}.", error.code),
                                recording_id: &claimed.recording_id,
                                status: "failed",
                                timestamp_ms: now_ms(),
                                title: "AI processing failed",
                            },
                        )
                        .map(|_row| ())
                    });
                    emit_processing_state(&app, &claimed.recording_id);
                }
            });
        }
    }

    Ok(())
}
