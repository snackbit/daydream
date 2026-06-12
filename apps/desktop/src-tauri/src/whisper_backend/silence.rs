use std::path::Path;

use super::{
    wav::read_mono_pcm16_wav, SilenceGateConfig, SpeechRange, WhisperBackendError,
    SILENCE_GATE_POST_ROLL_MS, SILENCE_GATE_PRE_ROLL_MS, SILENCE_GATE_WINDOW_MS,
    WAV_SAMPLE_RATE_HZ,
};

pub fn detect_speech_ranges(
    audio_path: &Path,
    config: SilenceGateConfig,
) -> Result<Vec<SpeechRange>, WhisperBackendError> {
    let wav = read_mono_pcm16_wav(audio_path)?;
    Ok(speech_ranges_from_samples(&wav.samples, config))
}

pub(super) fn speech_ranges_from_samples(
    samples: &[f32],
    config: SilenceGateConfig,
) -> Vec<SpeechRange> {
    if samples.is_empty() {
        return Vec::new();
    }

    let window_samples = ((WAV_SAMPLE_RATE_HZ * SILENCE_GATE_WINDOW_MS) / 1_000).max(1) as usize;
    let mut ranges = Vec::new();
    let mut active_start_ms = None;
    let mut active_end_ms = 0;
    for (window_index, window) in samples.chunks(window_samples).enumerate() {
        let start_ms = samples_to_ms((window_index * window_samples) as i64);
        let end_ms = samples_to_ms(((window_index * window_samples) + window.len()) as i64);
        let active = rms_dbfs(window) >= config.silence_threshold_db;
        match (active_start_ms, active) {
            (None, true) => {
                active_start_ms = Some(start_ms);
                active_end_ms = end_ms;
            }
            (Some(_), true) => active_end_ms = end_ms,
            (Some(start), false) => {
                ranges.push(SpeechRange {
                    end_ms: active_end_ms,
                    start_ms: start,
                });
                active_start_ms = None;
            }
            _ => {}
        }
    }
    if let Some(start_ms) = active_start_ms {
        ranges.push(SpeechRange {
            end_ms: active_end_ms,
            start_ms,
        });
    }

    let minimum_duration_ms = config.min_speech_ms.max(0);
    let filtered = ranges
        .into_iter()
        .filter(|range| range.end_ms.saturating_sub(range.start_ms) >= minimum_duration_ms)
        .collect::<Vec<_>>();
    let merged = merge_speech_ranges(filtered, config.merge_gap_ms.max(0));
    let duration_ms = samples_to_ms(samples.len() as i64);

    merged
        .into_iter()
        .map(|range| SpeechRange {
            start_ms: (range.start_ms - SILENCE_GATE_PRE_ROLL_MS).max(0),
            end_ms: (range.end_ms + SILENCE_GATE_POST_ROLL_MS).min(duration_ms),
        })
        .filter(|range| range.end_ms > range.start_ms)
        .collect()
}

fn merge_speech_ranges(ranges: Vec<SpeechRange>, merge_gap_ms: i64) -> Vec<SpeechRange> {
    let mut merged: Vec<SpeechRange> = Vec::new();
    for range in ranges {
        let Some(last) = merged.last_mut() else {
            merged.push(range);
            continue;
        };
        if range.start_ms.saturating_sub(last.end_ms) <= merge_gap_ms {
            last.end_ms = last.end_ms.max(range.end_ms);
        } else {
            merged.push(range);
        }
    }
    merged
}

fn rms_dbfs(samples: &[f32]) -> f64 {
    if samples.is_empty() {
        return f64::NEG_INFINITY;
    }
    let sum_squares = samples
        .iter()
        .map(|sample| {
            let value = f64::from(*sample);
            value * value
        })
        .sum::<f64>();
    let rms = (sum_squares / samples.len() as f64).sqrt();

    20.0 * rms.max(1e-12).log10()
}

fn samples_to_ms(samples: i64) -> i64 {
    ((samples * 1_000) + (WAV_SAMPLE_RATE_HZ / 2)) / WAV_SAMPLE_RATE_HZ
}
