//! Canonical source snapshot. No model output is accepted as recall evidence.
use crate::recall::types::{RecallCitation, RecallEvidence};
use rusqlite::{params, Connection, OptionalExtension};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

mod evidence_writer;
use evidence_writer::{insert_source, EvidenceMedia, Source};

pub fn privacy_revision(connection: &Connection) -> rusqlite::Result<i64> {
    connection.query_row("SELECT revision FROM recall_meta WHERE id=1", [], |row| {
        row.get(0)
    })
}

pub fn recall_privacy_revision(connection: &Connection) -> rusqlite::Result<i64> {
    connection.query_row(
        "SELECT privacy_revision FROM recall_meta WHERE id=1",
        [],
        |row| row.get(0),
    )
}

pub fn read_evidence(
    connection: &Connection,
    id: &str,
) -> rusqlite::Result<Option<RecallEvidence>> {
    connection.query_row("SELECT id,recording_id,segment_id,source_kind,start_offset_ms,end_offset_ms,started_at_ms,ended_at_ms,revision,text,source_track_id,source_id,provenance_json FROM recall_evidence WHERE id=?1", [id], evidence_row).optional()
}

pub fn list_evidence_range(
    connection: &Connection,
    start: i64,
    end: i64,
) -> rusqlite::Result<Vec<RecallEvidence>> {
    let mut statement=connection.prepare("SELECT id,recording_id,segment_id,source_kind,start_offset_ms,end_offset_ms,started_at_ms,ended_at_ms,revision,text,source_track_id,source_id,provenance_json FROM recall_evidence WHERE started_at_ms<?2 AND ended_at_ms>?1 ORDER BY started_at_ms,id")?;
    let rows = statement.query_map(params![start, end], evidence_row)?;
    rows.collect()
}

fn evidence_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RecallEvidence> {
    let id: String = row.get(0)?;
    let source_id: String = row.get(11)?;
    let source_kind: String = row.get(3)?;
    let provenance: serde_json::Value = serde_json::from_str(&row.get::<_, String>(12)?)
        .map_err(|_| rusqlite::Error::InvalidQuery)?;
    let document_id = (source_kind == "ocr").then(|| source_id.clone());
    let source_frame_ref = provenance
        .as_array()
        .and_then(|items| {
            items.iter().find(|item| {
                item.get("id").and_then(serde_json::Value::as_str) == document_id.as_deref()
            })
        })
        .and_then(|item| item.get("frame"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    Ok(RecallEvidence {
        citation: RecallCitation {
            evidence_id: id,
            recording_id: row.get(1)?,
            segment_id: row.get(2)?,
            source_kind,
            source_track_id: row.get(10)?,
            source_id,
            source_frame_ref,
            document_id,
            start_offset_ms: row.get(4)?,
            end_offset_ms: row.get(5)?,
            absolute_start_ms: row.get(6)?,
            absolute_end_ms: row.get(7)?,
            content_revision: row.get(8)?,
        },
        text: row.get(9)?,
    })
}

pub fn evidence_fingerprint(evidence: &[RecallEvidence]) -> String {
    let mut hasher = Sha256::new();
    for item in evidence {
        hasher.update((item.citation.evidence_id.len() as u64).to_le_bytes());
        hasher.update(item.citation.evidence_id.as_bytes());
        hasher.update(item.citation.content_revision.as_bytes());
    }
    format!("{:x}", hasher.finalize())
}

/// Must run under the AppStorage connection lock before source reads or publish.
/// A SAVEPOINT makes invalidation and replacement atomic even inside a caller's
/// transaction. Missing/ambiguous timing never acquires fabricated UTC values.
pub fn refresh_evidence(connection: &Connection) -> rusqlite::Result<i64> {
    refresh_evidence_scope(connection, None)
}

/// Metadata views revalidate only requested recordings; global search/index reads
/// keep the full refresh boundary. Dirty recordings outside this scope stay queued.
pub fn refresh_recording_evidence(
    connection: &Connection,
    recording_ids: &[String],
) -> rusqlite::Result<i64> {
    if recording_ids.is_empty() {
        return privacy_revision(connection);
    }
    let scope = serde_json::to_string(recording_ids).map_err(|_| rusqlite::Error::InvalidQuery)?;
    refresh_evidence_scope(connection, Some(&scope))
}

fn refresh_evidence_scope(connection: &Connection, scope: Option<&str>) -> rusqlite::Result<i64> {
    let dirty: bool = connection.query_row(
        "SELECT source_dirty FROM recall_meta WHERE id=1",
        [],
        |row| row.get(0),
    )?;
    let mut statement=connection.prepare("SELECT s.id,s.file_path,f.path,f.available,s.recording_id FROM recording_segments s LEFT JOIN recall_source_files f ON f.segment_id=s.id WHERE (?1 IS NULL OR s.recording_id IN(SELECT value FROM json_each(?1)))")?;
    let files = statement
        .query_map([scope], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<bool>>(3)?,
                row.get::<_, String>(4)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let file_changed = files.iter().any(|(_, path, cached, available, _)| {
        cached.as_ref() != Some(path) || *available != Some(std::path::Path::new(path).is_file())
    });
    if !dirty && !file_changed {
        return privacy_revision(connection);
    }
    connection.execute_batch("SAVEPOINT recall_refresh; CREATE TEMP TABLE IF NOT EXISTS recall_next AS SELECT * FROM recall_evidence WHERE 0; DELETE FROM recall_next;")?;
    let result = (|| {
        for (_, path, cached, available, recording) in &files {
            if cached.as_ref() != Some(path)
                || *available != Some(std::path::Path::new(path).is_file())
            {
                connection.execute(
                    "INSERT INTO recall_dirty_recordings VALUES(?1) ON CONFLICT DO NOTHING",
                    [recording],
                )?;
            }
        }
        refresh_inner(connection, scope)?;
        for (id, path, cached, available, _) in files {
            let exists = std::path::Path::new(&path).is_file();
            if cached.as_ref() != Some(&path) || available != Some(exists) {
                connection.execute("INSERT INTO recall_source_files(segment_id,path,available) VALUES(?1,?2,?3) ON CONFLICT(segment_id) DO UPDATE SET path=excluded.path,available=excluded.available",params![id,path,exists])?;
            }
        }
        connection.execute("DELETE FROM recall_next", [])?;
        connection.execute("DELETE FROM recall_dirty_recordings WHERE (?1 IS NULL OR recording_id IN(SELECT value FROM json_each(?1)))", [scope])?;
        connection.execute("UPDATE recall_meta SET source_dirty=EXISTS(SELECT 1 FROM recall_dirty_recordings) WHERE id=1", [])?;
        Ok(())
    })();
    match result {
        Ok(()) => {
            connection.execute_batch("RELEASE recall_refresh")?;
            privacy_revision(connection)
        }
        Err(error) => {
            let _ = connection.execute_batch("ROLLBACK TO recall_refresh; RELEASE recall_refresh");
            Err(error)
        }
    }
}

fn refresh_inner(connection: &Connection, scope: Option<&str>) -> rusqlite::Result<()> {
    let mut query=connection.prepare("SELECT r.id FROM recordings r JOIN recall_dirty_recordings d ON d.recording_id=r.id WHERE (?1 IS NULL OR r.id IN(SELECT value FROM json_each(?1)))")?;
    let recording_ids = query
        .query_map([scope], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    for id in recording_ids {
        let recording = super::get_recording(connection, &id)?;
        let segments = super::list_recording_segments(connection, &recording.id)?;
        let eligible = segments
            .iter()
            .filter(|segment| {
                segment.state == "finalized"
                    && segment.timeline_basis_version == Some(1)
                    && segment.timeline_base_ms.is_some()
                    && segment.duration_ms.is_some()
                    && std::path::Path::new(&segment.file_path).is_file()
            })
            .map(|segment| (segment.id.as_str(), segment))
            .collect::<BTreeMap<_, _>>();
        let redactions = super::list_redactions(connection, &recording.id)?;
        let media = EvidenceMedia::new(&eligible, &redactions);
        let visible = super::ai::recall_ocr_visibility_cte();
        // Consecutive exact documents form one recall item; support retains all
        // occurrences, including uncited frames consumed by interpretation.
        let sql = format!("{visible}{OCR_RUNS}");
        let mut statement = connection.prepare(&sql)?;
        let rows = statement.query_map(params![recording.id, Option::<String>::None], |row| {
            Ok(Source {
                id: format!("ocr:{}", row.get::<_, String>(0)?),
                segment_id: row.get(1)?,
                track: row.get(2)?,
                start: row.get(3)?,
                end: row.get(4)?,
                text: row.get(5)?,
                provenance: row.get(6)?,
                kind: "ocr".into(),
            })
        })?;
        for source in rows {
            insert_source(connection, &recording.id, &media, source?)?;
        }
        let mut statement = connection.prepare(TRANSCRIPTS)?;
        let rows = statement.query_map([&recording.id], |row| {
            Ok(Source {
                id: format!("transcript:{}", row.get::<_, String>(0)?),
                segment_id: row.get(1)?,
                track: row.get(2)?,
                start: row.get(3)?,
                end: row.get(4)?,
                text: row.get(5)?,
                provenance: row.get(6)?,
                kind: row.get(7)?,
            })
        })?;
        for source in rows {
            insert_source(connection, &recording.id, &media, source?)?;
        }
    }
    let changed: bool=connection.query_row("SELECT EXISTS(SELECT 1 FROM recall_next n LEFT JOIN recall_evidence e ON e.id=n.id WHERE e.id IS NULL OR e.revision!=n.revision) OR EXISTS(SELECT 1 FROM recall_evidence e JOIN recall_dirty_recordings d ON d.recording_id=e.recording_id WHERE (?1 IS NULL OR d.recording_id IN(SELECT value FROM json_each(?1))) AND NOT EXISTS(SELECT 1 FROM recall_next n WHERE n.id=e.id))",[scope],|row|row.get(0))?;
    if changed {
        connection.execute("DELETE FROM recall_evidence WHERE recording_id IN(SELECT recording_id FROM recall_dirty_recordings WHERE (?1 IS NULL OR recording_id IN(SELECT value FROM json_each(?1)))) AND NOT EXISTS(SELECT 1 FROM recall_next n WHERE n.id=recall_evidence.id)",[scope])?;
        connection.execute("INSERT INTO recall_evidence SELECT * FROM recall_next WHERE true ON CONFLICT(id) DO UPDATE SET recording_id=excluded.recording_id,segment_id=excluded.segment_id,source_kind=excluded.source_kind,source_track_id=excluded.source_track_id,source_id=excluded.source_id,start_offset_ms=excluded.start_offset_ms,end_offset_ms=excluded.end_offset_ms,started_at_ms=excluded.started_at_ms,ended_at_ms=excluded.ended_at_ms,revision=excluded.revision,text=excluded.text,provenance_json=excluded.provenance_json WHERE recall_evidence.revision!=excluded.revision",[])?;
        connection.execute("UPDATE recall_meta SET revision=revision+1 WHERE id=1", [])?;
    }
    Ok(())
}

const OCR_RUNS: &str = r#"
, previous AS (
 SELECT *,lag(normalized_text) OVER w previous_text,lag(sample_end_ms) OVER w previous_end,
 lag(segment_id) OVER w previous_segment,lag(config_fingerprint) OVER w previous_config,
 lag(provenance) OVER w previous_provenance
 FROM current_visible WHERE status='succeeded' AND normalized_text!=''
 AND (EXISTS(SELECT 1 FROM processing_jobs j WHERE j.id=current_visible.job_id AND j.status!='cancelled') OR EXISTS(SELECT 1 FROM processing_replacement_frames rf WHERE rf.chunk_id=current_visible.chunk_id AND rf.frame_ms=current_visible.frame_timestamp_ms))
 WINDOW w AS(PARTITION BY source_track_id ORDER BY frame_timestamp_ms,id)
), runs AS (
 SELECT *,sum(CASE WHEN normalized_text=previous_text AND frame_timestamp_ms=previous_end
 AND segment_id=previous_segment AND config_fingerprint=previous_config
 AND provenance='native' AND previous_provenance='native' THEN 0 ELSE 1 END)
 OVER(PARTITION BY source_track_id ORDER BY frame_timestamp_ms,id) run_id FROM previous
)
SELECT (SELECT first.id FROM runs first WHERE first.segment_id=r.segment_id AND first.source_track_id=r.source_track_id AND first.run_id=r.run_id ORDER BY first.frame_timestamp_ms,first.id LIMIT 1),
 segment_id,source_track_id,min(frame_timestamp_ms),max(sample_end_ms),text,
 json_group_array(json_object('id',id,'contentId',content_id,'config',config_fingerprint,'attempt',attempt_count,'frame',source_frame_ref))
FROM (SELECT * FROM runs ORDER BY frame_timestamp_ms,id) r GROUP BY segment_id,source_track_id,run_id
"#;

const TRANSCRIPTS: &str = r#"
SELECT t.id,t.segment_id,CASE WHEN t.source_role='microphone' THEN 'microphone-'||s.microphone_audio_stream_index ELSE 'desktop-audio-'||s.desktop_audio_stream_index END,t.start_offset_ms,t.end_offset_ms,t.text,
 json_object('jobId',t.job_id,'chunkId',t.chunk_id,'processingTrackId',t.track_id,'createdAt',t.created_at_ms,'provider',t.provider,'model',t.model),t.source_role
FROM published_transcript_segments t JOIN processing_chunks p ON p.id=t.chunk_id JOIN processing_jobs j ON j.id=t.job_id JOIN recording_segments s ON s.id=t.segment_id
WHERE t.recording_id=?1 AND trim(t.text)!='' AND (EXISTS(SELECT 1 FROM processing_speech_unit_transcripts st JOIN processing_speech_units u ON u.id=st.unit_id WHERE st.transcript_id=t.id AND u.status='published') OR ((j.status!='cancelled' OR EXISTS(SELECT 1 FROM processing_replacement_chunks rc JOIN processing_replacement_groups rg ON rg.id=rc.group_id WHERE rc.chunk_id=t.chunk_id AND rg.committed=1)) AND p.status IN('running','succeeded','completed-with-errors')))
AND (CASE WHEN t.source_role='microphone' THEN s.microphone_audio_stream_index ELSE s.desktop_audio_stream_index END) IS NOT NULL
AND NOT EXISTS(SELECT 1 FROM recording_protection_spans ps WHERE ps.recording_id=t.recording_id AND ps.status!='materialized')
AND NOT EXISTS(SELECT 1 FROM redactions r WHERE r.recording_id=t.recording_id AND r.start_offset_ms<t.end_offset_ms AND r.end_offset_ms>t.start_offset_ms AND
 (json_type(r.track_ids_json)!='array' OR json_array_length(r.track_ids_json)=0 OR EXISTS(SELECT 1 FROM json_each(r.track_ids_json) x WHERE x.type!='text') OR EXISTS(SELECT 1 FROM json_each(r.track_ids_json) x WHERE x.value=t.track_id OR x.value=p.track_id OR x.value=(CASE WHEN t.source_role='microphone' THEN 'microphone-'||s.microphone_audio_stream_index ELSE 'desktop-audio-'||s.desktop_audio_stream_index END))))
AND NOT EXISTS(SELECT 1 FROM processing_chunks newer JOIN processing_jobs nj ON nj.id=newer.job_id WHERE newer.recording_id=t.recording_id AND newer.segment_id=t.segment_id AND newer.source_kind='audio' AND newer.track_id=t.track_id AND newer.start_offset_ms<t.end_offset_ms AND newer.end_offset_ms>t.start_offset_ms AND nj.status!='cancelled' AND NOT EXISTS(SELECT 1 FROM processing_replacement_chunks rc WHERE rc.chunk_id=newer.id) AND (nj.created_at_ms>j.created_at_ms OR (nj.created_at_ms=j.created_at_ms AND nj.id>j.id)))
"#;

#[cfg(test)]
#[path = "recall_tests.rs"]
pub(crate) mod tests;

#[cfg(test)]
#[path = "recall_speech_tests.rs"]
mod speech_tests;
