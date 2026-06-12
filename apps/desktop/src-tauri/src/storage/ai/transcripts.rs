use rusqlite::{params, Connection, OptionalExtension};

use super::{
    rows::{transcript_segment_row, transcript_word_row},
    types::*,
};

pub fn create_transcript_segment(
    connection: &Connection,
    input: &CreateTranscriptSegmentInput,
) -> rusqlite::Result<TranscriptSegmentRow> {
    connection.execute(
        r#"
        INSERT INTO transcript_segments (
            id,
            recording_id,
            segment_id,
            track_id,
            source_role,
            start_offset_ms,
            end_offset_ms,
            provider,
            model,
            text,
            confidence,
            job_id,
            chunk_id,
            created_at_ms
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
        "#,
        params![
            input.id,
            input.recording_id,
            input.segment_id,
            input.track_id,
            input.source_role,
            input.start_offset_ms,
            input.end_offset_ms,
            input.provider,
            input.model,
            input.text,
            input.confidence,
            input.job_id,
            input.chunk_id,
            input.created_at_ms,
        ],
    )?;

    get_transcript_segment(connection, &input.id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)
}

pub fn get_transcript_segment(
    connection: &Connection,
    id: &str,
) -> rusqlite::Result<Option<TranscriptSegmentRow>> {
    connection
        .query_row(
            r#"
            SELECT
                id,
                recording_id,
                segment_id,
                track_id,
                source_role,
                start_offset_ms,
                end_offset_ms,
                provider,
                model,
                text,
                confidence,
                job_id,
                chunk_id,
                created_at_ms
            FROM transcript_segments
            WHERE id = ?1
            "#,
            [id],
            transcript_segment_row,
        )
        .optional()
}

pub fn list_transcript_segments(
    connection: &Connection,
    recording_id: &str,
) -> rusqlite::Result<Vec<TranscriptSegmentRow>> {
    let mut statement = connection.prepare(
        r#"
        SELECT
            id,
            recording_id,
            segment_id,
            track_id,
            source_role,
            start_offset_ms,
            end_offset_ms,
            provider,
            model,
            text,
            confidence,
            job_id,
            chunk_id,
            created_at_ms
        FROM transcript_segments
        WHERE recording_id = ?1
        ORDER BY track_id ASC, start_offset_ms ASC, end_offset_ms ASC, id ASC
        "#,
    )?;
    let rows = statement.query_map([recording_id], transcript_segment_row)?;

    rows.collect()
}

pub fn create_transcript_word(
    connection: &Connection,
    input: &CreateTranscriptWordInput,
) -> rusqlite::Result<()> {
    connection.execute(
        r#"
        INSERT INTO transcript_words (
            id,
            transcript_segment_id,
            start_offset_ms,
            end_offset_ms,
            text,
            probability
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        "#,
        params![
            input.id,
            input.transcript_segment_id,
            input.start_offset_ms,
            input.end_offset_ms,
            input.text,
            input.probability,
        ],
    )?;

    Ok(())
}

pub fn list_transcript_words(
    connection: &Connection,
    transcript_segment_id: &str,
) -> rusqlite::Result<Vec<TranscriptWordRow>> {
    let mut statement = connection.prepare(
        r#"
        SELECT
            id,
            transcript_segment_id,
            start_offset_ms,
            end_offset_ms,
            text,
            probability
        FROM transcript_words
        WHERE transcript_segment_id = ?1
        ORDER BY start_offset_ms ASC, end_offset_ms ASC, id ASC
        "#,
    )?;
    let rows = statement.query_map([transcript_segment_id], transcript_word_row)?;

    rows.collect()
}
