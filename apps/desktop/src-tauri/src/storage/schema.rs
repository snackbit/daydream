use std::path::Path;

use rusqlite::Connection;
use rusqlite_migration::{Migrations, M};

pub(super) const CORE_TABLES: [&str; 19] = [
    "app_settings",
    "ai_model_assets",
    "notifications",
    "recordings",
    "recording_tracks",
    "recording_segments",
    "recorder_scenes",
    "recorder_scene_sources",
    "recorder_portal_restore_tokens",
    "redactions",
    "capture_events",
    "processing_jobs",
    "processing_chunks",
    "transcript_segments",
    "transcript_words",
    "frame_activity_intervals",
    "ocr_regions",
    "vlm_observations",
    "frame_similarity_entries",
];

pub(super) fn open_connection(path: &Path) -> rusqlite::Result<Connection> {
    let connection = Connection::open(path)?;
    enable_connection_pragmas(&connection)?;

    Ok(connection)
}

pub(super) fn enable_connection_pragmas(connection: &Connection) -> rusqlite::Result<()> {
    connection.pragma_update(None, "foreign_keys", "ON")?;
    connection.pragma_update(None, "journal_mode", "WAL")?;
    connection.pragma_update(None, "synchronous", "NORMAL")?;

    Ok(())
}

pub(super) fn apply_migrations(connection: &mut Connection) -> rusqlite_migration::Result<()> {
    migrations().to_latest(connection)
}

pub(super) fn migrations() -> Migrations<'static> {
    Migrations::new(vec![
        M::up(
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
        ),
        M::up(
            r#"
        CREATE TABLE recorder_scenes (
            id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
            name TEXT NOT NULL,
            canvas_width INTEGER NOT NULL CHECK (canvas_width > 0),
            canvas_height INTEGER NOT NULL CHECK (canvas_height > 0),
            layout_mode TEXT NOT NULL CHECK (layout_mode IN ('auto-16-9', 'custom', 'horizontal', 'vertical')),
            created_at_ms INTEGER NOT NULL CHECK (created_at_ms >= 0),
            updated_at_ms INTEGER NOT NULL CHECK (updated_at_ms >= created_at_ms)
        );

        CREATE TABLE recordings_v2 (
            id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
            active_scene_id TEXT CHECK (active_scene_id IS NULL OR length(active_scene_id) = 36),
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
            quality_label TEXT NOT NULL CHECK (quality_label IN ('draft', 'standard', 'high', 'ocr', 'balanced-ocr')),
            storage_dir TEXT NOT NULL,
            output_root TEXT,
            primary_video_path TEXT,
            notes TEXT,
            FOREIGN KEY (active_scene_id) REFERENCES recorder_scenes(id) ON DELETE SET NULL
        );

        INSERT INTO recordings_v2 (
            id,
            created_at_ms,
            started_at_ms,
            ended_at_ms,
            status,
            default_fps,
            quality_label,
            storage_dir,
            primary_video_path,
            notes
        )
        SELECT
            id,
            created_at_ms,
            started_at_ms,
            ended_at_ms,
            status,
            default_fps,
            quality_label,
            storage_dir,
            primary_video_path,
            notes
        FROM recordings;

        CREATE TEMP TABLE recording_tracks_v1_backup AS SELECT * FROM recording_tracks;
        CREATE TEMP TABLE redactions_v1_backup AS SELECT * FROM redactions;
        CREATE TEMP TABLE capture_events_v1_backup AS SELECT * FROM capture_events;
        CREATE TEMP TABLE processing_jobs_v1_backup AS SELECT * FROM processing_jobs;

        DROP TABLE recordings;
        ALTER TABLE recordings_v2 RENAME TO recordings;

        INSERT INTO recording_tracks (
            id,
            recording_id,
            kind,
            label,
            source_id,
            path,
            codec,
            created_at_ms
        )
        SELECT
            id,
            recording_id,
            kind,
            label,
            source_id,
            path,
            codec,
            created_at_ms
        FROM recording_tracks_v1_backup;

        INSERT INTO redactions (
            id,
            recording_id,
            start_offset_ms,
            end_offset_ms,
            reason,
            created_at_ms
        )
        SELECT
            id,
            recording_id,
            start_offset_ms,
            end_offset_ms,
            reason,
            created_at_ms
        FROM redactions_v1_backup;

        INSERT INTO capture_events (
            id,
            recording_id,
            occurred_at_ms,
            offset_ms,
            kind,
            payload_json
        )
        SELECT
            id,
            recording_id,
            occurred_at_ms,
            offset_ms,
            kind,
            payload_json
        FROM capture_events_v1_backup;

        INSERT INTO processing_jobs (
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
        FROM processing_jobs_v1_backup;

        DROP TABLE recording_tracks_v1_backup;
        DROP TABLE redactions_v1_backup;
        DROP TABLE capture_events_v1_backup;
        DROP TABLE processing_jobs_v1_backup;

        CREATE TABLE recording_segments (
            id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
            recording_id TEXT NOT NULL CHECK (length(recording_id) = 36),
            segment_index INTEGER NOT NULL CHECK (segment_index > 0),
            file_path TEXT NOT NULL,
            started_at_ms INTEGER NOT NULL CHECK (started_at_ms >= 0),
            ended_at_ms INTEGER CHECK (
                ended_at_ms IS NULL
                OR (
                    ended_at_ms >= 0
                    AND ended_at_ms >= started_at_ms
                )
            ),
            duration_ms INTEGER CHECK (duration_ms IS NULL OR duration_ms >= 0),
            state TEXT NOT NULL CHECK (state IN ('recording', 'finalized', 'failed')),
            video_stream_index INTEGER CHECK (video_stream_index IS NULL OR video_stream_index >= 0),
            desktop_audio_stream_index INTEGER CHECK (desktop_audio_stream_index IS NULL OR desktop_audio_stream_index >= 0),
            microphone_audio_stream_index INTEGER CHECK (microphone_audio_stream_index IS NULL OR microphone_audio_stream_index >= 0),
            ffmpeg_exit_code INTEGER,
            error_code TEXT,
            created_at_ms INTEGER NOT NULL CHECK (created_at_ms >= 0),
            updated_at_ms INTEGER NOT NULL CHECK (updated_at_ms >= created_at_ms),
            UNIQUE (recording_id, segment_index),
            FOREIGN KEY (recording_id) REFERENCES recordings(id) ON DELETE CASCADE
        );

        CREATE TABLE recorder_scene_sources (
            id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
            scene_id TEXT NOT NULL CHECK (length(scene_id) = 36),
            source_id TEXT NOT NULL,
            source_kind TEXT NOT NULL CHECK (source_kind IN ('monitor')),
            display_name TEXT NOT NULL,
            x INTEGER NOT NULL,
            y INTEGER NOT NULL,
            width INTEGER NOT NULL CHECK (width > 0),
            height INTEGER NOT NULL CHECK (height > 0),
            crop_left INTEGER NOT NULL CHECK (crop_left >= 0),
            crop_top INTEGER NOT NULL CHECK (crop_top >= 0),
            crop_right INTEGER NOT NULL CHECK (crop_right >= 0),
            crop_bottom INTEGER NOT NULL CHECK (crop_bottom >= 0),
            z_index INTEGER NOT NULL,
            preserve_aspect_ratio INTEGER NOT NULL CHECK (preserve_aspect_ratio IN (0, 1)),
            created_at_ms INTEGER NOT NULL CHECK (created_at_ms >= 0),
            updated_at_ms INTEGER NOT NULL CHECK (updated_at_ms >= created_at_ms),
            FOREIGN KEY (scene_id) REFERENCES recorder_scenes(id) ON DELETE CASCADE
        );

        CREATE INDEX idx_recordings_active_scene_id ON recordings(active_scene_id);
        CREATE INDEX idx_recording_segments_recording_id ON recording_segments(recording_id);
        CREATE INDEX idx_recorder_scene_sources_scene_z ON recorder_scene_sources(scene_id, z_index);
        "#,
        ),
        M::up(
            r#"
        CREATE TABLE recorder_portal_restore_tokens (
            id TEXT PRIMARY KEY NOT NULL,
            portal TEXT NOT NULL CHECK (portal IN ('linux-screencast')),
            token TEXT NOT NULL CHECK (length(token) > 0),
            created_at_ms INTEGER NOT NULL CHECK (created_at_ms >= 0),
            updated_at_ms INTEGER NOT NULL CHECK (updated_at_ms >= created_at_ms),
            last_used_at_ms INTEGER CHECK (last_used_at_ms IS NULL OR last_used_at_ms >= created_at_ms)
        );
        "#,
        ),
        M::up(
            r#"
        ALTER TABLE recorder_scene_sources
            ADD COLUMN source_width INTEGER CHECK (source_width IS NULL OR source_width > 0);
        ALTER TABLE recorder_scene_sources
            ADD COLUMN source_height INTEGER CHECK (source_height IS NULL OR source_height > 0);
        "#,
        ),
        M::up(
            r#"
        -- Existing alpha databases have recorder_scenes referenced by child
        -- tables, and this migration runner wraps each migration in a
        -- transaction. Patch only the CHECK expression instead of rebuilding
        -- the foreign-key-linked table with PRAGMA foreign_keys disabled.
        PRAGMA writable_schema = ON;

        UPDATE sqlite_schema
        SET sql = replace(
            sql,
            'layout_mode TEXT NOT NULL CHECK (layout_mode IN (''horizontal'', ''custom''))',
            'layout_mode TEXT NOT NULL CHECK (layout_mode IN (''auto-16-9'', ''custom'', ''horizontal'', ''vertical''))'
        )
        WHERE type = 'table'
            AND name = 'recorder_scenes'
            AND sql LIKE '%layout_mode TEXT NOT NULL CHECK (layout_mode IN (''horizontal'', ''custom''))%';

        PRAGMA schema_version = 5000;
        PRAGMA writable_schema = OFF;
        "#,
        ),
        M::up(
            r#"
        ALTER TABLE recorder_scenes
            ADD COLUMN desktop_audio_source_id TEXT;
        ALTER TABLE recorder_scenes
            ADD COLUMN microphone_audio_source_id TEXT;
        "#,
        ),
        M::up_with_hook("SELECT 1;", |transaction| {
            let table_count: i64 = transaction.query_row(
                "SELECT COUNT(*) FROM sqlite_schema WHERE type = 'table' AND name = 'recording_segments'",
                [],
                |row| row.get(0),
            )?;

            if table_count == 0 {
                return Ok(());
            }

            let mut statement = transaction.prepare("PRAGMA table_info(recording_segments)")?;
            let columns = statement.query_map([], |row| row.get::<_, String>(1))?;
            for column in columns {
                if column? == "probe_json" {
                    return Ok(());
                }
            }

            transaction.execute(
                r#"
                ALTER TABLE recording_segments
                    ADD COLUMN probe_json TEXT CHECK (probe_json IS NULL OR json_valid(probe_json));
                "#,
                [],
            )?;

            Ok(())
        }),
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
        M::up(
            r#"
        CREATE TABLE notifications (
            id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
            source TEXT NOT NULL CHECK (
                source IN ('ai', 'capture', 'library', 'recorder', 'settings', 'system')
            ),
            topic TEXT NOT NULL,
            status TEXT NOT NULL CHECK (
                status IN ('cancelled', 'failed', 'info', 'running', 'succeeded', 'warning')
            ),
            title TEXT NOT NULL,
            message TEXT NOT NULL,
            actions_json TEXT NOT NULL DEFAULT '[]' CHECK (json_valid(actions_json)),
            created_at_ms INTEGER NOT NULL CHECK (created_at_ms >= 0),
            updated_at_ms INTEGER NOT NULL CHECK (updated_at_ms >= created_at_ms),
            dismissed_at_ms INTEGER CHECK (
                dismissed_at_ms IS NULL OR dismissed_at_ms >= created_at_ms
            )
        );

        CREATE INDEX idx_notifications_status_updated ON notifications(status, updated_at_ms);
        CREATE INDEX idx_notifications_source_topic ON notifications(source, topic);
        CREATE INDEX idx_notifications_dismissed_updated ON notifications(dismissed_at_ms, updated_at_ms);
        "#,
        ),
        M::up(
            r#"
        ALTER TABLE ai_model_assets RENAME TO ai_model_assets_v1;

        CREATE TABLE ai_model_assets (
            id TEXT PRIMARY KEY NOT NULL,
            provider TEXT NOT NULL CHECK (provider IN ('local-whisper-cpp', 'openai-compatible')),
            display_label TEXT NOT NULL,
            local_path TEXT,
            size_bytes INTEGER CHECK (size_bytes IS NULL OR size_bytes >= 0),
            checksum_algorithm TEXT,
            checksum TEXT,
            etag TEXT,
            source_url TEXT,
            downloaded_bytes INTEGER NOT NULL DEFAULT 0 CHECK (downloaded_bytes >= 0),
            total_bytes INTEGER CHECK (total_bytes IS NULL OR total_bytes >= 0),
            error_code TEXT,
            verification_status TEXT NOT NULL DEFAULT 'unknown' CHECK (verification_status IN ('unknown', 'pending', 'passed', 'failed')),
            download_status TEXT NOT NULL CHECK (download_status IN ('missing', 'queued', 'downloading', 'available', 'failed', 'cancelled')),
            self_test_status TEXT NOT NULL CHECK (self_test_status IN ('not-run', 'running', 'passed', 'failed')),
            created_at_ms INTEGER NOT NULL CHECK (created_at_ms >= 0),
            updated_at_ms INTEGER NOT NULL CHECK (updated_at_ms >= created_at_ms)
        );

        INSERT INTO ai_model_assets (
            id,
            provider,
            display_label,
            local_path,
            size_bytes,
            checksum_algorithm,
            checksum,
            etag,
            source_url,
            downloaded_bytes,
            total_bytes,
            error_code,
            verification_status,
            download_status,
            self_test_status,
            created_at_ms,
            updated_at_ms
        )
        SELECT
            id,
            provider,
            display_label,
            local_path,
            size_bytes,
            CASE WHEN sha256 IS NULL THEN NULL ELSE 'sha256' END,
            sha256,
            etag,
            NULL,
            CASE WHEN download_status = 'available' THEN COALESCE(size_bytes, 0) ELSE 0 END,
            size_bytes,
            NULL,
            CASE WHEN download_status = 'available' THEN 'passed' ELSE 'unknown' END,
            download_status,
            self_test_status,
            created_at_ms,
            updated_at_ms
        FROM ai_model_assets_v1;

        DROP TABLE ai_model_assets_v1;
        "#,
        ),
    ])
}
