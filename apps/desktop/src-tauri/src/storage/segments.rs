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
    pub capture_run_id: Option<String>,
    pub timeline_base_ms: Option<i64>,
    pub timeline_basis_version: Option<i64>,
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
    // Earlier migration hooks and upgrade fixtures use this storage boundary
    // before durable bases exist. Their playback semantics remain legacy.
    if !has_timeline_columns(connection) {
        connection.execute("INSERT INTO recording_segments(id,recording_id,segment_index,file_path,started_at_ms,state,created_at_ms,updated_at_ms) VALUES(?1,?2,?3,?4,?5,'recording',?5,?5)",params![input.id,input.recording_id,input.segment_index,input.file_path,input.started_at_ms])?;
        return get_recording_segment(connection, &input.id);
    }
    let predecessors = list_recording_segments(connection, &input.recording_id)?;
    let timeline_base = next_timeline_base(&predecessors, input.segment_index, input.started_at_ms);
    connection.execute(
        r#"
        INSERT INTO recording_segments (
            id,
            recording_id,
            segment_index,
            file_path,
            started_at_ms,
            timeline_base_ms,
            timeline_basis_version,
            state,
            created_at_ms,
            updated_at_ms
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, CASE WHEN ?6 IS NOT NULL THEN 1 END, 'recording', ?5, ?5)
        "#,
        params![
            input.id,
            input.recording_id,
            input.segment_index,
            input.file_path,
            input.started_at_ms,
            timeline_base,
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
    let sql = r#"
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
            probe_json,
            timeline_base_ms,
            timeline_basis_version,
            capture_run_id
        FROM recording_segments
        WHERE recording_id = ?1
        ORDER BY segment_index ASC
        "#;
    let sql = legacy_timeline_projection(connection, sql);
    let mut statement = connection.prepare(&sql)?;

    let rows = statement.query_map([recording_id], recording_segment_from_row)?;

    rows.collect()
}

fn get_recording_segment(
    connection: &Connection,
    id: &str,
) -> rusqlite::Result<RecordingSegmentRow> {
    let sql = r#"
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
            probe_json,
            timeline_base_ms,
            timeline_basis_version,
            capture_run_id
        FROM recording_segments
        WHERE id = ?1
        "#;
    connection.query_row(
        &legacy_timeline_projection(connection, sql),
        [id],
        recording_segment_from_row,
    )
}

fn has_timeline_columns(connection: &Connection) -> bool {
    connection
        .prepare("SELECT timeline_base_ms,timeline_basis_version FROM recording_segments LIMIT 0")
        .is_ok()
}
fn legacy_timeline_projection(connection: &Connection, sql: &str) -> String {
    let sql = if connection
        .prepare("SELECT capture_run_id FROM recording_segments LIMIT 0")
        .is_ok()
    {
        sql.to_string()
    } else {
        sql.replace("capture_run_id", "NULL AS capture_run_id")
    };
    if has_timeline_columns(connection) {
        sql
    } else {
        sql.replace(
            "timeline_base_ms,\n            timeline_basis_version",
            "NULL AS timeline_base_ms,\n            NULL AS timeline_basis_version",
        )
    }
}

fn recording_segment_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RecordingSegmentRow> {
    Ok(RecordingSegmentRow {
        capture_run_id: row.get(16)?,
        timeline_base_ms: row.get(14)?,
        timeline_basis_version: row.get(15)?,
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

/// Persist successors from metadata, never from current file availability.
fn next_timeline_base(segments: &[RecordingSegmentRow], index: i64, started: i64) -> Option<i64> {
    if index == 1 && segments.is_empty() {
        return Some(0);
    }
    let previous = segments.last()?;
    if previous.segment_index.checked_add(1)? != index {
        return None;
    }
    let base = previous.timeline_base_ms?;
    match previous.state.as_str() {
        "finalized" => base
            .checked_add(previous.duration_ms?)?
            .checked_add(started.checked_sub(previous.ended_at_ms?)?.max(0)),
        "failed" => Some(base),
        _ => None,
    }
}

/// Compatibility fallback for ambiguous legacy playback. Recall never treats
/// this fallback as a trusted absolute-time basis.
pub fn segment_playback_bases(segments: &[RecordingSegmentRow]) -> Vec<i64> {
    let mut cursor: i64 = 0;
    segments
        .iter()
        .enumerate()
        .map(|(index, segment)| {
            let base = segment.timeline_base_ms.unwrap_or(cursor);
            if segment.state == "finalized" && std::path::Path::new(&segment.file_path).is_file() {
                let duration = segment
                    .duration_ms
                    .or_else(|| {
                        segment
                            .ended_at_ms
                            .map(|end| end.saturating_sub(segment.started_at_ms))
                    })
                    .unwrap_or(0)
                    .max(0);
                let pause = segment
                    .ended_at_ms
                    .zip(segments.get(index + 1).map(|next| next.started_at_ms))
                    .map(|(end, start)| start.saturating_sub(end).max(0))
                    .unwrap_or(0);
                cursor = base.saturating_add(duration).saturating_add(pause);
            }
            base
        })
        .collect()
}

pub fn backfill_segment_timeline_bases(connection: &Connection) -> rusqlite::Result<()> {
    let ids = {
        let mut statement = connection.prepare("SELECT id FROM recordings")?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };
    for id in ids {
        let mut segments = list_recording_segments(connection, &id)?;
        for index in 0..segments.len() {
            if segments[index].timeline_base_ms.is_some() {
                continue;
            }
            // Missing finalized media means old offsets may have skipped it.
            if segments[..=index].iter().any(|segment| {
                segment.state == "finalized" && !std::path::Path::new(&segment.file_path).is_file()
            }) {
                break;
            }
            let Some(base) = next_timeline_base(
                &segments[..index],
                segments[index].segment_index,
                segments[index].started_at_ms,
            ) else {
                break;
            };
            let duration = segments[index].duration_ms.unwrap_or(0);
            let contradictory: bool = connection.query_row("SELECT EXISTS(SELECT 1 FROM transcript_segments WHERE segment_id=?1 AND (start_offset_ms<?2 OR end_offset_ms>?3)) OR EXISTS(SELECT 1 FROM ocr_frame_occurrences WHERE segment_id=?1 AND (frame_timestamp_ms<?2 OR sample_end_ms>?3)) OR EXISTS(SELECT 1 FROM processing_chunks WHERE segment_id=?1 AND (start_offset_ms<?2 OR end_offset_ms>?3))", params![segments[index].id, base, base.saturating_add(duration)], |row| row.get(0))?;
            if contradictory {
                break;
            }
            connection.execute("UPDATE recording_segments SET timeline_base_ms=?2,timeline_basis_version=1 WHERE id=?1 AND timeline_base_ms IS NULL", params![segments[index].id, base])?;
            segments[index].timeline_base_ms = Some(base);
            segments[index].timeline_basis_version = Some(1);
        }
    }
    Ok(())
}
