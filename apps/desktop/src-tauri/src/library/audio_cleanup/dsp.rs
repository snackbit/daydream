//! Only this adapter knows the WebRTC API. Frames are fixed at 10 ms.
use super::{AudioCleanupConfig, SAMPLE_RATE};
use std::sync::atomic::{AtomicBool, Ordering};
use webrtc_audio_processing::{
    config::{EchoCanceller, Pipeline},
    Config, Processor,
};

const FRAME: usize = SAMPLE_RATE / 100;
const DETECTOR_RATE: usize = 2_000;
const DECIMATION: usize = SAMPLE_RATE / DETECTOR_RATE;
const MATCH_WINDOW: usize = DETECTOR_RATE / 2;
const MAX_LAG: isize = DETECTOR_RATE as isize / 2;

pub(super) struct CleanedAudio {
    pub samples: Vec<f32>,
    pub matched_ms: i64,
}

fn processor() -> Result<Processor, String> {
    let processor =
        Processor::new(SAMPLE_RATE as u32).map_err(|_| "Echo cancellation could not start.")?;
    processor.set_config(Config {
        pipeline: Pipeline {
            multi_channel_render: true,
            multi_channel_capture: true,
            ..Default::default()
        },
        echo_canceller: Some(EchoCanceller::Full {
            stream_delay_ms: None,
        }),
        ..Default::default()
    });
    Ok(processor)
}

fn downsample(samples: &[f32], channels: usize, channel: usize) -> Vec<f32> {
    samples
        .chunks_exact(channels * DECIMATION)
        .map(|frame| {
            frame
                .chunks_exact(channels)
                .map(|sample| sample[channel])
                .sum::<f32>()
                / DECIMATION as f32
        })
        .collect()
}

/// Signed lag: microphone[t] resembles reference[t-lag]. Absolute correlation
/// also supports inverted polarity. Silence never qualifies as matching audio.
pub(super) fn delayed_match(mic: &[f32], reference: &[f32], start: usize) -> (f64, isize) {
    let end = (start + MATCH_WINDOW).min(mic.len());
    if end.saturating_sub(start) < MATCH_WINDOW / 2 {
        return (0.0, 0);
    }
    let correlation = |lag: isize| {
        let mut xy = 0.0f64;
        let mut xx = 0.0;
        let mut yy = 0.0;
        let mut n = 0;
        for (offset, &sample) in mic[start..end].iter().enumerate() {
            let index = (start + offset) as isize - lag;
            if index < 0 {
                continue;
            }
            let Some(&render) = reference.get(index as usize) else {
                continue;
            };
            let x = sample as f64;
            let y = render as f64;
            xy += x * y;
            xx += x * x;
            yy += y * y;
            n += 1;
        }
        if n < MATCH_WINDOW / 2 || xx < n as f64 * 1e-6 || yy < n as f64 * 1e-6 {
            0.0
        } else {
            (xy.abs() / (xx * yy).sqrt()).min(1.0)
        }
    };
    let mut best = (0.0, 0);
    // Coarse search keeps correlation out of the 48 kHz hot path.
    for lag in (-MAX_LAG..=MAX_LAG).step_by(4) {
        let score = correlation(lag);
        if score > best.0 {
            best = (score, lag);
        }
    }
    let center = best.1;
    for lag in (center - 3).max(-MAX_LAG)..=(center + 3).min(MAX_LAG) {
        let score = correlation(lag);
        if score > best.0 {
            best = (score, lag);
        }
    }
    best
}

pub(super) fn clean(
    mic: &[f32],
    mic_channels: usize,
    reference: &[f32],
    reference_channels: usize,
    config: &AudioCleanupConfig,
    cancel: &AtomicBool,
) -> Result<CleanedAudio, String> {
    if mic_channels == 0
        || mic_channels > 8
        || reference_channels == 0
        || reference_channels > 8
        || !mic.len().is_multiple_of(mic_channels)
        || !reference.len().is_multiple_of(reference_channels)
    {
        return Err("Audio channel layout is unsupported.".into());
    }
    if mic.iter().chain(reference).any(|value| !value.is_finite()) {
        return Err("Audio samples are invalid.".into());
    }
    let mut output = mic.to_vec();
    let mic_detection = downsample(mic, mic_channels, 0);
    let render_detection: Vec<_> = (0..reference_channels)
        .map(|channel| downsample(reference, reference_channels, channel))
        .collect();
    let processor = processor()?;
    let mut capture = vec![vec![0.0f32; FRAME]; mic_channels];
    let mut render = vec![vec![0.0f32; FRAME]; reference_channels];
    let samples = mic.len() / mic_channels;
    let mut matched_ms = 0;
    let mut active = false;
    let mut hold_until = 0;
    let mut blend = 0.0f32;
    let mut lag = 0isize;
    let mut silent_frames = 0;
    for start in (0..samples).step_by(FRAME) {
        if cancel.load(Ordering::Acquire) {
            return Err("Cleanup was cancelled.".into());
        }
        if start.is_multiple_of(SAMPLE_RATE / 2) {
            let mut best = (0.0, 0);
            for channel in &render_detection {
                let matched = delayed_match(&mic_detection, channel, start / DECIMATION);
                if matched.0 > best.0 {
                    best = matched;
                }
            }
            let threshold = if active {
                (config.match_threshold - 0.1).max(0.1)
            } else {
                config.match_threshold
            };
            if best.0 >= threshold {
                if (best.1 - lag).unsigned_abs() > DETECTOR_RATE / 20 {
                    processor.reinitialize();
                }
                // Keep render 20 ms ahead of its acoustic arrival. Exact
                // alignment can put the direct path before AEC3's causal filter
                // when the correlation estimate rounds past the true delay.
                lag = best.1 - DETECTOR_RATE as isize / 50;
                // Near-end speech can temporarily dominate correlation even
                // though the learned speaker path is unchanged. Let AEC3's
                // double-talk protection own that overlap instead of dropping
                // back to the uncleaned microphone on every spoken phrase.
                active = true;
                hold_until = start + SAMPLE_RATE * 10;
            } else if start >= hold_until {
                active = false;
            }
        }
        let count = FRAME.min(samples - start);
        for (channel, frame) in capture.iter_mut().enumerate() {
            frame.fill(0.0);
            for (offset, sample) in frame.iter_mut().take(count).enumerate() {
                *sample = mic[(start + offset) * mic_channels + channel];
            }
        }
        for (channel, frame) in render.iter_mut().enumerate() {
            frame.fill(0.0);
            for (offset, sample) in frame.iter_mut().enumerate() {
                let index = (start + offset) as isize - lag * DECIMATION as isize;
                if index >= 0 {
                    *sample = reference
                        .get(index as usize * reference_channels + channel)
                        .copied()
                        .unwrap_or(0.0);
                }
            }
        }
        let silent = capture.iter().flatten().all(|sample| sample.abs() < 1e-7)
            && render.iter().flatten().all(|sample| sample.abs() < 1e-7);
        if silent {
            silent_frames += 1;
            if silent_frames == 3 {
                processor.reinitialize();
                active = false;
                blend = 0.0;
                hold_until = 0;
            }
            continue;
        }
        silent_frames = 0;
        processor
            .analyze_render_frame(&render)
            .map_err(|_| "Speaker reference could not be analyzed.")?;
        processor
            .process_capture_frame(&mut capture)
            .map_err(|_| "Echo cancellation failed.")?;
        let target = if active { config.amount as f32 } else { 0.0 };
        if active {
            matched_ms += count as i64 * 1000 / SAMPLE_RATE as i64;
        }
        for offset in 0..count {
            // 50 ms crossfade when detection changes; output remains sample aligned.
            blend += (target - blend).clamp(-1.0 / 2400.0, 1.0 / 2400.0);
            for (channel, frame) in capture.iter().enumerate() {
                let index = (start + offset) * mic_channels + channel;
                output[index] =
                    (mic[index] * (1.0 - blend) + frame[offset] * blend).clamp(-1.0, 1.0);
            }
        }
    }
    Ok(CleanedAudio {
        samples: output,
        matched_ms,
    })
}
