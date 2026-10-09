use super::types::*;
use rusqlite::{params, Connection, OptionalExtension};

pub(super) fn display_occurrence_cte() -> String {
    eligible_occurrence_cte(
        "AND p.attempt_count=o.attempt_count AND p.status IN ('running','succeeded','failed','completed-with-errors')",
        true,
    )
}

pub(super) fn timeline_occurrence_cte() -> String {
    eligible_occurrence_cte(
        "AND p.attempt_count=o.attempt_count AND p.status IN ('running','succeeded','failed','completed-with-errors')",
        false,
    )
}

pub(super) const CURRENT_OCR_VISIBLE: &str = r#"
, current_visible AS (
 SELECT v.* FROM visible v JOIN processing_jobs current_job ON current_job.id=v.job_id
 WHERE NOT EXISTS(SELECT 1 FROM processing_chunks newer_chunk JOIN processing_jobs newer_job ON newer_job.id=newer_chunk.job_id
 WHERE newer_chunk.recording_id=v.recording_id AND newer_chunk.source_kind='ocr' AND newer_chunk.segment_id=v.segment_id
 AND newer_chunk.start_offset_ms<=v.frame_timestamp_ms AND newer_chunk.end_offset_ms>v.frame_timestamp_ms
 AND newer_job.status!='cancelled' AND (newer_job.created_at_ms>current_job.created_at_ms OR (newer_job.created_at_ms=current_job.created_at_ms AND newer_job.id>current_job.id)))
)
"#;

fn eligible_occurrence_cte(attempt_predicate: &str, include_text: bool) -> String {
    // A completed recognition remains a valid exact-content cache entry across
    // attempts. Display separately requires the current attempt; both paths
    // always apply current source redactions and protection before reading text.
    let text_columns = if include_text {
        ", COALESCE(c.text,'') AS text, COALESCE(c.normalized_text,'') AS normalized_text"
    } else {
        ""
    };
    let content_join = if include_text {
        "LEFT JOIN ocr_document_contents c ON c.id=o.content_id"
    } else {
        ""
    };
    format!(
        r#"
WITH visible AS (
 SELECT o.*{text_columns}
 FROM ocr_frame_occurrences o
 {content_join}
 JOIN processing_chunks p ON p.id=o.chunk_id
 WHERE o.recording_id=?1
 AND (?2 IS NULL OR o.source_track_id=?2)
 {attempt_predicate}
 AND NOT EXISTS(SELECT 1 FROM recording_protection_spans ps WHERE ps.recording_id=o.recording_id AND ps.status!='materialized')
 AND NOT EXISTS(SELECT 1 FROM redactions r WHERE r.recording_id=o.recording_id
   AND r.start_offset_ms<o.sample_end_ms AND r.end_offset_ms>o.frame_timestamp_ms
   AND (json_type(r.track_ids_json)!='array' OR json_array_length(r.track_ids_json)=0
    OR EXISTS(SELECT 1 FROM json_each(r.track_ids_json) invalid WHERE invalid.type!='text')
    OR EXISTS(SELECT 1 FROM json_each(r.track_ids_json) t WHERE t.value=o.source_track_id OR t.value=p.track_id)))
)
"#
    )
}

pub fn load_cached_ocr_document(
    connection: &Connection,
    recording_id: &str,
    source_track_id: &str,
    pixel_hash: &str,
    width: i64,
    height: i64,
    config_fingerprint: &str,
) -> rusqlite::Result<Option<OcrCachedContent>> {
    let eligible = eligible_occurrence_cte("", false);
    let sql = format!("{eligible} SELECT c.id,c.document_json FROM ocr_document_contents c WHERE c.recording_id=?1 AND c.source_track_id=?2 AND c.pixel_hash=?3 AND c.width=?4 AND c.height=?5 AND c.config_fingerprint=?6 AND EXISTS(SELECT 1 FROM visible v WHERE v.content_id=c.id AND v.status='succeeded' AND v.provenance='native') ORDER BY c.id DESC LIMIT 1");
    connection
        .query_row(
            &sql,
            params![
                recording_id,
                source_track_id,
                pixel_hash,
                width,
                height,
                config_fingerprint
            ],
            |row| {
                let json: String = row.get(1)?;
                let content =
                    serde_json::from_str(&json).map_err(|_| rusqlite::Error::InvalidQuery)?;
                Ok(OcrCachedContent {
                    content_id: row.get(0)?,
                    content,
                })
            },
        )
        .optional()
}

pub fn delete_ocr_documents_for_chunks(
    connection: &Connection,
    chunk_ids: &[String],
) -> rusqlite::Result<usize> {
    let mut deleted = 0;
    for id in chunk_ids {
        deleted +=
            connection.execute("DELETE FROM ocr_frame_occurrences WHERE chunk_id=?1", [id])?;
    }
    connection.execute("DELETE FROM ocr_document_contents WHERE NOT EXISTS(SELECT 1 FROM ocr_frame_occurrences o WHERE o.content_id=ocr_document_contents.id)",[])?;
    Ok(deleted)
}

/// Recall uses the same current-attempt, supersession and privacy rules as OCR
/// inspection; callers must not recreate these predicates independently.
pub fn recall_ocr_visibility_cte() -> String {
    format!("{}{CURRENT_OCR_VISIBLE}", display_occurrence_cte())
}
