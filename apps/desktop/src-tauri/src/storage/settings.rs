use rusqlite::{params, Connection, OptionalExtension};

#[derive(Clone, Debug, PartialEq)]
pub struct AppSettingRow {
    pub key: String,
    pub value_json: String,
    pub is_sensitive: bool,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SaveAppSettingInput {
    pub key: String,
    pub value_json: String,
    pub is_sensitive: bool,
    pub timestamp_ms: i64,
}

pub fn list_app_settings(connection: &Connection) -> rusqlite::Result<Vec<AppSettingRow>> {
    let mut statement = connection.prepare(
        r#"
        SELECT
            key,
            value_json,
            is_sensitive,
            created_at_ms,
            updated_at_ms
        FROM app_settings
        ORDER BY key
        "#,
    )?;

    let rows = statement.query_map([], app_setting_row)?;

    rows.collect()
}

pub fn get_app_setting(
    connection: &Connection,
    key: &str,
) -> rusqlite::Result<Option<AppSettingRow>> {
    connection
        .query_row(
            r#"
            SELECT
                key,
                value_json,
                is_sensitive,
                created_at_ms,
                updated_at_ms
            FROM app_settings
            WHERE key = ?1
            "#,
            [key],
            app_setting_row,
        )
        .optional()
}

pub fn save_app_setting(
    connection: &Connection,
    input: &SaveAppSettingInput,
) -> rusqlite::Result<AppSettingRow> {
    connection.execute(
        r#"
        INSERT INTO app_settings (
            key,
            value_json,
            is_sensitive,
            created_at_ms,
            updated_at_ms
        ) VALUES (?1, ?2, ?3, ?4, ?4)
        ON CONFLICT(key) DO UPDATE SET
            value_json = excluded.value_json,
            is_sensitive = excluded.is_sensitive,
            updated_at_ms = excluded.updated_at_ms
        "#,
        params![
            input.key,
            input.value_json,
            input.is_sensitive,
            input.timestamp_ms,
        ],
    )?;

    get_app_setting(connection, &input.key)?.ok_or(rusqlite::Error::QueryReturnedNoRows)
}

pub fn save_app_settings(
    connection: &mut Connection,
    inputs: &[SaveAppSettingInput],
) -> rusqlite::Result<Vec<AppSettingRow>> {
    let transaction = connection.transaction()?;

    for input in inputs {
        save_app_setting(&transaction, input)?;
    }

    let rows = list_app_settings(&transaction)?;
    transaction.commit()?;

    Ok(rows)
}

pub fn reset_app_settings(connection: &Connection) -> rusqlite::Result<()> {
    connection.execute("DELETE FROM app_settings", [])?;

    Ok(())
}

fn app_setting_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AppSettingRow> {
    Ok(AppSettingRow {
        key: row.get(0)?,
        value_json: row.get(1)?,
        is_sensitive: row.get::<_, i64>(2)? != 0,
        created_at_ms: row.get(3)?,
        updated_at_ms: row.get(4)?,
    })
}
