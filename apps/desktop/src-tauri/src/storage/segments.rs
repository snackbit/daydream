use rusqlite::{params, Connection};

#[derive(Clone, Debug, PartialEq)]
pub struct CreateRecordingSegmentInput {
    pub file_path: String,
    pub id: String,
    pub recording_id: String,
    pub segment_index: i64,
    pub started_at_ms: i64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FinalizeRecordingSegmentInput {
    pub desktop_audio_stream_index: Option<i64>,
    pub duration_ms: i64,
    pub ended_at_ms: i64,
    pub id: String,
    pub microphone_audio_stream_index: Option<i64>,
    pub probe_json: Option<String>,
    pub updated_at_ms: i64,
    pub video_stream_index: Option<i64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FailRecordingSegmentInput {
    pub ended_at_ms: i64,
    pub error_code: String,
    pub ffmpeg_exit_code: Option<i64>,
    pub id: String,
    pub updated_at_ms: i64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RecordingSegmentRow {
    pub desktop_audio_stream_index: Option<i64>,
    pub duration_ms: Option<i64>,
    pub ended_at_ms: Option<i64>,
    pub error_code: Option<String>,
    pub ffmpeg_exit_code: Option<i64>,
    pub file_path: String,
    pub id: String,
    pub microphone_audio_stream_index: Option<i64>,
    pub probe_json: Option<String>,
    pub recording_id: String,
    pub segment_index: i64,
    pub started_at_ms: i64,
    pub state: String,
    pub video_stream_index: Option<i64>,
}

pub fn create_recording_segment(
    connection: &Connection,
    input: &CreateRecordingSegmentInput,
) -> rusqlite::Result<RecordingSegmentRow> {
    connection.execute(
        r#"
        INSERT INTO recording_segments (
            id,
            recording_id,
            segment_index,
            file_path,
            started_at_ms,
            state,
            created_at_ms,
            updated_at_ms
        ) VALUES (?1, ?2, ?3, ?4, ?5, 'recording', ?5, ?5)
        "#,
        params![
            input.id,
            input.recording_id,
            input.segment_index,
            input.file_path,
            input.started_at_ms,
        ],
    )?;

    get_recording_segment(connection, &input.id)
}

pub fn finalize_recording_segment(
    connection: &Connection,
    input: &FinalizeRecordingSegmentInput,
) -> rusqlite::Result<RecordingSegmentRow> {
    connection.execute(
        r#"
        UPDATE recording_segments
        SET
            ended_at_ms = ?2,
            duration_ms = ?3,
            state = 'finalized',
            video_stream_index = ?4,
            desktop_audio_stream_index = ?5,
            microphone_audio_stream_index = ?6,
            ffmpeg_exit_code = 0,
            error_code = NULL,
            probe_json = ?7,
            updated_at_ms = ?8
        WHERE id = ?1
        "#,
        params![
            input.id,
            input.ended_at_ms,
            input.duration_ms,
            input.video_stream_index,
            input.desktop_audio_stream_index,
            input.microphone_audio_stream_index,
            input.probe_json,
            input.updated_at_ms,
        ],
    )?;

    get_recording_segment(connection, &input.id)
}

pub fn fail_recording_segment(
    connection: &Connection,
    input: &FailRecordingSegmentInput,
) -> rusqlite::Result<RecordingSegmentRow> {
    connection.execute(
        r#"
        UPDATE recording_segments
        SET
            ended_at_ms = ?2,
            duration_ms = CASE
                WHEN ?2 >= started_at_ms THEN ?2 - started_at_ms
                ELSE NULL
            END,
            state = 'failed',
            ffmpeg_exit_code = ?3,
            error_code = ?4,
            updated_at_ms = ?5
        WHERE id = ?1
        "#,
        params![
            input.id,
            input.ended_at_ms,
            input.ffmpeg_exit_code,
            input.error_code,
            input.updated_at_ms,
        ],
    )?;

    get_recording_segment(connection, &input.id)
}

pub fn list_recording_segments(
    connection: &Connection,
    recording_id: &str,
) -> rusqlite::Result<Vec<RecordingSegmentRow>> {
    let mut statement = connection.prepare(
        r#"
        SELECT
            id,
            recording_id,
            segment_index,
            file_path,
            started_at_ms,
            ended_at_ms,
            duration_ms,
            state,
            video_stream_index,
            desktop_audio_stream_index,
            microphone_audio_stream_index,
            ffmpeg_exit_code,
            error_code,
            probe_json
        FROM recording_segments
        WHERE recording_id = ?1
        ORDER BY segment_index ASC
        "#,
    )?;

    let rows = statement.query_map([recording_id], recording_segment_from_row)?;

    rows.collect()
}

fn get_recording_segment(
    connection: &Connection,
    id: &str,
) -> rusqlite::Result<RecordingSegmentRow> {
    connection.query_row(
        r#"
        SELECT
            id,
            recording_id,
            segment_index,
            file_path,
            started_at_ms,
            ended_at_ms,
            duration_ms,
            state,
            video_stream_index,
            desktop_audio_stream_index,
            microphone_audio_stream_index,
            ffmpeg_exit_code,
            error_code,
            probe_json
        FROM recording_segments
        WHERE id = ?1
        "#,
        [id],
        recording_segment_from_row,
    )
}

fn recording_segment_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RecordingSegmentRow> {
    Ok(RecordingSegmentRow {
        id: row.get(0)?,
        recording_id: row.get(1)?,
        segment_index: row.get(2)?,
        file_path: row.get(3)?,
        started_at_ms: row.get(4)?,
        ended_at_ms: row.get(5)?,
        duration_ms: row.get(6)?,
        state: row.get(7)?,
        video_stream_index: row.get(8)?,
        desktop_audio_stream_index: row.get(9)?,
        microphone_audio_stream_index: row.get(10)?,
        ffmpeg_exit_code: row.get(11)?,
        error_code: row.get(12)?,
        probe_json: row.get(13)?,
    })
}
