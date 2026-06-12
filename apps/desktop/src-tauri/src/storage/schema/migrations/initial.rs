use rusqlite_migration::M;

pub(in crate::storage) fn migrations() -> Vec<M<'static>> {
    vec![M::up(
        r#"
    CREATE TABLE recordings (
        id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
        created_at_ms INTEGER NOT NULL CHECK (created_at_ms >= 0),
        started_at_ms INTEGER CHECK (started_at_ms IS NULL OR started_at_ms >= 0),
        ended_at_ms INTEGER CHECK (
            ended_at_ms IS NULL
            OR (
                ended_at_ms >= 0
                AND (started_at_ms IS NULL OR ended_at_ms >= started_at_ms)
            )
        ),
        status TEXT NOT NULL CHECK (status IN ('planned', 'recording', 'paused', 'completed', 'failed', 'deleted')),
        default_fps REAL NOT NULL CHECK (default_fps > 0),
        quality_label TEXT NOT NULL CHECK (quality_label IN ('draft', 'standard', 'high', 'ocr')),
        storage_dir TEXT NOT NULL,
        primary_video_path TEXT,
        notes TEXT
    );

    CREATE TABLE recording_tracks (
        id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
        recording_id TEXT NOT NULL CHECK (length(recording_id) = 36),
        kind TEXT NOT NULL CHECK (kind IN ('screen', 'desktop-audio', 'microphone')),
        label TEXT NOT NULL,
        source_id TEXT,
        path TEXT,
        codec TEXT,
        created_at_ms INTEGER NOT NULL CHECK (created_at_ms >= 0),
        FOREIGN KEY (recording_id) REFERENCES recordings(id) ON DELETE CASCADE
    );

    CREATE TABLE redactions (
        id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
        recording_id TEXT NOT NULL CHECK (length(recording_id) = 36),
        start_offset_ms INTEGER NOT NULL CHECK (start_offset_ms >= 0),
        end_offset_ms INTEGER NOT NULL CHECK (end_offset_ms > start_offset_ms),
        reason TEXT,
        created_at_ms INTEGER NOT NULL CHECK (created_at_ms >= 0),
        FOREIGN KEY (recording_id) REFERENCES recordings(id) ON DELETE CASCADE
    );

    CREATE TABLE capture_events (
        id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
        recording_id TEXT NOT NULL CHECK (length(recording_id) = 36),
        occurred_at_ms INTEGER NOT NULL CHECK (occurred_at_ms >= 0),
        offset_ms INTEGER NOT NULL CHECK (offset_ms >= 0),
        kind TEXT NOT NULL CHECK (kind IN ('keyboard-activity', 'mouse-activity', 'focused-window', 'open-windows', 'capture-state')),
        payload_json TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(payload_json)),
        FOREIGN KEY (recording_id) REFERENCES recordings(id) ON DELETE CASCADE
    );

    CREATE TABLE processing_jobs (
        id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
        recording_id TEXT NOT NULL CHECK (length(recording_id) = 36),
        kind TEXT NOT NULL CHECK (kind IN ('chunking', 'frame-extraction', 'ocr', 'transcription', 'vlm', 'embedding', 'digest')),
        status TEXT NOT NULL CHECK (status IN ('queued', 'running', 'succeeded', 'failed', 'cancelled')),
        created_at_ms INTEGER NOT NULL CHECK (created_at_ms >= 0),
        updated_at_ms INTEGER NOT NULL CHECK (updated_at_ms >= created_at_ms),
        error TEXT,
        FOREIGN KEY (recording_id) REFERENCES recordings(id) ON DELETE CASCADE
    );

    CREATE INDEX idx_recording_tracks_recording_id ON recording_tracks(recording_id);
    CREATE INDEX idx_redactions_recording_interval ON redactions(recording_id, start_offset_ms, end_offset_ms);
    CREATE INDEX idx_capture_events_recording_offset ON capture_events(recording_id, offset_ms);
    CREATE INDEX idx_capture_events_kind ON capture_events(kind);
    CREATE INDEX idx_processing_jobs_recording_status ON processing_jobs(recording_id, status);
    CREATE INDEX idx_processing_jobs_kind_status ON processing_jobs(kind, status);
    "#,
    )]
}
