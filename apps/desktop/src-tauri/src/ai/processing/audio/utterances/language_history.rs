//! Durable, source-scoped language decisions; no captured text in this ledger.
use super::language::{Decision, State, HISTORY_MS};
use super::*;
use rusqlite::OptionalExtension;

fn decode(json: &str) -> rusqlite::Result<Decision> {
    serde_json::from_str(json).map_err(|_| rusqlite::Error::InvalidQuery)
}
pub(super) fn decision(
    connection: &rusqlite::Connection,
    id: &str,
) -> rusqlite::Result<Option<Decision>> {
    connection
        .query_row(
            "SELECT decision_json FROM processing_speech_languages WHERE unit_id=?1",
            [id],
            |row| row.get::<_, String>(0),
        )
        .optional()?
        .map(|json| decode(&json))
        .transpose()
}
pub(super) fn eligible(
    connection: &rusqlite::Connection,
    chunk: &ProcessingChunkRow,
    track: &RecordingTrackRow,
    current: &storage::SpeechUnit,
    candidate: &storage::SpeechUnit,
) -> rusqlite::Result<bool> {
    if candidate.job_id != chunk.job_id
        || candidate.track_id != track.id
        || candidate.fingerprint != current.fingerprint
        || candidate.start_ms < current.start_ms.saturating_sub(HISTORY_MS)
        || candidate.end_ms > current.start_ms
        || !matches!(candidate.status.as_str(), "published" | "staged")
    {
        return Ok(false);
    }
    if candidate.status == "published" && !connection.query_row("SELECT EXISTS(SELECT 1 FROM processing_speech_unit_transcripts u JOIN published_transcript_segments t ON t.id=u.transcript_id WHERE u.unit_id=?1)", [&candidate.id], |row| row.get::<_,bool>(0))? { return Ok(false); }
    publication::unit_eligible(connection, &chunk.recording_id, track, candidate)
}
pub(super) fn supported(
    connection: &rusqlite::Connection,
    chunk: &ProcessingChunkRow,
    track: &RecordingTrackRow,
    unit: &storage::SpeechUnit,
    value: &Decision,
) -> rusqlite::Result<bool> {
    for id in value.state.supporting_units() {
        if id == unit.id {
            if !publication::unit_eligible(connection, &chunk.recording_id, track, unit)? {
                return Ok(false);
            }
        } else {
            let Some(candidate) = storage::speech_unit(connection, id)? else {
                return Ok(false);
            };
            if !eligible(connection, chunk, track, unit, &candidate)? {
                return Ok(false);
            }
        }
    }
    Ok(true)
}
pub(super) fn previous(
    connection: &rusqlite::Connection,
    chunk: &ProcessingChunkRow,
    track: &RecordingTrackRow,
    unit: &storage::SpeechUnit,
) -> rusqlite::Result<State> {
    let mut query = connection.prepare("SELECT u.id,d.decision_json FROM processing_speech_languages d JOIN processing_speech_units u ON u.id=d.unit_id WHERE u.job_id=?1 AND u.track_id=?2 AND u.config_fingerprint=?3 AND u.end_ms<=?4 AND u.start_ms>=?4-?5 AND u.status IN('published','staged') ORDER BY u.end_ms DESC LIMIT 8")?;
    let rows = query
        .query_map(
            rusqlite::params![
                chunk.job_id,
                track.id,
                unit.fingerprint,
                unit.start_ms,
                HISTORY_MS
            ],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    for (id, json) in rows {
        let candidate =
            storage::speech_unit(connection, &id)?.ok_or(rusqlite::Error::InvalidQuery)?;
        let value = decode(&json)?;
        if eligible(connection, chunk, track, unit, &candidate)?
            && supported(connection, chunk, track, unit, &value)?
        {
            return Ok(value.state);
        }
    }
    Ok(State::default())
}
pub(super) fn save(
    connection: &rusqlite::Connection,
    id: &str,
    value: &Decision,
) -> rusqlite::Result<()> {
    let json = serde_json::to_string(value).map_err(|_| rusqlite::Error::InvalidQuery)?;
    connection.execute("INSERT INTO processing_speech_languages(unit_id,decision_json) VALUES(?1,?2) ON CONFLICT(unit_id) DO UPDATE SET decision_json=excluded.decision_json", rusqlite::params![id,json])?;
    Ok(())
}
pub(super) fn save_support(
    connection: &rusqlite::Connection,
    chunk: &ProcessingChunkRow,
    track: &RecordingTrackRow,
    id: &str,
    state: &State,
) -> rusqlite::Result<()> {
    connection.execute(
        "DELETE FROM processing_speech_language_support WHERE unit_id=?1",
        [id],
    )?;
    let segments = storage::list_recording_segments(connection, &chunk.recording_id)?;
    let bases = storage::segment_playback_bases(&segments);
    for source_id in state.confirmed_units() {
        let mut recorded = false;
        let source =
            storage::speech_unit(connection, source_id)?.ok_or(rusqlite::Error::InvalidQuery)?;
        for (segment, base) in segments.iter().zip(&bases) {
            let start = source.submitted_start_ms.max(*base);
            let end = source
                .submitted_end_ms
                .min(base.saturating_add(segment.duration_ms.unwrap_or_default()));
            if end <= start {
                continue;
            }
            let mut ids = vec![track.id.clone()];
            if let Some(canonical) = crate::recording_privacy::source_track_id(segment, &track.kind)
            {
                ids.push(canonical);
            }
            let ids = serde_json::to_string(&ids).map_err(|_| rusqlite::Error::InvalidQuery)?;
            connection.execute("INSERT INTO processing_speech_language_support(unit_id,source_unit_id,recording_id,track_ids_json,start_ms,end_ms) VALUES(?1,?2,?3,?4,?5,?6)", rusqlite::params![id,source_id,chunk.recording_id,ids,start,end])?;
            recorded = true;
        }
        if !recorded {
            return Err(rusqlite::Error::InvalidQuery);
        }
    }
    Ok(())
}
pub(super) fn stage(
    storage: &AppStorage,
    chunk: &ProcessingChunkRow,
    track: &RecordingTrackRow,
    unit: &storage::SpeechUnit,
    value: &transcription::VerboseTranscription,
    language: &Decision,
    dependent: bool,
) -> Result<(), AiError> {
    let json = serde_json::to_string(value).map_err(|_| {
        ai_error(
            "ai_processing_config_invalid",
            "Speech checkpoint could not be saved.",
        )
    })?;
    publication::checkpoint(storage, chunk, |connection| {
        if !publication::unit_eligible(connection, &chunk.recording_id, track, unit)?
            || (dependent && !supported(connection, chunk, track, unit, language)?)
        {
            return Ok(false);
        }
        storage::stage_speech_unit(connection, &unit.id, &json)?;
        save(connection, &unit.id, language)?;
        if dependent {
            save_support(connection, chunk, track, &unit.id, &language.state)?;
        }
        Ok(true)
    })
}
pub(super) struct Replacement<'a> {
    pub current: &'a storage::SpeechUnit,
    pub unit: &'a storage::SpeechUnit,
    pub value: transcription::VerboseTranscription,
    pub language: &'a Decision,
}
pub(super) fn replace(
    storage: &AppStorage,
    chunk: &ProcessingChunkRow,
    track: &RecordingTrackRow,
    settings: &settings::AppSettings,
    input: Replacement<'_>,
) -> Result<(), AiError> {
    let Replacement {
        current,
        unit,
        value,
        language,
    } = input;
    let model = selected_transcription_model(settings)?;
    let duration = unit.submitted_end_ms - unit.submitted_start_ms;
    let value = if unit.status == "published" {
        transcription::constrain_owned_transcription(
            value,
            duration,
            &[(
                (unit.start_ms - unit.submitted_start_ms) as f64 / 1000.,
                (unit.end_ms - unit.submitted_start_ms) as f64 / 1000.,
            )],
        )
    } else {
        value
    };
    publication::checkpoint(storage, chunk, |connection| {
        if !eligible(connection, chunk, track, current, unit)?
            || !supported(connection, chunk, track, current, language)?
        {
            return Ok(false);
        }
        if unit.status == "published" {
            let owner = storage::get_processing_chunk(connection, &unit.owner_chunk_id)?
                .ok_or(rusqlite::Error::InvalidQuery)?;
            connection.execute("DELETE FROM transcript_segments WHERE id IN(SELECT transcript_id FROM processing_speech_unit_transcripts WHERE unit_id=?1)", [&unit.id])?;
            transcript_persistence::write_speech_unit(
                connection, &owner, track, settings, &model, unit, value,
            )?;
            storage::publish_speech_unit(connection, unit, &owner, track)?;
        } else {
            let json = serde_json::to_string(&value).map_err(|_| rusqlite::Error::InvalidQuery)?;
            storage::stage_speech_unit(connection, &unit.id, &json)?;
        }
        save(connection, &unit.id, language)?;
        save_support(connection, chunk, track, &unit.id, &language.state)?;
        Ok(true)
    })
}
