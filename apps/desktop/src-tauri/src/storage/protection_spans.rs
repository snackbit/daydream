use rusqlite::{params, Connection, OptionalExtension};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordingProtectionSpanRow {
    pub action: String,
    pub closed_at_ms: Option<i64>,
    pub end_offset_ms: Option<i64>,
    pub id: String,
    pub opened_at_ms: i64,
    pub recording_id: String,
    pub redaction_id: Option<String>,
    pub rule_id: String,
    pub rule_label: String,
    pub start_offset_ms: i64,
    pub status: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreateRecordingProtectionSpanInput {
    pub action: String,
    pub id: String,
    pub opened_at_ms: i64,
    pub recording_id: String,
    pub rule_id: String,
    pub rule_label: String,
    pub start_offset_ms: i64,
}

pub fn create_recording_protection_span(
    connection: &Connection,
    input: &CreateRecordingProtectionSpanInput,
) -> rusqlite::Result<RecordingProtectionSpanRow> {
    connection.execute(
        r#"
        INSERT INTO recording_protection_spans (
            id,
            recording_id,
            rule_id,
            rule_label,
            action,
            status,
            start_offset_ms,
            opened_at_ms
        ) VALUES (?1, ?2, ?3, ?4, ?5, 'open', ?6, ?7)
        "#,
        params![
            input.id,
            input.recording_id,
            input.rule_id,
            input.rule_label,
            input.action,
            input.start_offset_ms,
            input.opened_at_ms,
        ],
    )?;

    get_recording_protection_span(connection, &input.id)?
        .ok_or(rusqlite::Error::QueryReturnedNoRows)
}

pub fn get_recording_protection_span(
    connection: &Connection,
    id: &str,
) -> rusqlite::Result<Option<RecordingProtectionSpanRow>> {
    connection
        .query_row(
            r#"
            SELECT
                id,
                recording_id,
                rule_id,
                rule_label,
                action,
                status,
                start_offset_ms,
                end_offset_ms,
                opened_at_ms,
                closed_at_ms,
                redaction_id
            FROM recording_protection_spans
            WHERE id = ?1
            "#,
            [id],
            protection_span_row,
        )
        .optional()
}

pub fn list_recording_protection_spans(
    connection: &Connection,
    recording_id: &str,
) -> rusqlite::Result<Vec<RecordingProtectionSpanRow>> {
    let mut statement = connection.prepare(
        r#"
        SELECT
            id,
            recording_id,
            rule_id,
            rule_label,
            action,
            status,
            start_offset_ms,
            end_offset_ms,
            opened_at_ms,
            closed_at_ms,
            redaction_id
        FROM recording_protection_spans
        WHERE recording_id = ?1
        ORDER BY start_offset_ms ASC, id ASC
        "#,
    )?;
    let rows = statement.query_map([recording_id], protection_span_row)?;

    rows.collect()
}

pub fn list_open_recording_protection_spans(
    connection: &Connection,
    recording_id: &str,
) -> rusqlite::Result<Vec<RecordingProtectionSpanRow>> {
    let mut statement = connection.prepare(
        r#"
        SELECT
            id,
            recording_id,
            rule_id,
            rule_label,
            action,
            status,
            start_offset_ms,
            end_offset_ms,
            opened_at_ms,
            closed_at_ms,
            redaction_id
        FROM recording_protection_spans
        WHERE recording_id = ?1
            AND status = 'open'
        ORDER BY start_offset_ms ASC, id ASC
        "#,
    )?;
    let rows = statement.query_map([recording_id], protection_span_row)?;

    rows.collect()
}

pub fn close_recording_protection_span(
    connection: &Connection,
    id: &str,
    end_offset_ms: i64,
    closed_at_ms: i64,
) -> rusqlite::Result<RecordingProtectionSpanRow> {
    connection.execute(
        r#"
        UPDATE recording_protection_spans
        SET status = 'closed',
            end_offset_ms = ?2,
            closed_at_ms = ?3
        WHERE id = ?1
            AND status = 'open'
        "#,
        params![id, end_offset_ms, closed_at_ms],
    )?;

    get_recording_protection_span(connection, id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)
}

pub fn mark_recording_protection_span_materialized(
    connection: &Connection,
    id: &str,
    redaction_id: &str,
) -> rusqlite::Result<RecordingProtectionSpanRow> {
    connection.execute(
        r#"
        UPDATE recording_protection_spans
        SET status = 'materialized',
            redaction_id = ?2
        WHERE id = ?1
            AND status = 'closed'
        "#,
        params![id, redaction_id],
    )?;

    get_recording_protection_span(connection, id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)
}

fn protection_span_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RecordingProtectionSpanRow> {
    Ok(RecordingProtectionSpanRow {
        id: row.get(0)?,
        recording_id: row.get(1)?,
        rule_id: row.get(2)?,
        rule_label: row.get(3)?,
        action: row.get(4)?,
        status: row.get(5)?,
        start_offset_ms: row.get(6)?,
        end_offset_ms: row.get(7)?,
        opened_at_ms: row.get(8)?,
        closed_at_ms: row.get(9)?,
        redaction_id: row.get(10)?,
    })
}
