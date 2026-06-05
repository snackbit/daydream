use rusqlite::{params, Connection, OptionalExtension};

#[derive(Clone, Debug, PartialEq)]
pub struct CreateRecordingInput {
    pub id: String,
    pub active_scene_id: Option<String>,
    pub created_at_ms: i64,
    pub default_fps: f64,
    pub notes: Option<String>,
    pub output_root: Option<String>,
    pub quality_label: String,
    pub storage_dir: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RecordingRow {
    pub id: String,
    pub active_scene_id: Option<String>,
    pub ended_at_ms: Option<i64>,
    pub output_root: Option<String>,
    pub quality_label: String,
    pub started_at_ms: Option<i64>,
    pub status: String,
    pub storage_dir: String,
}

pub fn create_recording(
    connection: &Connection,
    input: &CreateRecordingInput,
) -> rusqlite::Result<RecordingRow> {
    connection.execute(
        r#"
        INSERT INTO recordings (
            id,
            active_scene_id,
            created_at_ms,
            default_fps,
            notes,
            output_root,
            quality_label,
            status,
            storage_dir
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'planned', ?8)
        "#,
        params![
            input.id,
            input.active_scene_id,
            input.created_at_ms,
            input.default_fps,
            input.notes,
            input.output_root,
            input.quality_label,
            input.storage_dir,
        ],
    )?;

    get_recording(connection, &input.id)
}

pub fn get_recording(connection: &Connection, id: &str) -> rusqlite::Result<RecordingRow> {
    connection.query_row(
        r#"
        SELECT
            id,
            active_scene_id,
            ended_at_ms,
            output_root,
            quality_label,
            started_at_ms,
            status,
            storage_dir
        FROM recordings
        WHERE id = ?1
        "#,
        [id],
        |row| {
            Ok(RecordingRow {
                id: row.get(0)?,
                active_scene_id: row.get(1)?,
                ended_at_ms: row.get(2)?,
                output_root: row.get(3)?,
                quality_label: row.get(4)?,
                started_at_ms: row.get(5)?,
                status: row.get(6)?,
                storage_dir: row.get(7)?,
            })
        },
    )
}

pub fn get_latest_recording_id(connection: &Connection) -> rusqlite::Result<Option<String>> {
    connection
        .query_row(
            r#"
            SELECT id
            FROM recordings
            ORDER BY created_at_ms DESC, id ASC
            LIMIT 1
            "#,
            [],
            |row| row.get(0),
        )
        .optional()
}

pub fn mark_recording_started(
    connection: &Connection,
    id: &str,
    started_at_ms: i64,
) -> rusqlite::Result<RecordingRow> {
    connection.execute(
        r#"
        UPDATE recordings
        SET
            status = 'recording',
            started_at_ms = ?2,
            ended_at_ms = NULL
        WHERE id = ?1
        "#,
        params![id, started_at_ms],
    )?;

    get_recording(connection, id)
}

pub fn update_recording_status(
    connection: &Connection,
    id: &str,
    status: &str,
) -> rusqlite::Result<RecordingRow> {
    connection.execute(
        "UPDATE recordings SET status = ?2 WHERE id = ?1",
        params![id, status],
    )?;

    get_recording(connection, id)
}

pub fn mark_recording_completed(
    connection: &Connection,
    id: &str,
    ended_at_ms: i64,
) -> rusqlite::Result<RecordingRow> {
    connection.execute(
        r#"
        UPDATE recordings
        SET
            status = 'completed',
            ended_at_ms = ?2
        WHERE id = ?1
        "#,
        params![id, ended_at_ms],
    )?;

    get_recording(connection, id)
}
