use super::*;
use crate::storage::{self, AppStorage, RecordingSegmentRow};
use rusqlite::{params, Connection, OptionalExtension};
use sha2::{Digest, Sha256};
use std::{fs, path::Path};

pub(crate) fn fingerprint(connection: &Connection, id: &str) -> rusqlite::Result<String> {
    let segments = storage::list_recording_segments(connection, id)?;
    let mut hash = Sha256::new();
    hash.update(ENGINE_VERSION);
    for segment in segments {
        hash.update(
            serde_json::to_vec(&(
                &segment.id,
                &segment.file_path,
                segment.duration_ms,
                segment.desktop_audio_stream_index,
                segment.microphone_audio_stream_index,
                segment.capture_run_id,
                segment.timeline_base_ms,
                segment.probe_json,
            ))
            .map_err(|_| rusqlite::Error::InvalidQuery)?,
        );
        if let Ok(meta) = fs::metadata(&segment.file_path) {
            hash.update(meta.len().to_le_bytes());
            let modified = meta
                .modified()
                .ok()
                .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|time| time.as_nanos())
                .unwrap_or(0);
            hash.update(modified.to_le_bytes());
        } else {
            hash.update(b"missing");
        }
    }
    hash.update(privacy_fingerprint(connection, id)?);
    Ok(hex::encode(hash.finalize()))
}

/// Playback checks privacy without rescanning every media file per thumbnail.
pub(crate) fn privacy_fingerprint(connection: &Connection, id: &str) -> rusqlite::Result<String> {
    storage::get_recording(connection, id)?;
    let mut hash = Sha256::new();
    for cut in storage::list_redactions(connection, id)? {
        hash.update(
            serde_json::to_vec(&(
                &cut.id,
                cut.start_offset_ms,
                cut.end_offset_ms,
                cut.track_ids_json,
            ))
            .map_err(|_| rusqlite::Error::InvalidQuery)?,
        );
    }
    let protected = crate::recording_privacy::protection_ready(connection, id)?;
    hash.update([u8::from(protected)]);
    Ok(hex::encode(hash.finalize()))
}

/// Legacy processing-track UUIDs and captured source IDs describe the same
/// audio role. Retain both identities when deciding what may be decoded.
pub(super) fn eligible_redactions(
    connection: &Connection,
    id: &str,
) -> rusqlite::Result<Vec<storage::RedactionRow>> {
    let tracks = storage::list_recording_tracks(connection, id)?;
    let mut redactions = storage::list_redactions(connection, id)?;
    for cut in &mut redactions {
        let mut ids = crate::redaction_targets::parse_redaction_track_ids(&cut.track_ids_json)
            .unwrap_or_default();
        for track in &tracks {
            if matches!(track.kind.as_str(), "microphone" | "desktop-audio")
                && ids.contains(&track.id)
                && !ids.contains(&track.kind)
            {
                ids.push(track.kind.clone());
            }
        }
        cut.track_ids_json =
            serde_json::to_string(&ids).map_err(|_| rusqlite::Error::InvalidQuery)?;
    }
    Ok(redactions)
}

pub(super) fn manifest(
    connection: &Connection,
    id: &str,
    revision: &str,
) -> rusqlite::Result<CleanupManifest> {
    let (json, saved): (String, String) = connection.query_row(
        "SELECT manifest_json,source_fingerprint FROM audio_cleanup_generations WHERE id=?1 AND recording_id=?2 AND status='ready'",
        params![revision,id], |row| Ok((row.get(0)?, row.get(1)?)))?;
    if saved != fingerprint(connection, id)? {
        return Err(rusqlite::Error::InvalidQuery);
    }
    let blocks: Vec<CleanupBlock> =
        serde_json::from_str(&json).map_err(|_| rusqlite::Error::InvalidQuery)?;
    if blocks.iter().any(|block| !Path::new(&block.path).is_file()) {
        return Err(rusqlite::Error::InvalidQuery);
    }
    Ok(CleanupManifest {
        revision: revision.into(),
        blocks,
    })
}

pub(crate) fn active_manifest(
    connection: &Connection,
    id: &str,
) -> rusqlite::Result<Option<CleanupManifest>> {
    let revision: Option<String> = connection
        .query_row(
            "SELECT active_revision FROM recording_audio_cleanup WHERE recording_id=?1",
            [id],
            |row| row.get(0),
        )
        .optional()?
        .flatten();
    revision
        .map(|revision| manifest(connection, id, &revision))
        .transpose()
}

/// Called only at new-job admission. Absence in old jobs explicitly means raw audio.
pub(crate) fn pin_job(
    connection: &Connection,
    job_id: &str,
    recording_id: &str,
) -> rusqlite::Result<()> {
    let revision = active_manifest(connection, recording_id)?.map(|value| value.revision);
    connection.execute("INSERT INTO processing_audio_cleanup_inputs(job_id,revision) VALUES(?1,?2) ON CONFLICT(job_id) DO NOTHING", params![job_id,revision])?;
    Ok(())
}
pub(crate) fn pinned_manifest(
    connection: &Connection,
    job: &str,
    id: &str,
) -> rusqlite::Result<Option<CleanupManifest>> {
    let revision: Option<String> = connection
        .query_row(
            "SELECT revision FROM processing_audio_cleanup_inputs WHERE job_id=?1",
            [job],
            |row| row.get(0),
        )
        .optional()?
        .flatten();
    revision
        .map(|revision| manifest(connection, id, &revision))
        .transpose()
}

pub(crate) fn recover_interrupted(storage: &AppStorage) -> rusqlite::Result<()> {
    storage.with_connection(|connection| {
        connection.execute("UPDATE audio_cleanup_generations SET status='cancelled',error='Cleanup was interrupted. Retry to resume completed blocks.' WHERE status='running'", [])?;
        Ok(())
    })
}

pub(super) fn save_checkpoint(
    storage: &AppStorage,
    id: &str,
    blocks: &[CleanupBlock],
) -> Result<(), String> {
    let json = serde_json::to_string(blocks).map_err(|_| "Cleanup progress could not be saved.")?;
    let completed: i64 = blocks.iter().map(|block| block.duration_ms).sum();
    let matched: i64 = blocks.iter().map(|block| block.matched_ms).sum();
    let changed = storage.with_connection(|connection| connection.execute(
        "UPDATE audio_cleanup_generations SET manifest_json=?2,completed_ms=?3,matched_ms=?4 WHERE id=?1 AND status='running'",
        params![id,json,completed,matched])).map_err(|_| "Cleanup progress could not be saved.")?;
    if changed != 1 {
        return Err("Cleanup was cancelled or its sources changed.".into());
    }
    Ok(())
}

pub(crate) fn segment_blocks<'a>(
    manifest: &'a CleanupManifest,
    segment: &RecordingSegmentRow,
) -> Vec<&'a CleanupBlock> {
    manifest
        .blocks
        .iter()
        .filter(|block| block.segment_id == segment.id)
        .collect()
}
