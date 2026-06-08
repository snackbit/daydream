use rusqlite::{params, Connection, OptionalExtension};

#[derive(Clone, Debug, PartialEq)]
pub struct AppSettingsRow {
    pub record_fps: f64,
    pub record_video_bitrate_kbps: i64,
    pub storage_retention_days: i64,
    pub storage_size_limit_bytes: i64,
    pub updated_at_ms: i64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SaveAppSettingsInput {
    pub record_fps: f64,
    pub record_video_bitrate_kbps: i64,
    pub storage_retention_days: i64,
    pub storage_size_limit_bytes: i64,
    pub updated_at_ms: i64,
}

pub fn get_app_settings(connection: &Connection) -> rusqlite::Result<Option<AppSettingsRow>> {
    connection
        .query_row(
            r#"
            SELECT
                record_fps,
                record_video_bitrate_kbps,
                storage_retention_days,
                storage_size_limit_bytes,
                updated_at_ms
            FROM app_settings
            WHERE id = 1
            "#,
            [],
            |row| {
                Ok(AppSettingsRow {
                    record_fps: row.get(0)?,
                    record_video_bitrate_kbps: row.get(1)?,
                    storage_retention_days: row.get(2)?,
                    storage_size_limit_bytes: row.get(3)?,
                    updated_at_ms: row.get(4)?,
                })
            },
        )
        .optional()
}

pub fn save_app_settings(
    connection: &Connection,
    input: &SaveAppSettingsInput,
) -> rusqlite::Result<AppSettingsRow> {
    connection.execute(
        r#"
        INSERT INTO app_settings (
            id,
            record_fps,
            record_video_bitrate_kbps,
            storage_retention_days,
            storage_size_limit_bytes,
            updated_at_ms
        ) VALUES (1, ?1, ?2, ?3, ?4, ?5)
        ON CONFLICT(id) DO UPDATE SET
            record_fps = excluded.record_fps,
            record_video_bitrate_kbps = excluded.record_video_bitrate_kbps,
            storage_retention_days = excluded.storage_retention_days,
            storage_size_limit_bytes = excluded.storage_size_limit_bytes,
            updated_at_ms = excluded.updated_at_ms
        "#,
        params![
            input.record_fps,
            input.record_video_bitrate_kbps,
            input.storage_retention_days,
            input.storage_size_limit_bytes,
            input.updated_at_ms,
        ],
    )?;

    get_app_settings(connection)?.ok_or(rusqlite::Error::QueryReturnedNoRows)
}

pub fn reset_app_settings(connection: &Connection) -> rusqlite::Result<()> {
    connection.execute("DELETE FROM app_settings WHERE id = 1", [])?;

    Ok(())
}
