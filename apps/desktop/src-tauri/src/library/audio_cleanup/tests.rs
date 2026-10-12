use super::*;
use std::sync::atomic::AtomicBool;

fn music(count: usize) -> Vec<f32> {
    (0..count)
        .map(|i| {
            let t = i as f32 / SAMPLE_RATE as f32;
            let envelope = 0.6 + 0.4 * (t * 2.3).sin();
            envelope
                * (0.20 * (2.0 * std::f32::consts::PI * (173.0 * t + 9.0 * t * t)).sin()
                    + 0.14 * (2.0 * std::f32::consts::PI * (317.0 * t + 3.0 * t * t)).sin()
                    + 0.10 * (2.0 * std::f32::consts::PI * 619.0 * t).sin())
        })
        .collect()
}
fn echo(reference: &[f32]) -> Vec<f32> {
    let delay = SAMPLE_RATE * 80 / 1000;
    let tail = SAMPLE_RATE * 19 / 1000;
    (0..reference.len())
        .map(|i| {
            0.55 * reference.get(i.wrapping_sub(delay)).copied().unwrap_or(0.0)
                + 0.12
                    * reference
                        .get(i.wrapping_sub(delay + tail))
                        .copied()
                        .unwrap_or(0.0)
        })
        .collect()
}
fn power(samples: &[f32]) -> f64 {
    samples
        .iter()
        .map(|sample| (*sample as f64).powi(2))
        .sum::<f64>()
        / samples.len() as f64
}
fn auto() -> AudioCleanupConfig {
    AudioCleanupConfig {
        mode: CleanupMode::Auto,
        ..Default::default()
    }
}

#[test]
fn real_aec_reduces_delayed_reverberant_music() {
    let reference = music(SAMPLE_RATE * 8);
    let input = echo(&reference);
    let result = dsp::clean(&input, 1, &reference, 1, &auto(), &AtomicBool::new(false)).unwrap();
    assert_eq!(result.samples.len(), input.len());
    assert!(result.matched_ms > 5000);
    let from = SAMPLE_RATE * 3;
    let reduction = 10.0 * (power(&input[from..]) / power(&result.samples[from..])).log10();
    assert!(
        reduction >= 12.0,
        "Speaker bleed reduction was only {reduction:.1} dB"
    );
}
#[test]
fn real_aec_preserves_voice_over_music_and_stereo_duration() {
    let reference = music(SAMPLE_RATE * 9);
    let bleed = echo(&reference);
    let voice: Vec<_> = (0..reference.len())
        .map(|i| {
            let t = i as f32 / SAMPLE_RATE as f32;
            if t < 3.0 {
                0.0
            } else {
                0.12 * (2.0 * std::f32::consts::PI * (433.0 * t + 0.3 * t * t)).sin()
            }
        })
        .collect();
    let input: Vec<_> = bleed
        .iter()
        .zip(&voice)
        .flat_map(|(bleed, voice)| [bleed + voice, bleed * 0.8 + voice])
        .collect();
    let result = dsp::clean(&input, 2, &reference, 1, &auto(), &AtomicBool::new(false)).unwrap();
    assert_eq!(result.samples.len(), input.len());
    let from = SAMPLE_RATE * 5;
    let channel: Vec<_> = result
        .samples
        .chunks_exact(2)
        .skip(from)
        .map(|frame| frame[0])
        .collect();
    let expected = &voice[from..];
    let projection = channel
        .iter()
        .zip(expected)
        .map(|(a, b)| *a as f64 * *b as f64)
        .sum::<f64>()
        / expected.iter().map(|a| (*a as f64).powi(2)).sum::<f64>();
    assert!(
        (0.7..=1.3).contains(&projection),
        "Voice gain changed to {projection:.2}"
    );
    let remaining: Vec<_> = channel.iter().zip(expected).map(|(a, b)| a - b).collect();
    let residual_ratio = power(&remaining) / power(&bleed[from..]);
    assert!(residual_ratio<0.35,"Speaker bleed ratio during speech was {residual_ratio:.2}, voice gain {projection:.2}, matched {} ms",result.matched_ms);
}
#[test]
fn unrelated_reference_silence_zero_amount_and_cancellation_are_safe() {
    let reference = music(SAMPLE_RATE * 2);
    let input: Vec<_> = (0..reference.len())
        .map(|i| 0.2 * (i as f32 * 0.107).sin())
        .collect();
    let result = dsp::clean(&input, 1, &reference, 1, &auto(), &AtomicBool::new(false)).unwrap();
    assert_eq!(result.samples, input);
    assert_eq!(result.matched_ms, 0);
    let silent = vec![0.0; reference.len()];
    assert_eq!(
        dsp::clean(&silent, 1, &silent, 1, &auto(), &AtomicBool::new(false))
            .unwrap()
            .samples,
        silent
    );
    let mut config = auto();
    config.amount = 0.0;
    assert_eq!(
        dsp::clean(
            &reference,
            1,
            &reference,
            1,
            &config,
            &AtomicBool::new(false)
        )
        .unwrap()
        .samples,
        reference
    );
    assert!(dsp::clean(&input, 1, &reference, 1, &auto(), &AtomicBool::new(true)).is_err());
}

#[test]
fn changing_delay_reacquires_the_path_and_clipping_keeps_bounded_output() {
    let reference = music(SAMPLE_RATE * 10);
    let input: Vec<_> = (0..reference.len())
        .map(|i| {
            let delay = if i < SAMPLE_RATE * 4 {
                SAMPLE_RATE * 80 / 1000
            } else {
                SAMPLE_RATE * 150 / 1000
            };
            reference.get(i.wrapping_sub(delay)).copied().unwrap_or(0.0) * 0.6
        })
        .collect();
    let output = dsp::clean(&input, 1, &reference, 1, &auto(), &AtomicBool::new(false)).unwrap();
    assert!(
        power(&output.samples[SAMPLE_RATE * 7..]) < power(&input[SAMPLE_RATE * 7..]) * 0.1,
        "The changed delay did not converge"
    );
    let clipped: Vec<_> = reference
        .iter()
        .map(|sample| (sample * 8.0).clamp(-1.0, 1.0))
        .collect();
    let output = dsp::clean(&clipped, 1, &reference, 1, &auto(), &AtomicBool::new(false)).unwrap();
    assert_eq!(output.samples.len(), clipped.len());
    assert!(output
        .samples
        .iter()
        .all(|sample| sample.is_finite() && sample.abs() <= 1.0));
}

#[test]
fn migration_defaults_and_invalid_thresholds_preserve_old_recordings() {
    let mut db = rusqlite::Connection::open_in_memory().unwrap();
    crate::storage::migrations().to_latest(&mut db).unwrap();
    assert_eq!(AudioCleanupConfig::default().mode, CleanupMode::Off);
    let mut config = auto();
    config.match_threshold = f64::NAN;
    assert!(config.validate().is_err());
    config.match_threshold = 0.65;
    config.amount = 1.1;
    assert!(config.validate().is_err());
    assert_eq!(
        db.query_row("SELECT count(*) FROM recording_audio_cleanup", [], |row| {
            row.get::<_, i64>(0)
        })
        .unwrap(),
        0
    );
}

#[test]
fn queued_cancellation_does_not_wait_for_another_cleanup() {
    tauri::async_runtime::block_on(async {
        let active = runtime::worker_slot(&AtomicBool::new(false)).await.unwrap();
        let cancelled = AtomicBool::new(true);
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(1),
            runtime::worker_slot(&cancelled),
        )
        .await
        .unwrap();
        assert!(result.is_err());
        drop(active);
    });
}
