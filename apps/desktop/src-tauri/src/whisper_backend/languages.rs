use std::ffi::{CStr, CString};

use super::{bindings, SupportedWhisperLanguage, WhisperBackendError};

pub fn supported_languages() -> Vec<SupportedWhisperLanguage> {
    let max_id = unsafe { bindings::whisper_lang_max_id() };
    (0..=max_id)
        .filter_map(|id| {
            let code = unsafe { bindings::whisper_lang_str(id) };
            let label = unsafe { bindings::whisper_lang_str_full(id) };
            if code.is_null() || label.is_null() {
                return None;
            }

            Some(SupportedWhisperLanguage {
                id: unsafe { CStr::from_ptr(code) }
                    .to_string_lossy()
                    .into_owned(),
                label: title_case_language_label(
                    &unsafe { CStr::from_ptr(label) }.to_string_lossy(),
                ),
            })
        })
        .collect()
}

pub fn language_supported(language: &str) -> bool {
    let Ok(language) = whisper_language_cstring(language) else {
        return false;
    };

    (unsafe { bindings::whisper_lang_id(language.as_ptr()) }) >= 0
}

pub(super) fn whisper_language_cstring(language: &str) -> Result<CString, WhisperBackendError> {
    let language = if language.trim().is_empty() {
        "auto"
    } else {
        language.trim()
    };
    CString::new(language.as_bytes()).map_err(|_error| WhisperBackendError {
        code: "ai_transcription_language_invalid".to_string(),
        message: "Transcription language contains an invalid byte.".to_string(),
    })
}

fn title_case_language_label(label: &str) -> String {
    label
        .split(' ')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
