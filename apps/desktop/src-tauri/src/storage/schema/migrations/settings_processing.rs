use rusqlite_migration::M;

pub(in crate::storage) fn migrations() -> Vec<M<'static>> {
    vec![
        M::up(
            r#"
    CREATE TABLE app_settings (
        id INTEGER PRIMARY KEY NOT NULL CHECK (id = 1),
        record_fps REAL NOT NULL CHECK (
            record_fps >= 0.016666666666666666
            AND record_fps <= 5.0
        ),
        record_video_bitrate_kbps INTEGER NOT NULL CHECK (
            record_video_bitrate_kbps >= 250
            AND record_video_bitrate_kbps <= 10000
        ),
        storage_retention_days INTEGER NOT NULL CHECK (
            storage_retention_days >= 1
            AND storage_retention_days <= 365
        ),
        storage_size_limit_bytes INTEGER NOT NULL CHECK (
            storage_size_limit_bytes >= 1073741824
        ),
        updated_at_ms INTEGER NOT NULL CHECK (updated_at_ms >= 0)
    );
    "#,
        ),
        M::up(
            r#"
    CREATE TABLE app_settings_v2 (
        key TEXT PRIMARY KEY NOT NULL,
        value_json TEXT NOT NULL CHECK (json_valid(value_json)),
        is_sensitive INTEGER NOT NULL DEFAULT 0 CHECK (is_sensitive IN (0, 1)),
        created_at_ms INTEGER NOT NULL CHECK (created_at_ms >= 0),
        updated_at_ms INTEGER NOT NULL CHECK (updated_at_ms >= created_at_ms)
    );

    INSERT INTO app_settings_v2 (
        key,
        value_json,
        is_sensitive,
        created_at_ms,
        updated_at_ms
    )
    SELECT 'record.fps', CAST(record_fps AS TEXT), 0, updated_at_ms, updated_at_ms
    FROM app_settings
    WHERE id = 1;

    INSERT INTO app_settings_v2 (
        key,
        value_json,
        is_sensitive,
        created_at_ms,
        updated_at_ms
    )
    SELECT 'record.videoBitrateKbps', CAST(record_video_bitrate_kbps AS TEXT), 0, updated_at_ms, updated_at_ms
    FROM app_settings
    WHERE id = 1;

    INSERT INTO app_settings_v2 (
        key,
        value_json,
        is_sensitive,
        created_at_ms,
        updated_at_ms
    )
    SELECT 'storage.retentionDays', CAST(storage_retention_days AS TEXT), 0, updated_at_ms, updated_at_ms
    FROM app_settings
    WHERE id = 1;

    INSERT INTO app_settings_v2 (
        key,
        value_json,
        is_sensitive,
        created_at_ms,
        updated_at_ms
    )
    SELECT 'storage.sizeLimitBytes', CAST(storage_size_limit_bytes AS TEXT), 0, updated_at_ms, updated_at_ms
    FROM app_settings
    WHERE id = 1;

    DROP TABLE app_settings;
    ALTER TABLE app_settings_v2 RENAME TO app_settings;

    CREATE TABLE IF NOT EXISTS processing_jobs (
        id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
        recording_id TEXT NOT NULL CHECK (length(recording_id) = 36),
        kind TEXT NOT NULL CHECK (kind IN ('chunking', 'frame-extraction', 'ocr', 'transcription', 'vlm', 'embedding', 'digest')),
        status TEXT NOT NULL CHECK (status IN ('queued', 'running', 'succeeded', 'failed', 'cancelled')),
        created_at_ms INTEGER NOT NULL CHECK (created_at_ms >= 0),
        updated_at_ms INTEGER NOT NULL CHECK (updated_at_ms >= created_at_ms),
        error TEXT,
        FOREIGN KEY (recording_id) REFERENCES recordings(id) ON DELETE CASCADE
    );

    CREATE TABLE processing_jobs_v2 (
        id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
        recording_id TEXT NOT NULL CHECK (length(recording_id) = 36),
        kind TEXT NOT NULL CHECK (
            kind IN (
                'activity',
                'ai-processing',
                'chunking',
                'digest',
                'embedding',
                'frame-extraction',
                'ocr',
                'transcription',
                'vlm'
            )
        ),
        status TEXT NOT NULL CHECK (
            status IN ('queued', 'running', 'paused', 'succeeded', 'failed', 'cancelled')
        ),
        created_at_ms INTEGER NOT NULL CHECK (created_at_ms >= 0),
        updated_at_ms INTEGER NOT NULL CHECK (updated_at_ms >= created_at_ms),
        error TEXT,
        FOREIGN KEY (recording_id) REFERENCES recordings(id) ON DELETE CASCADE
    );

    INSERT INTO processing_jobs_v2 (
        id,
        recording_id,
        kind,
        status,
        created_at_ms,
        updated_at_ms,
        error
    )
    SELECT
        id,
        recording_id,
        kind,
        status,
        created_at_ms,
        updated_at_ms,
        error
    FROM processing_jobs;

    DROP TABLE processing_jobs;
    ALTER TABLE processing_jobs_v2 RENAME TO processing_jobs;

    CREATE TABLE ai_model_assets (
        id TEXT PRIMARY KEY NOT NULL,
        provider TEXT NOT NULL CHECK (provider IN ('local-whisper-cpp', 'openai-compatible')),
        display_label TEXT NOT NULL,
        local_path TEXT,
        size_bytes INTEGER CHECK (size_bytes IS NULL OR size_bytes >= 0),
        sha256 TEXT,
        etag TEXT,
        download_status TEXT NOT NULL CHECK (download_status IN ('missing', 'downloading', 'available', 'failed')),
        self_test_status TEXT NOT NULL CHECK (self_test_status IN ('not-run', 'running', 'passed', 'failed')),
        created_at_ms INTEGER NOT NULL CHECK (created_at_ms >= 0),
        updated_at_ms INTEGER NOT NULL CHECK (updated_at_ms >= created_at_ms)
    );

    CREATE TABLE processing_chunks (
        id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
        job_id TEXT NOT NULL CHECK (length(job_id) = 36),
        recording_id TEXT NOT NULL CHECK (length(recording_id) = 36),
        source_kind TEXT NOT NULL CHECK (source_kind IN ('audio', 'frame', 'activity', 'ocr', 'vlm')),
        segment_id TEXT CHECK (segment_id IS NULL OR length(segment_id) = 36),
        start_offset_ms INTEGER CHECK (start_offset_ms IS NULL OR start_offset_ms >= 0),
        end_offset_ms INTEGER CHECK (
            end_offset_ms IS NULL
            OR (
                end_offset_ms >= 0
                AND (start_offset_ms IS NULL OR end_offset_ms >= start_offset_ms)
            )
        ),
        status TEXT NOT NULL CHECK (status IN ('queued', 'running', 'succeeded', 'failed', 'cancelled')),
        attempt_count INTEGER NOT NULL DEFAULT 0 CHECK (attempt_count >= 0),
        progress REAL NOT NULL DEFAULT 0 CHECK (progress >= 0 AND progress <= 1),
        error_code TEXT,
        result_code TEXT,
        created_at_ms INTEGER NOT NULL CHECK (created_at_ms >= 0),
        updated_at_ms INTEGER NOT NULL CHECK (updated_at_ms >= created_at_ms),
        FOREIGN KEY (job_id) REFERENCES processing_jobs(id) ON DELETE CASCADE,
        FOREIGN KEY (recording_id) REFERENCES recordings(id) ON DELETE CASCADE,
        FOREIGN KEY (segment_id) REFERENCES recording_segments(id) ON DELETE SET NULL
    );

    CREATE TABLE transcript_segments (
        id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
        recording_id TEXT NOT NULL CHECK (length(recording_id) = 36),
        segment_id TEXT NOT NULL CHECK (length(segment_id) = 36),
        track_id TEXT NOT NULL CHECK (length(track_id) = 36),
        source_role TEXT NOT NULL CHECK (source_role IN ('desktop-audio', 'microphone')),
        start_offset_ms INTEGER NOT NULL CHECK (start_offset_ms >= 0),
        end_offset_ms INTEGER NOT NULL CHECK (end_offset_ms >= start_offset_ms),
        provider TEXT NOT NULL,
        model TEXT,
        text TEXT NOT NULL,
        confidence REAL CHECK (confidence IS NULL OR (confidence >= 0 AND confidence <= 1)),
        job_id TEXT NOT NULL CHECK (length(job_id) = 36),
        chunk_id TEXT NOT NULL CHECK (length(chunk_id) = 36),
        created_at_ms INTEGER NOT NULL CHECK (created_at_ms >= 0),
        FOREIGN KEY (recording_id) REFERENCES recordings(id) ON DELETE CASCADE,
        FOREIGN KEY (segment_id) REFERENCES recording_segments(id) ON DELETE CASCADE,
        FOREIGN KEY (track_id) REFERENCES recording_tracks(id) ON DELETE CASCADE,
        FOREIGN KEY (job_id) REFERENCES processing_jobs(id) ON DELETE CASCADE,
        FOREIGN KEY (chunk_id) REFERENCES processing_chunks(id) ON DELETE CASCADE
    );

    CREATE TABLE transcript_words (
        id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
        transcript_segment_id TEXT NOT NULL CHECK (length(transcript_segment_id) = 36),
        start_offset_ms INTEGER NOT NULL CHECK (start_offset_ms >= 0),
        end_offset_ms INTEGER NOT NULL CHECK (end_offset_ms >= start_offset_ms),
        text TEXT NOT NULL,
        probability REAL CHECK (probability IS NULL OR (probability >= 0 AND probability <= 1)),
        FOREIGN KEY (transcript_segment_id) REFERENCES transcript_segments(id) ON DELETE CASCADE
    );

    CREATE TABLE frame_activity_intervals (
        id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
        recording_id TEXT NOT NULL CHECK (length(recording_id) = 36),
        source_kind TEXT NOT NULL CHECK (source_kind IN ('keyboard', 'mouse', 'frame-delta', 'merged')),
        state TEXT NOT NULL CHECK (state IN ('active', 'idle', 'unknown')),
        start_offset_ms INTEGER NOT NULL CHECK (start_offset_ms >= 0),
        end_offset_ms INTEGER NOT NULL CHECK (end_offset_ms >= start_offset_ms),
        score REAL CHECK (score IS NULL OR (score >= 0 AND score <= 1)),
        regions_json TEXT CHECK (regions_json IS NULL OR json_valid(regions_json)),
        job_id TEXT NOT NULL CHECK (length(job_id) = 36),
        chunk_id TEXT CHECK (chunk_id IS NULL OR length(chunk_id) = 36),
        FOREIGN KEY (recording_id) REFERENCES recordings(id) ON DELETE CASCADE,
        FOREIGN KEY (job_id) REFERENCES processing_jobs(id) ON DELETE CASCADE,
        FOREIGN KEY (chunk_id) REFERENCES processing_chunks(id) ON DELETE SET NULL
    );

    CREATE TABLE ocr_regions (
        id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
        recording_id TEXT NOT NULL CHECK (length(recording_id) = 36),
        frame_timestamp_ms INTEGER NOT NULL CHECK (frame_timestamp_ms >= 0),
        source_frame_ref TEXT NOT NULL,
        region_x INTEGER NOT NULL CHECK (region_x >= 0),
        region_y INTEGER NOT NULL CHECK (region_y >= 0),
        region_width INTEGER NOT NULL CHECK (region_width > 0),
        region_height INTEGER NOT NULL CHECK (region_height > 0),
        text TEXT NOT NULL,
        confidence REAL CHECK (confidence IS NULL OR (confidence >= 0 AND confidence <= 1)),
        provider TEXT NOT NULL,
        model TEXT,
        job_id TEXT NOT NULL CHECK (length(job_id) = 36),
        chunk_id TEXT NOT NULL CHECK (length(chunk_id) = 36),
        FOREIGN KEY (recording_id) REFERENCES recordings(id) ON DELETE CASCADE,
        FOREIGN KEY (job_id) REFERENCES processing_jobs(id) ON DELETE CASCADE,
        FOREIGN KEY (chunk_id) REFERENCES processing_chunks(id) ON DELETE CASCADE
    );

    CREATE TABLE vlm_observations (
        id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
        recording_id TEXT NOT NULL CHECK (length(recording_id) = 36),
        frame_timestamp_ms INTEGER NOT NULL CHECK (frame_timestamp_ms >= 0),
        source_frame_ref TEXT NOT NULL,
        region_x INTEGER NOT NULL CHECK (region_x >= 0),
        region_y INTEGER NOT NULL CHECK (region_y >= 0),
        region_width INTEGER NOT NULL CHECK (region_width > 0),
        region_height INTEGER NOT NULL CHECK (region_height > 0),
        text TEXT NOT NULL,
        confidence REAL CHECK (confidence IS NULL OR (confidence >= 0 AND confidence <= 1)),
        provider TEXT NOT NULL,
        model TEXT,
        prompt_version TEXT NOT NULL,
        observation_kind TEXT NOT NULL DEFAULT 'screen-observation'
            CHECK (observation_kind IN ('corrected-ocr', 'screen-observation')),
        job_id TEXT NOT NULL CHECK (length(job_id) = 36),
        chunk_id TEXT NOT NULL CHECK (length(chunk_id) = 36),
        FOREIGN KEY (recording_id) REFERENCES recordings(id) ON DELETE CASCADE,
        FOREIGN KEY (job_id) REFERENCES processing_jobs(id) ON DELETE CASCADE,
        FOREIGN KEY (chunk_id) REFERENCES processing_chunks(id) ON DELETE CASCADE
    );

    CREATE TABLE frame_similarity_entries (
        id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
        recording_id TEXT NOT NULL CHECK (length(recording_id) = 36),
        source_frame_ref TEXT NOT NULL,
        region_x INTEGER NOT NULL CHECK (region_x >= 0),
        region_y INTEGER NOT NULL CHECK (region_y >= 0),
        region_width INTEGER NOT NULL CHECK (region_width > 0),
        region_height INTEGER NOT NULL CHECK (region_height > 0),
        signature TEXT NOT NULL,
        last_processed_timestamp_ms INTEGER NOT NULL CHECK (last_processed_timestamp_ms >= 0),
        job_id TEXT NOT NULL CHECK (length(job_id) = 36),
        chunk_id TEXT CHECK (chunk_id IS NULL OR length(chunk_id) = 36),
        FOREIGN KEY (recording_id) REFERENCES recordings(id) ON DELETE CASCADE,
        FOREIGN KEY (job_id) REFERENCES processing_jobs(id) ON DELETE CASCADE,
        FOREIGN KEY (chunk_id) REFERENCES processing_chunks(id) ON DELETE SET NULL
    );

    CREATE INDEX idx_processing_jobs_recording_status ON processing_jobs(recording_id, status);
    CREATE INDEX idx_processing_jobs_kind_status ON processing_jobs(kind, status);
    CREATE INDEX idx_processing_chunks_job_status ON processing_chunks(job_id, status);
    CREATE INDEX idx_processing_chunks_recording_source ON processing_chunks(recording_id, source_kind);
    CREATE INDEX idx_transcript_segments_recording_time ON transcript_segments(recording_id, start_offset_ms, end_offset_ms);
    CREATE INDEX idx_transcript_words_segment_time ON transcript_words(transcript_segment_id, start_offset_ms);
    CREATE INDEX idx_frame_activity_intervals_recording_time ON frame_activity_intervals(recording_id, start_offset_ms, end_offset_ms);
    CREATE INDEX idx_ocr_regions_recording_frame ON ocr_regions(recording_id, frame_timestamp_ms);
    CREATE INDEX idx_vlm_observations_recording_frame ON vlm_observations(recording_id, frame_timestamp_ms);
    CREATE INDEX idx_frame_similarity_recording_frame ON frame_similarity_entries(recording_id, source_frame_ref);
    "#,
        ),
    ]
}
