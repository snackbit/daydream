use std::ffi::{CStr, CString};
use std::sync::{atomic::AtomicBool, Arc};

use super::*;

#[test]
fn cpu_backend_disables_gpu_context() {
    let params = context_params_for_backend(WhisperExecutionBackend::Cpu);

    assert!(!params.use_gpu);
    assert!(!params.flash_attn);
    assert_eq!(params.gpu_device, 0);
}

#[test]
fn gpu_backend_enables_gpu_context() {
    let params = context_params_for_backend(WhisperExecutionBackend::Gpu);

    assert!(params.use_gpu);
    assert_eq!(params.gpu_device, 0);
}

#[test]
fn whisper_full_params_auto_detects_and_transcribes() {
    let cancel = Arc::new(AtomicBool::new(false));
    let language = CString::new("auto").expect("language");

    let params = whisper_full_params(
        language.as_c_str(),
        DEFAULT_WHISPER_MAX_INITIAL_TIMESTAMP_SECONDS,
        true,
        &cancel,
    );

    assert_eq!(
        params.max_initial_ts,
        DEFAULT_WHISPER_MAX_INITIAL_TIMESTAMP_SECONDS
    );
    assert!(params.token_timestamps);
    assert!(!params.detect_language);
    assert_eq!(
        unsafe { CStr::from_ptr(params.language) }
            .to_string_lossy()
            .as_ref(),
        "auto"
    );
    assert!(!params.translate);
}

#[test]
fn silence_gate_skips_silent_and_low_noise_audio() {
    let config = test_silence_gate_config();

    assert!(speech_ranges_from_samples(&vec![0.0; samples_for_ms(1_000)], config).is_empty());
    assert!(speech_ranges_from_samples(&vec![0.001; samples_for_ms(1_000)], config).is_empty());
}

#[test]
fn silence_gate_detects_speech_range_after_initial_silence() {
    let config = test_silence_gate_config();
    let mut samples = vec![0.0; samples_for_ms(1_000)];
    samples.extend(vec![0.1; samples_for_ms(500)]);
    samples.extend(vec![0.0; samples_for_ms(1_000)]);

    let ranges = speech_ranges_from_samples(&samples, config);

    assert_eq!(
        ranges,
        vec![SpeechRange {
            start_ms: 700,
            end_ms: 1_650,
        }]
    );
}

#[test]
fn silence_gate_merges_close_speech_ranges() {
    let config = SilenceGateConfig {
        merge_gap_ms: 500,
        min_speech_ms: 100,
        silence_threshold_db: -45.0,
    };
    let mut samples = vec![0.0; samples_for_ms(500)];
    samples.extend(vec![0.1; samples_for_ms(300)]);
    samples.extend(vec![0.0; samples_for_ms(300)]);
    samples.extend(vec![0.1; samples_for_ms(300)]);
    samples.extend(vec![0.0; samples_for_ms(500)]);

    let ranges = speech_ranges_from_samples(&samples, config);

    assert_eq!(
        ranges,
        vec![SpeechRange {
            start_ms: 200,
            end_ms: 1_550,
        }]
    );
}

#[test]
fn backend_setting_maps_supported_values() {
    assert_eq!(
        WhisperExecutionBackend::from_setting("cpu").expect("cpu"),
        WhisperExecutionBackend::Cpu
    );
    assert_eq!(
        WhisperExecutionBackend::from_setting("vulkan").expect("vulkan"),
        WhisperExecutionBackend::Gpu
    );
    assert_eq!(
        WhisperExecutionBackend::from_setting("cuda").expect("cuda"),
        WhisperExecutionBackend::Gpu
    );
    assert_eq!(
        WhisperExecutionBackend::from_setting("metal")
            .expect_err("invalid backend")
            .code,
        "ai_transcription_backend_invalid"
    );
}

#[test]
fn wav_reader_accepts_16khz_mono_pcm() {
    let path = temp_wav_path();
    std::fs::write(&path, test_wav_bytes()).expect("write wav");

    let wav = read_mono_pcm16_wav(&path).expect("read wav");

    assert_eq!(wav.samples.len(), 2);
    assert_eq!(wav.samples[0], 0.0);
    assert!(wav.samples[1] > 0.9);
    let _ = std::fs::remove_file(path);
}

#[test]
fn wav_reader_accepts_empty_pcm_for_silent_transcription() {
    let path = temp_wav_path();
    std::fs::write(&path, test_wav_bytes_with_samples(&[])).expect("write wav");

    let wav = read_mono_pcm16_wav(&path).expect("read wav");
    let transcription = empty_transcription(Some(0.0));

    assert!(wav.samples.is_empty());
    assert_eq!(transcription.duration, Some(0.0));
    assert_eq!(transcription.text, "");
    assert!(transcription.segments.is_empty());
    assert!(transcription.words.is_empty());
    let _ = std::fs::remove_file(path);
}

#[test]
fn wav_reader_rejects_non_wav_data() {
    let path = temp_wav_path();
    std::fs::write(&path, b"not wav").expect("write invalid");

    let error = read_mono_pcm16_wav(&path).expect_err("invalid wav");

    assert_eq!(error.code, "ai_transcription_audio_invalid");
    let _ = std::fs::remove_file(path);
}

fn temp_wav_path() -> std::path::PathBuf {
    std::env::temp_dir().join(format!("daydream-wav-test-{}.wav", uuid::Uuid::now_v7()))
}

fn test_silence_gate_config() -> SilenceGateConfig {
    SilenceGateConfig {
        merge_gap_ms: 300,
        min_speech_ms: 200,
        silence_threshold_db: -45.0,
    }
}

fn samples_for_ms(duration_ms: i64) -> usize {
    ((WAV_SAMPLE_RATE_HZ * duration_ms) / 1_000) as usize
}

fn test_wav_bytes() -> Vec<u8> {
    test_wav_bytes_with_samples(&[0_i16, i16::MAX])
}

fn test_wav_bytes_with_samples(samples: &[i16]) -> Vec<u8> {
    let data_len = samples.len() * 2;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_len as u32).to_le_bytes());
    bytes.extend_from_slice(b"WAVE");
    bytes.extend_from_slice(b"fmt ");
    bytes.extend_from_slice(&16_u32.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&16_000_u32.to_le_bytes());
    bytes.extend_from_slice(&32_000_u32.to_le_bytes());
    bytes.extend_from_slice(&2_u16.to_le_bytes());
    bytes.extend_from_slice(&16_u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&(data_len as u32).to_le_bytes());
    for sample in samples {
        bytes.extend_from_slice(&sample.to_le_bytes());
    }
    bytes
}
