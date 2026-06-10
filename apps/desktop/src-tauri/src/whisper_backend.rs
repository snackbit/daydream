use std::{
    ffi::{CStr, CString},
    os::raw::c_void,
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use crate::transcription::{
    VerboseTranscription, VerboseTranscriptionSegment, VerboseTranscriptionWord,
};

#[allow(
    dead_code,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals
)]
mod bindings {
    include!(concat!(env!("OUT_DIR"), "/whisper_bindings.rs"));
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WhisperBackendStatus {
    pub cuda_compiled: bool,
    pub vulkan_compiled: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WhisperBackendError {
    pub code: String,
    pub message: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WhisperExecutionBackend {
    Cpu,
    Gpu,
}

impl WhisperExecutionBackend {
    pub fn from_setting(value: &str) -> Result<Self, WhisperBackendError> {
        match value {
            "cpu" => Ok(Self::Cpu),
            "vulkan" | "cuda" => Ok(Self::Gpu),
            _ => Err(WhisperBackendError {
                code: "ai_transcription_backend_invalid".to_string(),
                message: "Local transcription backend must be CPU, Vulkan, or CUDA.".to_string(),
            }),
        }
    }
}

pub fn backend_status() -> WhisperBackendStatus {
    WhisperBackendStatus {
        cuda_compiled: cfg!(feature = "whisper-cuda"),
        vulkan_compiled: cfg!(feature = "whisper-vulkan"),
    }
}

pub fn load_and_unload_model(
    path: &Path,
    backend: WhisperExecutionBackend,
) -> Result<(), WhisperBackendError> {
    let _model = LoadedWhisperModel::load(path, backend)?;

    Ok(())
}

pub struct LoadedWhisperModel {
    context: *mut bindings::whisper_context,
}

impl LoadedWhisperModel {
    pub fn load(
        path: &Path,
        backend: WhisperExecutionBackend,
    ) -> Result<Self, WhisperBackendError> {
        let path = CString::new(path.to_string_lossy().as_bytes()).map_err(|_error| {
            WhisperBackendError {
                code: "ai_model_path_invalid".to_string(),
                message: "Model path contains an invalid byte.".to_string(),
            }
        })?;

        let params = context_params_for_backend(backend);
        let context =
            unsafe { bindings::whisper_init_from_file_with_params(path.as_ptr(), params) };
        if context.is_null() {
            return Err(WhisperBackendError {
                code: "ai_model_load_failed".to_string(),
                message: "Whisper model could not be loaded.".to_string(),
            });
        }

        Ok(Self { context })
    }

    pub fn transcribe_wav(
        &mut self,
        audio_path: &Path,
        word_timestamps: bool,
        cancel: Arc<AtomicBool>,
    ) -> Result<VerboseTranscription, WhisperBackendError> {
        let wav = read_mono_pcm16_wav(audio_path)?;
        if cancel.load(Ordering::SeqCst) {
            return Err(cancelled_error());
        }

        run_whisper_full(self.context, &wav.samples, word_timestamps, &cancel).map(
            |mut transcription| {
                transcription.duration = Some(wav.samples.len() as f64 / 16_000.0);
                transcription
            },
        )
    }
}

impl Drop for LoadedWhisperModel {
    fn drop(&mut self) {
        if !self.context.is_null() {
            unsafe { bindings::whisper_free(self.context) };
            self.context = std::ptr::null_mut();
        }
    }
}

#[allow(dead_code)]
fn load_and_unload_model_without_state(
    path: &Path,
    backend: WhisperExecutionBackend,
) -> Result<(), WhisperBackendError> {
    let path =
        CString::new(path.to_string_lossy().as_bytes()).map_err(|_error| WhisperBackendError {
            code: "ai_model_path_invalid".to_string(),
            message: "Model path contains an invalid byte.".to_string(),
        })?;

    let params = context_params_for_backend(backend);
    let context =
        unsafe { bindings::whisper_init_from_file_with_params_no_state(path.as_ptr(), params) };
    if context.is_null() {
        return Err(WhisperBackendError {
            code: "ai_model_load_failed".to_string(),
            message: "Whisper model could not be loaded.".to_string(),
        });
    }

    unsafe { bindings::whisper_free(context) };

    Ok(())
}

#[allow(dead_code)]
pub fn transcribe_wav(
    model_path: &Path,
    audio_path: &Path,
    backend: WhisperExecutionBackend,
    word_timestamps: bool,
    cancel: Arc<AtomicBool>,
) -> Result<VerboseTranscription, WhisperBackendError> {
    let mut model = LoadedWhisperModel::load(model_path, backend)?;
    model.transcribe_wav(audio_path, word_timestamps, cancel)
}

fn context_params_for_backend(
    backend: WhisperExecutionBackend,
) -> bindings::whisper_context_params {
    let mut params = unsafe { bindings::whisper_context_default_params() };
    match backend {
        WhisperExecutionBackend::Cpu => {
            params.use_gpu = false;
            params.flash_attn = false;
            params.gpu_device = 0;
        }
        WhisperExecutionBackend::Gpu => {
            params.use_gpu = true;
            params.gpu_device = 0;
        }
    }
    params
}

fn run_whisper_full(
    context: *mut bindings::whisper_context,
    samples: &[f32],
    word_timestamps: bool,
    cancel: &Arc<AtomicBool>,
) -> Result<VerboseTranscription, WhisperBackendError> {
    let mut params = unsafe {
        bindings::whisper_full_default_params(
            bindings::whisper_sampling_strategy_WHISPER_SAMPLING_GREEDY,
        )
    };
    params.print_realtime = false;
    params.print_progress = false;
    params.print_timestamps = false;
    params.print_special = false;
    params.token_timestamps = word_timestamps;
    params.abort_callback = Some(whisper_abort_callback);
    params.abort_callback_user_data = Arc::as_ptr(cancel) as *mut c_void;

    let status =
        unsafe { bindings::whisper_full(context, params, samples.as_ptr(), samples.len() as i32) };
    if cancel.load(Ordering::SeqCst) {
        return Err(cancelled_error());
    }
    if status != 0 {
        return Err(WhisperBackendError {
            code: "ai_transcription_provider_failed".to_string(),
            message: "Whisper transcription failed.".to_string(),
        });
    }

    let segment_count = unsafe { bindings::whisper_full_n_segments(context) };
    let mut segments = Vec::new();
    let mut words = Vec::new();
    let mut text = String::new();
    for segment_index in 0..segment_count {
        let segment_text = unsafe {
            let raw = bindings::whisper_full_get_segment_text(context, segment_index);
            if raw.is_null() {
                String::new()
            } else {
                CStr::from_ptr(raw).to_string_lossy().into_owned()
            }
        };
        text.push_str(&segment_text);
        let start = whisper_timestamp_seconds(unsafe {
            bindings::whisper_full_get_segment_t0(context, segment_index)
        });
        let end = whisper_timestamp_seconds(unsafe {
            bindings::whisper_full_get_segment_t1(context, segment_index)
        });
        segments.push(VerboseTranscriptionSegment {
            end,
            id: segment_index as i64,
            start,
            text: segment_text,
        });
        if word_timestamps {
            append_segment_words(context, segment_index, &mut words);
        }
    }

    Ok(VerboseTranscription {
        duration: None,
        language: None,
        segments,
        task: Some("transcribe".to_string()),
        text,
        words,
    })
}

unsafe extern "C" fn whisper_abort_callback(user_data: *mut c_void) -> bool {
    if user_data.is_null() {
        return false;
    }
    let cancel = &*(user_data as *const AtomicBool);
    cancel.load(Ordering::SeqCst)
}

fn append_segment_words(
    context: *mut bindings::whisper_context,
    segment_index: i32,
    words: &mut Vec<VerboseTranscriptionWord>,
) {
    let token_count = unsafe { bindings::whisper_full_n_tokens(context, segment_index) };
    for token_index in 0..token_count {
        let token_data =
            unsafe { bindings::whisper_full_get_token_data(context, segment_index, token_index) };
        if token_data.t0 < 0 || token_data.t1 < token_data.t0 {
            continue;
        }
        let token_text = unsafe {
            let raw = bindings::whisper_full_get_token_text(context, segment_index, token_index);
            if raw.is_null() {
                String::new()
            } else {
                CStr::from_ptr(raw).to_string_lossy().trim().to_string()
            }
        };
        if token_text.is_empty() {
            continue;
        }
        words.push(VerboseTranscriptionWord {
            end: whisper_timestamp_seconds(token_data.t1),
            probability: token_data.p.is_finite().then_some(token_data.p as f64),
            start: whisper_timestamp_seconds(token_data.t0),
            word: token_text,
        });
    }
}

fn whisper_timestamp_seconds(value: i64) -> f64 {
    value as f64 * 0.01
}

#[derive(Debug)]
struct WavAudio {
    samples: Vec<f32>,
}

fn read_mono_pcm16_wav(path: &Path) -> Result<WavAudio, WhisperBackendError> {
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

fn cancelled_error() -> WhisperBackendError {
    WhisperBackendError {
        code: "ai_transcription_cancelled".to_string(),
        message: "Transcription was cancelled.".to_string(),
    }
}

#[cfg(test)]
mod tests {
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

    fn test_wav_bytes() -> Vec<u8> {
        let samples = [0_i16, i16::MAX];
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
}
