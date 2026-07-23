use rusqlite_migration::M;

pub(in crate::storage) fn migrations() -> Vec<M<'static>> {
    vec![M::up(
        r#"
    CREATE TABLE visual_frame_processing_states (
        id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
        recording_id TEXT NOT NULL CHECK (length(recording_id) = 36),
        job_id TEXT NOT NULL CHECK (length(job_id) = 36),
        chunk_id TEXT NOT NULL CHECK (length(chunk_id) = 36),
        source_kind TEXT NOT NULL CHECK (source_kind IN ('ocr', 'vlm')),
        segment_id TEXT CHECK (segment_id IS NULL OR length(segment_id) = 36),
        frame_timestamp_ms INTEGER NOT NULL CHECK (frame_timestamp_ms >= 0),
        source_frame_ref TEXT NOT NULL,
        region_x INTEGER NOT NULL CHECK (region_x >= 0),
        region_y INTEGER NOT NULL CHECK (region_y >= 0),
        region_width INTEGER NOT NULL CHECK (region_width > 0),
        region_height INTEGER NOT NULL CHECK (region_height > 0),
        status TEXT NOT NULL CHECK (status IN ('queued', 'running', 'succeeded', 'failed', 'cancelled')),
        progress REAL NOT NULL DEFAULT 0 CHECK (progress >= 0 AND progress <= 1),
        error_code TEXT,
        output_count INTEGER NOT NULL DEFAULT 0 CHECK (output_count >= 0),
        created_at_ms INTEGER NOT NULL CHECK (created_at_ms >= 0),
        updated_at_ms INTEGER NOT NULL CHECK (updated_at_ms >= created_at_ms),
        FOREIGN KEY (recording_id) REFERENCES recordings(id) ON DELETE CASCADE,
        FOREIGN KEY (job_id) REFERENCES processing_jobs(id) ON DELETE CASCADE,
        FOREIGN KEY (chunk_id) REFERENCES processing_chunks(id) ON DELETE CASCADE,
        FOREIGN KEY (segment_id) REFERENCES recording_segments(id) ON DELETE SET NULL
    );

    CREATE UNIQUE INDEX idx_visual_frame_processing_states_unique
        ON visual_frame_processing_states(chunk_id, source_kind, source_frame_ref);
    CREATE INDEX idx_visual_frame_processing_states_recording
        ON visual_frame_processing_states(recording_id, source_kind, frame_timestamp_ms);
    CREATE INDEX idx_visual_frame_processing_states_chunk_status
        ON visual_frame_processing_states(chunk_id, status);
    "#,
    )]
}
