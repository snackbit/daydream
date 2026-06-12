use rusqlite::{params, Connection, OptionalExtension};

#[derive(Clone, Debug, PartialEq)]
pub struct RedactionRow {
    pub created_at_ms: i64,
    pub end_offset_ms: i64,
    pub id: String,
    pub reason: Option<String>,
    pub recording_id: String,
    pub start_offset_ms: i64,
    pub track_ids_json: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CreateRedactionInput {
    pub created_at_ms: i64,
    pub end_offset_ms: i64,
    pub id: String,
    pub reason: Option<String>,
    pub recording_id: String,
    pub start_offset_ms: i64,
    pub track_ids_json: String,
}

pub fn redaction_applies_to_track_range(
    redaction: &RedactionRow,
    track_id: &str,
    start_offset_ms: i64,
    end_offset_ms: i64,
) -> bool {
    if start_offset_ms >= redaction.end_offset_ms || end_offset_ms <= redaction.start_offset_ms {
        return false;
    }

    redaction_track_ids(&redaction.track_ids_json)
        .iter()
        .any(|redacted_track_id| redacted_track_id == track_id)
}

fn redaction_track_ids(track_ids_json: &str) -> Vec<String> {
    serde_json::from_str::<Vec<String>>(track_ids_json).unwrap_or_default()
}

pub fn create_redaction(
    connection: &Connection,
    input: &CreateRedactionInput,
) -> rusqlite::Result<RedactionRow> {
    connection.execute(
        r#"
        INSERT INTO redactions (
            id,
            recording_id,
            start_offset_ms,
            end_offset_ms,
            reason,
            created_at_ms,
            track_ids_json
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        "#,
        params![
            input.id,
            input.recording_id,
            input.start_offset_ms,
            input.end_offset_ms,
            input.reason,
            input.created_at_ms,
            input.track_ids_json,
        ],
    )?;

    get_redaction(connection, &input.id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)
}

pub fn get_redaction(connection: &Connection, id: &str) -> rusqlite::Result<Option<RedactionRow>> {
    connection
        .query_row(
            r#"
            SELECT
                id,
                recording_id,
                start_offset_ms,
                end_offset_ms,
                reason,
                created_at_ms,
                track_ids_json
            FROM redactions
            WHERE id = ?1
            "#,
            [id],
            redaction_from_row,
        )
        .optional()
}

pub fn list_redactions(
    connection: &Connection,
    recording_id: &str,
) -> rusqlite::Result<Vec<RedactionRow>> {
    let mut statement = connection.prepare(
        r#"
        SELECT
            id,
            recording_id,
            start_offset_ms,
            end_offset_ms,
            reason,
            created_at_ms,
            track_ids_json
        FROM redactions
        WHERE recording_id = ?1
        ORDER BY start_offset_ms ASC, end_offset_ms ASC, id ASC
        "#,
    )?;
    let rows = statement.query_map([recording_id], redaction_from_row)?;

    rows.collect()
}

pub fn delete_redaction(
    connection: &Connection,
    recording_id: &str,
    redaction_id: &str,
) -> rusqlite::Result<bool> {
    let deleted = connection.execute(
        "DELETE FROM redactions WHERE recording_id = ?1 AND id = ?2",
        [recording_id, redaction_id],
    )?;

    Ok(deleted > 0)
}

pub fn delete_redactions_for_recording(
    connection: &Connection,
    recording_id: &str,
) -> rusqlite::Result<usize> {
    connection.execute(
        "DELETE FROM redactions WHERE recording_id = ?1",
        [recording_id],
    )
}

fn redaction_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RedactionRow> {
    Ok(RedactionRow {
        id: row.get(0)?,
        recording_id: row.get(1)?,
        start_offset_ms: row.get(2)?,
        end_offset_ms: row.get(3)?,
        reason: row.get(4)?,
        created_at_ms: row.get(5)?,
        track_ids_json: row.get(6)?,
    })
}
