use super::{extraction::extract_audio_stream, *};

pub(super) struct SpeechRangeAudio {
    pub(super) path: PathBuf,
    pub(super) temp_audio: Option<TempAudioFile>,
}

#[cfg(test)]
pub(super) fn plan_transcription_ranges(
    model_path: &Path,
    audio_path: &Path,
    slice_duration_ms: i64,
    silence_gate_enabled: bool,
    config: whisper_backend::SilenceGateConfig,
    cancel: Arc<AtomicBool>,
) -> Result<Vec<whisper_backend::SpeechRange>, AiError> {
    plan_transcription_ranges_with_context(
        model_path,
        audio_path,
        slice_duration_ms,
        silence_gate_enabled,
        config,
        cancel,
        None,
    )
}

#[cfg(test)]
pub(super) fn plan_transcription_ranges_with_context(
    model_path: &Path,
    audio_path: &Path,
    slice_duration_ms: i64,
    silence_gate_enabled: bool,
    config: whisper_backend::SilenceGateConfig,
    cancel: Arc<AtomicBool>,
    context: Option<(Vec<f32>, i64)>,
) -> Result<Vec<whisper_backend::SpeechRange>, AiError> {
    let speech = if let Some((samples, prefix)) = context {
        let ranges = whisper_backend::vad::detect_speech_samples(
            model_path,
            &samples,
            config.min_speech_ms,
            cancel,
        )
        .map_err(whisper_backend_error)?;
        ranges
            .into_iter()
            .filter_map(|range| {
                let start_ms = (range.start_ms - prefix).max(0);
                let end_ms = (range.end_ms - prefix).min(slice_duration_ms);
                (end_ms > start_ms).then_some(whisper_backend::SpeechRange { start_ms, end_ms })
            })
            .collect()
    } else {
        whisper_backend::vad::detect_speech_ranges(
            model_path,
            audio_path,
            config.min_speech_ms,
            cancel,
        )
        .map_err(whisper_backend_error)?
    };
    let loudness = if silence_gate_enabled {
        whisper_backend::detect_speech_ranges(audio_path, config).map_err(whisper_backend_error)?
    } else {
        vec![whisper_backend::SpeechRange {
            end_ms: slice_duration_ms,
            start_ms: 0,
        }]
    };
    // Intersect inside each utterance. Never merge after intersection or padding:
    // a long loudness range must not reconnect separate VAD utterances.
    Ok(speech
        .into_iter()
        .flat_map(|speech| {
            loudness.iter().filter_map(move |loud| {
                let start_ms = speech.start_ms.max(loud.start_ms);
                let end_ms = speech.end_ms.min(loud.end_ms).min(slice_duration_ms);
                (end_ms > start_ms).then_some(whisper_backend::SpeechRange { start_ms, end_ms })
            })
        })
        .collect())
}

pub(super) fn plan_submitted_transcription_ranges(
    model_path: &Path,
    audio_path: &Path,
    owned_duration: i64,
    prefix: i64,
    silence_gate_enabled: bool,
    config: whisper_backend::SilenceGateConfig,
    cancel: Arc<AtomicBool>,
) -> Result<Vec<whisper_backend::SpeechRange>, AiError> {
    let speech = whisper_backend::vad::detect_owned_speech_ranges(
        model_path,
        audio_path,
        config.min_speech_ms,
        cancel,
        prefix,
        prefix + owned_duration,
    )
    .map_err(whisper_backend_error)?;
    if !silence_gate_enabled {
        return Ok(speech);
    }
    let loudness =
        whisper_backend::detect_speech_ranges(audio_path, config).map_err(whisper_backend_error)?;
    Ok(speech
        .into_iter()
        .flat_map(|speech| {
            loudness.iter().filter_map(move |loud| {
                let start_ms = speech.start_ms.max(loud.start_ms);
                let end_ms = speech.end_ms.min(loud.end_ms);
                (end_ms > start_ms && end_ms > prefix && start_ms < prefix + owned_duration)
                    .then_some(whisper_backend::SpeechRange { start_ms, end_ms })
            })
        })
        .collect())
}

pub(super) fn audio_path_for_speech_range(
    app: &AppHandle,
    chunk: &ProcessingChunkRow,
    full_duration_ms: i64,
    range: &whisper_backend::SpeechRange,
    range_index: usize,
    full_audio_path: &Path,
    cancel: &AtomicBool,
) -> Result<SpeechRangeAudio, AiError> {
    if range.start_ms == 0 && range.end_ms == full_duration_ms {
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
        full_audio_path.to_str().ok_or_else(|| {
            ai_error(
                "ai_processing_audio_path_invalid",
                "The temporary audio path is invalid.",
            )
        })?,
        0,
        range.start_ms,
        range_duration_ms,
        temp_audio.path(),
        cancel,
    )?;

    Ok(SpeechRangeAudio {
        path: temp_audio.path().to_path_buf(),
        temp_audio: Some(temp_audio),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn typing_is_not_speech_even_when_the_optional_loudness_gate_is_disabled() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/vad/typing.wav");
        let model =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/vad/ggml-silero-v6.2.0.bin");
        let ranges = plan_transcription_ranges(
            &model,
            &path,
            2_000,
            false,
            whisper_backend::SilenceGateConfig {
                merge_gap_ms: 1_000,
                min_speech_ms: 250,
                silence_threshold_db: -40.0,
            },
            Arc::new(AtomicBool::new(false)),
        )
        .unwrap();
        assert!(ranges.is_empty(), "Typing must not be submitted to Whisper");
    }
    #[test]
    fn utterances_stay_separate_even_with_a_long_loudness_merge_gap() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let speech = whisper_backend::vad::read_detection_samples(
            &root.join("tests/fixtures/vad/reply.wav"),
        )
        .unwrap();
        let mut samples = speech.clone();
        samples.extend(vec![0.0; 32_000]);
        samples.extend(speech);
        let duration = samples.len() as i64 * 1000 / 16000;
        let ranges = plan_transcription_ranges_with_context(
            &root.join("resources/vad/ggml-silero-v6.2.0.bin"),
            Path::new("not-read-with-disabled-loudness"),
            duration,
            false,
            whisper_backend::SilenceGateConfig {
                merge_gap_ms: 10_000,
                min_speech_ms: 250,
                silence_threshold_db: -40.0,
            },
            Arc::new(AtomicBool::new(false)),
            Some((samples, 0)),
        )
        .unwrap();
        assert_eq!(
            ranges.len(),
            2,
            "Loudness merging must not reconnect separate VAD utterances."
        );
        assert!(ranges[0].end_ms < ranges[1].start_ms);
    }
}
