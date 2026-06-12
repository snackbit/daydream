use rusqlite::{params, Connection, OptionalExtension};

use super::{
    jobs::update_processing_job_status,
    rows::{capture_event_row, processing_chunk_row},
    types::*,
};

pub fn create_processing_chunk(
    connection: &Connection,
    input: &CreateProcessingChunkInput,
) -> rusqlite::Result<ProcessingChunkRow> {
    connection.execute(
        r#"
        INSERT INTO processing_chunks (
            id,
            job_id,
            recording_id,
            source_kind,
            segment_id,
            start_offset_ms,
            end_offset_ms,
            status,
            attempt_count,
            progress,
            error_code,
            result_code,
            track_id,
            created_at_ms,
            updated_at_ms
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
        "#,
        params![
            input.id,
            input.job_id,
            input.recording_id,
            input.source_kind,
            input.segment_id,
            input.start_offset_ms,
            input.end_offset_ms,
            input.status,
            input.attempt_count,
            input.progress,
            input.error_code,
            input.result_code,
            input.track_id,
            input.created_at_ms,
            input.updated_at_ms,
        ],
    )?;

    get_processing_chunk(connection, &input.id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)
}

pub fn create_capture_event(
    connection: &Connection,
    input: &CreateCaptureEventInput,
) -> rusqlite::Result<()> {
    connection.execute(
        r#"
        INSERT INTO capture_events (
            id,
            recording_id,
            occurred_at_ms,
            offset_ms,
            kind,
            payload_json
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        "#,
        params![
            input.id,
            input.recording_id,
            input.occurred_at_ms,
            input.offset_ms,
            input.kind,
            input.payload_json,
        ],
    )?;

    Ok(())
}

pub fn list_capture_events(
    connection: &Connection,
    recording_id: &str,
    kinds: &[&str],
    start_offset_ms: i64,
    end_offset_ms: i64,
) -> rusqlite::Result<Vec<CaptureEventRow>> {
    if kinds.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders = (0..kinds.len())
        .map(|index| format!("?{}", index + 4))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        r#"
        SELECT
            id,
            recording_id,
            occurred_at_ms,
            offset_ms,
            kind,
            payload_json
        FROM capture_events
        WHERE recording_id = ?1
            AND offset_ms >= ?2
            AND offset_ms <= ?3
            AND kind IN ({placeholders})
        ORDER BY offset_ms ASC, kind ASC, id ASC
        "#
    );
    let mut params: Vec<&dyn rusqlite::ToSql> = Vec::with_capacity(kinds.len() + 3);
    params.push(&recording_id);
    params.push(&start_offset_ms);
    params.push(&end_offset_ms);
    for kind in kinds {
        params.push(kind);
    }
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map(params.as_slice(), capture_event_row)?;

    rows.collect()
}

pub fn get_processing_chunk(
    connection: &Connection,
    id: &str,
) -> rusqlite::Result<Option<ProcessingChunkRow>> {
    connection
        .query_row(
            r#"
            SELECT
                id,
                job_id,
                recording_id,
                source_kind,
                segment_id,
                start_offset_ms,
                end_offset_ms,
                status,
                attempt_count,
                progress,
                error_code,
                result_code,
                track_id,
                created_at_ms,
                updated_at_ms
            FROM processing_chunks
            WHERE id = ?1
            "#,
            [id],
            processing_chunk_row,
        )
        .optional()
}

pub fn list_processing_chunks(
    connection: &Connection,
    job_id: &str,
) -> rusqlite::Result<Vec<ProcessingChunkRow>> {
    let mut statement = connection.prepare(
        r#"
        SELECT
            id,
            job_id,
            recording_id,
            source_kind,
            segment_id,
            start_offset_ms,
            end_offset_ms,
            status,
            attempt_count,
            progress,
            error_code,
            result_code,
            track_id,
            created_at_ms,
            updated_at_ms
        FROM processing_chunks
        WHERE job_id = ?1
        ORDER BY start_offset_ms ASC, id ASC
        "#,
    )?;
    let rows = statement.query_map([job_id], processing_chunk_row)?;

    rows.collect()
}

pub fn list_processing_chunks_by_status(
    connection: &Connection,
    job_id: &str,
    statuses: &[&str],
    limit: Option<usize>,
) -> rusqlite::Result<Vec<ProcessingChunkRow>> {
    if statuses.is_empty() || limit == Some(0) {
        return Ok(Vec::new());
    }
    let placeholders = (0..statuses.len())
        .map(|index| format!("?{}", index + 2))
        .collect::<Vec<_>>()
        .join(", ");
    let limit_clause = limit
        .map(|_| format!(" LIMIT ?{}", statuses.len() + 2))
        .unwrap_or_default();
    let sql = format!(
        r#"
        SELECT
            id,
            job_id,
            recording_id,
            source_kind,
            segment_id,
            start_offset_ms,
            end_offset_ms,
            status,
            attempt_count,
            progress,
            error_code,
            result_code,
            track_id,
            created_at_ms,
            updated_at_ms
        FROM processing_chunks
        WHERE job_id = ?1
            AND status IN ({placeholders})
        ORDER BY start_offset_ms ASC, id ASC
        {limit_clause}
        "#,
    );
    let limit_i64 = limit.map(|value| i64::try_from(value).unwrap_or(i64::MAX));
    let mut params: Vec<&dyn rusqlite::ToSql> = Vec::with_capacity(statuses.len() + 2);
    params.push(&job_id);
    for status in statuses {
        params.push(status);
    }
    if let Some(value) = &limit_i64 {
        params.push(value);
    }
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map(params.as_slice(), processing_chunk_row)?;

    rows.collect()
}

pub fn claim_next_queued_processing_chunk(
    connection: &Connection,
    job_id: &str,
    progress: f64,
    timestamp_ms: i64,
) -> rusqlite::Result<Option<ProcessingChunkRow>> {
    let chunk = list_processing_chunks_by_status(connection, job_id, &["queued"], Some(1))?
        .into_iter()
        .next();
    let Some(chunk) = chunk else {
        return Ok(None);
    };

    let updated = connection.execute(
        r#"
        UPDATE processing_chunks
        SET status = 'running',
            progress = ?2,
            error_code = NULL,
            result_code = NULL,
            attempt_count = attempt_count + 1,
            updated_at_ms = ?3
        WHERE id = ?1
            AND status = 'queued'
        "#,
        params![&chunk.id, progress, timestamp_ms],
    )?;
    if updated == 0 {
        return Ok(None);
    }

    get_processing_chunk(connection, &chunk.id)
}

pub fn update_processing_chunk_status(
    connection: &Connection,
    id: &str,
    status: &str,
    progress: f64,
    error_code: Option<&str>,
    timestamp_ms: i64,
) -> rusqlite::Result<ProcessingChunkRow> {
    connection.execute(
        r#"
        UPDATE processing_chunks
        SET status = ?2,
            progress = ?3,
            error_code = ?4,
            result_code = NULL,
            attempt_count = CASE WHEN ?2 = 'running' THEN attempt_count + 1 ELSE attempt_count END,
            updated_at_ms = ?5
        WHERE id = ?1
        "#,
        params![id, status, progress, error_code, timestamp_ms],
    )?;

    get_processing_chunk(connection, id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)
}

pub fn update_processing_chunk_result_code(
    connection: &Connection,
    id: &str,
    result_code: Option<&str>,
    timestamp_ms: i64,
) -> rusqlite::Result<ProcessingChunkRow> {
    connection.execute(
        r#"
        UPDATE processing_chunks
        SET result_code = ?2,
            updated_at_ms = ?3
        WHERE id = ?1
        "#,
        params![id, result_code, timestamp_ms],
    )?;

    get_processing_chunk(connection, id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)
}

pub fn requeue_processing_chunks(
    connection: &Connection,
    job_id: &str,
    from_statuses: &[&str],
    timestamp_ms: i64,
) -> rusqlite::Result<usize> {
    if from_statuses.is_empty() {
        return Ok(0);
    }
    let placeholders = (0..from_statuses.len())
        .map(|index| format!("?{}", index + 3))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        r#"
        UPDATE processing_chunks
        SET status = 'queued',
            progress = 0,
            error_code = NULL,
            result_code = NULL,
            updated_at_ms = ?2
        WHERE job_id = ?1
            AND status IN ({placeholders})
        "#,
    );
    let mut params: Vec<&dyn rusqlite::ToSql> = Vec::with_capacity(from_statuses.len() + 2);
    params.push(&job_id);
    params.push(&timestamp_ms);
    for status in from_statuses {
        params.push(status);
    }

    connection.execute(&sql, params.as_slice())
}

pub fn cancel_processing_chunks(
    connection: &Connection,
    job_id: &str,
    from_statuses: &[&str],
    timestamp_ms: i64,
) -> rusqlite::Result<usize> {
    if from_statuses.is_empty() {
        return Ok(0);
    }
    let placeholders = (0..from_statuses.len())
        .map(|index| format!("?{}", index + 3))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        r#"
        UPDATE processing_chunks
        SET status = 'cancelled',
            error_code = 'ai_processing_cancelled',
            updated_at_ms = ?2
        WHERE job_id = ?1
            AND status IN ({placeholders})
        "#,
    );
    let mut params: Vec<&dyn rusqlite::ToSql> = Vec::with_capacity(from_statuses.len() + 2);
    params.push(&job_id);
    params.push(&timestamp_ms);
    for status in from_statuses {
        params.push(status);
    }

    connection.execute(&sql, params.as_slice())
}

pub fn delete_transcript_output_for_chunks(
    connection: &Connection,
    chunk_ids: &[String],
) -> rusqlite::Result<usize> {
    if chunk_ids.is_empty() {
        return Ok(0);
    }
    let placeholders = (0..chunk_ids.len())
        .map(|index| format!("?{}", index + 1))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!("DELETE FROM transcript_segments WHERE chunk_id IN ({placeholders})");
    let params = chunk_ids
        .iter()
        .map(|id| id as &dyn rusqlite::ToSql)
        .collect::<Vec<_>>();

    connection.execute(&sql, params.as_slice())
}

pub fn delete_activity_output_for_chunks(
    connection: &Connection,
    chunk_ids: &[String],
) -> rusqlite::Result<usize> {
    if chunk_ids.is_empty() {
        return Ok(0);
    }
    let placeholders = (0..chunk_ids.len())
        .map(|index| format!("?{}", index + 1))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!("DELETE FROM frame_activity_intervals WHERE chunk_id IN ({placeholders})");
    let params = chunk_ids
        .iter()
        .map(|id| id as &dyn rusqlite::ToSql)
        .collect::<Vec<_>>();

    connection.execute(&sql, params.as_slice())
}

pub fn delete_visual_output_for_chunks(
    connection: &Connection,
    chunk_ids: &[String],
) -> rusqlite::Result<usize> {
    if chunk_ids.is_empty() {
        return Ok(0);
    }
    let placeholders = (0..chunk_ids.len())
        .map(|index| format!("?{}", index + 1))
        .collect::<Vec<_>>()
        .join(", ");
    let params = chunk_ids
        .iter()
        .map(|id| id as &dyn rusqlite::ToSql)
        .collect::<Vec<_>>();
    let ocr_sql = format!("DELETE FROM ocr_regions WHERE chunk_id IN ({placeholders})");
    let vlm_sql = format!("DELETE FROM vlm_observations WHERE chunk_id IN ({placeholders})");
    let similarity_sql =
        format!("DELETE FROM frame_similarity_entries WHERE chunk_id IN ({placeholders})");
    let deleted = connection.execute(&ocr_sql, params.as_slice())?
        + connection.execute(&vlm_sql, params.as_slice())?
        + connection.execute(&similarity_sql, params.as_slice())?;

    Ok(deleted)
}

pub fn retry_processing_job_chunks(
    connection: &Connection,
    job_id: &str,
    timestamp_ms: i64,
) -> rusqlite::Result<ProcessingJobRow> {
    let chunks =
        list_processing_chunks_by_status(connection, job_id, &["failed", "cancelled"], None)?;
    let chunk_ids = chunks
        .iter()
        .map(|chunk| chunk.id.clone())
        .collect::<Vec<_>>();
    delete_transcript_output_for_chunks(connection, &chunk_ids)?;
    delete_activity_output_for_chunks(connection, &chunk_ids)?;
    delete_visual_output_for_chunks(connection, &chunk_ids)?;
    requeue_processing_chunks(connection, job_id, &["failed", "cancelled"], timestamp_ms)?;
    update_processing_job_status(connection, job_id, "queued", None, timestamp_ms)
}
