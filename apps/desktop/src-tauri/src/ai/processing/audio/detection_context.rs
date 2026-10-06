//! A small eligible halo lets VAD recognize utterances crossing processing cuts.
//! Saved audio is never modified; submitted context and transcript ownership are bounded.
use super::chunk_input::AudioChunkInput;
use super::*;

struct ContextPart {
    segment: RecordingSegmentRow,
    stream: i64,
    start: i64,
    duration: i64,
}
pub(super) struct DetectionContext {
    pub audio: TempAudioFile,
    pub prefix_ms: i64,
    pub duration_ms: i64,
}

pub(super) fn load_context(
    app: &AppHandle,
    chunk: &ProcessingChunkRow,
    input: &AudioChunkInput,
    main: &Path,
    cancel: &AtomicBool,
) -> Result<Option<DetectionContext>, AiError> {
    let (before, after) = app
        .state::<AppStorage>()
        .with_connection(|connection| {
            let segments = storage::list_recording_segments(connection, &chunk.recording_id)?;
            let redactions = storage::list_redactions(connection, &chunk.recording_id)?;
            let index = segments
                .iter()
                .position(|segment| segment.id == input.segment.id)
                .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
            let role = &input.track.kind;
            let eligible = |segment: &RecordingSegmentRow,
                            start: i64,
                            end: i64|
             -> rusqlite::Result<bool> {
                let ids = vec![
                    input.track.id.clone(),
                    crate::recording_privacy::source_track_id(segment, role)
                        .ok_or(rusqlite::Error::InvalidQuery)?,
                ];
                Ok(crate::recording_privacy::range_protection_ready(
                    connection,
                    &chunk.recording_id,
                    start,
                    end,
                )? && !crate::recording_privacy::range_redacted(&redactions, &ids, start, end))
            };
            let before = if input.slice_start_ms > 0 {
                let duration = input.slice_start_ms.min(500);
                eligible(
                    &input.segment,
                    input.chunk_start_offset_ms - duration,
                    input.chunk_start_offset_ms,
                )?
                .then_some(ContextPart {
                    segment: input.segment.clone(),
                    stream: input.stream_index,
                    start: input.slice_start_ms - duration,
                    duration,
                })
            } else if let Some(previous) = index
                .checked_sub(1)
                .and_then(|index| segments.get(index))
                .filter(|previous| {
                    previous.state == "finalized"
                        && previous.ended_at_ms == Some(input.segment.started_at_ms)
                })
            {
                let duration = previous.duration_ms.unwrap_or(0).clamp(0, 500);
                let stream = if role == "microphone" {
                    previous.microphone_audio_stream_index
                } else {
                    previous.desktop_audio_stream_index
                };
                if duration > 0
                    && eligible(
                        previous,
                        input.chunk_start_offset_ms - duration,
                        input.chunk_start_offset_ms,
                    )?
                {
                    stream.map(|stream| ContextPart {
                        segment: previous.clone(),
                        stream,
                        start: previous.duration_ms.unwrap_or(0) - duration,
                        duration,
                    })
                } else {
                    None
                }
            } else {
                None
            };
            let owned_end = input.chunk_start_offset_ms + input.slice_duration_ms;
            let remaining = input.segment.duration_ms.unwrap_or(0)
                - input.slice_start_ms
                - input.slice_duration_ms;
            let after = if remaining > 0 {
                let duration = remaining.min(500);
                eligible(&input.segment, owned_end, owned_end + duration)?.then_some(ContextPart {
                    segment: input.segment.clone(),
                    stream: input.stream_index,
                    start: input.slice_start_ms + input.slice_duration_ms,
                    duration,
                })
            } else if let Some(next) = segments.get(index + 1).filter(|next| {
                next.state == "finalized" && input.segment.ended_at_ms == Some(next.started_at_ms)
            }) {
                let duration = next.duration_ms.unwrap_or(0).clamp(0, 500);
                let stream = if role == "microphone" {
                    next.microphone_audio_stream_index
                } else {
                    next.desktop_audio_stream_index
                };
                if duration > 0 && eligible(next, owned_end, owned_end + duration)? {
                    stream.map(|stream| ContextPart {
                        segment: next.clone(),
                        stream,
                        start: 0,
                        duration,
                    })
                } else {
                    None
                }
            } else {
                None
            };
            Ok((before, after))
        })
        .map_err(storage_error)?;
    if before.is_none() && after.is_none() {
        return Ok(None);
    }
    let extract = |part: ContextPart, suffix: &str| -> Result<Vec<f32>, AiError> {
        check_atomic_cancel(cancel)?;
        let file = TempAudioFile::new(main.with_extension(suffix));
        extract_audio_stream(
            &part.segment.file_path,
            part.stream,
            part.start,
            part.duration,
            file.path(),
            cancel,
        )?;
        whisper_backend::vad::read_detection_samples(file.path()).map_err(whisper_backend_error)
    };
    let mut samples = if let Some(before) = before {
        extract(before, "vad-before.wav")?
    } else {
        Vec::new()
    };
    let prefix = samples.len() as i64 * 1000 / 16_000;
    samples
        .extend(whisper_backend::vad::read_detection_samples(main).map_err(whisper_backend_error)?);
    if let Some(after) = after {
        samples.extend(extract(after, "vad-after.wav")?);
    }
    check_atomic_cancel(cancel)?;
    let audio = TempAudioFile::new(main.with_extension("vad-context.wav"));
    whisper_backend::vad::write_detection_samples(audio.path(), &samples)
        .map_err(whisper_backend_error)?;
    Ok(Some(DetectionContext {
        audio,
        prefix_ms: prefix,
        duration_ms: samples.len() as i64 * 1000 / 16000,
    }))
}
