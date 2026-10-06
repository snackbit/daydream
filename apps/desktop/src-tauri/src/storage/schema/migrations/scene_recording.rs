use rusqlite_migration::M;

pub(in crate::storage) fn migrations() -> Vec<M<'static>> {
    vec![
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

    -- The outer migration runner disables foreign keys for table rebuilds.
    -- Clear surviving child rows before restoring their explicit backups.
    DELETE FROM recording_tracks;
    DELETE FROM redactions;
    DELETE FROM capture_events;
    DELETE FROM processing_jobs;

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
    ]
}
