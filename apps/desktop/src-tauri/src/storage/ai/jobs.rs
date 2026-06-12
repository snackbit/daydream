use rusqlite::{params, Connection, OptionalExtension};

use super::{rows::processing_job_row, types::*};

pub fn create_processing_job(
    connection: &Connection,
    input: &CreateProcessingJobInput,
) -> rusqlite::Result<ProcessingJobRow> {
    connection.execute(
        r#"
        INSERT INTO processing_jobs (
            id,
            recording_id,
            kind,
            status,
            created_at_ms,
            updated_at_ms,
            error
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        "#,
        params![
            input.id,
            input.recording_id,
            input.kind,
            input.status,
            input.created_at_ms,
            input.updated_at_ms,
            input.error,
        ],
    )?;

    get_processing_job(connection, &input.id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)
}

pub fn list_processing_jobs(
    connection: &Connection,
    recording_id: Option<&str>,
) -> rusqlite::Result<Vec<ProcessingJobRow>> {
    let sql = r#"
        SELECT
            id,
            recording_id,
            kind,
            status,
            created_at_ms,
            updated_at_ms,
            error
        FROM processing_jobs
        WHERE (?1 IS NULL OR recording_id = ?1)
        ORDER BY updated_at_ms DESC, created_at_ms DESC
        "#;
    let mut statement = connection.prepare(sql)?;
    let rows = statement.query_map([recording_id], processing_job_row)?;

    rows.collect()
}

pub fn get_latest_processing_job_for_recording(
    connection: &Connection,
    recording_id: &str,
    kind: &str,
) -> rusqlite::Result<Option<ProcessingJobRow>> {
    connection
        .query_row(
            r#"
            SELECT
                id,
                recording_id,
                kind,
                status,
                created_at_ms,
                updated_at_ms,
                error
            FROM processing_jobs
            WHERE recording_id = ?1
                AND kind = ?2
            ORDER BY updated_at_ms DESC, created_at_ms DESC
            LIMIT 1
            "#,
            params![recording_id, kind],
            processing_job_row,
        )
        .optional()
}

pub fn list_processing_jobs_by_status(
    connection: &Connection,
    kind: &str,
    statuses: &[&str],
    limit: Option<usize>,
) -> rusqlite::Result<Vec<ProcessingJobRow>> {
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
            recording_id,
            kind,
            status,
            created_at_ms,
            updated_at_ms,
            error
        FROM processing_jobs
        WHERE kind = ?1
            AND status IN ({placeholders})
        ORDER BY updated_at_ms ASC, created_at_ms ASC
        {limit_clause}
        "#,
    );
    let limit_i64 = limit.map(|value| i64::try_from(value).unwrap_or(i64::MAX));
    let mut params: Vec<&dyn rusqlite::ToSql> = Vec::with_capacity(statuses.len() + 2);
    params.push(&kind);
    for status in statuses {
        params.push(status);
    }
    if let Some(value) = &limit_i64 {
        params.push(value);
    }
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map(params.as_slice(), processing_job_row)?;

    rows.collect()
}

pub fn get_processing_job(
    connection: &Connection,
    id: &str,
) -> rusqlite::Result<Option<ProcessingJobRow>> {
    connection
        .query_row(
            r#"
            SELECT
                id,
                recording_id,
                kind,
                status,
                created_at_ms,
                updated_at_ms,
                error
            FROM processing_jobs
            WHERE id = ?1
            "#,
            [id],
            processing_job_row,
        )
        .optional()
}

pub fn update_processing_job_status(
    connection: &Connection,
    id: &str,
    status: &str,
    error: Option<&str>,
    timestamp_ms: i64,
) -> rusqlite::Result<ProcessingJobRow> {
    connection.execute(
        r#"
        UPDATE processing_jobs
        SET status = ?2,
            error = ?3,
            updated_at_ms = ?4
        WHERE id = ?1
        "#,
        params![id, status, error, timestamp_ms],
    )?;

    get_processing_job(connection, id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)
}

pub fn processing_job_chunk_progress(
    connection: &Connection,
    job_id: &str,
) -> rusqlite::Result<(i64, i64, f64)> {
    connection.query_row(
        r#"
        SELECT
            COUNT(*),
            COALESCE(SUM(CASE WHEN status = 'succeeded' THEN 1 ELSE 0 END), 0),
            COALESCE(AVG(progress), 0)
        FROM processing_chunks
        WHERE job_id = ?1
        "#,
        [job_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )
}
