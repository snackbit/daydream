use super::super::super::*;

pub(super) fn fail_chunk(
    storage: &AppStorage,
    chunk: &ProcessingChunkRow,
    code: &str,
    timestamp: i64,
) -> Result<(), AiError> {
    storage.with_connection_mut(|connection|{
        let transaction=connection.transaction()?;
        let current=storage::get_processing_chunk(&transaction,&chunk.id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)?;
        if current.attempt_count!=chunk.attempt_count||current.status!="running"{return Ok(());}
        let job=storage::get_processing_job(&transaction,&chunk.job_id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)?;
        if job.status=="paused" {storage::update_processing_chunk_status(&transaction,&chunk.id,"queued",0.0,None,timestamp)?;transaction.commit()?;return Ok(());}
        if job.status!="running" {storage::update_processing_chunk_status(&transaction,&chunk.id,"cancelled",0.0,Some("ai_processing_cancelled"),timestamp)?;transaction.commit()?;return Ok(());}
        transaction.execute("UPDATE visual_frame_processing_states SET status='failed',progress=1,error_code=?2,updated_at_ms=?3 WHERE chunk_id=?1 AND status IN ('queued','running')",rusqlite::params![chunk.id,code,timestamp])?;
        let limit=super::retry_policy::automatic_retry_limit(code);
        if limit>0&&storage::schedule_processing_chunk_retry(&transaction,chunk,code,limit,timestamp)?{transaction.commit()?;return Ok(());}
        let successful:i64=transaction.query_row("SELECT (SELECT COUNT(*) FROM processing_audio_ranges WHERE chunk_id=?1 AND status='succeeded')+(SELECT COUNT(*) FROM visual_frame_processing_states WHERE chunk_id=?1 AND status='succeeded')",[&chunk.id],|row|row.get(0))?;
        storage::update_processing_chunk_status(&transaction,&chunk.id,if successful>0{"completed-with-errors"}else{"failed"},1.0,Some(code),timestamp)?;
        transaction.commit()
    }).map_err(storage_error)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pausing_and_stale_attempts_cannot_be_overwritten_by_late_errors() {
        let storage = storage::test_storage(PathBuf::new());
        let (chunk, _) = storage
            .with_connection(|connection| {
                Ok(crate::ai::tests::test_transcription_chunk(connection))
            })
            .unwrap();
        storage
            .with_connection(|connection| {
                storage::update_processing_job_status(connection, &chunk.job_id, "paused", None, 10)
            })
            .unwrap();
        fail_chunk(
            &storage,
            &chunk,
            "ai_transcription_local_worker_crashed",
            11,
        )
        .unwrap();
        storage
            .with_connection(|connection| {
                assert_eq!(
                    storage::get_processing_chunk(connection, &chunk.id)?
                        .unwrap()
                        .status,
                    "queued"
                );
                storage::update_processing_job_status(
                    connection,
                    &chunk.job_id,
                    "running",
                    None,
                    12,
                )?;
                storage::claim_processing_chunk(connection, &chunk.id, 0.05, 13)?;
                Ok(())
            })
            .unwrap();
        fail_chunk(&storage, &chunk, "ai_transcription_remote_auth_failed", 14).unwrap();
        storage
            .with_connection(|connection| {
                assert_eq!(
                    storage::get_processing_chunk(connection, &chunk.id)?
                        .unwrap()
                        .status,
                    "running"
                );
                Ok(())
            })
            .unwrap();
    }
    #[test]
    fn automatic_retries_are_bounded_and_preserve_completed_speech() {
        let storage = storage::test_storage(PathBuf::new());
        let (mut chunk, _) = storage
            .with_connection(|connection| {
                Ok(crate::ai::tests::test_transcription_chunk(connection))
            })
            .unwrap();
        storage
            .with_connection(|connection| {
                storage::save_audio_range_checkpoint(
                    connection,
                    &chunk.id,
                    0,
                    500,
                    "synthetic",
                    None,
                )
            })
            .unwrap();
        for attempt in 0..3 {
            fail_chunk(&storage, &chunk, "ai_transcription_timeout", 100 + attempt).unwrap();
            let state = storage
                .with_connection(|connection| storage::get_processing_chunk(connection, &chunk.id))
                .unwrap()
                .unwrap();
            if attempt < 2 {
                assert_eq!(state.status, "queued");
                chunk = storage
                    .with_connection(|connection| {
                        storage::claim_processing_chunk(connection, &chunk.id, 0.05, 200 + attempt)
                    })
                    .unwrap()
                    .unwrap();
            } else {
                assert_eq!(state.status, "completed-with-errors");
            }
        }
        assert!(storage
            .with_connection(|connection| storage::audio_range_succeeded(
                connection,
                &chunk.id,
                0,
                500,
                "synthetic"
            ))
            .unwrap());
    }
}
