use super::read::{timeline_occurrence_cte, CURRENT_OCR_VISIBLE};
use crate::ai::{
    GetOcrTimelineFramesInput, OcrHistoryCursor, OcrTimelineFrame, OcrTimelineFramePage,
};
use rusqlite::{params, Connection};

pub fn get_ocr_timeline_frames(
    connection: &Connection,
    input: &GetOcrTimelineFramesInput,
) -> rusqlite::Result<OcrTimelineFramePage> {
    let eligible = timeline_occurrence_cte();
    let limit = input.limit.unwrap_or(1000).clamp(1, 5000);
    let sql=format!("{eligible}{CURRENT_OCR_VISIBLE} SELECT id,recording_id,segment_id,source_track_id,frame_timestamp_ms,source_frame_ref,status,error_code,sample_end_ms,CASE WHEN status='succeeded' THEN NOT EXISTS(SELECT 1 FROM ocr_document_contents c WHERE c.id=current_visible.content_id AND c.normalized_text!='') ELSE NULL END,CASE WHEN status='succeeded' THEN (SELECT substr(c.text,1,160) FROM ocr_document_contents c WHERE c.id=current_visible.content_id) ELSE NULL END FROM current_visible WHERE (?3 IS NULL OR frame_timestamp_ms>=?3) AND (?4 IS NULL OR frame_timestamp_ms<?4) AND (?5 IS NULL OR frame_timestamp_ms>?5 OR (frame_timestamp_ms=?5 AND id>?6)) ORDER BY frame_timestamp_ms,id LIMIT ?7");
    let mut statement = connection.prepare(&sql)?;
    let mut frames = statement
        .query_map(
            params![
                input.recording_id,
                input.source_track_id,
                input.start_timestamp_ms,
                input.end_timestamp_ms,
                input.cursor.as_ref().map(|cursor| cursor.timestamp_ms),
                input.cursor.as_ref().map(|cursor| cursor.id.as_str()),
                (limit + 1) as i64
            ],
            |row| {
                let id: String = row.get(0)?;
                let status: String = row.get(6)?;
                let ocr_output_count = i64::from(status == "succeeded");
                Ok(OcrTimelineFrame {
                    document_id: id.clone(),
                    id,
                    recording_id: row.get(1)?,
                    segment_id: row.get(2)?,
                    source_track_id: row.get(3)?,
                    frame_timestamp_ms: row.get(4)?,
                    source_frame_ref: row.get(5)?,
                    status,
                    error_code: row.get(7)?,
                    sample_end_ms: row.get(8)?,
                    text_empty: row.get(9)?,
                    text_preview: row.get(10)?,
                    ocr_output_count,
                })
            },
        )?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let next_cursor = if frames.len() > limit {
        frames.truncate(limit);
        frames.last().map(|frame| OcrHistoryCursor {
            timestamp_ms: frame.frame_timestamp_ms,
            id: frame.id.clone(),
        })
    } else {
        None
    };
    Ok(OcrTimelineFramePage {
        recording_id: input.recording_id.clone(),
        frames,
        next_cursor,
    })
}
