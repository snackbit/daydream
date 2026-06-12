use std::path::Path;

use super::WhisperBackendError;

#[derive(Debug)]
pub(super) struct WavAudio {
    pub(super) samples: Vec<f32>,
}

pub(super) fn read_mono_pcm16_wav(path: &Path) -> Result<WavAudio, WhisperBackendError> {
    let bytes = std::fs::read(path).map_err(|error| WhisperBackendError {
        code: "ai_transcription_audio_read_failed".to_string(),
        message: format!("Audio file could not be read: {error}"),
    })?;
    if bytes.len() < 44 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err(audio_invalid_error());
    }

    let mut cursor = 12;
    let mut audio_format = None;
    let mut channels = None;
    let mut sample_rate = None;
    let mut bits_per_sample = None;
    let mut data = None;
    while cursor + 8 <= bytes.len() {
        let chunk_id = &bytes[cursor..cursor + 4];
        let chunk_size =
            u32::from_le_bytes(bytes[cursor + 4..cursor + 8].try_into().unwrap()) as usize;
        cursor += 8;
        if cursor + chunk_size > bytes.len() {
            return Err(audio_invalid_error());
        }
        let chunk = &bytes[cursor..cursor + chunk_size];
        match chunk_id {
            b"fmt " if chunk.len() >= 16 => {
                audio_format = Some(u16::from_le_bytes(chunk[0..2].try_into().unwrap()));
                channels = Some(u16::from_le_bytes(chunk[2..4].try_into().unwrap()));
                sample_rate = Some(u32::from_le_bytes(chunk[4..8].try_into().unwrap()));
                bits_per_sample = Some(u16::from_le_bytes(chunk[14..16].try_into().unwrap()));
            }
            b"data" => data = Some(chunk.to_vec()),
            _ => {}
        }
        cursor += chunk_size + (chunk_size % 2);
    }

    if audio_format != Some(1)
        || channels != Some(1)
        || sample_rate != Some(16_000)
        || bits_per_sample != Some(16)
    {
        return Err(audio_invalid_error());
    }
    let data = data.ok_or_else(audio_invalid_error)?;
    if data.len() % 2 != 0 {
        return Err(audio_invalid_error());
    }
    let samples = data
        .chunks_exact(2)
        .map(|sample| i16::from_le_bytes(sample.try_into().unwrap()) as f32 / 32768.0)
        .collect();

    Ok(WavAudio { samples })
}

fn audio_invalid_error() -> WhisperBackendError {
    WhisperBackendError {
        code: "ai_transcription_audio_invalid".to_string(),
        message: "Audio must be 16 kHz mono PCM WAV for local transcription.".to_string(),
    }
}
