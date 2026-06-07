use rusqlite::{params, Connection};

pub const LINUX_SCREENCAST_MONITOR_RESTORE_TOKEN_ID: &str = "linux-screencast-monitor";

#[derive(Clone, Debug, PartialEq)]
pub struct PortalRestoreTokenRow {
    pub created_at_ms: i64,
    pub id: String,
    pub last_used_at_ms: Option<i64>,
    pub portal: String,
    pub token: String,
    pub updated_at_ms: i64,
}

pub fn get_portal_restore_token(
    connection: &Connection,
    id: &str,
) -> rusqlite::Result<Option<PortalRestoreTokenRow>> {
    match connection.query_row(
        r#"
        SELECT
            id,
            portal,
            token,
            created_at_ms,
            updated_at_ms,
            last_used_at_ms
        FROM recorder_portal_restore_tokens
        WHERE id = ?1
        "#,
        [id],
        portal_restore_token_from_row,
    ) {
        Ok(row) => Ok(Some(row)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(error) => Err(error),
    }
}

pub fn upsert_portal_restore_token(
    connection: &Connection,
    id: &str,
    portal: &str,
    token: &str,
    timestamp_ms: i64,
) -> rusqlite::Result<PortalRestoreTokenRow> {
    connection.execute(
        r#"
        INSERT INTO recorder_portal_restore_tokens (
            id,
            portal,
            token,
            created_at_ms,
            updated_at_ms,
            last_used_at_ms
        ) VALUES (?1, ?2, ?3, ?4, ?4, ?4)
        ON CONFLICT(id) DO UPDATE SET
            portal = excluded.portal,
            token = excluded.token,
            updated_at_ms = excluded.updated_at_ms,
            last_used_at_ms = excluded.last_used_at_ms
        "#,
        params![id, portal, token, timestamp_ms],
    )?;

    get_portal_restore_token(connection, id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)
}

pub fn mark_portal_restore_token_used(
    connection: &Connection,
    id: &str,
    timestamp_ms: i64,
) -> rusqlite::Result<()> {
    connection.execute(
        r#"
        UPDATE recorder_portal_restore_tokens
        SET last_used_at_ms = ?2
        WHERE id = ?1
        "#,
        params![id, timestamp_ms],
    )?;

    Ok(())
}

pub fn delete_portal_restore_token(connection: &Connection, id: &str) -> rusqlite::Result<()> {
    connection.execute(
        r#"
        DELETE FROM recorder_portal_restore_tokens
        WHERE id = ?1
        "#,
        [id],
    )?;

    Ok(())
}

fn portal_restore_token_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<PortalRestoreTokenRow> {
    Ok(PortalRestoreTokenRow {
        id: row.get(0)?,
        portal: row.get(1)?,
        token: row.get(2)?,
        created_at_ms: row.get(3)?,
        updated_at_ms: row.get(4)?,
        last_used_at_ms: row.get(5)?,
    })
}
