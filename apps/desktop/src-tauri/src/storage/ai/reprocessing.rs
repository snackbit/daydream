//! Durable publication keeps previous results until a replacement unit succeeds.
use super::*;
use rusqlite::{params, Connection, OptionalExtension};
type ReplacementGroup = (String, String, String, Option<String>, String, i64, i64);

pub fn replacement_pending(connection: &Connection, chunk_id: &str) -> rusqlite::Result<bool> {
    connection.query_row("SELECT EXISTS(SELECT 1 FROM processing_replacement_chunks rc JOIN processing_replacement_groups rg ON rg.id=rc.group_id WHERE rc.chunk_id=?1 AND rg.committed=0)",[chunk_id],|row|row.get(0))
}

/// Each successful sampled frame replaces only its source and sampling window.
/// Failed or cancelled frames never call this function.
pub fn publish_replacement_frame(
    connection: &Connection,
    chunk: &ProcessingChunkRow,
    source: &str,
    start: i64,
    end: i64,
) -> rusqlite::Result<()> {
    if !replacement_pending(connection, &chunk.id)? {
        return Ok(());
    }
    if replacement_superseded(connection, &chunk.id)? {
        return Err(rusqlite::Error::InvalidQuery);
    }
    if chunk.source_kind == "ocr" {
        connection.execute("DELETE FROM ocr_regions AS region WHERE id IN (SELECT value FROM ocr_frame_occurrences o,json_each(o.raw_links_json) WHERE o.recording_id=?1 AND o.source_track_id=?2 AND o.frame_timestamp_ms>=?3 AND o.frame_timestamp_ms<?4 AND o.chunk_id!=?5) AND NOT EXISTS(SELECT 1 FROM ocr_frame_occurrences retained,json_each(retained.raw_links_json) link WHERE link.value=region.id AND NOT(retained.recording_id=?1 AND retained.source_track_id=?2 AND retained.frame_timestamp_ms>=?3 AND retained.frame_timestamp_ms<?4 AND retained.chunk_id!=?5))",params![chunk.recording_id,source,start,end,chunk.id])?;
        connection.execute("DELETE FROM ocr_frame_occurrences WHERE recording_id=?1 AND source_track_id=?2 AND frame_timestamp_ms>=?3 AND frame_timestamp_ms<?4 AND chunk_id!=?5",params![chunk.recording_id,source,start,end,chunk.id])?;
    } else if chunk.source_kind == "vlm" {
        connection.execute("DELETE FROM vlm_observations WHERE recording_id=?1 AND frame_timestamp_ms>=?2 AND frame_timestamp_ms<?3 AND chunk_id!=?4 AND chunk_id IN (SELECT id FROM processing_chunks WHERE track_id=?5 OR (track_id IS NULL AND ?5 LIKE 'video-%'))",params![chunk.recording_id,start,end,chunk.id,source])?;
    } else {
        return Err(rusqlite::Error::InvalidQuery);
    }
    connection.execute(
        "INSERT OR IGNORE INTO processing_replacement_frames(chunk_id,frame_ms) VALUES(?1,?2)",
        params![chunk.id, start],
    )?;
    Ok(())
}

/// The caller owns a transaction containing the final successful child status.
/// A cross-fragment utterance is published only after all its children succeed.
pub fn publish_ready_replacement(connection: &Connection, chunk_id: &str) -> rusqlite::Result<()> {
    let group:Option<ReplacementGroup>=connection.query_row("SELECT rg.id,rg.recording_id,rg.source_track_id,rg.source_role,rg.source_kind,rg.start_ms,rg.end_ms FROM processing_replacement_groups rg JOIN processing_replacement_chunks rc ON rc.group_id=rg.id WHERE rc.chunk_id=?1 AND rg.committed=0",[chunk_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?))).optional()?;
    let Some((id, recording, source, role, kind, start, end)) = group else {
        return Ok(());
    };
    let incomplete:bool=connection.query_row("SELECT EXISTS(SELECT 1 FROM processing_replacement_chunks rc JOIN processing_chunks p ON p.id=rc.chunk_id WHERE rc.group_id=?1 AND p.status!='succeeded')",[&id],|r|r.get(0))?;
    if incomplete {
        return Ok(());
    }
    if replacement_superseded(connection, chunk_id)? {
        return Err(rusqlite::Error::InvalidQuery);
    }
    if !crate::recording_privacy::protection_ready(connection, &recording)? {
        return Err(rusqlite::Error::InvalidQuery);
    }
    let redacted:bool=connection.query_row("SELECT EXISTS(SELECT 1 FROM redactions WHERE recording_id=?1 AND start_offset_ms<?3 AND end_offset_ms>?2 AND (json_type(track_ids_json)!='array' OR json_array_length(track_ids_json)=0 OR EXISTS(SELECT 1 FROM json_each(track_ids_json) WHERE type!='text' OR value=?4)))",params![recording,start,end,source],|r|r.get(0))?;
    if redacted {
        return Err(rusqlite::Error::InvalidQuery);
    }
    if kind == "audio" {
        let canonical:bool=connection.query_row("SELECT EXISTS(SELECT 1 FROM processing_speech_unit_chunks uc JOIN processing_replacement_chunks rc ON rc.chunk_id=uc.chunk_id WHERE rc.group_id=?1)",[&id],|row|row.get(0))?;
        if canonical {
            let pending:bool=connection.query_row("SELECT EXISTS(SELECT 1 FROM processing_speech_units u JOIN processing_speech_unit_chunks uc ON uc.unit_id=u.id JOIN processing_replacement_chunks rc ON rc.chunk_id=uc.chunk_id WHERE rc.group_id=?1 AND u.status!='published')",[&id],|row|row.get(0))?;
            if pending {
                return Ok(());
            }
            connection.execute(
                "UPDATE processing_replacement_groups SET committed=1 WHERE id=?1",
                [id],
            )?;
            return Ok(());
        }
        connection.execute("DELETE FROM transcript_segments WHERE recording_id=?1 AND source_role=?2 AND start_offset_ms<?4 AND end_offset_ms>?3 AND chunk_id NOT IN (SELECT chunk_id FROM processing_replacement_chunks WHERE group_id=?5)",params![recording,role,start,end,id])?;
    }
    connection.execute(
        "UPDATE processing_replacement_groups SET committed=1 WHERE id=?1",
        [id],
    )?;
    Ok(())
}

pub(super) fn replacement_superseded(
    connection: &Connection,
    chunk_id: &str,
) -> rusqlite::Result<bool> {
    connection.query_row(
        r#"
        SELECT EXISTS(
          SELECT 1 FROM processing_replacement_chunks rc
          JOIN processing_replacement_groups old ON old.id=rc.group_id
          JOIN processing_jobs current ON current.id=old.job_id
          JOIN processing_replacement_groups newer
            ON newer.recording_id=old.recording_id
            AND newer.source_track_id=old.source_track_id
            AND newer.source_kind=old.source_kind
          JOIN processing_jobs nj ON nj.id=newer.job_id
          WHERE rc.chunk_id=?1 AND newer.id!=old.id
            AND newer.start_ms<old.end_ms AND newer.end_ms>old.start_ms
            AND (nj.created_at_ms>current.created_at_ms
              OR (nj.created_at_ms=current.created_at_ms AND nj.id>current.id))
            AND (nj.status!='cancelled' OR newer.committed=1 OR EXISTS(
              SELECT 1 FROM processing_replacement_chunks published
              JOIN processing_replacement_frames frame ON frame.chunk_id=published.chunk_id
              WHERE published.group_id=newer.id))
        )"#,
        [chunk_id],
        |r| r.get(0),
    )
}
