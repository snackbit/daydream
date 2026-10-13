use rusqlite_migration::M;

pub(in crate::storage) fn migrations() -> Vec<M<'static>> {
    vec![M::up(
        r#"
ALTER TABLE processing_chunk_runtime ADD COLUMN phase_started_at_ms INTEGER;
ALTER TABLE processing_chunk_runtime ADD COLUMN last_progress_at_ms INTEGER;
CREATE TRIGGER processing_phase_changed AFTER UPDATE OF phase ON processing_chunk_runtime
WHEN OLD.phase != NEW.phase BEGIN
 UPDATE processing_chunk_runtime SET phase_started_at_ms=CAST(unixepoch('subsec')*1000 AS INTEGER) WHERE chunk_id=NEW.chunk_id;
END;
CREATE TRIGGER processing_progress_changed AFTER UPDATE OF progress ON processing_chunks
WHEN OLD.progress != NEW.progress AND NEW.status IN('running','succeeded','completed-with-errors') BEGIN
 UPDATE processing_chunk_runtime SET last_progress_at_ms=NEW.updated_at_ms WHERE chunk_id=NEW.id;
END;
"#,
    )]
}

#[cfg(test)]
mod tests {
    use crate::storage;
    #[test]
    fn migration_preserves_existing_runtime_and_leaves_historical_measurements_unknown() {
        let mut connection = rusqlite::Connection::open_in_memory().unwrap();
        crate::storage::migrations()
            .to_version(&mut connection, 37)
            .unwrap();
        let recording = uuid::Uuid::now_v7().to_string();
        let job = uuid::Uuid::now_v7().to_string();
        let chunk = uuid::Uuid::now_v7().to_string();
        storage::create_recording(
            &connection,
            &storage::CreateRecordingInput {
                id: recording.clone(),
                active_scene_id: None,
                created_at_ms: 1,
                default_fps: 1.,
                notes: None,
                output_root: None,
                quality_label: "balanced-ocr".into(),
                storage_dir: "/tmp/synthetic-overview-migration".into(),
            },
        )
        .unwrap();
        storage::create_processing_job(
            &connection,
            &storage::CreateProcessingJobInput {
                id: job.clone(),
                recording_id: recording.clone(),
                kind: "ai-processing".into(),
                status: "queued".into(),
                created_at_ms: 1,
                updated_at_ms: 1,
                error: None,
            },
        )
        .unwrap();
        storage::create_processing_chunk(
            &connection,
            &storage::CreateProcessingChunkInput {
                id: chunk.clone(),
                job_id: job,
                recording_id: recording,
                source_kind: "ocr".into(),
                segment_id: None,
                track_id: None,
                start_offset_ms: Some(0),
                end_offset_ms: Some(1000),
                attempt_count: 0,
                progress: 0.,
                error_code: None,
                result_code: None,
                status: "queued".into(),
                created_at_ms: 1,
                updated_at_ms: 1,
            },
        )
        .unwrap();
        connection.execute("INSERT INTO processing_chunk_runtime(chunk_id,started_at_ms,phase) VALUES(?1,20,'recognizing-text')",[chunk]).unwrap();
        crate::storage::migrations()
            .to_latest(&mut connection)
            .unwrap();
        let stored:(String,i64,Option<i64>,Option<i64>)=connection.query_row("SELECT phase,started_at_ms,phase_started_at_ms,last_progress_at_ms FROM processing_chunk_runtime",[],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?))).unwrap();
        assert_eq!(stored, ("recognizing-text".into(), 20, None, None));
    }
}
