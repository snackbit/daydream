use rusqlite::{params, Connection, OptionalExtension};

use super::{rows::recording_track_row, types::*};

pub fn upsert_recording_track(
    connection: &Connection,
    input: &UpsertRecordingTrackInput,
) -> rusqlite::Result<RecordingTrackRow> {
    connection.execute(
        r#"
        INSERT INTO recording_tracks (
            id,
            recording_id,
            kind,
            label,
            source_id,
            path,
            codec,
            created_at_ms
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
        ON CONFLICT(id) DO UPDATE SET
            label = excluded.label,
            source_id = excluded.source_id,
            path = excluded.path,
            codec = excluded.codec
        "#,
        params![
            input.id,
            input.recording_id,
            input.kind,
            input.label,
            input.source_id,
            input.path,
            input.codec,
            input.created_at_ms,
        ],
    )?;

    get_recording_track(connection, &input.id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)
}

pub fn get_recording_track(
    connection: &Connection,
    id: &str,
) -> rusqlite::Result<Option<RecordingTrackRow>> {
    connection
        .query_row(
            r#"
            SELECT
                id,
                recording_id,
                kind,
                label,
                source_id,
                path,
                codec,
                created_at_ms
            FROM recording_tracks
            WHERE id = ?1
            "#,
            [id],
            recording_track_row,
        )
        .optional()
}

pub fn get_recording_track_by_kind(
    connection: &Connection,
    recording_id: &str,
    kind: &str,
) -> rusqlite::Result<Option<RecordingTrackRow>> {
    connection
        .query_row(
            r#"
            SELECT
                id,
                recording_id,
                kind,
                label,
                source_id,
                path,
                codec,
                created_at_ms
            FROM recording_tracks
            WHERE recording_id = ?1
                AND kind = ?2
            ORDER BY created_at_ms ASC, id ASC
            LIMIT 1
            "#,
            params![recording_id, kind],
            recording_track_row,
        )
        .optional()
}

pub fn list_recording_tracks(
    connection: &Connection,
    recording_id: &str,
) -> rusqlite::Result<Vec<RecordingTrackRow>> {
    let mut statement = connection.prepare(
        r#"
        SELECT
            id,
            recording_id,
            kind,
            label,
            source_id,
            path,
            codec,
            created_at_ms
        FROM recording_tracks
        WHERE recording_id = ?1
        ORDER BY kind ASC, created_at_ms ASC, id ASC
        "#,
    )?;
    let rows = statement.query_map([recording_id], recording_track_row)?;

    rows.collect()
}

pub fn recording_has_redactions(
    connection: &Connection,
    recording_id: &str,
) -> rusqlite::Result<bool> {
    connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM redactions WHERE recording_id = ?1)",
        [recording_id],
        |row| row.get::<_, bool>(0),
    )
}
