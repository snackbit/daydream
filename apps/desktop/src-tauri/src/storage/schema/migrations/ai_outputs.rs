use rusqlite_migration::M;

pub(in crate::storage) fn migrations() -> Vec<M<'static>> {
    vec![
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
        M::up(
            r#"
    ALTER TABLE recordings
        ADD COLUMN title TEXT;
    "#,
        ),
        M::up(
            r#"
    ALTER TABLE processing_chunks
        ADD COLUMN track_id TEXT
            CHECK (track_id IS NULL OR length(track_id) = 36)
            REFERENCES recording_tracks(id) ON DELETE SET NULL;

    CREATE INDEX idx_processing_chunks_track_status ON processing_chunks(track_id, status);
    "#,
        ),
        M::up_with_hook("SELECT 1;", |transaction| {
            let mut statement = transaction.prepare("PRAGMA table_info(processing_chunks)")?;
            let columns = statement.query_map([], |row| row.get::<_, String>(1))?;
            for column in columns {
                if column? == "result_code" {
                    return Ok(());
                }
            }

            transaction.execute(
                r#"
            ALTER TABLE processing_chunks
                ADD COLUMN result_code TEXT;
            "#,
                [],
            )?;

            Ok(())
        }),
        M::up_with_hook("SELECT 1;", |transaction| {
            let mut statement = transaction.prepare("PRAGMA table_info(redactions)")?;
            let columns = statement.query_map([], |row| row.get::<_, String>(1))?;
            for column in columns {
                if column? == "track_ids_json" {
                    return Ok(());
                }
            }

            transaction.execute(
                r#"
            ALTER TABLE redactions
                ADD COLUMN track_ids_json TEXT NOT NULL DEFAULT '[]'
                CHECK (json_valid(track_ids_json));
            "#,
                [],
            )?;

            Ok(())
        }),
        M::up_with_hook("SELECT 1;", |transaction| {
            let mut statement = transaction.prepare("PRAGMA table_info(vlm_observations)")?;
            let columns = statement.query_map([], |row| row.get::<_, String>(1))?;
            for column in columns {
                if column? == "observation_kind" {
                    return Ok(());
                }
            }

            transaction.execute(
                r#"
            ALTER TABLE vlm_observations
                ADD COLUMN observation_kind TEXT NOT NULL DEFAULT 'screen-observation'
                CHECK (observation_kind IN ('corrected-ocr', 'screen-observation'));
            "#,
                [],
            )?;

            Ok(())
        }),
    ]
}
