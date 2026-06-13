use rusqlite_migration::M;

pub(in crate::storage) fn migrations() -> Vec<M<'static>> {
    vec![M::up(
        r#"
    ALTER TABLE ai_model_assets RENAME TO ai_model_assets_v2;

    CREATE TABLE ai_model_assets (
        id TEXT PRIMARY KEY NOT NULL,
        provider TEXT NOT NULL CHECK (provider IN ('local-whisper-cpp', 'openai-compatible', 'local-tesseract-ocr')),
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
    FROM ai_model_assets_v2;

    DROP TABLE ai_model_assets_v2;
    "#,
    )]
}
