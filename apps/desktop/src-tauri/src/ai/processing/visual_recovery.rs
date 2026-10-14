//! Explicit local recovery drains owners before scoped visual replacement.
use super::super::*;
struct RecoveryGuard(String);
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct StoppedJob {
    id: String,
    status: String,
    pending: Vec<String>,
}
fn recovering() -> &'static Mutex<HashSet<String>> {
    static ACTIVE: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    ACTIVE.get_or_init(|| Mutex::new(HashSet::new()))
}
impl Drop for RecoveryGuard {
    fn drop(&mut self) {
        recovering()
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .remove(&self.0);
    }
}

pub(crate) async fn optimize(app: &AppHandle, recording: &str) -> Result<ProcessingJob, AiError> {
    if !recovering()
        .lock()
        .map_err(|_| {
            ai_error(
                "ai_processing_worker_state_failed",
                "Recovery state is unavailable.",
            )
        })?
        .insert(recording.into())
    {
        return Err(ai_error(
            "ai_processing_retry_pending",
            "Visual recovery is already running.",
        ));
    }
    let _recovery = RecoveryGuard(recording.into());
    let storage = app.state::<AppStorage>();
    let status = storage
        .with_connection(|connection| storage::get_recording(connection, recording))
        .map_err(storage_error)?
        .status;
    if matches!(status.as_str(), "recording" | "paused") {
        return Err(ai_error(
            "ai_reprocessing_recording_active",
            "Finish recording before replacing visual processing.",
        ));
    }
    resolve_tesseract_runtime(app)?;
    let jobs = storage
        .with_connection(|connection| {
            let jobs = storage::list_processing_jobs(connection, Some(recording))?;
            let mut owners = Vec::new();
            let mut saved=connection.prepare("SELECT j.id,r.visual_recovery_json FROM processing_job_runtime r JOIN processing_jobs j ON j.id=r.job_id WHERE j.recording_id=?1 AND r.visual_recovery_json IS NOT NULL")?;
            for snapshot in saved.query_map([recording],|row|Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?)))? {
                let (id,snapshot)=snapshot?;
                let saved:StoppedJob=serde_json::from_str(&snapshot).map_err(|_|rusqlite::Error::InvalidQuery)?;
                if saved.id!=id || !matches!(saved.status.as_str(),"queued"|"running"|"paused") {return Err(rusqlite::Error::InvalidQuery);}
                owners.push(saved);
            }
            for job in jobs
                .into_iter()
                .filter(|job| matches!(job.status.as_str(), "queued" | "running" | "paused"))
            {
                if owners.iter().any(|saved|saved.id==job.id) {continue;}
                // Cancel only mixed/visual owners. Waveform files remain independent.
                let chunks = storage::list_processing_chunks(connection, &job.id)?;
                if chunks
                    .iter()
                    .any(|chunk| matches!(chunk.source_kind.as_str(), "ocr" | "vlm"))
                {
                    let retained = chunks
                        .iter()
                        .filter(|chunk| {
                            matches!(chunk.status.as_str(), "queued" | "running")
                        })
                        .map(|chunk| chunk.id.clone())
                        .collect::<Vec<_>>();
                    owners.push(StoppedJob {id:job.id,status:job.status,pending:retained});
                }
            }
            Ok(owners)
        })
        .map_err(storage_error)?;
    stop_owners(&storage, &jobs)?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(65);
    loop {
        let owned = app.state::<super::ProcessingWorkerOwners>().snapshot();
        let active = storage
            .with_connection(|connection| {
                Ok(storage::list_processing_jobs(connection, Some(recording))?
                    .iter()
                    .any(|job| owned.contains_key(&job.id)))
            })
            .map_err(storage_error)?;
        if !active {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(ai_error(
                "ai_processing_retry_pending",
                "Visual workers are still stopping. Retry when they have stopped.",
            ));
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let _privacy = tokio::time::timeout_at(
        deadline,
        crate::recording_privacy::mutation_lease(recording),
    )
    .await
    .map_err(|_| {
        ai_error(
            "ai_processing_retry_pending",
            "Recording processing is still stopping. Retry when it has stopped.",
        )
    })?;
    let result = admit_replacement(&storage, recording, &jobs);
    emit_processing_state(app, recording);
    result
}

// The caller has drained worker ownership and holds the recording mutation lease.
fn admit_replacement(
    storage: &AppStorage,
    recording: &str,
    jobs: &[StoppedJob],
) -> Result<ProcessingJob, AiError> {
    let result = settings::use_fast_local_processing(storage)
        .map_err(settings_error)
        .and_then(|_| super::reprocessing::whole_visual(storage, recording));
    // Failed admission must return all interrupted work to its original owner.
    restore_retained(storage, jobs, result.is_ok())?;
    result
}
fn stop_owners(storage: &AppStorage, jobs: &[StoppedJob]) -> Result<(), AiError> {
    storage.with_connection_mut(|connection| {
        let transaction=connection.transaction()?;
        for job in jobs {
            let snapshot=serde_json::to_string(job).map_err(|_|rusqlite::Error::InvalidQuery)?;
            transaction.execute("INSERT INTO processing_job_runtime(job_id,visual_recovery_json) VALUES(?1,?2) ON CONFLICT(job_id) DO UPDATE SET visual_recovery_json=excluded.visual_recovery_json",rusqlite::params![job.id,snapshot])?;
            storage::cancel_processing_chunks(&transaction,&job.id,&["queued","running"],now_ms())?;
            storage::update_processing_job_status(&transaction,&job.id,"completed-with-errors",Some("ai_processing_retry_pending"),now_ms())?;
        }
        transaction.commit()
    }).map_err(storage_error)
}
fn restore_retained(
    storage: &AppStorage,
    jobs: &[StoppedJob],
    replacement_admitted: bool,
) -> Result<(), AiError> {
    storage.with_connection_mut(|connection| {
        let transaction=connection.transaction()?;
        for job in jobs {
            for chunk in &job.pending {
                transaction.execute("UPDATE processing_chunks SET status='queued',progress=0,error_code=NULL,updated_at_ms=?2 WHERE id=?1 AND job_id=?3 AND status='cancelled' AND (?4=0 OR source_kind NOT IN('ocr','vlm'))",rusqlite::params![chunk,now_ms(),job.id,replacement_admitted])?;
            }
            let pending: bool = transaction.query_row("SELECT EXISTS(SELECT 1 FROM processing_chunks WHERE job_id=?1 AND status IN('queued','running'))",[&job.id],|row|row.get(0))?;
            let restored=if !pending {"completed-with-errors"}else if job.status=="paused" {"paused"}else{"queued"};
            storage::update_processing_job_status(&transaction,&job.id,restored,None,now_ms())?;
            transaction.execute("UPDATE processing_job_runtime SET visual_recovery_json=NULL WHERE job_id=?1",[&job.id])?;
        }
        transaction.commit()
    }).map_err(storage_error)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn blocked_replacement_restores_visual_work_without_resuming_independent_paused_waveforms() {
        for status in ["running", "paused"] {
            let root =
                std::env::temp_dir().join(format!("daydream-recovery-fixture-{}", Uuid::now_v7()));
            fs::create_dir_all(&root).unwrap();
            let media = root.join("synthetic.mkv");
            // Admission checks presence; this blocked scenario never decodes media.
            fs::write(&media, b"synthetic-source").unwrap();
            let storage = storage::test_storage(std::env::temp_dir());
            let visual_id = Uuid::now_v7().to_string();
            let waveform_job = Uuid::now_v7().to_string();
            let chunk = storage.with_connection(|connection| {
                let (chunk, _) = crate::ai::tests::test_transcription_chunk(connection);
                connection.execute("UPDATE recordings SET status='completed',ended_at_ms=10001 WHERE id=?1",[&chunk.recording_id])?;
                connection.execute("UPDATE recording_segments SET state='finalized',file_path=?2,video_stream_index=0,duration_ms=10000,ended_at_ms=10001,timeline_base_ms=0,timeline_basis_version=1 WHERE id=?1",rusqlite::params![chunk.segment_id,media.to_string_lossy()])?;
                storage::update_processing_job_status(connection,&chunk.job_id,status,None,now_ms())?;
                connection.execute("INSERT INTO processing_chunks(id,job_id,recording_id,source_kind,segment_id,track_id,start_offset_ms,end_offset_ms,status,attempt_count,progress,error_code,result_code,created_at_ms,updated_at_ms) SELECT ?2,job_id,recording_id,'ocr',segment_id,NULL,start_offset_ms,end_offset_ms,status,attempt_count,progress,error_code,result_code,created_at_ms,updated_at_ms FROM processing_chunks WHERE id=?1",rusqlite::params![chunk.id,visual_id])?;
                storage::create_processing_job(connection, &CreateProcessingJobInput {
                    id: waveform_job.clone(), recording_id: chunk.recording_id.clone(), kind: "waveform".into(), status: "paused".into(), created_at_ms: 1, updated_at_ms: 1, error: None,
                })?;
                Ok(chunk)
            }).unwrap();
            let jobs = vec![StoppedJob {
                id: chunk.job_id.clone(),
                status: status.into(),
                pending: vec![chunk.id.clone(), visual_id.clone()],
            }];
            stop_owners(&storage, &jobs).unwrap();
            let error = admit_replacement(&storage, &chunk.recording_id, &jobs).unwrap_err();
            assert_eq!(error.code, "ai_reprocessing_preview_changed");
            storage
                .with_connection(|connection| {
                    assert_eq!(
                        storage::get_processing_chunk(connection, &visual_id)?
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
                    assert_eq!(
                        storage::get_processing_job(connection, &chunk.job_id)?
                            .unwrap()
                            .status,
                        if status == "paused" {
                            "paused"
                        } else {
                            "queued"
                        }
                    );
                    assert_eq!(
                        storage::get_processing_job(connection, &waveform_job)?
                            .unwrap()
                            .status,
                        "paused"
                    );
                    let snapshot: Option<String> = connection.query_row(
                        "SELECT visual_recovery_json FROM processing_job_runtime WHERE job_id=?1",
                        [&chunk.job_id],
                        |row| row.get(0),
                    )?;
                    assert!(
                        snapshot.is_none(),
                        "restored work no longer needs a recovery snapshot"
                    );
                    assert_eq!(
                        storage::list_processing_jobs(connection, Some(&chunk.recording_id))?.len(),
                        2
                    );
                    Ok(())
                })
                .unwrap();
            fs::remove_dir_all(root).unwrap();
        }
    }
    #[test]
    fn persisted_recovery_restores_nonvisual_work_and_keeps_pause_intent() {
        for status in ["running", "paused"] {
            let storage = storage::test_storage(std::env::temp_dir());
            let visual_id = Uuid::now_v7().to_string();
            let chunk = storage.with_connection(|connection| {
                let (chunk, _) = crate::ai::tests::test_transcription_chunk(connection);
                connection.execute("INSERT INTO processing_chunks(id,job_id,recording_id,source_kind,segment_id,track_id,start_offset_ms,end_offset_ms,status,attempt_count,progress,error_code,result_code,created_at_ms,updated_at_ms) SELECT ?2,job_id,recording_id,'ocr',segment_id,track_id,start_offset_ms,end_offset_ms,status,attempt_count,progress,error_code,result_code,created_at_ms,updated_at_ms FROM processing_chunks WHERE id=?1",rusqlite::params![chunk.id,visual_id])?;
                Ok(chunk)
            }).unwrap();
            let jobs = vec![StoppedJob {
                id: chunk.job_id.clone(),
                status: status.into(),
                pending: vec![chunk.id.clone(), visual_id.clone()],
            }];
            stop_owners(&storage, &jobs).unwrap();
            // Simulate a new caller after interrupted recovery by reading only
            // the durable snapshot; no original in-memory intent is retained.
            let saved: StoppedJob = storage
                .with_connection(|connection| {
                    let json: String = connection.query_row(
                        "SELECT visual_recovery_json FROM processing_job_runtime WHERE job_id=?1",
                        [&chunk.job_id],
                        |row| row.get(0),
                    )?;
                    Ok(serde_json::from_str(&json).unwrap())
                })
                .unwrap();
            restore_retained(&storage, &[saved], true).unwrap();
            storage
                .with_connection(|connection| {
                    assert_eq!(
                        storage::get_processing_chunk(connection, &chunk.id)?
                            .unwrap()
                            .status,
                        "queued"
                    );
                    assert_eq!(
                        storage::get_processing_chunk(connection, &visual_id)?
                            .unwrap()
                            .status,
                        "cancelled"
                    );
                    assert_eq!(
                        storage::get_processing_job(connection, &chunk.job_id)?
                            .unwrap()
                            .status,
                        if status == "paused" {
                            "paused"
                        } else {
                            "queued"
                        }
                    );
                    let snapshot: Option<String> = connection.query_row(
                        "SELECT visual_recovery_json FROM processing_job_runtime WHERE job_id=?1",
                        [&chunk.job_id],
                        |row| row.get(0),
                    )?;
                    assert!(snapshot.is_none());
                    Ok(())
                })
                .unwrap();
        }
    }
    #[test]
    fn recovery_keeps_completed_audio_in_recall_after_draining_a_mixed_owner() {
        let storage = storage::test_storage(std::env::temp_dir());
        let (fixture, job) = storage
            .with_connection(|connection| {
                let fixture = storage::seed_recall_fixture(connection);
                let job = connection.query_row(
                    "SELECT job_id FROM processing_chunks WHERE recording_id=?1",
                    [&fixture.recording],
                    |row| row.get::<_, String>(0),
                )?;
                storage::refresh_evidence(connection)?;
                connection.execute(
                    "UPDATE processing_chunks SET status='succeeded',progress=1 WHERE job_id=?1",
                    [&job],
                )?;
                storage::refresh_evidence(connection)?;
                assert_eq!(
                    connection
                        .query_row("SELECT COUNT(*) FROM recall_evidence", [], |row| row
                            .get::<_, i64>(0))?,
                    1
                );
                storage::update_processing_job_status(
                    connection,
                    &job,
                    "cancelled",
                    None,
                    now_ms(),
                )?;
                storage::refresh_evidence(connection)?;
                assert_eq!(
                    connection
                        .query_row("SELECT COUNT(*) FROM recall_evidence", [], |row| row
                            .get::<_, i64>(0))?,
                    0
                );
                Ok((fixture, job))
            })
            .unwrap();
        let jobs = vec![StoppedJob {
            id: job,
            status: "running".into(),
            pending: vec![],
        }];
        stop_owners(&storage, &jobs).unwrap();
        restore_retained(&storage, &jobs, true).unwrap();
        storage
            .with_connection(|connection| {
                storage::refresh_evidence(connection)?;
                assert_eq!(
                    connection.query_row(
                        "SELECT COUNT(*) FROM recall_evidence WHERE recording_id=?1",
                        [&fixture.recording],
                        |row| row.get::<_, i64>(0)
                    )?,
                    1
                );
                Ok(())
            })
            .unwrap();
    }
}
