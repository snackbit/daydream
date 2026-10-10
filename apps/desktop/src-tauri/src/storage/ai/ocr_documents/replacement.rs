//! Preserve eligible coverage around a replacement before its occurrence is upserted.
use super::{
    read::{timeline_occurrence_cte, CURRENT_OCR_VISIBLE},
    types::OcrFrameInput,
};
use crate::storage::ProcessingChunkRow;
use rusqlite::{params, Connection};

pub(super) fn retain_unreplaced_intervals(
    connection: &Connection,
    chunk: &ProcessingChunkRow,
    input: &OcrFrameInput,
) -> rusqlite::Result<()> {
    // Check the original interval's visibility before splitting. A shorter
    // interval must not make a previously redacted source frame eligible.
    let sql = format!("{}{CURRENT_OCR_VISIBLE} SELECT id,frame_timestamp_ms,sample_end_ms FROM current_visible WHERE segment_id=?3 AND chunk_id!=?4 AND frame_timestamp_ms<?6 AND sample_end_ms>?5 ORDER BY frame_timestamp_ms DESC", timeline_occurrence_cte());
    let intervals = connection
        .prepare(&sql)?
        .query_map(
            params![
                chunk.recording_id,
                input.source_track_id,
                chunk.segment_id,
                chunk.id,
                input.frame_timestamp_ms,
                input.sample_end_ms
            ],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, i64>(2)?,
                ))
            },
        )?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    for (id, start, end) in intervals {
        if end > input.sample_end_ms {
            let retained_id = uuid::Uuid::now_v7().to_string();
            // The interval begins later, but its captured frame reference,
            // raw links, engine and original attempt retain their provenance.
            connection.execute("INSERT OR IGNORE INTO ocr_frame_occurrences(id,recording_id,segment_id,source_track_id,frame_timestamp_ms,sample_end_ms,source_frame_ref,width,height,engine,provider,model,config_fingerprint,status,error_code,content_id,cache_hit,provenance,job_id,chunk_id,attempt_count,raw_links_json) SELECT ?2,recording_id,segment_id,source_track_id,?3,sample_end_ms,source_frame_ref,width,height,engine,provider,model,config_fingerprint,status,error_code,content_id,cache_hit,provenance,job_id,chunk_id,attempt_count,raw_links_json FROM ocr_frame_occurrences WHERE id=?1",params![id,retained_id,input.sample_end_ms])?;
            // Successful frames remain visible after cancellation or retry.
            connection.execute("INSERT OR IGNORE INTO processing_replacement_frames(chunk_id,frame_ms) SELECT o.chunk_id,?2 FROM ocr_frame_occurrences o JOIN processing_replacement_frames rf ON rf.chunk_id=o.chunk_id AND rf.frame_ms=o.frame_timestamp_ms WHERE o.id=?1 AND EXISTS(SELECT 1 FROM ocr_frame_occurrences WHERE id=?3)",params![id,input.sample_end_ms,retained_id])?;
        }
        if start < input.frame_timestamp_ms {
            connection.execute(
                "UPDATE ocr_frame_occurrences SET sample_end_ms=?2 WHERE id=?1",
                params![id, input.frame_timestamp_ms],
            )?;
        }
    }
    Ok(())
}
