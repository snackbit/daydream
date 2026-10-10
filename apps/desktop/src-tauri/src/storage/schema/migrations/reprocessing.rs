use rusqlite_migration::M;

pub(in crate::storage) fn migrations() -> Vec<M<'static>> {
    vec![M::up(
        r#"
CREATE TABLE processing_replacement_groups (
 id TEXT PRIMARY KEY,
 recording_id TEXT NOT NULL REFERENCES recordings(id) ON DELETE CASCADE,
 job_id TEXT NOT NULL REFERENCES processing_jobs(id) ON DELETE CASCADE,
 source_track_id TEXT NOT NULL,
 source_role TEXT,
 source_kind TEXT NOT NULL CHECK(source_kind IN ('audio','ocr','vlm')),
 start_ms INTEGER NOT NULL CHECK(start_ms>=0),
 end_ms INTEGER NOT NULL CHECK(end_ms>start_ms),
 committed INTEGER NOT NULL DEFAULT 0 CHECK(committed IN (0,1))
);
CREATE TABLE processing_replacement_chunks (
 chunk_id TEXT PRIMARY KEY REFERENCES processing_chunks(id) ON DELETE CASCADE,
 group_id TEXT NOT NULL REFERENCES processing_replacement_groups(id) ON DELETE CASCADE
);
CREATE INDEX idx_replacement_group_chunks ON processing_replacement_chunks(group_id);
CREATE TABLE processing_replacement_frames (
 chunk_id TEXT NOT NULL REFERENCES processing_chunks(id) ON DELETE CASCADE,
 frame_ms INTEGER NOT NULL,
 PRIMARY KEY(chunk_id,frame_ms)
);
CREATE TRIGGER replacement_publication_recall_dirty AFTER UPDATE OF committed ON processing_replacement_groups
WHEN NEW.committed=1 AND OLD.committed=0 BEGIN
 INSERT OR IGNORE INTO recall_dirty_recordings(recording_id) VALUES(NEW.recording_id);
 UPDATE recall_meta SET source_dirty=1 WHERE id=1;
END;
"#,
    )]
}
