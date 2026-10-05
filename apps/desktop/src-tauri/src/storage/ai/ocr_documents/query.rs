use super::{
    read::{display_occurrence_cte, CURRENT_OCR_VISIBLE},
    types::OcrContentInput,
};
use crate::ai::{
    GetOcrDocumentInput, GetOcrHistoryInput, OcrDocument, OcrDocumentSelection, OcrHistoryCursor,
    OcrHistoryEntry, OcrHistoryPage,
};
use crate::storage;
use rusqlite::{params, Connection, OptionalExtension};

const RUNS: &str = r#"
, previous AS (
 SELECT *, LAG(status) OVER w AS previous_status,LAG(normalized_text) OVER w AS previous_text,
 LAG(sample_end_ms) OVER w AS previous_end,LAG(segment_id) OVER w AS previous_segment,
 LAG(config_fingerprint) OVER w AS previous_config,LAG(provenance) OVER w AS previous_provenance
 FROM current_visible WINDOW w AS (PARTITION BY source_track_id ORDER BY frame_timestamp_ms,id)
), boundaries AS (
 SELECT *, CASE WHEN status='succeeded' AND previous_status='succeeded' AND normalized_text!=''
 AND normalized_text=previous_text AND frame_timestamp_ms=previous_end AND segment_id=previous_segment
 AND config_fingerprint=previous_config AND provenance='native' AND previous_provenance='native' THEN 0 ELSE 1 END AS boundary FROM previous
), numbered AS (
 SELECT *, SUM(boundary) OVER(PARTITION BY source_track_id ORDER BY frame_timestamp_ms,id ROWS UNBOUNDED PRECEDING) AS run_id FROM boundaries
), grouped AS (
 SELECT source_track_id,run_id,MIN(frame_timestamp_ms) AS observed_start_ms,MAX(frame_timestamp_ms) AS observed_end_ms,
 COUNT(*) AS occurrence_count,json_group_array(frame_timestamp_ms) AS support_timestamps_json,MAX(cache_hit) AS any_cache_hit FROM numbered GROUP BY source_track_id,run_id
)
"#;

const ENTRY_COLUMNS: &str = "n.id,n.recording_id,n.segment_id,n.source_track_id,n.frame_timestamp_ms,n.source_frame_ref,n.width,n.height,n.engine,n.provider,n.model,n.config_fingerprint,n.text,n.provenance,n.status,n.error_code,g.observed_start_ms,g.observed_end_ms,g.occurrence_count,g.support_timestamps_json,g.any_cache_hit";

pub fn get_ocr_history(
    connection: &Connection,
    input: &GetOcrHistoryInput,
) -> rusqlite::Result<OcrHistoryPage> {
    let limit = input.limit.unwrap_or(100).clamp(1, 200);
    let visible = display_occurrence_cte();
    let sql=format!("{visible}{CURRENT_OCR_VISIBLE}{RUNS} SELECT {ENTRY_COLUMNS} FROM numbered n JOIN grouped g ON g.source_track_id=n.source_track_id AND g.run_id=n.run_id WHERE n.frame_timestamp_ms=g.observed_start_ms AND (?3 IS NULL OR g.observed_start_ms<?3 OR (g.observed_start_ms=?3 AND n.id<?4)) ORDER BY g.observed_start_ms DESC,n.id DESC LIMIT ?5");
    let mut statement = connection.prepare(&sql)?;
    let mut items = statement
        .query_map(
            params![
                input.recording_id,
                input.source_track_id,
                input.cursor.as_ref().map(|cursor| cursor.timestamp_ms),
                input.cursor.as_ref().map(|cursor| cursor.id.as_str()),
                (limit + 1) as i64
            ],
            entry_row,
        )?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let next_cursor = if items.len() > limit {
        items.truncate(limit);
        items.last().map(|entry| OcrHistoryCursor {
            timestamp_ms: entry.observed_start_ms,
            id: entry.id.clone(),
        })
    } else {
        None
    };
    Ok(OcrHistoryPage { items, next_cursor })
}

pub fn get_ocr_document(
    connection: &Connection,
    input: &GetOcrDocumentInput,
) -> rusqlite::Result<OcrDocumentSelection> {
    let selected_columns = ENTRY_COLUMNS.replace("g.any_cache_hit", "n.cache_hit");
    let visible = display_occurrence_cte();
    let sql=format!("{visible}{CURRENT_OCR_VISIBLE}{RUNS} SELECT {selected_columns},n.content_id,c.document_json,n.raw_links_json FROM numbered n JOIN grouped g ON g.source_track_id=n.source_track_id AND g.run_id=n.run_id LEFT JOIN ocr_document_contents c ON c.id=n.content_id WHERE (?3 IS NULL AND n.frame_timestamp_ms<=?4 AND n.sample_end_ms>?4) OR n.id=?3 ORDER BY n.frame_timestamp_ms DESC,n.id DESC LIMIT 1");
    let document = connection
        .query_row(
            &sql,
            params![
                input.recording_id,
                input.source_track_id,
                input.document_id,
                input.timestamp_ms
            ],
            |row| {
                let entry = entry_row(row)?;
                let content_id = row.get(21)?;
                let json: Option<String> = row.get(22)?;
                let links_json: String = row.get(23)?;
                let links: std::collections::HashMap<String, String> =
                    serde_json::from_str(&links_json).map_err(|_| rusqlite::Error::InvalidQuery)?;
                let mut blocks = match json {
                    Some(json) => {
                        serde_json::from_str::<OcrContentInput>(&json)
                            .map_err(|_| rusqlite::Error::InvalidQuery)?
                            .blocks
                    }
                    None => vec![],
                };
                for block in &mut blocks {
                    block.raw_region_ids.clear();
                    for word in &mut block.words {
                        word.raw_region_id = links.get(&word.id).cloned();
                        if let Some(id) = &word.raw_region_id {
                            block.raw_region_ids.push(id.clone());
                        }
                    }
                }
                Ok(OcrDocument {
                    entry,
                    content_id,
                    blocks,
                })
            },
        )
        .optional()?;
    Ok(OcrDocumentSelection {
        recording_id: input.recording_id.clone(),
        source_track_id: input.source_track_id.clone(),
        document,
        job: selected_ocr_job(connection, input)?,
    })
}

fn selected_ocr_job(
    connection: &Connection,
    input: &GetOcrDocumentInput,
) -> rusqlite::Result<Option<crate::ai::ProcessingJob>> {
    let sql = r#"SELECT j.id FROM processing_jobs j JOIN processing_chunks p ON p.job_id=j.id JOIN recording_segments s ON s.id=p.segment_id
 WHERE j.recording_id=?1 AND p.source_kind='ocr' AND ('video-'||s.video_stream_index)=?2 AND p.start_offset_ms<=?3 AND p.end_offset_ms>?3
 AND NOT EXISTS(SELECT 1 FROM recording_protection_spans ps WHERE ps.recording_id=j.recording_id AND ps.status!='materialized')
 AND NOT EXISTS(SELECT 1 FROM redactions r WHERE r.recording_id=j.recording_id AND r.start_offset_ms<=?3 AND r.end_offset_ms>?3
 AND (json_type(r.track_ids_json)!='array' OR json_array_length(r.track_ids_json)=0 OR EXISTS(SELECT 1 FROM json_each(r.track_ids_json) invalid WHERE invalid.type!='text') OR EXISTS(SELECT 1 FROM json_each(r.track_ids_json) t WHERE t.value=?2 OR t.value=p.track_id)))
 ORDER BY j.created_at_ms DESC,j.id DESC LIMIT 1"#;
    let id: Option<String> = connection
        .query_row(
            sql,
            params![
                input.recording_id,
                input.source_track_id,
                input.timestamp_ms
            ],
            |row| row.get(0),
        )
        .optional()?;
    let Some(id) = id else {
        return Ok(None);
    };
    let Some(row) = storage::get_processing_job(connection, &id)? else {
        return Ok(None);
    };
    let (total, succeeded, average) = storage::processing_job_chunk_progress(connection, &id)?;
    let mut job = crate::ai::processing_job_from_row(row);
    if total > 0 {
        job.progress = if total == succeeded { 1.0 } else { average };
    }
    Ok(Some(job))
}

fn entry_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<OcrHistoryEntry> {
    let timestamps: String = row.get(19)?;
    Ok(OcrHistoryEntry {
        id: row.get(0)?,
        recording_id: row.get(1)?,
        segment_id: row.get(2)?,
        source_track_id: row.get(3)?,
        frame_timestamp_ms: row.get(4)?,
        source_frame_ref: row.get(5)?,
        width: positive_dimension(row.get(6)?),
        height: positive_dimension(row.get(7)?),
        engine: row.get(8)?,
        provider: row.get(9)?,
        model: row.get(10)?,
        config_fingerprint: row.get(11)?,
        text: row.get(12)?,
        provenance: row.get(13)?,
        status: row.get(14)?,
        error_code: row.get(15)?,
        observed_start_ms: row.get(16)?,
        observed_end_ms: row.get(17)?,
        occurrence_count: row
            .get::<_, i64>(18)?
            .try_into()
            .map_err(|_| rusqlite::Error::InvalidQuery)?,
        support_timestamps_ms: serde_json::from_str(&timestamps)
            .map_err(|_| rusqlite::Error::InvalidQuery)?,
        cache_hit: row.get(20)?,
    })
}

fn positive_dimension(value: i64) -> Option<i64> {
    (value > 0).then_some(value)
}
