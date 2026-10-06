//! Whole-job retry retains durable successful units and resets retry scope.
use super::{
    chunks::{
        delete_activity_output_for_chunks, delete_transcript_output_for_chunks,
        list_processing_chunks_by_status, requeue_processing_chunks,
    },
    jobs::update_processing_job_status,
    types::ProcessingJobRow,
};
use rusqlite::Connection;

pub fn retry_processing_job_chunks(
    connection: &Connection,
    job_id: &str,
    timestamp_ms: i64,
) -> rusqlite::Result<ProcessingJobRow> {
    let chunks = list_processing_chunks_by_status(
        connection,
        job_id,
        &["failed", "cancelled", "completed-with-errors"],
        None,
    )?;
    let chunk_ids = chunks
        .iter()
        .map(|chunk| chunk.id.clone())
        .collect::<Vec<_>>();
    let legacy_chunks = chunk_ids
        .iter()
        .filter_map(|id| {
            connection
                .query_row(
                    "SELECT COUNT(*) FROM processing_audio_ranges WHERE chunk_id=?1",
                    [id],
                    |row| row.get::<_, i64>(0),
                )
                .map(|count| (count == 0).then_some(id.clone()))
                .transpose()
        })
        .collect::<rusqlite::Result<Vec<_>>>()?;
    delete_transcript_output_for_chunks(connection, &legacy_chunks)?;
    delete_activity_output_for_chunks(connection, &chunk_ids)?;
    prune_incomplete_visual_output(connection, &chunk_ids)?;
    requeue_processing_chunks(
        connection,
        job_id,
        &["failed", "cancelled", "completed-with-errors"],
        timestamp_ms,
    )?;
    connection.execute("UPDATE processing_chunk_runtime SET selection_json=NULL,retry_count=0,next_retry_at_ms=NULL,phase='queued' WHERE chunk_id IN (SELECT id FROM processing_chunks WHERE job_id=?1 AND status='queued')", [job_id])?;
    update_processing_job_status(connection, job_id, "queued", None, timestamp_ms)
}

fn prune_incomplete_visual_output(
    connection: &Connection,
    chunk_ids: &[String],
) -> rusqlite::Result<()> {
    // Raw legacy output without a successful checkpoint remains attempt-owned.
    // Committed frames and their failure states stay durable across retries.
    for chunk in chunk_ids {
        for table in [
            "ocr_regions",
            "vlm_observations",
            "frame_similarity_entries",
        ] {
            connection.execute(&format!("DELETE FROM {table} AS output WHERE output.chunk_id=?1 AND NOT EXISTS(SELECT 1 FROM visual_frame_processing_states frame WHERE frame.chunk_id=output.chunk_id AND frame.source_frame_ref=output.source_frame_ref AND frame.status='succeeded')"),[chunk])?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
