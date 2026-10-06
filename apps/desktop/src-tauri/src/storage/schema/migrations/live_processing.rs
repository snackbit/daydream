use rusqlite_migration::M;
pub(in crate::storage) fn migrations() -> Vec<M<'static>> {
    vec![M::up(
        r#"
CREATE TABLE recording_live_state(
 recording_id TEXT PRIMARY KEY REFERENCES recordings(id) ON DELETE CASCADE,
 classifier_required INTEGER NOT NULL CHECK(classifier_required IN (0,1)),
 classified_through_ms INTEGER NOT NULL DEFAULT 0 CHECK(classified_through_ms>=0)
);
CREATE INDEX idx_processing_chunk_identity ON processing_chunks(job_id,segment_id,source_kind,track_id,start_offset_ms,end_offset_ms);
"#,
    )]
}
