use super::super::{bindings, WhisperBackendError};
use super::{error, MODEL_SHA256};
use sha2::{Digest, Sha256};
use std::{ffi::CString, path::Path, ptr::NonNull};

pub(super) struct VadModel(NonNull<bindings::whisper_vad_context>);
struct VadSegments(NonNull<bindings::whisper_vad_segments>);

impl VadModel {
    pub(super) fn load(path: &Path) -> Result<Self, WhisperBackendError> {
        let bytes = std::fs::read(path).map_err(|_| {
            error(
                "ai_transcription_vad_model_missing",
                "The bundled speech detector is missing. Repair the application installation.",
            )
        })?;
        if hex::encode(Sha256::digest(&bytes)) != MODEL_SHA256 {
            return Err(error("ai_transcription_vad_model_corrupt", "The bundled speech detector failed verification. Repair the application installation."));
        }
        let path = CString::new(path.to_string_lossy().as_bytes()).map_err(|_| {
            error(
                "ai_transcription_vad_model_invalid",
                "The speech detector resource path is invalid.",
            )
        })?;
        // The checked model is read-only; this context stays on its owning
        // blocking thread and is never shared or marked Send/Sync.
        let context = unsafe {
            let mut params = bindings::whisper_vad_default_context_params();
            params.use_gpu = false;
            params.n_threads = 1;
            bindings::whisper_vad_init_from_file_with_params(path.as_ptr(), params)
        };
        NonNull::new(context).map(Self).ok_or_else(|| {
            error(
                "ai_transcription_vad_init_failed",
                "The speech detector could not initialize.",
            )
        })
    }

    pub(super) fn detect(
        &self,
        samples: &[f32],
        min_speech_ms: i64,
    ) -> Result<Vec<(f32, f32)>, WhisperBackendError> {
        let count = i32::try_from(samples.len()).map_err(|_| {
            error(
                "ai_transcription_vad_input_invalid",
                "Speech detection input is too large.",
            )
        })?;
        let pointer = unsafe {
            let mut params = bindings::whisper_vad_default_params();
            params.threshold = 0.5;
            params.min_speech_duration_ms = min_speech_ms.clamp(0, i32::MAX as i64) as i32;
            params.min_silence_duration_ms = 400;
            params.speech_pad_ms = 0;
            params.samples_overlap = 0.0;
            params.max_speech_duration_s = 30.0;
            bindings::whisper_vad_segments_from_samples(
                self.0.as_ptr(),
                params,
                samples.as_ptr(),
                count,
            )
        };
        let segments = VadSegments(NonNull::new(pointer).ok_or_else(|| {
            error(
                "ai_transcription_vad_inference_failed",
                "Speech detection failed. Retry this audio section.",
            )
        })?);
        let count = unsafe { bindings::whisper_vad_segments_n_segments(segments.0.as_ptr()) };
        if count < 0 || count as usize > samples.len() {
            return Err(error(
                "ai_transcription_vad_output_invalid",
                "The speech detector returned invalid ranges.",
            ));
        }
        let mut ranges = Vec::with_capacity(count as usize);
        for index in 0..count {
            ranges.push(unsafe {
                (
                    bindings::whisper_vad_segments_get_segment_t0(segments.0.as_ptr(), index),
                    bindings::whisper_vad_segments_get_segment_t1(segments.0.as_ptr(), index),
                )
            });
        }
        Ok(ranges)
    }
}

impl Drop for VadModel {
    fn drop(&mut self) {
        unsafe {
            bindings::whisper_vad_free(self.0.as_ptr());
        }
    }
}
impl Drop for VadSegments {
    fn drop(&mut self) {
        unsafe {
            bindings::whisper_vad_free_segments(self.0.as_ptr());
        }
    }
}
