use rusqlite::{params, Connection, OptionalExtension};

use super::{rows::ai_model_asset_row, types::*};

pub fn save_ai_model_asset(
    connection: &Connection,
    input: &SaveAiModelAssetInput,
) -> rusqlite::Result<AiModelAssetRow> {
    connection.execute(
        r#"
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
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?16)
        ON CONFLICT(id) DO UPDATE SET
            provider = excluded.provider,
            display_label = excluded.display_label,
            local_path = excluded.local_path,
            size_bytes = excluded.size_bytes,
            checksum_algorithm = excluded.checksum_algorithm,
            checksum = excluded.checksum,
            etag = excluded.etag,
            source_url = excluded.source_url,
            downloaded_bytes = excluded.downloaded_bytes,
            total_bytes = excluded.total_bytes,
            error_code = excluded.error_code,
            verification_status = excluded.verification_status,
            download_status = excluded.download_status,
            self_test_status = excluded.self_test_status,
            updated_at_ms = excluded.updated_at_ms
        "#,
        params![
            input.id,
            input.provider,
            input.display_label,
            input.local_path,
            input.size_bytes,
            input.checksum_algorithm,
            input.checksum,
            input.etag,
            input.source_url,
            input.downloaded_bytes,
            input.total_bytes,
            input.error_code,
            input.verification_status,
            input.download_status,
            input.self_test_status,
            input.timestamp_ms,
        ],
    )?;

    get_ai_model_asset(connection, &input.id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)
}

pub fn get_ai_model_asset(
    connection: &Connection,
    id: &str,
) -> rusqlite::Result<Option<AiModelAssetRow>> {
    connection
        .query_row(
            r#"
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
            FROM ai_model_assets
            WHERE id = ?1
            "#,
            [id],
            ai_model_asset_row,
        )
        .optional()
}

pub fn list_ai_model_assets(connection: &Connection) -> rusqlite::Result<Vec<AiModelAssetRow>> {
    let mut statement = connection.prepare(
        r#"
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
        FROM ai_model_assets
        ORDER BY id
        "#,
    )?;

    let rows = statement.query_map([], ai_model_asset_row)?;

    rows.collect()
}

pub fn delete_ai_model_asset(connection: &Connection, id: &str) -> rusqlite::Result<()> {
    connection.execute("DELETE FROM ai_model_assets WHERE id = ?1", [id])?;

    Ok(())
}
