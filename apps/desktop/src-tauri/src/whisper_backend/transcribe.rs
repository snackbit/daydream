use std::{
    ffi::CStr,
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

use super::{
    bindings, languages::whisper_language_cstring, model::LoadedWhisperModel, WhisperBackendError,
    WhisperExecutionBackend,
};

pub fn transcribe_wav(
    model_path: &Path,
    audio_path: &Path,
    backend: WhisperExecutionBackend,
    language: &str,
    max_initial_timestamp_seconds: f32,
    word_timestamps: bool,
    cancel: Arc<AtomicBool>,
) -> Result<VerboseTranscription, WhisperBackendError> {
    let mut model = LoadedWhisperModel::load(model_path, backend)?;
    model.transcribe_wav(
        audio_path,
        language,
        max_initial_timestamp_seconds,
        word_timestamps,
        cancel,
    )
}

pub(super) fn run_whisper_full(
    context: *mut bindings::whisper_context,
    samples: &[f32],
    language: &str,
    max_initial_timestamp_seconds: f32,
    word_timestamps: bool,
    cancel: &Arc<AtomicBool>,
) -> Result<VerboseTranscription, WhisperBackendError> {
    let language = whisper_language_cstring(language)?;
    let params = whisper_full_params(
        language.as_c_str(),
        max_initial_timestamp_seconds,
        word_timestamps,
        cancel,
    );

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

pub(super) fn whisper_full_params(
    language: &CStr,
    max_initial_timestamp_seconds: f32,
    word_timestamps: bool,
    cancel: &Arc<AtomicBool>,
) -> bindings::whisper_full_params {
    let mut params = unsafe {
        bindings::whisper_full_default_params(
            bindings::whisper_sampling_strategy_WHISPER_SAMPLING_GREEDY,
        )
    };
    params.translate = false;
    params.language = language.as_ptr();
    params.detect_language = false;
    params.print_realtime = false;
    params.print_progress = false;
    params.print_timestamps = false;
    params.print_special = false;
    params.max_initial_ts = max_initial_timestamp_seconds;
    params.token_timestamps = word_timestamps;
    params.abort_callback = Some(whisper_abort_callback);
    params.abort_callback_user_data = Arc::as_ptr(cancel) as *mut c_void;
    params
}

unsafe extern "C" fn whisper_abort_callback(user_data: *mut c_void) -> bool {
    if user_data.is_null() {
        return false;
    }
    let cancel = unsafe { &*(user_data as *const AtomicBool) };
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

pub(super) fn cancelled_error() -> WhisperBackendError {
    WhisperBackendError {
        code: "ai_transcription_cancelled".to_string(),
        message: "Transcription was cancelled.".to_string(),
    }
}

pub(super) fn empty_transcription(duration: Option<f64>) -> VerboseTranscription {
    VerboseTranscription {
        duration,
        language: None,
        segments: Vec::new(),
        task: Some("transcribe".to_string()),
        text: String::new(),
        words: Vec::new(),
    }
}
