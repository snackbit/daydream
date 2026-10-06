use rusqlite_migration::M;

pub(in crate::storage) fn migrations() -> Vec<M<'static>> {
    vec![M::up_with_hook(
        r#"
CREATE TABLE processing_jobs_new (
 id TEXT PRIMARY KEY CHECK(length(id)=36),
 recording_id TEXT NOT NULL REFERENCES recordings(id) ON DELETE CASCADE,
 kind TEXT NOT NULL CHECK(kind IN ('activity','ai-processing','chunking','digest','embedding','frame-extraction','ocr','transcription','vlm')),
 status TEXT NOT NULL CHECK(status IN ('queued','running','paused','succeeded','completed-with-errors','failed','cancelled')),
 created_at_ms INTEGER NOT NULL CHECK(created_at_ms>=0),
 updated_at_ms INTEGER NOT NULL CHECK(updated_at_ms>=created_at_ms), error TEXT
);
INSERT INTO processing_jobs_new (
 id,recording_id,kind,status,created_at_ms,updated_at_ms,error
)
SELECT id,recording_id,kind,status,created_at_ms,updated_at_ms,error FROM processing_jobs;
DROP TABLE processing_jobs;
ALTER TABLE processing_jobs_new RENAME TO processing_jobs;
CREATE INDEX idx_processing_jobs_recording_status ON processing_jobs(recording_id,status);
CREATE INDEX idx_processing_jobs_kind_status ON processing_jobs(kind,status);
CREATE TABLE processing_chunks_new (
 id TEXT PRIMARY KEY CHECK(length(id)=36),
 job_id TEXT NOT NULL REFERENCES processing_jobs(id) ON DELETE CASCADE,
 recording_id TEXT NOT NULL REFERENCES recordings(id) ON DELETE CASCADE,
 source_kind TEXT NOT NULL CHECK(source_kind IN ('audio','frame','activity','ocr','vlm')),
 segment_id TEXT REFERENCES recording_segments(id) ON DELETE SET NULL,
 start_offset_ms INTEGER CHECK(start_offset_ms IS NULL OR start_offset_ms>=0),
 end_offset_ms INTEGER CHECK(end_offset_ms IS NULL OR (end_offset_ms>=0 AND (start_offset_ms IS NULL OR end_offset_ms>=start_offset_ms))),
 status TEXT NOT NULL CHECK(status IN ('queued','running','succeeded','completed-with-errors','failed','cancelled')),
 attempt_count INTEGER NOT NULL DEFAULT 0 CHECK(attempt_count>=0),
 progress REAL NOT NULL DEFAULT 0 CHECK(progress>=0 AND progress<=1),
 error_code TEXT,result_code TEXT,
 created_at_ms INTEGER NOT NULL CHECK(created_at_ms>=0),
 updated_at_ms INTEGER NOT NULL CHECK(updated_at_ms>=created_at_ms),
 track_id TEXT REFERENCES recording_tracks(id) ON DELETE SET NULL
);
-- Historical ALTER TABLE migrations appended result_code after track_id.
-- Named columns preserve data independently of the physical schema layout.
INSERT INTO processing_chunks_new (
 id,job_id,recording_id,source_kind,segment_id,start_offset_ms,end_offset_ms,
 status,attempt_count,progress,error_code,result_code,created_at_ms,updated_at_ms,track_id
)
SELECT id,job_id,recording_id,source_kind,segment_id,start_offset_ms,end_offset_ms,
 status,attempt_count,progress,error_code,result_code,created_at_ms,updated_at_ms,track_id
FROM processing_chunks;
DROP TABLE processing_chunks;
ALTER TABLE processing_chunks_new RENAME TO processing_chunks;
CREATE INDEX idx_processing_chunks_job_status ON processing_chunks(job_id,status);
CREATE INDEX idx_processing_chunks_recording_source ON processing_chunks(recording_id,source_kind);
CREATE INDEX idx_processing_chunks_source_time ON processing_chunks(job_id,source_kind,segment_id,start_offset_ms,end_offset_ms);
CREATE TABLE processing_chunk_runtime (
 chunk_id TEXT PRIMARY KEY REFERENCES processing_chunks(id) ON DELETE CASCADE,
 started_at_ms INTEGER, next_retry_at_ms INTEGER, retry_count INTEGER NOT NULL DEFAULT 0,
 phase TEXT NOT NULL DEFAULT 'queued'
);
CREATE TABLE processing_job_runtime (
 job_id TEXT PRIMARY KEY REFERENCES processing_jobs(id) ON DELETE CASCADE,
 input_open INTEGER NOT NULL DEFAULT 0 CHECK(input_open IN (0,1)),
 config_json TEXT CHECK(config_json IS NULL OR json_valid(config_json))
);
CREATE TABLE processing_audio_ranges (
 chunk_id TEXT NOT NULL REFERENCES processing_chunks(id) ON DELETE CASCADE,
 start_ms INTEGER NOT NULL CHECK(start_ms>=0),end_ms INTEGER NOT NULL CHECK(end_ms>start_ms),
 config_fingerprint TEXT NOT NULL,
 status TEXT NOT NULL CHECK(status IN ('succeeded','failed')),error_code TEXT,
 PRIMARY KEY(chunk_id,start_ms,end_ms,config_fingerprint)
);
"#,
        |transaction| {
            let mut statement = transaction.prepare("PRAGMA foreign_key_check")?;
            if statement.query([])?.next()?.is_some() {
                return Err(rusqlite::Error::InvalidQuery.into());
            }
            Ok(())
        },
    )]
}
