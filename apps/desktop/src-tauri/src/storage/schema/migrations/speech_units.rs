use rusqlite_migration::M;

pub(super) fn migrations() -> Vec<M<'static>> {
    vec![
        M::up(
            r#"
CREATE TABLE processing_audio_strategies (
 job_id TEXT PRIMARY KEY REFERENCES processing_jobs(id) ON DELETE CASCADE,
 version INTEGER NOT NULL CHECK(version=2)
);
CREATE TABLE processing_speech_units (
 id TEXT PRIMARY KEY,
 job_id TEXT NOT NULL REFERENCES processing_jobs(id) ON DELETE CASCADE,
 track_id TEXT NOT NULL REFERENCES recording_tracks(id) ON DELETE CASCADE,
 owner_chunk_id TEXT NOT NULL REFERENCES processing_chunks(id) ON DELETE CASCADE,
 start_ms INTEGER NOT NULL CHECK(start_ms>=0),
 end_ms INTEGER NOT NULL CHECK(end_ms>start_ms),
 submitted_start_ms INTEGER NOT NULL CHECK(submitted_start_ms>=0),
 submitted_end_ms INTEGER NOT NULL CHECK(submitted_end_ms>=submitted_start_ms),
 right_open INTEGER NOT NULL CHECK(right_open IN (0,1)),
 status TEXT NOT NULL DEFAULT 'queued' CHECK(status IN ('queued','staged','published','failed')),
 config_fingerprint TEXT NOT NULL,
 output_json TEXT CHECK(output_json IS NULL OR json_valid(output_json)),
 error_code TEXT,
 UNIQUE(job_id,track_id,start_ms)
);
CREATE TABLE processing_speech_unit_chunks (
 unit_id TEXT NOT NULL REFERENCES processing_speech_units(id) ON DELETE CASCADE,
 chunk_id TEXT NOT NULL REFERENCES processing_chunks(id) ON DELETE CASCADE,
 PRIMARY KEY(unit_id,chunk_id)
);
CREATE TABLE processing_speech_unit_transcripts (
 unit_id TEXT NOT NULL REFERENCES processing_speech_units(id) ON DELETE CASCADE,
 transcript_id TEXT PRIMARY KEY REFERENCES transcript_segments(id) ON DELETE CASCADE
);
CREATE INDEX idx_speech_units_source ON processing_speech_units(job_id,track_id,start_ms,status);
CREATE VIEW published_transcript_segments AS
SELECT t.* FROM transcript_segments t
WHERE NOT EXISTS(SELECT 1 FROM processing_speech_unit_transcripts st JOIN processing_speech_units u ON u.id=st.unit_id WHERE st.transcript_id=t.id AND u.status!='published')
AND (EXISTS(SELECT 1 FROM processing_speech_unit_transcripts st WHERE st.transcript_id=t.id) OR NOT EXISTS(SELECT 1 FROM processing_replacement_chunks rc JOIN processing_replacement_groups rg ON rg.id=rc.group_id WHERE rc.chunk_id=t.chunk_id AND rg.committed=0))
AND NOT EXISTS(SELECT 1 FROM processing_speech_units u JOIN processing_jobs newer ON newer.id=u.job_id JOIN processing_jobs old ON old.id=t.job_id WHERE u.status='published' AND u.track_id=t.track_id AND u.start_ms<t.end_offset_ms AND u.end_ms>t.start_offset_ms AND (newer.created_at_ms>old.created_at_ms OR (newer.created_at_ms=old.created_at_ms AND newer.id>old.id)));
"#,
        ),
        M::up(
            r#"
CREATE INDEX idx_speech_units_publication ON processing_speech_units(track_id,status,start_ms,end_ms,job_id);
CREATE INDEX idx_speech_unit_chunks_chunk ON processing_speech_unit_chunks(chunk_id,unit_id);
"#,
        ),
    ]
}

#[cfg(test)]
mod tests {
    #[test]
    fn migration_upgrades_existing_unit_tables_and_indexes_publication_by_source() {
        let mut db = rusqlite::Connection::open_in_memory().unwrap();
        crate::storage::schema::migrations::migrations()
            .to_version(&mut db, 32)
            .unwrap();
        crate::storage::schema::apply_migrations(&mut db).unwrap();
        let mut query=db.prepare("EXPLAIN QUERY PLAN SELECT id FROM published_transcript_segments WHERE recording_id='synthetic'").unwrap();
        let plan = query
            .query_map([], |r| r.get::<_, String>(3))
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap();
        assert!(plan
            .iter()
            .any(|step| step.contains("idx_speech_units_publication")));
        assert!(!plan.iter().any(|step| step == "SCAN u"));
    }
}
