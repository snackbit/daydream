use super::{cleanup_source, prepared::PreparedSource};
use crate::{
    library::LibraryTrackKind,
    storage::{self, AppStorage, RecordingRow},
};
use rusqlite::OptionalExtension;
use std::{path::Path, sync::atomic::AtomicBool};

pub(crate) fn sources(
    storage: &AppStorage,
    id: &str,
) -> Result<(RecordingRow, Vec<PreparedSource>), String> {
    recording_sources(storage, id, None)
}

fn recording_sources(
    storage: &AppStorage,
    id: &str,
    selected: Option<&storage::ProcessingChunkRow>,
) -> Result<(RecordingRow, Vec<PreparedSource>), String> {
    storage
        .with_connection(|connection| {
            let recording = storage::get_recording(connection, id)?;
            let segments = storage::list_recording_segments(connection, id)?;
            let bases = storage::segment_playback_bases(&segments);
            let mut tracks = storage::list_recording_tracks(connection, id)?;
            let redactions = storage::list_redactions(connection, id)?;
            let cleanup = crate::library::audio_cleanup::active_manifest(connection, id)?;
            let mut sources = Vec::new();
            for (segment, base) in segments.iter().zip(bases) {
                if selected
                    .is_some_and(|chunk| chunk.segment_id.as_deref() != Some(segment.id.as_str()))
                {
                    continue;
                }
                if segment.state != "finalized" || !Path::new(&segment.file_path).is_file() {
                    continue;
                }
                let end = base + segment.duration_ms.unwrap_or(0);
                for (kind, role, stream) in [
                    (
                        LibraryTrackKind::DesktopAudio,
                        "desktop-audio",
                        segment.desktop_audio_stream_index,
                    ),
                    (
                        LibraryTrackKind::Microphone,
                        "microphone",
                        segment.microphone_audio_stream_index,
                    ),
                ] {
                    let Some(stream) = stream else {
                        continue;
                    };
                    let track = if let Some(track) = tracks.iter().find(|track| track.kind == role)
                    {
                        track.clone()
                    } else {
                        let track = storage::upsert_recording_track(
                            connection,
                            &storage::UpsertRecordingTrackInput {
                                id: uuid::Uuid::now_v7().to_string(),
                                recording_id: id.into(),
                                kind: role.into(),
                                label: if role == "microphone" {
                                    "Microphone"
                                } else {
                                    "Desktop audio"
                                }
                                .into(),
                                codec: None,
                                path: None,
                                source_id: None,
                                created_at_ms: super::super::now_ms(),
                            },
                        )?;
                        tracks.push(track.clone());
                        track
                    };
                    let canonical = crate::recording_privacy::source_track_id(segment, role)
                        .unwrap_or_default();
                    if selected
                        .is_some_and(|chunk| chunk.track_id.as_deref() != Some(track.id.as_str()))
                    {
                        continue;
                    }
                    let mut cuts = redactions
                        .iter()
                        .filter(|cut| {
                            storage::redaction_applies_to_track_range(cut, &track.id, base, end)
                                || storage::redaction_applies_to_track_range(
                                    cut, &canonical, base, end,
                                )
                        })
                        .map(|cut| (cut.start_offset_ms.max(base), cut.end_offset_ms.min(end)))
                        .collect::<Vec<_>>();
                    cuts.sort_unstable();
                    let mut cursor = base;
                    let mut ranges = Vec::new();
                    for (from, to) in cuts {
                        if from > cursor {
                            ranges.push((cursor, from));
                        }
                        cursor = cursor.max(to);
                    }
                    if cursor < end {
                        ranges.push((cursor, end));
                    }
                    for (start, end) in ranges {
                        if !crate::recording_privacy::range_protection_ready(
                            connection, id, start, end,
                        )? {
                            continue;
                        }
                        let source = cleanup_source(segment, &kind, cleanup.as_ref());
                        let source_stream = if source.file_path != segment.file_path {
                            0
                        } else {
                            stream
                        };
                        sources.push(
                            PreparedSource::new(
                                &recording,
                                source,
                                track.id.clone(),
                                role.into(),
                                source_stream,
                                base,
                                start..end,
                            )
                            .map_err(|_| rusqlite::Error::InvalidQuery)?,
                        );
                    }
                }
            }
            Ok((recording, sources))
        })
        .map_err(|_| "waveform_source_unavailable".into())
}

/// Admission is cheap metadata work. It neither decodes media nor needs AI consent.
pub(crate) fn admit_recording(storage: &AppStorage, id: &str) -> Result<(), String> {
    let (recording, sources) = sources(storage, id)?;
    let input_open = matches!(recording.status.as_str(), "recording" | "paused");
    storage.with_connection(|connection| {
        if let Some(job) = storage::get_latest_processing_job_for_recording(connection, id, "waveform")? {
            connection.execute("INSERT INTO processing_job_runtime(job_id,input_open) VALUES(?1,?2) ON CONFLICT(job_id) DO UPDATE SET input_open=excluded.input_open",rusqlite::params![job.id,input_open])?;
        }
        Ok(())
    }).map_err(|_| "waveform_admission_failed")?;
    let missing = sources
        .into_iter()
        .filter(|source| source.maximum().is_err())
        .collect::<Vec<_>>();
    if missing.is_empty() {
        return Ok(());
    }
    storage.with_connection_mut(|connection| {
        let transaction = connection.transaction()?;
        let now = super::super::now_ms();
        let latest = storage::get_latest_processing_job_for_recording(&transaction, id, "waveform")?;
        if latest.as_ref().is_some_and(|job| job.status == "cancelled" && job.error.as_deref() != Some("ai_processing_privacy_changed")) { return transaction.commit(); }
        let mut job_id = latest.as_ref().filter(|job| matches!(job.status.as_str(),"queued"|"running"|"paused")).map(|job|job.id.clone());
        for source in missing {
            let key = source.path.to_string_lossy().into_owned();
            let existing = transaction.query_row("SELECT c.id,c.job_id,c.status,j.error FROM waveform_sources s JOIN processing_chunks c ON c.id=s.chunk_id JOIN processing_jobs j ON j.id=c.job_id WHERE s.source_key=?1", [&key], |row| Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?,row.get::<_,String>(2)?,row.get::<_,Option<String>>(3)?))).optional()?;
            if let Some((chunk, job, status, error)) = existing {
                if status == "succeeded" {
                    storage::update_processing_chunk_status(&transaction, &chunk, "queued", 0.0, None, now)?;
                    if storage::get_processing_job(&transaction,&job)?.is_some_and(|job| !matches!(job.status.as_str(),"running"|"paused")) { storage::update_processing_job_status(&transaction, &job, "queued", None, now)?; }
                    continue;
                }
                if status != "cancelled" || error.as_deref() != Some("ai_processing_privacy_changed") { continue; }
            }
            let job = if let Some(job) = &job_id { job.clone() } else {
                let job = uuid::Uuid::now_v7().to_string();
                storage::create_processing_job(&transaction, &storage::CreateProcessingJobInput {
                    id: job.clone(), recording_id: id.into(), kind: "waveform".into(), status: "queued".into(),
                    created_at_ms: now, updated_at_ms: now, error: None,
                })?;
                transaction.execute("INSERT INTO processing_job_runtime(job_id,input_open) VALUES(?1,?2)",rusqlite::params![job,input_open])?;
                job_id = Some(job.clone()); job
            };
            let chunk = uuid::Uuid::now_v7().to_string();
            storage::create_processing_chunk(&transaction, &storage::CreateProcessingChunkInput {
                id: chunk.clone(), job_id: job, recording_id: id.into(), source_kind: "waveform".into(),
                segment_id: Some(source.segment.id), track_id: Some(source.track_id), start_offset_ms: Some(source.start_ms), end_offset_ms: Some(source.end_ms),
                status: "queued".into(), progress: 0.0, attempt_count: 0, error_code: None, result_code: None, created_at_ms: now, updated_at_ms: now,
            })?;
            transaction.execute("INSERT INTO waveform_sources(source_key,recording_id,chunk_id) VALUES(?1,?2,?3) ON CONFLICT(source_key) DO UPDATE SET chunk_id=excluded.chunk_id", rusqlite::params![key,id,chunk])?;
        }
        transaction.commit()
    }).map_err(|_| "waveform_admission_failed".into())
}

pub(crate) fn prepare_chunk(
    storage: &AppStorage,
    chunk: &storage::ProcessingChunkRow,
    cancel: &AtomicBool,
) -> Result<(), String> {
    let (_, sources) = recording_sources(storage, &chunk.recording_id, Some(chunk))?;
    let source = sources
        .into_iter()
        .find(|source| {
            Some(source.segment.id.as_str()) == chunk.segment_id.as_deref()
                && Some(source.track_id.as_str()) == chunk.track_id.as_deref()
                && Some(source.start_ms) == chunk.start_offset_ms
                && Some(source.end_ms) == chunk.end_offset_ms
        })
        .ok_or("waveform_source_unavailable")?;
    source.prepare(cancel)
}

pub(crate) fn admit_batch(storage: &AppStorage) -> Result<(), String> {
    let ids = storage
        .with_connection(|connection| {
            let mut query = connection.prepare(
                "SELECT recording_id,generation FROM waveform_admission ORDER BY rowid LIMIT 16",
            )?;
            let rows = query
                .query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(rows)
        })
        .map_err(|_| "waveform_admission_failed")?;
    let mut failure = false;
    for (id, generation) in ids {
        if admit_recording(storage, &id).is_err() {
            failure = true;
            continue;
        }
        storage
            .with_connection(|connection| {
                connection
                    .execute(
                        "DELETE FROM waveform_admission WHERE recording_id=?1 AND generation=?2",
                        rusqlite::params![id, generation],
                    )
                    .map(|_| ())
            })
            .map_err(|_| "waveform_admission_failed")?;
    }
    if failure {
        Err("waveform_admission_failed".into())
    } else {
        Ok(())
    }
}
