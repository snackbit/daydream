use std::{
    path::Path,
    sync::{atomic::AtomicBool, Arc},
};

use crate::transcription::VerboseTranscription;

#[cfg(test)]
const DEFAULT_WHISPER_MAX_INITIAL_TIMESTAMP_SECONDS: f32 = 3.0;
const WAV_SAMPLE_RATE_HZ: i64 = 16_000;
const SILENCE_GATE_WINDOW_MS: i64 = 100;
const SILENCE_GATE_PRE_ROLL_MS: i64 = 300;
const SILENCE_GATE_POST_ROLL_MS: i64 = 150;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SilenceGateConfig {
    pub merge_gap_ms: i64,
    pub min_speech_ms: i64,
    pub silence_threshold_db: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpeechRange {
    pub end_ms: i64,
    pub start_ms: i64,
}

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SupportedWhisperLanguage {
    pub id: String,
    pub label: String,
}

mod backend;
mod languages;
mod model;
mod silence;
#[cfg(test)]
mod tests;
mod transcribe;
mod wav;

pub use backend::{backend_status, WhisperExecutionBackend};
pub use languages::{language_supported, supported_languages};
pub use model::LoadedWhisperModel;
pub use silence::detect_speech_ranges;

#[allow(dead_code)]
// Kept as a model self-test hook for settings and future diagnostics flows.
pub fn load_and_unload_model(
    path: &Path,
    backend: WhisperExecutionBackend,
) -> Result<(), WhisperBackendError> {
    let _model = LoadedWhisperModel::load(path, backend)?;

    Ok(())
}

#[allow(dead_code)]
// Kept for callers that need one-shot transcription without retaining model state.
pub fn transcribe_wav(
    model_path: &Path,
    audio_path: &Path,
    backend: WhisperExecutionBackend,
    language: &str,
    max_initial_timestamp_seconds: f32,
    word_timestamps: bool,
    cancel: Arc<AtomicBool>,
) -> Result<VerboseTranscription, WhisperBackendError> {
    transcribe::transcribe_wav(
        model_path,
        audio_path,
        backend,
        language,
        max_initial_timestamp_seconds,
        word_timestamps,
        cancel,
    )
}

#[cfg(test)]
use backend::context_params_for_backend;
#[cfg(test)]
use silence::speech_ranges_from_samples;
#[cfg(test)]
use transcribe::{empty_transcription, whisper_full_params};
#[cfg(test)]
use wav::read_mono_pcm16_wav;
