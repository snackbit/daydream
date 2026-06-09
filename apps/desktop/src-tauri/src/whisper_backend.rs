use std::{ffi::CString, path::Path};

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

pub fn backend_status() -> WhisperBackendStatus {
    WhisperBackendStatus {
        cuda_compiled: cfg!(feature = "whisper-cuda"),
        vulkan_compiled: cfg!(feature = "whisper-vulkan"),
    }
}

pub fn load_and_unload_model(path: &Path) -> Result<(), WhisperBackendError> {
    let path =
        CString::new(path.to_string_lossy().as_bytes()).map_err(|_error| WhisperBackendError {
            code: "ai_model_path_invalid".to_string(),
            message: "Model path contains an invalid byte.".to_string(),
        })?;

    let params = unsafe { bindings::whisper_context_default_params() };
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
