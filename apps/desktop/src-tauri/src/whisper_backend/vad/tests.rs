use super::super::wav::read_mono_pcm16_wav;
use super::*;

fn model_path() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/vad/ggml-silero-v6.2.0.bin")
}
fn fixture(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/vad")
        .join(name)
}

#[test]
fn bundled_detector_rejects_typing_and_breath_like_noise() {
    let model = model::VadModel::load(&model_path()).unwrap();
    let typing = read_mono_pcm16_wav(&fixture("typing.wav")).unwrap();
    assert!(model.detect(&typing.samples, 250).unwrap().is_empty());
    let mut seed = 7u32;
    let mut filtered = 0f32;
    let breath: Vec<f32> = (0..32_000)
        .map(|i| {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            let noise = (seed as f32 / u32::MAX as f32) * 2.0 - 1.0;
            filtered += 0.08 * (noise - filtered);
            filtered * 0.3 * (std::f32::consts::PI * i as f32 / 32_000.0).sin()
        })
        .collect();
    assert!(model.detect(&breath, 250).unwrap().is_empty());
}

#[test]
fn real_detector_keeps_quiet_speech_and_speech_with_typing() {
    let model = model::VadModel::load(&model_path()).unwrap();
    let speech = read_mono_pcm16_wav(&fixture("speech.wav")).unwrap();
    let quiet: Vec<f32> = speech.samples.iter().map(|sample| sample * 0.25).collect();
    assert!(!model.detect(&quiet, 250).unwrap().is_empty());
    let typing = read_mono_pcm16_wav(&fixture("typing.wav")).unwrap();
    let mixture: Vec<f32> = speech
        .samples
        .iter()
        .enumerate()
        .map(|(i, sample)| {
            (sample + typing.samples[i % typing.samples.len()] * 0.2).clamp(-1.0, 1.0)
        })
        .collect();
    assert!(!model.detect(&mixture, 250).unwrap().is_empty());
}

#[test]
fn real_detector_keeps_a_brief_reply_at_a_chunk_boundary() {
    let speech = read_mono_pcm16_wav(&fixture("reply.wav")).unwrap();
    let ranges = detect_speech_samples(
        &model_path(),
        &speech.samples,
        250,
        Arc::new(AtomicBool::new(false)),
    )
    .unwrap();
    assert!(
        !ranges.is_empty(),
        "A synthetic short reply must survive balanced VAD."
    );
    assert!(ranges
        .iter()
        .all(|range| range.end_ms <= speech.samples.len() as i64 * 1000 / 16000));
    let mut context = vec![0.0; 29_800 * 16];
    context.extend(speech.samples);
    context.resize(31_000 * 16, 0.0);
    let ranges = detect_speech_samples(
        &model_path(),
        &context,
        250,
        Arc::new(AtomicBool::new(false)),
    )
    .unwrap();
    assert!(
        ranges
            .iter()
            .any(|range| range.start_ms < 30_000 && range.end_ms > 30_000),
        "The detector needs context across the owned processing boundary."
    );
}

#[test]
fn speech_ranges_end_near_speech_instead_of_the_chunk_end() {
    let model = model::VadModel::load(&model_path()).unwrap();
    let speech = read_mono_pcm16_wav(&fixture("speech.wav")).unwrap();
    let mut samples = vec![0.0; 16_000];
    samples.extend_from_slice(&speech.samples);
    samples.resize(30 * 16_000, 0.0);
    let ranges = model.detect(&samples, 250).unwrap();
    assert!(!ranges.is_empty());
    let speech_end_ms = 1_000 + (speech.samples.len() as i64 * 1_000) / 16_000;
    for (start, end) in ranges {
        let range = ranges::padded_range(start, end, 30_000).unwrap();
        assert!(range.end_ms <= speech_end_ms + 550);
        assert!(range.start_ms >= 0 && range.end_ms < 10_000);
    }
}

#[test]
fn centiseconds_asymmetric_padding_and_invalid_timestamps() {
    assert_eq!(
        ranges::padded_range(123.0, 143.0, 2_000).unwrap(),
        SpeechRange {
            start_ms: 930,
            end_ms: 1580
        }
    );
    assert_eq!(
        ranges::padded_range(0.0, 195.0, 2_000).unwrap(),
        SpeechRange {
            start_ms: 0,
            end_ms: 2_000
        }
    );
    for (start, end) in [
        (f32::NAN, 5.0),
        (5.0, f32::INFINITY),
        (-1.0, 3.0),
        (5.0, 4.0),
        (300.0, 400.0),
    ] {
        assert!(ranges::padded_range(start, end, 2_000).is_err());
    }
}

#[test]
fn cancelled_detection_does_not_open_audio_or_load_a_model() {
    let error = detect_speech_ranges(
        Path::new("missing-model"),
        Path::new("missing-audio"),
        250,
        Arc::new(AtomicBool::new(true)),
    )
    .unwrap_err();
    assert_eq!(error.code, "ai_transcription_cancelled");
    assert_eq!(
        model::VadModel::load(Path::new("missing-model"))
            .err()
            .unwrap()
            .code,
        "ai_transcription_vad_model_missing"
    );
    assert_eq!(
        model::VadModel::load(&fixture("typing.wav"))
            .err()
            .unwrap()
            .code,
        "ai_transcription_vad_model_corrupt"
    );
}
