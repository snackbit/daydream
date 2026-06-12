use super::{chunk_input::AudioChunkInput, extraction::extract_audio_stream, *};

pub(super) struct SpeechRangeAudio {
    pub(super) path: PathBuf,
    pub(super) temp_audio: Option<TempAudioFile>,
}

pub(super) fn plan_transcription_ranges(
    audio_path: &Path,
    slice_duration_ms: i64,
    silence_gate_enabled: bool,
    merge_gap_ms: i64,
    min_speech_ms: i64,
    silence_threshold_db: f64,
) -> Result<Vec<whisper_backend::SpeechRange>, AiError> {
    let ranges = if silence_gate_enabled {
        whisper_backend::detect_speech_ranges(
            audio_path,
            whisper_backend::SilenceGateConfig {
                merge_gap_ms,
                min_speech_ms,
                silence_threshold_db,
            },
        )
        .map_err(whisper_backend_error)?
    } else {
        vec![whisper_backend::SpeechRange {
            end_ms: slice_duration_ms,
            start_ms: 0,
        }]
    }
    .into_iter()
    .filter(|range| range.end_ms > range.start_ms)
    .collect();

    Ok(ranges)
}

pub(super) fn audio_path_for_speech_range(
    app: &AppHandle,
    chunk: &ProcessingChunkRow,
    input: &AudioChunkInput,
    range: &whisper_backend::SpeechRange,
    range_index: usize,
    full_audio_path: &Path,
    cancel: &AtomicBool,
) -> Result<SpeechRangeAudio, AiError> {
    if range.start_ms == 0 && range.end_ms == input.slice_duration_ms {
        return Ok(SpeechRangeAudio {
            path: full_audio_path.to_path_buf(),
            temp_audio: None,
        });
    }

    let range_duration_ms = range.end_ms.checked_sub(range.start_ms).ok_or_else(|| {
        ai_error(
            "ai_processing_audio_range_invalid",
            "Detected transcription range is invalid.",
        )
    })?;
    let range_path = processing_temp_audio_range_path(app, &chunk.job_id, &chunk.id, range_index)?;
    let temp_audio = TempAudioFile::new(range_path);
    extract_audio_stream(
        &input.segment.file_path,
        input.stream_index,
        input.slice_start_ms + range.start_ms,
        range_duration_ms,
        temp_audio.path(),
        cancel,
    )?;

    Ok(SpeechRangeAudio {
        path: temp_audio.path().to_path_buf(),
        temp_audio: Some(temp_audio),
    })
}
