use super::types::*;
use crate::{
    recording_privacy,
    storage::{self, ProcessingChunkRow},
};
use rusqlite::{params, Connection, OptionalExtension};

pub fn normalize_ocr_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn save_processing_job_ocr_config(
    connection: &Connection,
    job_id: &str,
    config_json: &str,
) -> rusqlite::Result<()> {
    let _: serde_json::Value =
        serde_json::from_str(config_json).map_err(|_| rusqlite::Error::InvalidQuery)?;
    connection.execute("INSERT INTO processing_job_ocr_configs(job_id, config_json) VALUES (?1,?2) ON CONFLICT(job_id) DO NOTHING", params![job_id, config_json])?;
    Ok(())
}

pub fn get_processing_job_ocr_config(
    connection: &Connection,
    job_id: &str,
) -> rusqlite::Result<Option<String>> {
    connection
        .query_row(
            "SELECT config_json FROM processing_job_ocr_configs WHERE job_id=?1",
            [job_id],
            |row| row.get(0),
        )
        .optional()
}

pub fn save_ocr_frame_result(
    connection: &mut Connection,
    chunk: &ProcessingChunkRow,
    input: &OcrFrameInput,
) -> rusqlite::Result<bool> {
    validate_frame(input)?;
    let transaction = connection.transaction()?;
    if !recording_privacy::chunk_active(&transaction, chunk)? {
        return Ok(false);
    }
    let segment = storage::list_recording_segments(&transaction, &chunk.recording_id)?
        .into_iter()
        .find(|segment| Some(segment.id.as_str()) == chunk.segment_id.as_deref())
        .ok_or(rusqlite::Error::InvalidQuery)?;
    if recording_privacy::source_track_id(&segment, "video").as_deref()
        != Some(input.source_track_id.as_str())
        || input.frame_timestamp_ms < chunk.start_offset_ms.unwrap_or(i64::MAX)
        || input.sample_end_ms > chunk.end_offset_ms.unwrap_or(-1)
    {
        return Err(rusqlite::Error::InvalidQuery);
    }
    let (content_id, links) = if input.status == "succeeded" {
        let (id, links) = super::content::store_content(&transaction, chunk, input)?;
        (Some(id), links)
    } else {
        (None, serde_json::Map::new())
    };
    let existing = transaction.query_row("SELECT id FROM ocr_frame_occurrences WHERE recording_id=?1 AND segment_id=?2 AND source_track_id=?3 AND frame_timestamp_ms=?4",params![chunk.recording_id,segment.id,input.source_track_id,input.frame_timestamp_ms], |row| row.get::<_,String>(0)).optional()?;
    if let Some(id) = existing {
        transaction.execute("DELETE FROM ocr_regions WHERE id IN (SELECT value FROM json_each((SELECT raw_links_json FROM ocr_frame_occurrences WHERE id=?1)))", [&id])?;
    }
    for raw in &input.raw_regions {
        if raw.recording_id != chunk.recording_id
            || raw.chunk_id != chunk.id
            || raw.job_id != chunk.job_id
            || raw.frame_timestamp_ms != input.frame_timestamp_ms
        {
            return Err(rusqlite::Error::InvalidQuery);
        }
        storage::create_ocr_region(&transaction, raw)?;
    }
    transaction.execute("INSERT INTO ocr_frame_occurrences(id,recording_id,segment_id,source_track_id,frame_timestamp_ms,sample_end_ms,source_frame_ref,width,height,engine,provider,model,config_fingerprint,status,error_code,content_id,cache_hit,provenance,job_id,chunk_id,attempt_count,raw_links_json) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,'native',?18,?19,?20,?21) ON CONFLICT(recording_id,segment_id,source_track_id,frame_timestamp_ms) DO UPDATE SET id=excluded.id,sample_end_ms=excluded.sample_end_ms,source_frame_ref=excluded.source_frame_ref,width=excluded.width,height=excluded.height,engine=excluded.engine,provider=excluded.provider,model=excluded.model,config_fingerprint=excluded.config_fingerprint,status=excluded.status,error_code=excluded.error_code,content_id=excluded.content_id,cache_hit=excluded.cache_hit,provenance='native',job_id=excluded.job_id,chunk_id=excluded.chunk_id,attempt_count=excluded.attempt_count,raw_links_json=excluded.raw_links_json",params![input.id,chunk.recording_id,segment.id,input.source_track_id,input.frame_timestamp_ms,input.sample_end_ms,input.source_frame_ref,input.width,input.height,input.engine,input.provider,input.model,input.config_fingerprint,input.status,input.error_code,content_id,input.cache_content_id.is_some(),chunk.job_id,chunk.id,chunk.attempt_count,serde_json::Value::Object(links).to_string()])?;
    transaction.commit()?;
    Ok(true)
}

fn validate_frame(input: &OcrFrameInput) -> rusqlite::Result<()> {
    if input.width <= 0
        || input.height <= 0
        || input.frame_timestamp_ms < 0
        || input.sample_end_ms <= input.frame_timestamp_ms
        || !matches!(input.engine.as_str(), "tesseract" | "vision")
        || !matches!(input.status.as_str(), "succeeded" | "failed" | "gap")
        || input.config_fingerprint.is_empty()
        || input.pixel_hash.is_empty()
        || input.source_frame_ref.is_empty()
        || (input.status != "succeeded"
            && (input.content.is_some()
                || input.cache_content_id.is_some()
                || !input.raw_regions.is_empty()))
    {
        return Err(rusqlite::Error::InvalidQuery);
    }
    Ok(())
}
