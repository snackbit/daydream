use std::{
    ffi::CString,
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use crate::transcription::VerboseTranscription;

use super::{
    backend::context_params_for_backend,
    bindings,
    transcribe::run_whisper_full,
    transcribe::{cancelled_error, empty_transcription},
    wav::read_mono_pcm16_wav,
    WhisperBackendError, WhisperExecutionBackend, WAV_SAMPLE_RATE_HZ,
};

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
        language: &str,
        max_initial_timestamp_seconds: f32,
        word_timestamps: bool,
        cancel: Arc<AtomicBool>,
    ) -> Result<VerboseTranscription, WhisperBackendError> {
        let wav = read_mono_pcm16_wav(audio_path)?;
        if cancel.load(Ordering::SeqCst) {
            return Err(cancelled_error());
        }
        if wav.samples.is_empty() {
            return Ok(empty_transcription(Some(0.0)));
        }

        run_whisper_full(
            self.context,
            &wav.samples,
            language,
            max_initial_timestamp_seconds,
            word_timestamps,
            &cancel,
        )
        .map(|mut transcription| {
            transcription.duration = Some(wav.samples.len() as f64 / WAV_SAMPLE_RATE_HZ as f64);
            transcription
        })
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
// Kept as an integration probe for whisper.cpp no-state loading.
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
