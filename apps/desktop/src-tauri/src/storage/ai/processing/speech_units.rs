use crate::storage::{ProcessingChunkRow, RecordingTrackRow};
use rusqlite::{params, Connection, OptionalExtension};

#[derive(Clone, Debug)]
pub struct SpeechUnit {
    pub id: String,
    pub job_id: String,
    pub track_id: String,
    pub owner_chunk_id: String,
    pub start_ms: i64,
    pub end_ms: i64,
    pub submitted_start_ms: i64,
    pub submitted_end_ms: i64,
    pub right_open: bool,
    pub status: String,
    pub fingerprint: String,
    pub output_json: Option<String>,
}
pub fn speech_strategy(connection: &Connection, job_id: &str) -> rusqlite::Result<bool> {
    connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM processing_audio_strategies WHERE job_id=?1)",
        [job_id],
        |row| row.get(0),
    )
}
pub fn enable_speech_strategy(connection: &Connection, job_id: &str) -> rusqlite::Result<()> {
    connection.execute(
        "INSERT OR IGNORE INTO processing_audio_strategies(job_id,version) VALUES(?1,2)",
        [job_id],
    )?;
    Ok(())
}
pub fn speech_units(
    connection: &Connection,
    job: &str,
    track: &str,
) -> rusqlite::Result<Vec<SpeechUnit>> {
    let mut query=connection.prepare("SELECT id,job_id,track_id,owner_chunk_id,start_ms,end_ms,submitted_start_ms,submitted_end_ms,right_open,status,config_fingerprint,output_json FROM processing_speech_units WHERE job_id=?1 AND track_id=?2 ORDER BY start_ms")?;
    let result = query
        .query_map(params![job, track], |row| {
            Ok(SpeechUnit {
                id: row.get(0)?,
                job_id: row.get(1)?,
                track_id: row.get(2)?,
                owner_chunk_id: row.get(3)?,
                start_ms: row.get(4)?,
                end_ms: row.get(5)?,
                submitted_start_ms: row.get(6)?,
                submitted_end_ms: row.get(7)?,
                right_open: row.get(8)?,
                status: row.get(9)?,
                fingerprint: row.get(10)?,
                output_json: row.get(11)?,
            })
        })?
        .collect();
    result
}
pub fn save_speech_unit(connection: &Connection, unit: &SpeechUnit) -> rusqlite::Result<()> {
    connection.execute("INSERT OR IGNORE INTO processing_speech_units(id,job_id,track_id,owner_chunk_id,start_ms,end_ms,submitted_start_ms,submitted_end_ms,right_open,config_fingerprint) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",params![unit.id,unit.job_id,unit.track_id,unit.owner_chunk_id,unit.start_ms,unit.end_ms,unit.submitted_start_ms,unit.submitted_end_ms,unit.right_open,unit.fingerprint])?;
    connection.execute("INSERT OR IGNORE INTO processing_speech_unit_chunks(unit_id,chunk_id) SELECT ?1,id FROM processing_chunks WHERE job_id=?2 AND track_id=?3 AND source_kind='audio' AND start_offset_ms<?5 AND end_offset_ms>?4",params![unit.id,unit.job_id,unit.track_id,unit.start_ms,unit.end_ms])?;
    Ok(())
}
pub fn stage_speech_unit(connection: &Connection, id: &str, json: &str) -> rusqlite::Result<()> {
    connection.execute("UPDATE processing_speech_units SET status='staged',output_json=?2,error_code=NULL WHERE id=?1 AND status!='published'",params![id,json])?;
    Ok(())
}
pub fn fail_speech_unit(connection: &Connection, id: &str, code: &str) -> rusqlite::Result<()> {
    connection.execute("UPDATE processing_speech_units SET status='failed',error_code=?2 WHERE id=?1 AND status!='published'",params![id,code])?;
    let mut query=connection.prepare("SELECT p.id,MAX(u.start_ms,p.start_offset_ms)-p.start_offset_ms,MIN(u.end_ms,p.end_offset_ms)-p.start_offset_ms,u.config_fingerprint FROM processing_speech_units u JOIN processing_speech_unit_chunks uc ON uc.unit_id=u.id JOIN processing_chunks p ON p.id=uc.chunk_id WHERE u.id=?1 AND u.status='failed'")?;
    let children = query
        .query_map([id], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, String>(3)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    for (child, start, end, fingerprint) in children {
        super::runtime::save_audio_range_checkpoint(
            connection,
            &child,
            start,
            end,
            &fingerprint,
            Some(code),
        )?;
    }
    Ok(())
}

/// Called inside the same transaction that writes the new transcript and checkpoints.
pub fn publish_speech_unit(
    connection: &Connection,
    unit: &SpeechUnit,
    chunk: &ProcessingChunkRow,
    track: &RecordingTrackRow,
) -> rusqlite::Result<()> {
    let superseded:bool=connection.query_row("SELECT EXISTS(SELECT 1 FROM processing_jobs old JOIN processing_jobs newer ON newer.recording_id=old.recording_id JOIN processing_chunks p ON p.job_id=newer.id WHERE old.id=?1 AND p.source_kind='audio' AND p.track_id=?2 AND p.start_offset_ms<?4 AND p.end_offset_ms>?3 AND (newer.created_at_ms>old.created_at_ms OR (newer.created_at_ms=old.created_at_ms AND newer.id>old.id)) AND newer.status!='cancelled')",params![unit.job_id,track.id,unit.start_ms,unit.end_ms],|row|row.get(0))?;
    if superseded {
        return Err(rusqlite::Error::InvalidQuery);
    }
    connection.execute("UPDATE processing_speech_units SET status='published',output_json=NULL,error_code=NULL WHERE id=?1",[&unit.id])?;
    // Retain a previous segment until every unit needed to replace it succeeds.
    // The publication view withholds its stale text meanwhile.
    connection.execute("DELETE FROM transcript_segments AS old WHERE old.recording_id=?1 AND old.source_role=?2 AND old.start_offset_ms<?4 AND old.end_offset_ms>?3 AND old.job_id!=?5 AND old.start_offset_ms>=(SELECT MIN(start_ms) FROM processing_speech_units WHERE job_id=?5 AND track_id=?6) AND old.end_offset_ms<=(SELECT MAX(end_ms) FROM processing_speech_units WHERE job_id=?5 AND track_id=?6) AND NOT EXISTS(SELECT 1 FROM processing_speech_units u WHERE u.job_id=?5 AND u.track_id=?6 AND u.start_ms<old.end_offset_ms AND u.end_ms>old.start_offset_ms AND u.status!='published')",params![chunk.recording_id,track.kind,unit.start_ms,unit.end_ms,unit.job_id,track.id])?;
    // Source-time publication, not the number of detected utterances, owns progress.
    let mut query=connection.prepare("SELECT id,start_offset_ms,end_offset_ms FROM processing_chunks WHERE job_id=?1 AND track_id=?2 AND source_kind='audio' AND start_offset_ms<?4 AND end_offset_ms>?3")?;
    let children = query
        .query_map(
            params![unit.job_id, track.id, unit.start_ms, unit.end_ms],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            },
        )?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    for (id, start, end) in children {
        super::runtime::save_audio_range_checkpoint(
            connection,
            &id,
            unit.start_ms.max(start) - start,
            unit.end_ms.min(end) - start,
            &unit.fingerprint,
            None,
        )?;
        // A unit can finish in a later fragment after an earlier scheduling
        // chunk has succeeded. Complete every affected replacement group.
        crate::storage::publish_ready_replacement(connection, &id)?;
    }
    Ok(())
}

pub fn speech_unit_error(
    connection: &Connection,
    chunk_id: &str,
) -> rusqlite::Result<Option<String>> {
    connection.query_row("SELECT u.error_code FROM processing_speech_units u JOIN processing_speech_unit_chunks c ON c.unit_id=u.id WHERE c.chunk_id=?1 AND u.status='failed' ORDER BY u.start_ms LIMIT 1",[chunk_id],|row|row.get(0)).optional()
}
