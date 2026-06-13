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
        log_capacity_full_throttled(running.len(), app_settings.ai.processing.max_parallel_jobs);
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
    log_worker_tick(running.len(), queued.len(), available);

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

fn log_capacity_full_throttled(running_job_count: usize, max_parallel_jobs: u32) {
    static LAST_LOGGED_AT: OnceLock<Mutex<Option<Instant>>> = OnceLock::new();
    let now = Instant::now();
    let lock = LAST_LOGGED_AT.get_or_init(|| Mutex::new(None));
    let Ok(mut last_logged_at) = lock.lock() else {
        return;
    };
    if last_logged_at.is_some_and(|previous| now.duration_since(previous) < Duration::from_secs(10))
    {
        return;
    }
    *last_logged_at = Some(now);
    tracing::debug!(
        running_job_count,
        max_parallel_jobs,
        "ai_processing_worker_capacity_full"
    );
}

fn log_worker_tick(running_job_count: usize, queued_job_count: usize, available_job_slots: usize) {
    if !should_log_worker_tick(queued_job_count) {
        return;
    }

    tracing::debug!(
        running_job_count,
        queued_job_count,
        available_job_slots,
        "ai_processing_worker_tick"
    );
}

fn should_log_worker_tick(queued_job_count: usize) -> bool {
    queued_job_count > 0
}

#[cfg(test)]
mod tests {
    use super::should_log_worker_tick;

    #[test]
    fn worker_tick_log_is_silent_when_no_jobs_are_queued() {
        assert!(!should_log_worker_tick(0));
    }

    #[test]
    fn worker_tick_log_is_emitted_when_jobs_are_queued() {
        assert!(should_log_worker_tick(1));
        assert!(should_log_worker_tick(4));
    }
}
