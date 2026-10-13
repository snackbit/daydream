use rusqlite::{params, Connection};

use super::{rows::activity_interval_row, types::*};

pub fn create_activity_interval(
    connection: &Connection,
    input: &CreateActivityIntervalInput,
) -> rusqlite::Result<()> {
    connection.execute(
        r#"
        INSERT INTO frame_activity_intervals (
            id,
            recording_id,
            source_kind,
            state,
            start_offset_ms,
            end_offset_ms,
            score,
            regions_json,
            job_id,
            chunk_id
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
        "#,
        params![
            input.id,
            input.recording_id,
            input.source_kind,
            input.state,
            input.start_offset_ms,
            input.end_offset_ms,
            input.score,
            input.regions_json,
            input.job_id,
            input.chunk_id,
        ],
    )?;

    Ok(())
}

pub fn list_activity_intervals(
    connection: &Connection,
    recording_id: &str,
) -> rusqlite::Result<Vec<ActivityIntervalRow>> {
    list_activity_intervals_in_range(connection, recording_id, None)
}

pub fn list_activity_intervals_in_range(
    connection: &Connection,
    recording_id: &str,
    range: Option<(i64, i64)>,
) -> rusqlite::Result<Vec<ActivityIntervalRow>> {
    list_activity_intervals_in_windows(connection, recording_id, range, None)
}

pub fn list_activity_intervals_in_windows(
    connection: &Connection,
    recording_id: &str,
    range: Option<(i64, i64)>,
    review: Option<(i64, i64)>,
) -> rusqlite::Result<Vec<ActivityIntervalRow>> {
    let mut statement = connection.prepare(
        r#"
        SELECT
            id,
            recording_id,
            source_kind,
            state,
            start_offset_ms,
            end_offset_ms,
            score,
            regions_json,
            job_id,
            chunk_id
        FROM frame_activity_intervals
        WHERE recording_id = ?1
          AND (?2 IS NULL OR (end_offset_ms>?2 AND start_offset_ms<?3)
               OR (?4 IS NOT NULL AND end_offset_ms>?4 AND start_offset_ms<?5))
        ORDER BY source_kind ASC, start_offset_ms ASC, end_offset_ms ASC, id ASC
        "#,
    )?;
    let rows = statement.query_map(
        params![
            recording_id,
            range.map(|r| r.0),
            range.map(|r| r.1),
            review.map(|r| r.0),
            review.map(|r| r.1)
        ],
        activity_interval_row,
    )?;

    rows.collect()
}
