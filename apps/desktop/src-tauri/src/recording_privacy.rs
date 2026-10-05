//! Shared identity, eligibility and mutation leases for reversible redactions.
use crate::storage::{self, AppStorage, ProcessingChunkRow, RecordingSegmentRow, RedactionRow};
use rusqlite::Connection;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock, Weak},
};
use tokio::sync::{OwnedRwLockReadGuard, OwnedRwLockWriteGuard, RwLock};

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
    if !protection_ready(connection, &chunk.recording_id)? {
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
    let start = chunk.start_offset_ms.ok_or(rusqlite::Error::InvalidQuery)?;
    let end = chunk.end_offset_ms.ok_or(rusqlite::Error::InvalidQuery)?;
    if chunk.source_kind == "activity" {
        return Ok(!redactions
            .iter()
            .any(|redaction| start < redaction.end_offset_ms && end > redaction.start_offset_ms));
    }
    let mut ids = Vec::new();
    if let Some(track_id) = &chunk.track_id {
        ids.push(track_id.clone());
    }
    let role = if chunk.source_kind == "audio" {
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
    if let Some(id) = source_track_id(segment, &role) {
        ids.push(id);
    }
    Ok(!range_redacted(&redactions, &ids, start, end))
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
