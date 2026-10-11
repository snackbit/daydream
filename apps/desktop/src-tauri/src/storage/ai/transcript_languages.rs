use super::reprocessing::replacement_superseded;
use super::*;
use rusqlite::{params, Connection};

pub const LANGUAGE_CONTEXT_GAP_MS: i64 = 15_000;
pub const LANGUAGE_CONTEXT_MIN_CONFIDENCE: f64 = 0.60;

#[derive(Clone, Debug)]
pub struct TranscriptLanguageContext {
    pub language: String,
    pub confidence: f64,
    pub detection_segment_id: Option<String>,
    pub config_fingerprint: String,
    pub detection_start_ms: i64,
    pub detection_end_ms: i64,
}

pub fn save_transcript_language_context(
    connection: &Connection,
    segment_id: &str,
    detection_id: &str,
    context: &TranscriptLanguageContext,
) -> rusqlite::Result<()> {
    connection.execute("INSERT INTO transcript_language_context(transcript_segment_id,detection_segment_id,language,confidence,config_fingerprint,detection_start_ms,detection_end_ms) VALUES(?1,?2,?3,?4,?5,?6,?7)", params![segment_id,detection_id,context.language,context.confidence,context.config_fingerprint,context.detection_start_ms,context.detection_end_ms])?;
    Ok(())
}

/// Only preceding, successful source text can renew a language hint. A source
/// window may extend slightly across a finalized fragment boundary.
pub fn preceding_transcript_language(
    connection: &Connection,
    chunk: &ProcessingChunkRow,
    track_id: &str,
    start_ms: i64,
    fingerprint: &str,
    provider: &str,
    model: &str,
) -> rusqlite::Result<Option<TranscriptLanguageContext>> {
    let mut statement = connection.prepare(r#"
SELECT l.language,l.confidence,l.detection_segment_id,l.detection_start_ms,l.detection_end_ms,t.start_offset_ms,t.end_offset_ms,t.chunk_id,d.chunk_id
FROM transcript_language_context l
JOIN transcript_segments t ON t.id=l.transcript_segment_id
JOIN transcript_segments d ON d.id=l.detection_segment_id
JOIN processing_chunks c ON c.id=t.chunk_id
WHERE t.recording_id=?1 AND t.track_id=?2 AND l.config_fingerprint=?3
 AND t.provider=?4 AND t.model=?5 AND t.start_offset_ms<?6
 AND t.end_offset_ms>=?6-?7 AND (t.end_offset_ms<=?6 OR c.end_offset_ms<=?6)
ORDER BY t.end_offset_ms DESC,t.start_offset_ms DESC,t.id DESC
"#)?;
    let candidates = statement.query_map(
        params![
            chunk.recording_id,
            track_id,
            fingerprint,
            provider,
            model,
            start_ms,
            LANGUAGE_CONTEXT_GAP_MS
        ],
        |row| {
            Ok((
                TranscriptLanguageContext {
                    language: row.get(0)?,
                    confidence: row.get(1)?,
                    detection_segment_id: Some(row.get(2)?),
                    detection_start_ms: row.get(3)?,
                    detection_end_ms: row.get(4)?,
                    config_fingerprint: fingerprint.into(),
                },
                row.get::<_, i64>(5)?,
                row.get::<_, i64>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, String>(8)?,
            ))
        },
    )?;
    let redactions = crate::storage::list_redactions(connection, &chunk.recording_id)?;
    let segments = crate::storage::list_recording_segments(connection, &chunk.recording_id)?;
    let bases = crate::storage::segment_playback_bases(&segments);
    let track = get_recording_track(connection, track_id)?.ok_or(rusqlite::Error::InvalidQuery)?;
    for candidate in candidates {
        let (context, start, end, owner, detection_owner) = candidate?;
        let mut eligible = true;
        for id in [owner, detection_owner] {
            let source =
                get_processing_chunk(connection, &id)?.ok_or(rusqlite::Error::InvalidQuery)?;
            if replacement_superseded(connection, &id)?
                || (replacement_pending(connection, &id)? && source.job_id != chunk.job_id)
            {
                eligible = false;
                break;
            }
        }
        let ranges = [
            (start, end),
            (context.detection_start_ms, context.detection_end_ms),
        ];
        let ids = [track_id.to_string()];
        let redacted = ranges.iter().any(|(start, end)| {
            crate::recording_privacy::range_redacted(&redactions, &ids, *start, *end)
        }) || segments.iter().zip(&bases).any(|(segment, base)| {
            let Some(alias) = crate::recording_privacy::source_track_id(segment, &track.kind)
            else {
                return false;
            };
            let ids = [alias];
            ranges.iter().any(|(start, end)| {
                let start = (*start).max(*base);
                let end = (*end).min(base.saturating_add(segment.duration_ms.unwrap_or_default()));
                start < end
                    && crate::recording_privacy::range_redacted(&redactions, &ids, start, end)
            })
        });
        if redacted {
            continue;
        }
        if !crate::recording_privacy::range_protection_ready(
            connection,
            &chunk.recording_id,
            context.detection_start_ms,
            context.detection_end_ms,
        )? || !crate::recording_privacy::range_protection_ready(
            connection,
            &chunk.recording_id,
            start,
            end,
        )? {
            continue;
        }
        if eligible {
            return Ok(Some(context));
        }
    }
    Ok(None)
}
