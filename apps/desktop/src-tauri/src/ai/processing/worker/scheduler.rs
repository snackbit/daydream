use super::super::super::*;
use super::execution::process_recording_ai_job;

fn runnable_jobs(
    connection: &rusqlite::Connection,
    statuses: &[&str],
    limit: Option<usize>,
) -> rusqlite::Result<Vec<ProcessingJobRow>> {
    if statuses == ["queued"] {
        let mut statement=connection.prepare("SELECT j.id,j.recording_id,j.kind,j.status,j.created_at_ms,j.updated_at_ms,j.error FROM processing_jobs j WHERE j.kind IN ('ai-processing','ocr','waveform') AND j.status='queued' AND (EXISTS(SELECT 1 FROM processing_chunks c LEFT JOIN processing_chunk_runtime r ON r.chunk_id=c.id WHERE c.job_id=j.id AND c.status='queued' AND (r.next_retry_at_ms IS NULL OR r.next_retry_at_ms<=?1)) OR (NOT EXISTS(SELECT 1 FROM processing_chunks WHERE job_id=j.id AND status IN ('queued','running')) AND NOT EXISTS(SELECT 1 FROM processing_job_runtime WHERE job_id=j.id AND input_open=1))) ORDER BY j.updated_at_ms,j.id LIMIT ?2")?;
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
    jobs.extend(storage::list_processing_jobs_by_status(
        connection, "waveform", statuses, limit,
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
                // Older writers could finish the parent before all admitted sections.
                // Resume that authorized work; explicit failure/cancellation stays terminal.
                if job.status == "succeeded"
                    && !storage::list_processing_chunks_by_status(
                        connection,
                        &job.id,
                        &["queued", "running"],
                        Some(1),
                    )?
                    .is_empty()
                {
                    storage::requeue_processing_chunks(
                        connection,
                        &job.id,
                        &["running"],
                        timestamp_ms,
                    )?;
                    storage::update_processing_job_status(
                        connection,
                        &job.id,
                        "queued",
                        None,
                        timestamp_ms,
                    )?;
                    continue;
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
                        &["queued", "running"],
                        timestamp_ms,
                    )?;
                }
            }
            Ok(())
        })
        .map_err(storage_error)
}

#[cfg(test)]
mod recovery_tests {
    use super::*;
    #[test]
    fn completed_parent_with_unfinished_authorized_sections_resumes_but_cancelled_jobs_do_not() {
        let storage = storage::test_storage(
            std::env::temp_dir().join(format!("synthetic-processing-recovery-{}", Uuid::now_v7())),
        );
        let chunk = storage
            .with_connection(|connection| {
                let (chunk, _) = crate::ai::tests::test_transcription_chunk(connection);
                storage::update_processing_job_status(
                    connection,
                    &chunk.job_id,
                    "succeeded",
                    None,
                    now_ms(),
                )?;
                Ok(chunk)
            })
            .unwrap();
        recover_stale_processing(&storage).unwrap();
        storage
            .with_connection(|connection| {
                assert_eq!(
                    storage::get_processing_job(connection, &chunk.job_id)?
                        .unwrap()
                        .status,
                    "queued"
                );
                assert_eq!(
                    storage::get_processing_chunk(connection, &chunk.id)?
                        .unwrap()
                        .status,
                    "queued"
                );
                storage::update_processing_job_status(
                    connection,
                    &chunk.job_id,
                    "cancelled",
                    None,
                    now_ms(),
                )?;
                Ok(())
            })
            .unwrap();
        recover_stale_processing(&storage).unwrap();
        storage
            .with_connection(|connection| {
                assert_eq!(
                    storage::get_processing_job(connection, &chunk.job_id)?
                        .unwrap()
                        .status,
                    "cancelled"
                );
                assert_eq!(
                    storage::get_processing_chunk(connection, &chunk.id)?
                        .unwrap()
                        .status,
                    "cancelled"
                );
                Ok(())
            })
            .unwrap();
    }
}

pub(in crate::ai) async fn run_processing_worker_tick(app: &AppHandle) -> Result<(), AiError> {
    let storage = app.state::<AppStorage>();
    let app_settings = settings::load_app_settings(&storage).map_err(settings_error)?;
    let owners = app.state::<super::ownership::ProcessingWorkerOwners>();
    let owned = owners.snapshot();
    let running = storage
        .with_connection(|connection| runnable_jobs(connection, &["running"], None))
        .map_err(storage_error)?;
    // An unowned running row cannot do useful work or retain a capacity slot.
    for job in &running {
        if !owned.contains_key(&job.id) {
            super::completion::fail_processing_job(
                app,
                &storage,
                &job.id,
                "ai_processing_job_worker_failed",
            )?;
        }
    }
    let remote_only = storage
        .with_connection(|connection| {
            let mut ids = std::collections::BTreeSet::new();
            for id in owned.keys() {
                if observations_only(connection, id)? {
                    ids.insert(id.clone());
                }
            }
            Ok(ids)
        })
        .map_err(storage_error)?;
    let mut remote_available = app_settings.ai.processing.max_parallel_jobs.max(1) as usize;
    remote_available = remote_available.saturating_sub(remote_only.len());
    let mut available = app_settings
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
        .saturating_sub(
            owned
                .iter()
                .filter(|(id, kind)| kind.as_str() != "waveform" && !remote_only.contains(*id))
                .count() as u32,
        ) as usize;
    let mut waveform_available = !owned.values().any(|kind| kind == "waveform");
    if available == 0 && !waveform_available && remote_available == 0 {
        log_capacity_full_throttled(running.len(), app_settings.ai.processing.max_parallel_jobs);
        return Ok(());
    }
    let queued = storage
        .with_connection(|connection| runnable_jobs(connection, &["queued"], Some(200)))
        .map_err(storage_error)?;
    log_worker_tick(running.len(), queued.len(), available);

    for job in queued {
        let remote = storage
            .with_connection(|connection| observations_only(connection, &job.id))
            .map_err(storage_error)?;
        if job.kind == "waveform" {
            if !waveform_available {
                continue;
            }
        } else if remote {
            if remote_available == 0 {
                continue;
            }
        } else {
            if available == 0 {
                continue;
            }
        }
        let Some(ownership) = owners.reserve(&job.id, &job.kind) else {
            continue;
        };
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
            if claimed.kind == "waveform" {
                waveform_available = false;
            } else if remote {
                remote_available -= 1;
            } else {
                available -= 1;
            }
            tracing::info!(
                recording_id = %claimed.recording_id,
                job_id = %claimed.id,
                "ai_processing_job_claimed"
            );
            crate::ai::results::emit_processing_state_with_source(
                app,
                &claimed.recording_id,
                (claimed.kind == "waveform").then_some("waveform"),
            );
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                let worker_app = app.clone();
                let worker_id = claimed.id.clone();
                let worker_ownership = ownership.clone();
                let result = tauri::async_runtime::spawn(async move {
                    process_recording_ai_job(worker_app, worker_id, worker_ownership).await
                })
                .await;
                let code = match result {
                    Ok(Ok(())) => None,
                    Ok(Err(error)) => Some(error.code),
                    Err(_) => Some("ai_processing_job_worker_failed".into()),
                };
                if let Some(code) = code {
                    if let Err(error) = super::completion::fail_processing_job(
                        &app,
                        &app.state::<AppStorage>(),
                        &claimed.id,
                        &code,
                    ) {
                        tracing::warn!(error_code=%error.code,"Processing worker failure could not be saved.");
                    }
                }
                drop(ownership);
                crate::background_tasks::emit_changed(&app, Some(&claimed.recording_id));
            });
        }
    }

    Ok(())
}

pub(crate) fn observations_only(
    connection: &rusqlite::Connection,
    job: &str,
) -> rusqlite::Result<bool> {
    connection.query_row("SELECT EXISTS(SELECT 1 FROM processing_chunks WHERE job_id=?1 AND source_kind='vlm' AND status IN('queued','running')) AND NOT EXISTS(SELECT 1 FROM processing_chunks WHERE job_id=?1 AND source_kind!='vlm' AND status IN('queued','running')) AND NOT EXISTS(SELECT 1 FROM processing_job_runtime WHERE job_id=?1 AND input_open=1)",[job],|row|row.get(0))
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
    fn finished_local_work_releases_capacity_while_observations_keep_their_owner() {
        let storage = crate::storage::test_storage(std::env::temp_dir());
        storage.with_connection(|connection| {
            let (chunk,_)=crate::ai::tests::test_transcription_chunk(connection);
            connection.execute("UPDATE processing_chunks SET source_kind='vlm' WHERE id=?1",[&chunk.id])?;
            assert!(super::observations_only(connection,&chunk.job_id)?);
            crate::storage::set_processing_job_runtime(connection,&chunk.job_id,true,"{}")?;
            assert!(!super::observations_only(connection,&chunk.job_id)?);
            crate::storage::set_processing_job_runtime(connection,&chunk.job_id,false,"{}")?;
            crate::storage::create_processing_chunk(connection,&crate::storage::CreateProcessingChunkInput {id:uuid::Uuid::now_v7().to_string(),job_id:chunk.job_id.clone(),recording_id:chunk.recording_id,source_kind:"ocr".into(),track_id:None,segment_id:chunk.segment_id,start_offset_ms:Some(0),end_offset_ms:Some(10_000),status:"queued".into(),progress:0.,attempt_count:0,error_code:None,result_code:None,created_at_ms:1,updated_at_ms:1})?;
            assert!(!super::observations_only(connection,&chunk.job_id)?);
            connection.execute("UPDATE processing_chunks SET status='succeeded',progress=1 WHERE job_id=?1 AND source_kind='ocr'",[&chunk.job_id])?;
            assert!(super::observations_only(connection,&chunk.job_id)?);
            Ok(())
        }).unwrap();
    }

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
