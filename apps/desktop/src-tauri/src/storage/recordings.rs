use rusqlite::{params, Connection, OptionalExtension};

const ACTIVE_RECORDING_STATUSES: &[&str] = &["planned", "recording", "paused"];
const DELETABLE_RECORDING_STATUSES: &[&str] = &["completed", "failed", "recovered-partial"];
const INTERRUPTED_RECORDING_ERROR_CODE: &str = "recording_interrupted_by_shutdown";

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
    pub created_at_ms: i64,
    pub default_fps: f64,
    pub id: String,
    pub active_scene_id: Option<String>,
    pub ended_at_ms: Option<i64>,
    pub output_root: Option<String>,
    pub quality_label: String,
    pub started_at_ms: Option<i64>,
    pub status: String,
    pub storage_dir: String,
    pub title: Option<String>,
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
            created_at_ms,
            default_fps,
            ended_at_ms,
            output_root,
            quality_label,
            started_at_ms,
            status,
            storage_dir,
            title
        FROM recordings
        WHERE id = ?1
        "#,
        [id],
        recording_from_row,
    )
}

pub fn list_recordings(connection: &Connection) -> rusqlite::Result<Vec<RecordingRow>> {
    let mut statement = connection.prepare(
        r#"
        SELECT
            id,
            active_scene_id,
            created_at_ms,
            default_fps,
            ended_at_ms,
            output_root,
            quality_label,
            started_at_ms,
            status,
            storage_dir,
            title
        FROM recordings
        WHERE status != 'deleted'
        ORDER BY COALESCE(started_at_ms, created_at_ms) DESC, id ASC
        "#,
    )?;

    let rows = statement.query_map([], recording_from_row)?;

    rows.collect()
}

/// Select by authoritative segment clocks before doing filesystem or source reads.
pub fn list_recordings_in_range(
    connection: &Connection,
    start_ms: i64,
    end_ms: i64,
) -> rusqlite::Result<Vec<RecordingRow>> {
    let mut statement=connection.prepare("SELECT DISTINCT s.recording_id FROM recording_segments s JOIN recordings r ON r.id=s.recording_id WHERE r.status != 'deleted' AND s.state='finalized' AND s.duration_ms>0 AND s.started_at_ms<?2 AND s.started_at_ms+s.duration_ms>?1")?;
    let ids = statement
        .query_map(rusqlite::params![start_ms, end_ms], |row| {
            row.get::<_, String>(0)
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    ids.into_iter()
        .map(|id| get_recording(connection, &id))
        .collect()
}

pub fn delete_recording(connection: &Connection, id: &str) -> rusqlite::Result<()> {
    let status = connection
        .query_row("SELECT status FROM recordings WHERE id = ?1", [id], |row| {
            row.get::<_, String>(0)
        })
        .optional()?;

    if let Some(status) = status {
        if !is_deletable_recording_status(&status) {
            return Err(rusqlite::Error::InvalidParameterName(format!(
                "recording status {status} is not deletable"
            )));
        }
    }

    connection.execute("DELETE FROM recordings WHERE id = ?1", [id])?;

    Ok(())
}

pub fn rename_recording(
    connection: &Connection,
    id: &str,
    title: Option<&str>,
) -> rusqlite::Result<RecordingRow> {
    let status = connection
        .query_row("SELECT status FROM recordings WHERE id = ?1", [id], |row| {
            row.get::<_, String>(0)
        })
        .optional()?;

    if let Some(status) = status {
        if !is_deletable_recording_status(&status) {
            return Err(rusqlite::Error::InvalidParameterName(format!(
                "recording status {status} is not renameable"
            )));
        }
    }

    connection.execute(
        "UPDATE recordings SET title = ?2 WHERE id = ?1",
        params![id, title],
    )?;

    get_recording(connection, id)
}

pub fn recover_interrupted_recordings(
    connection: &Connection,
    _ended_at_ms: i64,
) -> rusqlite::Result<usize> {
    let transaction = connection.unchecked_transaction()?;
    let connection = &transaction;
    let ids = {
        let mut statement = connection.prepare(
            "SELECT id FROM recordings WHERE status IN ('planned','recording','paused')",
        )?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };
    connection.execute(
        r#"
        UPDATE recording_segments
        SET
            ended_at_ms = started_at_ms,
            duration_ms = 0,
            state = 'failed',
            ffmpeg_exit_code = NULL,
            error_code = ?1,
            updated_at_ms = created_at_ms
        WHERE state = 'recording'
            AND recording_id IN (
                SELECT id
                FROM recordings
                WHERE status IN ('planned', 'recording', 'paused')
            )
        "#,
        params![INTERRUPTED_RECORDING_ERROR_CODE],
    )?;

    let recovered = connection.execute(
        r#"
        UPDATE recordings
        SET
            status = 'failed',
            ended_at_ms = COALESCE(
                (
                    SELECT MAX(ended_at_ms)
                    FROM recording_segments
                    WHERE recording_segments.recording_id = recordings.id
                ),
                started_at_ms,
                created_at_ms
            )
        WHERE status IN ('planned', 'recording', 'paused')
        "#,
        [],
    )?;
    for id in ids {
        crate::recording_privacy::seal_interrupted_capture(connection, &id, _ended_at_ms)?;
    }
    transaction.commit()?;
    Ok(recovered)
}

pub fn is_active_recording_status(status: &str) -> bool {
    ACTIVE_RECORDING_STATUSES.contains(&status)
}

pub fn is_deletable_recording_status(status: &str) -> bool {
    DELETABLE_RECORDING_STATUSES.contains(&status)
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

fn recording_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RecordingRow> {
    Ok(RecordingRow {
        id: row.get(0)?,
        active_scene_id: row.get(1)?,
        created_at_ms: row.get(2)?,
        default_fps: row.get(3)?,
        ended_at_ms: row.get(4)?,
        output_root: row.get(5)?,
        quality_label: row.get(6)?,
        started_at_ms: row.get(7)?,
        status: row.get(8)?,
        storage_dir: row.get(9)?,
        title: row.get(10)?,
    })
}
