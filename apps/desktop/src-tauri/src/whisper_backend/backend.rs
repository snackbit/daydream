use super::{bindings, WhisperBackendError, WhisperBackendStatus};

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

pub(super) fn context_params_for_backend(
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
