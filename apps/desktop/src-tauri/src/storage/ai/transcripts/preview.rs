use rusqlite::{params, Connection};
pub struct TranscriptPreviewRow {
    pub id: String,
    pub segment_id: String,
    pub start_offset_ms: i64,
    pub end_offset_ms: i64,
    pub text: String,
    pub truncated: bool,
}

/// Apply publication and privacy eligibility before LIMIT. SQLite bounds both
/// row count and text size; words and whole-recording transcripts are not loaded.
pub fn transcript_preview_entries(
    connection: &Connection,
    recording_id: &str,
    source_role: &str,
    start: i64,
    end: i64,
) -> rusqlite::Result<Vec<TranscriptPreviewRow>> {
    let segments = crate::storage::list_recording_segments(connection, recording_id)?;
    let media = crate::timeline_lanes::media_track_ids_from_segments(&segments);
    let cuts = crate::storage::list_redactions(connection, recording_id)?.into_iter().map(|cut| serde_json::json!({
        "start": cut.start_offset_ms, "end": cut.end_offset_ms,
        "all": crate::redaction_targets::parse_redaction_track_ids(&cut.track_ids_json).unwrap_or_default().is_empty(),
        "targets": crate::redaction_targets::expanded_redaction_track_ids(&cut.track_ids_json, &media)
    })).collect::<Vec<_>>();
    let cuts_json = serde_json::to_string(&cuts).map_err(|_| rusqlite::Error::InvalidQuery)?;
    let mut statement = connection.prepare(r#"
        SELECT t.id,t.segment_id,t.start_offset_ms,t.end_offset_ms,substr(t.text,1,240),length(t.text)>240
        FROM published_transcript_segments t JOIN recording_segments s ON s.id=t.segment_id
        WHERE t.recording_id=?1 AND t.source_role=?2 AND t.start_offset_ms<?4 AND t.end_offset_ms>?3
        AND NOT EXISTS(SELECT 1 FROM json_each(?5) r
          WHERE json_extract(r.value,'$.start')<t.end_offset_ms AND json_extract(r.value,'$.end')>t.start_offset_ms
          AND (json_extract(r.value,'$.all') OR EXISTS(SELECT 1 FROM json_each(json_extract(r.value,'$.targets')) j
            WHERE j.value=t.track_id OR j.value=t.source_role||'-'||CASE WHEN t.source_role='microphone' THEN s.microphone_audio_stream_index ELSE s.desktop_audio_stream_index END)))
        ORDER BY t.start_offset_ms,t.end_offset_ms,t.id LIMIT 3
    "#)?;
    let rows = statement.query_map(
        params![recording_id, source_role, start, end, cuts_json],
        |row| {
            Ok(TranscriptPreviewRow {
                id: row.get(0)?,
                segment_id: row.get(1)?,
                start_offset_ms: row.get(2)?,
                end_offset_ms: row.get(3)?,
                text: row.get(4)?,
                truncated: row.get(5)?,
            })
        },
    )?;
    rows.collect()
}
