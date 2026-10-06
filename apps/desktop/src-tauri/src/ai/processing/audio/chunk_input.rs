use super::*;

#[derive(Clone)]
pub(super) struct AudioChunkInput {
    pub(super) chunk_start_offset_ms: i64,
    pub(super) segment: RecordingSegmentRow,
    pub(super) slice_duration_ms: i64,
    pub(super) slice_start_ms: i64,
    pub(super) stream_index: i64,
    pub(super) track: RecordingTrackRow,
}

pub(super) fn load_audio_chunk_input(
    app: &AppHandle,
    chunk: &ProcessingChunkRow,
) -> Result<AudioChunkInput, AiError> {
    let storage = app.state::<AppStorage>();
    let (segment, track, segment_playback_offset_ms) = storage
        .with_connection(|connection| {
            let segment_id = chunk
                .segment_id
                .as_deref()
                .ok_or(rusqlite::Error::InvalidQuery)?;
            let segments = storage::list_recording_segments(connection, &chunk.recording_id)?;
            let segment_playback_offset_ms = playback_offset_for_segment(&segments, segment_id)
                .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
            let segment = segments
                .into_iter()
                .find(|candidate| candidate.id == segment_id)
                .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
            let track_id = chunk
                .track_id
                .as_deref()
                .ok_or(rusqlite::Error::InvalidQuery)?;
            let track = storage::get_recording_track(connection, track_id)?
                .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
            Ok((segment, track, segment_playback_offset_ms))
        })
        .map_err(storage_error)?;
    let stream_index = match track.kind.as_str() {
        "desktop-audio" => segment.desktop_audio_stream_index,
        "microphone" => segment.microphone_audio_stream_index,
        _ => None,
    }
    .ok_or_else(|| {
        ai_error(
            "ai_processing_audio_stream_missing",
            "Audio stream is missing.",
        )
    })?;
    let chunk_start_offset_ms = chunk.start_offset_ms.ok_or_else(|| {
        ai_error(
            "ai_processing_chunk_offset_missing",
            "Processing chunk start offset is missing.",
        )
    })?;
    let chunk_end_offset_ms = chunk.end_offset_ms.ok_or_else(|| {
        ai_error(
            "ai_processing_chunk_offset_missing",
            "Processing chunk end offset is missing.",
        )
    })?;
    let slice_start_ms = chunk_start_offset_ms
        .checked_sub(segment_playback_offset_ms)
        .ok_or_else(|| {
            ai_error(
                "ai_processing_chunk_offset_invalid",
                "Processing chunk offset is outside its segment.",
            )
        })?;
    let slice_duration_ms = chunk_end_offset_ms
        .checked_sub(chunk_start_offset_ms)
        .filter(|duration| *duration > 0)
        .ok_or_else(|| {
            ai_error(
                "ai_processing_chunk_duration_invalid",
                "Processing chunk duration is invalid.",
            )
        })?;

    Ok(AudioChunkInput {
        chunk_start_offset_ms,
        segment,
        slice_duration_ms,
        slice_start_ms,
        stream_index,
        track,
    })
}
