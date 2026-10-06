use rusqlite::{params, Connection, OptionalExtension};

pub fn processing_job_input_open(connection: &Connection, job_id: &str) -> rusqlite::Result<bool> {
    Ok(connection
        .query_row(
            "SELECT input_open FROM processing_job_runtime WHERE job_id=?1",
            [job_id],
            |row| row.get(0),
        )
        .optional()?
        .unwrap_or(false))
}

pub fn set_processing_job_runtime(
    connection: &Connection,
    job_id: &str,
    input_open: bool,
    config: &str,
) -> rusqlite::Result<()> {
    connection.execute("INSERT INTO processing_job_runtime(job_id,input_open,config_json) VALUES(?1,?2,?3) ON CONFLICT(job_id) DO UPDATE SET input_open=excluded.input_open", params![job_id,input_open,config])?;
    Ok(())
}

pub fn processing_job_config(
    connection: &Connection,
    job_id: &str,
) -> rusqlite::Result<Option<String>> {
    connection
        .query_row(
            "SELECT config_json FROM processing_job_runtime WHERE job_id=?1",
            [job_id],
            |row| row.get(0),
        )
        .optional()
        .map(Option::flatten)
}

pub fn processing_chunk_config(
    connection: &Connection,
    chunk_id: &str,
) -> rusqlite::Result<Option<String>> {
    connection
        .query_row(
            "SELECT config_json FROM processing_chunk_runtime WHERE chunk_id=?1",
            [chunk_id],
            |row| row.get(0),
        )
        .optional()
        .map(Option::flatten)
}

pub fn processing_chunk_selection(
    connection: &Connection,
    chunk_id: &str,
) -> rusqlite::Result<Option<String>> {
    connection
        .query_row(
            "SELECT selection_json FROM processing_chunk_runtime WHERE chunk_id=?1",
            [chunk_id],
            |row| row.get(0),
        )
        .optional()
        .map(Option::flatten)
}

pub fn set_processing_chunk_total(
    connection: &Connection,
    chunk_id: &str,
    total: i64,
) -> rusqlite::Result<()> {
    connection.execute("INSERT INTO processing_chunk_runtime(chunk_id,total_units) VALUES(?1,?2) ON CONFLICT(chunk_id) DO UPDATE SET total_units=excluded.total_units",params![chunk_id,total.max(0)])?;
    Ok(())
}

pub fn set_processing_chunk_phase(
    connection: &Connection,
    chunk_id: &str,
    phase: &str,
) -> rusqlite::Result<()> {
    connection.execute("INSERT INTO processing_chunk_runtime(chunk_id,phase) VALUES(?1,?2) ON CONFLICT(chunk_id) DO UPDATE SET phase=excluded.phase", params![chunk_id,phase])?;
    Ok(())
}

pub fn processing_chunk_retry_ready(
    connection: &Connection,
    chunk_id: &str,
    timestamp: i64,
) -> rusqlite::Result<bool> {
    Ok(connection.query_row("SELECT next_retry_at_ms IS NULL OR next_retry_at_ms<=?2 FROM processing_chunk_runtime WHERE chunk_id=?1", params![chunk_id,timestamp], |row| row.get(0)).optional()?.unwrap_or(true))
}

pub fn schedule_processing_chunk_retry(
    connection: &Connection,
    chunk: &super::super::ProcessingChunkRow,
    error: &str,
    limit: i64,
    timestamp: i64,
) -> rusqlite::Result<bool> {
    let count: i64 = connection
        .query_row(
            "SELECT retry_count FROM processing_chunk_runtime WHERE chunk_id=?1",
            [&chunk.id],
            |row| row.get(0),
        )
        .optional()?
        .unwrap_or(0);
    if count >= limit {
        return Ok(false);
    }
    let delay = if count == 0 { 2_000 } else { 10_000 };
    let changed = connection.execute("UPDATE processing_chunks SET status='queued',error_code=?2,updated_at_ms=?3 WHERE id=?1 AND status='running' AND attempt_count=?4 AND EXISTS(SELECT 1 FROM processing_jobs WHERE id=processing_chunks.job_id AND status='running')", params![chunk.id,error,timestamp,chunk.attempt_count])?;
    if changed == 0 {
        return Ok(false);
    }
    connection.execute("INSERT INTO processing_chunk_runtime(chunk_id,retry_count,next_retry_at_ms,phase) VALUES(?1,?2,?3,'retrying') ON CONFLICT(chunk_id) DO UPDATE SET retry_count=excluded.retry_count,next_retry_at_ms=excluded.next_retry_at_ms,phase='retrying'", params![chunk.id,count+1,timestamp+delay])?;
    Ok(true)
}

pub fn first_processing_chunk_failure(
    connection: &Connection,
    chunk_id: &str,
) -> rusqlite::Result<Option<String>> {
    connection.query_row("SELECT error_code FROM visual_frame_processing_states WHERE chunk_id=?1 AND status='failed' AND error_code IS NOT NULL UNION ALL SELECT r.error_code FROM processing_audio_ranges r WHERE r.chunk_id=?1 AND r.status='failed' AND r.error_code IS NOT NULL AND NOT EXISTS(SELECT 1 FROM processing_audio_ranges newer WHERE newer.chunk_id=r.chunk_id AND newer.start_ms=r.start_ms AND newer.end_ms=r.end_ms AND (newer.updated_at_ms>r.updated_at_ms OR (newer.updated_at_ms=r.updated_at_ms AND newer.rowid>r.rowid))) LIMIT 1", [chunk_id], |row| row.get(0)).optional()
}

pub fn audio_range_succeeded(
    connection: &Connection,
    chunk_id: &str,
    start: i64,
    end: i64,
    fingerprint: &str,
) -> rusqlite::Result<bool> {
    Ok(connection.query_row("SELECT status='succeeded' FROM processing_audio_ranges WHERE chunk_id=?1 AND start_ms=?2 AND end_ms=?3 AND config_fingerprint=?4", params![chunk_id,start,end,fingerprint], |row| row.get(0)).optional()?.unwrap_or(false))
}

pub fn save_audio_range_checkpoint(
    connection: &Connection,
    chunk_id: &str,
    start: i64,
    end: i64,
    fingerprint: &str,
    error: Option<&str>,
) -> rusqlite::Result<()> {
    connection.execute("INSERT INTO processing_audio_ranges(chunk_id,start_ms,end_ms,config_fingerprint,status,error_code,updated_at_ms) VALUES(?1,?2,?3,?4,?5,?6,?7) ON CONFLICT(chunk_id,start_ms,end_ms,config_fingerprint) DO UPDATE SET status=excluded.status,error_code=excluded.error_code,updated_at_ms=excluded.updated_at_ms", params![chunk_id,start,end,fingerprint,if error.is_some(){"failed"}else{"succeeded"},error,crate::background_tasks::now_ms()])?;
    Ok(())
}
