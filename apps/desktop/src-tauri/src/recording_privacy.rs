//! Shared identity, eligibility and mutation leases for reversible redactions.
use crate::storage::{self, AppStorage, ProcessingChunkRow, RecordingSegmentRow, RedactionRow};
use rusqlite::Connection;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock, Weak},
};
use tokio::sync::{OwnedRwLockReadGuard, OwnedRwLockWriteGuard, RwLock};
pub(crate) type ProcessingLease = Arc<OwnedRwLockReadGuard<()>>;

/// Resolve a crashed capture conservatively. Candidate state existed only in
/// the monitor; anything after its durable watermark becomes a reversible mask.
pub(crate) fn seal_interrupted_capture(
    connection: &Connection,
    id: &str,
    timestamp: i64,
) -> rusqlite::Result<()> {
    use rusqlite::OptionalExtension;
    let recording = storage::get_recording(connection, id)?;
    let start = recording.started_at_ms.unwrap_or(recording.created_at_ms);
    let end = storage::list_recording_segments(connection, id)?
        .iter()
        .filter_map(|segment| segment.ended_at_ms)
        .max()
        .unwrap_or(start)
        .saturating_sub(start)
        .max(0);
    let state:Option<(bool,i64)>=connection.query_row("SELECT classifier_required,classified_through_ms FROM recording_live_state WHERE recording_id=?1",[id],|row|Ok((row.get(0)?,row.get(1)?))).optional()?;
    if let Some((true, watermark)) = state {
        if end > watermark {
            storage::create_redaction(
                connection,
                &storage::CreateRedactionInput {
                    id: uuid::Uuid::now_v7().to_string(),
                    recording_id: id.into(),
                    start_offset_ms: watermark,
                    end_offset_ms: end,
                    created_at_ms: timestamp.max(0),
                    track_ids_json: "[]".into(),
                    reason: Some("Recording protection: metadata gap after interruption".into()),
                },
            )?;
        }
    }
    for span in storage::list_recording_protection_spans(connection, id)? {
        if span.status == "materialized" {
            continue;
        }
        let span = if span.status == "open" {
            storage::close_recording_protection_span(
                connection,
                &span.id,
                end.max(span.start_offset_ms),
                timestamp.max(span.opened_at_ms),
            )?
        } else {
            span
        };
        if let Some(boundary) = span
            .end_offset_ms
            .filter(|boundary| *boundary > span.start_offset_ms)
        {
            let cut = storage::create_redaction(
                connection,
                &storage::CreateRedactionInput {
                    id: uuid::Uuid::now_v7().to_string(),
                    recording_id: id.into(),
                    start_offset_ms: span.start_offset_ms,
                    end_offset_ms: boundary,
                    created_at_ms: timestamp.max(0),
                    track_ids_json: "[]".into(),
                    reason: Some("Recording protection recovered after interruption".into()),
                },
            )?;
            storage::mark_recording_protection_span_materialized(connection, &span.id, &cut.id)?;
        } else {
            connection.execute(
                "UPDATE recording_protection_spans SET status='materialized' WHERE id=?1",
                [&span.id],
            )?;
        }
    }
    connection.execute("UPDATE recording_live_state SET classified_through_ms=MAX(classified_through_ms,?2) WHERE recording_id=?1",rusqlite::params![id,end])?;
    connection.execute("UPDATE processing_job_runtime SET input_open=0 WHERE job_id IN(SELECT id FROM processing_jobs WHERE recording_id=?1)",[id])?;
    Ok(())
}

fn recording_lock(recording_id: &str) -> Arc<RwLock<()>> {
    static LOCKS: OnceLock<Mutex<HashMap<String, Weak<RwLock<()>>>>> = OnceLock::new();
    let mut locks = LOCKS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .expect("privacy locks");
    locks.retain(|_, lock| lock.strong_count() > 0);
    if let Some(lock) = locks.get(recording_id).and_then(Weak::upgrade) {
        return lock;
    }
    let lock = Arc::new(RwLock::new(()));
    locks.insert(recording_id.to_string(), Arc::downgrade(&lock));
    lock
}

pub(crate) async fn processing_lease(recording_id: &str) -> Arc<OwnedRwLockReadGuard<()>> {
    Arc::new(recording_lock(recording_id).read_owned().await)
}
pub(crate) async fn mutation_lease(recording_id: &str) -> OwnedRwLockWriteGuard<()> {
    recording_lock(recording_id).write_owned().await
}
pub(crate) fn try_mutation_lease(recording_id: &str) -> Option<OwnedRwLockWriteGuard<()>> {
    recording_lock(recording_id).try_write_owned().ok()
}

pub(crate) fn source_track_id(segment: &RecordingSegmentRow, role: &str) -> Option<String> {
    let index = match role {
        "screen" | "video" | "ocr" | "vlm" => segment.video_stream_index,
        "desktop-audio" => segment.desktop_audio_stream_index,
        "microphone" => segment.microphone_audio_stream_index,
        _ => None,
    }?;
    let prefix = if matches!(role, "screen" | "video" | "ocr" | "vlm") {
        "video"
    } else {
        role
    };
    Some(format!("{prefix}-{index}"))
}

pub(crate) fn range_redacted(
    redactions: &[RedactionRow],
    track_ids: &[String],
    start: i64,
    end: i64,
) -> bool {
    redactions.iter().any(|redaction| {
        if start >= redaction.end_offset_ms || end <= redaction.start_offset_ms {
            return false;
        }
        let targets =
            crate::redaction_targets::parse_redaction_track_ids(&redaction.track_ids_json)
                .unwrap_or_default();
        targets.is_empty() || targets.iter().any(|target| track_ids.contains(target))
    })
}

pub(crate) fn protection_ready(
    connection: &Connection,
    recording_id: &str,
) -> rusqlite::Result<bool> {
    Ok(
        storage::list_recording_protection_spans(connection, recording_id)?
            .iter()
            .all(|span| span.status == "materialized"),
    )
}

pub(crate) fn chunk_eligible(
    connection: &Connection,
    chunk: &ProcessingChunkRow,
) -> rusqlite::Result<bool> {
    let start = chunk.start_offset_ms.ok_or(rusqlite::Error::InvalidQuery)?;
    let end = chunk.end_offset_ms.ok_or(rusqlite::Error::InvalidQuery)?;
    if !range_protection_ready(connection, &chunk.recording_id, start, end)? {
        return Ok(false);
    }
    let segments = storage::list_recording_segments(connection, &chunk.recording_id)?;
    let Some(segment) = segments
        .iter()
        .find(|segment| Some(segment.id.as_str()) == chunk.segment_id.as_deref())
    else {
        return Ok(false);
    };
    let redactions = storage::list_redactions(connection, &chunk.recording_id)?;
    if chunk.source_kind == "activity" {
        return Ok(!redactions
            .iter()
            .any(|redaction| start < redaction.end_offset_ms && end > redaction.start_offset_ms));
    }
    let mut ids = Vec::new();
    if let Some(track_id) = &chunk.track_id {
        ids.push(track_id.clone());
    }
    let role = if matches!(chunk.source_kind.as_str(), "audio" | "waveform") {
        let Some(track) = chunk
            .track_id
            .as_deref()
            .map(|id| storage::get_recording_track(connection, id))
            .transpose()?
            .flatten()
        else {
            return Ok(false);
        };
        track.kind
    } else {
        chunk.source_kind.clone()
    };
    if let Some(id) =
        if chunk.track_id.is_some() && matches!(chunk.source_kind.as_str(), "ocr" | "vlm") {
            None
        } else {
            source_track_id(segment, &role)
        }
    {
        ids.push(id);
    }
    Ok(!range_redacted(&redactions, &ids, start, end))
}

pub(crate) fn range_protection_ready(
    connection: &Connection,
    recording_id: &str,
    start: i64,
    end: i64,
) -> rusqlite::Result<bool> {
    let recording = storage::get_recording(connection, recording_id)?;
    if !matches!(recording.status.as_str(), "recording" | "paused") {
        return protection_ready(connection, recording_id);
    }
    if matches!(recording.status.as_str(), "recording" | "paused") {
        use rusqlite::OptionalExtension;
        let watermark: Option<i64> = connection
            .query_row(
                "SELECT classified_through_ms FROM recording_live_state WHERE recording_id=?1",
                [recording_id],
                |row| row.get(0),
            )
            .optional()?;
        if watermark.is_none_or(|watermark| end > watermark) {
            return Ok(false);
        }
    }
    Ok(
        storage::list_recording_protection_spans(connection, recording_id)?
            .iter()
            .all(|span| {
                span.status == "materialized"
                    || end <= span.start_offset_ms
                    || span.end_offset_ms.is_some_and(|boundary| start >= boundary)
            }),
    )
}

pub(crate) fn chunk_active(
    connection: &Connection,
    chunk: &ProcessingChunkRow,
) -> rusqlite::Result<bool> {
    let Some(current) = storage::get_processing_chunk(connection, &chunk.id)? else {
        return Ok(false);
    };
    let Some(job) = storage::get_processing_job(connection, &chunk.job_id)? else {
        return Ok(false);
    };
    Ok(current.status == "running"
        && current.attempt_count == chunk.attempt_count
        && job.status == "running"
        && chunk_eligible(connection, &current)?)
}

/// Cancel before awaiting the writer: never hold the database mutex while draining workers.
pub(crate) fn cancel_recording_processing(
    storage: &AppStorage,
    recording_id: &str,
    timestamp_ms: i64,
) -> rusqlite::Result<()> {
    crate::library::audio_cleanup::request_cancel(recording_id);
    storage.with_connection(|connection| {
        for job in storage::list_processing_jobs(connection, Some(recording_id))? {
            if matches!(job.status.as_str(), "queued" | "running" | "paused") {
                storage::cancel_processing_chunks(
                    connection,
                    &job.id,
                    &["queued", "running"],
                    timestamp_ms,
                )?;
                storage::update_processing_job_status(
                    connection,
                    &job.id,
                    "cancelled",
                    Some("ai_processing_privacy_changed"),
                    timestamp_ms,
                )?;
            }
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mutation_success_waits_until_existing_processing_has_drained() {
        tauri::async_runtime::block_on(async {
            let recording_id = uuid::Uuid::now_v7().to_string();
            let reader = processing_lease(&recording_id).await;
            assert!(try_mutation_lease(&recording_id).is_none());
            let (started_tx, started_rx) = tokio::sync::oneshot::channel();
            let (finished_tx, mut finished_rx) = tokio::sync::oneshot::channel();
            let writer = tauri::async_runtime::spawn(async move {
                started_tx.send(()).unwrap();
                let _writer = mutation_lease(&recording_id).await;
                finished_tx.send(()).unwrap();
            });
            started_rx.await.unwrap();
            assert!(matches!(
                finished_rx.try_recv(),
                Err(tokio::sync::oneshot::error::TryRecvError::Empty)
            ));
            drop(reader);
            finished_rx.await.unwrap();
            writer.await.unwrap();
        });
    }
}
