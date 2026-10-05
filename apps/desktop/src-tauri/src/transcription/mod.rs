use std::{
    path::{Path, PathBuf},
    time::Duration,
};

mod http;
mod local_protocol;
mod local_receipts;
mod local_requests;
mod local_scratch;
mod local_server;
mod local_worker;
mod provider;
mod remote;
mod types;

use crate::whisper_backend;
pub use local_server::run_whisper_worker_process;
pub use provider::TranscriptionProviderManager;
pub use types::{
    LocalTranscriptionInput, LocalTranscriptionWarmup, TranscriptionError, TranscriptionRequest,
    VerboseTranscription, VerboseTranscriptionSegment, VerboseTranscriptionWord,
};

const TRANSCRIPTION_PATH: &str = "/v1/audio/transcriptions";
const LOCAL_WARMUP_PATH: &str = "/internal/local-transcription/warm";
const MAX_TRANSCRIPTION_UPLOAD_BYTES: u64 = 128 * 1024 * 1024;
const HTTP_HEADER_LIMIT_BYTES: usize = 32 * 1024;
const LOCAL_BACKEND_FORM_FIELD: &str = "daydream_local_backend";
const LOCAL_WORKER_STARTUP_TIMEOUT: Duration = Duration::from_secs(10);
const LOCAL_WORKER_WARMUP_TIMEOUT: Duration = Duration::from_secs(30 * 60);

fn transcription_endpoint_url(base_url: &str) -> Result<String, TranscriptionError> {
    let trimmed = base_url.trim().trim_end_matches('/');
    if !(trimmed.starts_with("http://") || trimmed.starts_with("https://")) {
        return Err(transcription_error(
            "ai_transcription_remote_unconfigured",
            "Remote transcription endpoint must be an HTTP URL.",
        ));
    }
    if trimmed.ends_with(TRANSCRIPTION_PATH) {
        Ok(trimmed.to_string())
    } else if trimmed.ends_with("/v1") {
        Ok(format!("{trimmed}/audio/transcriptions"))
    } else {
        Ok(format!("{trimmed}{TRANSCRIPTION_PATH}"))
    }
}

fn local_whisper_model_path(app_data_dir: &Path, model_id: &str) -> PathBuf {
    app_data_dir
        .join("models")
        .join("whisper.cpp")
        .join(format!("ggml-{model_id}.bin"))
}

fn transcription_error(code: &str, message: &str) -> TranscriptionError {
    TranscriptionError {
        code: code.to_string(),
        message: message.to_string(),
    }
}

fn file_error(error: std::io::Error) -> TranscriptionError {
    transcription_error(
        "ai_transcription_file_failed",
        &format!("Transcription file operation failed: {error}"),
    )
}

fn local_bind_error(error: std::io::Error) -> TranscriptionError {
    transcription_error(
        "ai_transcription_local_server_unavailable",
        &format!("Local transcription endpoint could not start: {error}"),
    )
}

fn network_io_error(error: std::io::Error) -> TranscriptionError {
    transcription_error(
        "ai_transcription_network_failed",
        &format!("Transcription network operation failed: {error}"),
    )
}

fn network_error(error: reqwest::Error) -> TranscriptionError {
    if error.is_timeout() {
        return transcription_error(
            "ai_transcription_timeout",
            "Transcription provider request timed out.",
        );
    }
    transcription_error(
        "ai_transcription_provider_failed",
        "Transcription provider request failed.",
    )
}

fn worker_spawn_error(error: impl std::fmt::Display) -> TranscriptionError {
    transcription_error(
        "ai_transcription_local_worker_unavailable",
        &format!("Local transcription worker could not start: {error}"),
    )
}

fn worker_unavailable_error() -> TranscriptionError {
    transcription_error(
        "ai_transcription_local_worker_unavailable",
        "Local transcription worker is unavailable.",
    )
}

fn request_build_error(error: impl std::fmt::Display) -> TranscriptionError {
    transcription_error(
        "ai_transcription_request_invalid",
        &format!("Transcription request could not be built: {error}"),
    )
}

fn whisper_error(error: whisper_backend::WhisperBackendError) -> TranscriptionError {
    TranscriptionError {
        code: error.code,
        message: error.message,
    }
}

#[cfg(test)]
mod tests;
