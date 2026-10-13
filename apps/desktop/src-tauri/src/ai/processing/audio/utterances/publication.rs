use super::super::*;

/// A stopped or superseded attempt is cancellation, not a fatal storage failure.
pub(super) fn checkpoint(
    storage: &AppStorage,
    chunk: &ProcessingChunkRow,
    write: impl FnOnce(&rusqlite::Connection) -> rusqlite::Result<bool>,
) -> Result<(), AiError> {
    let saved = storage
        .with_connection_mut(|connection| {
            let tx = connection.transaction()?;
            if !crate::recording_privacy::chunk_active(&tx, chunk)? {
                return Ok(false);
            }
            if !write(&tx)? {
                return Ok(false);
            }
            tx.commit()?;
            Ok(true)
        })
        .map_err(storage_error)?;
    if saved {
        Ok(())
    } else {
        Err(ai_error(
            "ai_processing_cancelled",
            "The audio processing attempt is no longer eligible.",
        ))
    }
}

pub(super) fn prune_ineligible_units(
    storage: &AppStorage,
    chunk: &ProcessingChunkRow,
    track: &RecordingTrackRow,
    runs: &[super::input::SourceRun],
) -> Result<(), AiError> {
    storage.with_connection_mut(|connection|{
        let tx=connection.transaction()?;
        for unit in storage::speech_units(&tx,&chunk.job_id,&track.id)? {
            let eligible=runs.iter().any(|run|unit.start_ms>=run.start&&unit.end_ms<=run.end&&unit.submitted_start_ms>=run.start&&unit.submitted_end_ms<=run.end);
            if eligible{continue;}
            tx.execute("DELETE FROM transcript_segments WHERE id IN(SELECT transcript_id FROM processing_speech_unit_transcripts WHERE unit_id=?1)",[&unit.id])?;
            tx.execute("DELETE FROM processing_audio_ranges WHERE chunk_id IN(SELECT chunk_id FROM processing_speech_unit_chunks WHERE unit_id=?1) AND config_fingerprint=?2",rusqlite::params![unit.id,unit.fingerprint])?;
            tx.execute("DELETE FROM processing_speech_units WHERE id=?1",[&unit.id])?;
        }
        tx.commit()
    }).map_err(storage_error)
}

pub(super) fn publish(
    storage: &AppStorage,
    chunk: &ProcessingChunkRow,
    track: &RecordingTrackRow,
    settings: &settings::AppSettings,
    unit: &storage::SpeechUnit,
    value: transcription::VerboseTranscription,
) -> Result<(), AiError> {
    let duration = unit.submitted_end_ms - unit.submitted_start_ms;
    let owned = [(
        (unit.start_ms - unit.submitted_start_ms) as f64 / 1000.0,
        (unit.end_ms - unit.submitted_start_ms) as f64 / 1000.0,
    )];
    let value = transcription::constrain_owned_transcription(value, duration, &owned);
    let model = selected_transcription_model(settings)?;
    checkpoint(storage, chunk, |tx| {
        if !unit_eligible(tx, &chunk.recording_id, track, unit)? {
            return Ok(false);
        }
        let published: bool = tx.query_row(
            "SELECT status='published' FROM processing_speech_units WHERE id=?1",
            [&unit.id],
            |r| r.get(0),
        )?;
        if !published {
            transcript_persistence::write_speech_unit(
                tx, chunk, track, settings, &model, unit, value,
            )?;
            storage::publish_speech_unit(tx, unit, chunk, track)?;
        }
        Ok(true)
    })
}

pub(super) fn unit_eligible(
    tx: &rusqlite::Connection,
    recording_id: &str,
    track: &RecordingTrackRow,
    unit: &storage::SpeechUnit,
) -> rusqlite::Result<bool> {
    let (eligible_start, eligible_end) = if unit.submitted_end_ms == unit.submitted_start_ms {
        (unit.start_ms, unit.end_ms)
    } else {
        (unit.submitted_start_ms, unit.submitted_end_ms)
    };
    let redactions = storage::list_redactions(tx, recording_id)?;
    let segments = storage::list_recording_segments(tx, recording_id)?;
    let bases = storage::segment_playback_bases(&segments);
    for (source, base) in segments.iter().zip(bases) {
        let start = eligible_start.max(base);
        let end = eligible_end.min(base + source.duration_ms.unwrap_or(0));
        if end <= start {
            continue;
        }
        let mut ids = vec![track.id.clone()];
        if let Some(id) = crate::recording_privacy::source_track_id(source, &track.kind) {
            ids.push(id);
        }
        if crate::recording_privacy::range_redacted(&redactions, &ids, start, end) {
            return Ok(false);
        }
    }
    if !crate::recording_privacy::range_protection_ready(
        tx,
        recording_id,
        eligible_start,
        eligible_end,
    )? {
        return Ok(false);
    }
    Ok(true)
}

pub(super) fn finish_with_published_neighbor(
    storage: &AppStorage,
    chunk: &ProcessingChunkRow,
    track: &RecordingTrackRow,
    settings: &settings::AppSettings,
    left: &storage::SpeechUnit,
    mut value: transcription::VerboseTranscription,
) -> Result<bool, AiError> {
    let next = storage
        .with_connection(|c| {
            Ok(storage::speech_units(c, &chunk.job_id, &track.id)?
                .into_iter()
                .find(|unit| unit.start_ms == left.end_ms && unit.status == "published"))
        })
        .map_err(storage_error)?;
    let Some(next) = next else {
        return Ok(false);
    };
    let mut reference = storage
        .with_connection(|c| super::reference::published_speech_unit(c, &next))
        .map_err(storage_error)?;
    super::stitching::reconcile_published_neighbor(
        &mut value,
        left.submitted_start_ms,
        &mut reference,
        next.submitted_start_ms,
        left.submitted_start_ms.max(next.submitted_start_ms),
        left.submitted_end_ms.min(next.submitted_end_ms),
    );
    publish(storage, chunk, track, settings, left, value)?;
    Ok(true)
}
