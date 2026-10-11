//! CPU-only speech detection. No transcription model or remote provider is used.
mod model;
mod ranges;
#[cfg(test)]
mod tests;

use super::{wav::read_mono_pcm16_wav, SpeechRange, WhisperBackendError};
use std::{
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

pub const MODEL_SHA256: &str = "2aa269b785eeb53a82983a20501ddf7c1d9c48e33ab63a41391ac6c9f7fb6987";
pub const MODEL_FILE: &str = "vad/ggml-silero-v6.2.0.bin";

pub(crate) fn detect_unpadded_speech_ranges(
    model_path: &Path,
    audio_path: &Path,
    min_speech_ms: i64,
    cancel: Arc<AtomicBool>,
) -> Result<Vec<SpeechRange>, WhisperBackendError> {
    check_cancel(&cancel)?;
    let samples = read_detection_samples(audio_path)?;
    if samples.len() > 30 * 16_000 {
        return Err(error(
            "ai_transcription_vad_input_invalid",
            "Speech detection input exceeds its bounded window.",
        ));
    }
    if samples.is_empty() {
        return Ok(Vec::new());
    }
    let model = model::VadModel::load(model_path)?;
    let detected = model.detect(&samples, min_speech_ms)?;
    check_cancel(&cancel)?;
    let duration = samples.len() as i64 / 16;
    detected
        .into_iter()
        .map(|(start, end)| {
            ranges::padded_range(start, end, duration)?;
            Ok(SpeechRange {
                start_ms: (start as f64 * 10.0).floor() as i64,
                end_ms: ((end as f64 * 10.0).ceil() as i64).min(duration),
            })
        })
        .collect()
}

#[cfg(test)]
pub fn detect_speech_ranges(
    model_path: &Path,
    audio_path: &Path,
    min_speech_ms: i64,
    cancel: Arc<AtomicBool>,
) -> Result<Vec<SpeechRange>, WhisperBackendError> {
    check_cancel(&cancel)?;
    let audio = read_mono_pcm16_wav(audio_path)?;
    detect_speech_samples(model_path, &audio.samples, min_speech_ms, cancel)
}

pub(crate) fn read_detection_samples(path: &Path) -> Result<Vec<f32>, WhisperBackendError> {
    Ok(read_mono_pcm16_wav(path)?.samples)
}

pub(crate) fn write_detection_samples(
    path: &Path,
    samples: &[f32],
) -> Result<(), WhisperBackendError> {
    use std::io::Write;
    if samples.len() > 31 * 16_000 {
        return Err(error(
            "ai_transcription_vad_input_invalid",
            "Speech context exceeds its bounded window.",
        ));
    }
    let length = (samples.len() * 2) as u32;
    let mut file = std::fs::File::create(path).map_err(|_| {
        error(
            "ai_transcription_audio_write_failed",
            "Speech context could not be prepared.",
        )
    })?;
    let mut bytes = Vec::with_capacity(44 + length as usize);
    bytes.extend(b"RIFF");
    bytes.extend((length + 36).to_le_bytes());
    bytes.extend(b"WAVEfmt ");
    bytes.extend(16u32.to_le_bytes());
    bytes.extend(1u16.to_le_bytes());
    bytes.extend(1u16.to_le_bytes());
    bytes.extend(16_000u32.to_le_bytes());
    bytes.extend(32_000u32.to_le_bytes());
    bytes.extend(2u16.to_le_bytes());
    bytes.extend(16u16.to_le_bytes());
    bytes.extend(b"data");
    bytes.extend(length.to_le_bytes());
    for sample in samples {
        bytes.extend(((sample * 32768.0).round().clamp(-32768.0, 32767.0) as i16).to_le_bytes());
    }
    file.write_all(&bytes).map_err(|_| {
        error(
            "ai_transcription_audio_write_failed",
            "Speech context could not be prepared.",
        )
    })
}

pub(crate) fn detect_owned_speech_ranges(
    model_path: &Path,
    audio_path: &Path,
    min_speech_ms: i64,
    cancel: Arc<AtomicBool>,
    owned_start_ms: i64,
    owned_end_ms: i64,
) -> Result<Vec<SpeechRange>, WhisperBackendError> {
    check_cancel(&cancel)?;
    let samples = read_detection_samples(audio_path)?;
    if samples.len() > 31 * 16_000 {
        return Err(error(
            "ai_transcription_vad_input_invalid",
            "Speech context exceeds its bounded window.",
        ));
    }
    if samples.is_empty() {
        return Ok(vec![]);
    }
    let model = model::VadModel::load(model_path)?;
    check_cancel(&cancel)?;
    let detected = model.detect(&samples, min_speech_ms)?;
    check_cancel(&cancel)?;
    let duration = samples.len() as i64 * 1000 / 16000;
    let mut result = Vec::new();
    for (start, end) in detected {
        let padded = ranges::padded_range(start, end, duration)?;
        // Padding alone is not evidence of speech inside the owned section.
        if (start as f64 * 10.0) < owned_end_ms as f64
            && (end as f64 * 10.0) > owned_start_ms as f64
        {
            result.push(padded);
        }
    }
    Ok(result)
}

#[cfg(test)]
pub(crate) fn detect_speech_samples(
    model_path: &Path,
    samples: &[f32],
    min_speech_ms: i64,
    cancel: Arc<AtomicBool>,
) -> Result<Vec<SpeechRange>, WhisperBackendError> {
    check_cancel(&cancel)?;
    if samples.is_empty() {
        return Ok(Vec::new());
    }
    // Each invocation has bounded input and owns the native context until it
    // returns. Cancelling cannot release a privacy lease while C is still using it.
    if samples.len() > 16_000 * 31 {
        return Err(error(
            "ai_transcription_vad_input_invalid",
            "Speech detection input exceeds its bounded window.",
        ));
    }
    let duration_ms = (samples.len() as i64 * 1_000) / 16_000;
    let model = model::VadModel::load(model_path)?;
    check_cancel(&cancel)?;
    let ranges = model.detect(samples, min_speech_ms)?;
    check_cancel(&cancel)?;
    ranges
        .into_iter()
        .map(|(start, end)| ranges::padded_range(start, end, duration_ms))
        .collect()
}

fn check_cancel(cancel: &AtomicBool) -> Result<(), WhisperBackendError> {
    if cancel.load(Ordering::SeqCst) {
        return Err(error(
            "ai_transcription_cancelled",
            "Speech detection was cancelled.",
        ));
    }
    Ok(())
}

fn error(code: &str, message: &str) -> WhisperBackendError {
    WhisperBackendError {
        code: code.into(),
        message: message.into(),
    }
}
