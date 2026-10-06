use super::super::super::*;
use super::execution::process_recording_ai_job;

fn runnable_jobs(
    connection: &rusqlite::Connection,
    statuses: &[&str],
    limit: Option<usize>,
) -> rusqlite::Result<Vec<ProcessingJobRow>> {
    if statuses == ["queued"] {
        let mut statement=connection.prepare("SELECT j.id,j.recording_id,j.kind,j.status,j.created_at_ms,j.updated_at_ms,j.error FROM processing_jobs j WHERE j.kind IN ('ai-processing','ocr') AND j.status='queued' AND (EXISTS(SELECT 1 FROM processing_chunks c LEFT JOIN processing_chunk_runtime r ON r.chunk_id=c.id WHERE c.job_id=j.id AND c.status='queued' AND (r.next_retry_at_ms IS NULL OR r.next_retry_at_ms<=?1)) OR (NOT EXISTS(SELECT 1 FROM processing_chunks WHERE job_id=j.id AND status IN ('queued','running')) AND NOT EXISTS(SELECT 1 FROM processing_job_runtime WHERE job_id=j.id AND input_open=1))) ORDER BY j.updated_at_ms,j.id LIMIT ?2")?;
        let rows = statement.query_map(
            rusqlite::params![now_ms(), limit.unwrap_or(200) as i64],
            |row| {
                Ok(ProcessingJobRow {
                    id: row.get(0)?,
                    recording_id: row.get(1)?,
                    kind: row.get(2)?,
                    status: row.get(3)?,
                    created_at_ms: row.get(4)?,
                    updated_at_ms: row.get(5)?,
                    error: row.get(6)?,
                })
            },
        )?;
        return rows.collect();
    }
    let mut jobs = storage::list_processing_jobs_by_status(
        connection,
        AI_PROCESSING_JOB_KIND,
        statuses,
        limit,
    )?;
    jobs.extend(storage::list_processing_jobs_by_status(
        connection, "ocr", statuses, limit,
    )?);
    jobs.sort_by_key(|job| job.updated_at_ms);
    if let Some(limit) = limit {
        jobs.truncate(limit);
    }
    Ok(jobs)
}

pub(in crate::ai) fn recover_stale_processing(storage: &AppStorage) -> Result<(), AiError> {
    storage
        .with_connection(|connection| {
            let timestamp_ms = now_ms();
            let jobs = runnable_jobs(
                connection,
                &[
                    "running",
                    "queued",
                    "paused",
                    "cancelled",
                    "failed",
                    "succeeded",
                    "completed-with-errors",
                ],
                None,
            )?;
            for job in jobs {
                let recording = storage::get_recording(connection, &job.recording_id)?;
                if !matches!(recording.status.as_str(), "recording" | "paused") {
                    connection.execute(
                        "UPDATE processing_job_runtime SET input_open=0 WHERE job_id=?1",
                        [&job.id],
                    )?;
                }
                if matches!(job.status.as_str(), "running" | "queued" | "paused") {
                    storage::requeue_processing_chunks(
                        connection,
                        &job.id,
                        &["running"],
                        timestamp_ms,
                    )?;
                    if job.status == "running" {
                        storage::update_processing_job_status(
                            connection,
                            &job.id,
                            "queued",
                            None,
                            timestamp_ms,
                        )?;
                    }
                } else {
                    storage::cancel_processing_chunks(
                        connection,
                        &job.id,
                        &["running"],
                        timestamp_ms,
                    )?;
                }
            }
            Ok(())
        })
        .map_err(storage_error)
}

pub(in crate::ai) async fn run_processing_worker_tick(app: &AppHandle) -> Result<(), AiError> {
    let storage = app.state::<AppStorage>();
    let app_settings = settings::load_app_settings(&storage).map_err(settings_error)?;
    let running = storage
        .with_connection(|connection| runnable_jobs(connection, &["running"], None))
        .map_err(storage_error)?;
    let available = app_settings
        .ai
        .processing
        .max_parallel_jobs
        .min(
            if app_settings.ai.processing.resource_mode == "background" {
                1
            } else {
                u32::MAX
            },
        )
        .saturating_sub(running.len() as u32) as usize;
    if available == 0 {
        log_capacity_full_throttled(running.len(), app_settings.ai.processing.max_parallel_jobs);
        return Ok(());
    }
    let queued = storage
        .with_connection(|connection| runnable_jobs(connection, &["queued"], Some(available)))
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
