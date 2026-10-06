use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet, HashSet, VecDeque},
    ffi::{OsStr, OsString},
    fs::{self, File, OpenOptions},
    io::{Cursor, Read, Write},
    path::{Path, PathBuf},
    process::Command,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex, OnceLock,
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use futures_util::StreamExt;
use reqwest::{
    header::{AUTHORIZATION, CONTENT_LENGTH, ETAG, IF_RANGE, RANGE},
    StatusCode,
};
use sha1::{Digest, Sha1};
use tauri::{AppHandle, Emitter, Manager, State};
use uuid::Uuid;

use crate::{
    ffmpeg,
    notifications::Notification,
    secret_crypto, settings,
    storage::{
        self, ActivityIntervalRow, AiModelAssetRow, AppStorage, CaptureEventRow,
        CreateActivityIntervalInput, CreateFrameSimilarityEntryInput, CreateProcessingChunkInput,
        CreateProcessingJobInput, CreateTranscriptSegmentInput, CreateTranscriptWordInput,
        CreateVisualFrameProcessingStateInput, CreateVisualRegionInput, NotificationRow,
        OcrRegionRow, ProcessingChunkRow, ProcessingJobRow, RecordingSegmentRow, RecordingTrackRow,
        RedactionRow, SaveAiModelAssetInput, SaveAppSettingInput, TranscriptSegmentRow,
        TranscriptWordRow, UpsertRecordingTrackInput, VisualFrameProcessingStateRow,
        VlmObservationRow,
    },
    transcription::{
        self, LocalTranscriptionWarmup, TranscriptionProviderManager, TranscriptionRequest,
    },
    whisper_backend,
};

pub const AI_MODELS_CHANGED_EVENT: &str = "ai-models-changed";
pub const AI_PROCESSING_CHANGED_EVENT: &str = "ai-processing-changed";
mod notification_helpers;
use notification_helpers::{
    model_download_cancel_action, model_download_retry_action, notification_from_row,
    processing_failed_actions, processing_paused_actions, processing_retry_actions,
    processing_running_actions, save_job_progress_notification, save_job_running_notification,
    save_model_download_notification, save_model_notification, save_processing_notification,
    status_to_notification_status, ModelDownloadNotificationInput, ProcessingNotificationInput,
};

const MODEL_PROGRESS_UPDATE_INTERVAL: Duration = Duration::from_millis(500);
const MODEL_PROGRESS_UPDATE_BYTES: i64 = 4 * 1024 * 1024;
const AI_PROCESSING_JOB_KIND: &str = "ai-processing";
const MAX_TRANSCRIPTION_CHUNK_DURATION_MS: i64 = 30_000;
const PROCESSING_WORKER_INTERVAL: Duration = Duration::from_millis(500);
const TRANSCRIPTION_TIMEOUT: Duration = Duration::from_secs(30 * 60);
const VLM_TIMEOUT: Duration = Duration::from_secs(120);
const FRAME_DELTA_SAMPLE_INTERVAL_MS: i64 = 1_000;
const FRAME_DELTA_WIDTH: usize = 64;
const FRAME_DELTA_HEIGHT: usize = 36;
const FRAME_DELTA_THRESHOLD: f64 = 0.005;
const VISUAL_DELTA_PIXEL_DIFF_THRESHOLD: u32 = 30;
const VISUAL_DELTA_MIN_CHANGED_PIXELS_PER_TILE: usize = 8;
const VISUAL_DELTA_TILE_CHANGE_RATIO: f64 = 0.0005;
const VISUAL_ABRUPT_TRANSITION_RATIO: f64 = 0.25;
const VISUAL_FULL_FRAME_CHANGED_RATIO: f64 = 0.75;
const VLM_MAX_IMAGE_LONG_EDGE_PX: usize = 1_600;
const VLM_MAX_OCR_CANDIDATES: usize = 80;
const VLM_MAX_OCR_TEXT_CHARS: usize = 8_000;
const VLM_PROMPT_VERSION: &str = "m2-visual-observation-v2";
const TRANSCRIPTION_RESULT_SKIPPED_SILENCE: &str = "skipped-silence";

pub(crate) mod commands;
mod models;
mod ocr_runtime;
mod ocr_types;
pub(crate) use ocr_types::*;
pub(crate) mod processing;
#[cfg(test)]
pub(crate) use processing::cancel_job_if_inactive;
pub(crate) use processing::{retry_processing_units_for_action, RetryProcessingUnitsInput};
mod results;
mod types;
pub(super) use models::*;
pub use models::{recover_model_downloads, AiModelManager};
pub(crate) use ocr_runtime::*;
pub(crate) use processing::cancel_recording_processing_for_privacy;
pub(crate) use processing::reprocess_recording_audio;
pub(crate) use processing::reprocess_recording_ocr;
use processing::*;
pub(crate) use processing::{
    admit_live_recording, enqueue_recording_ai_processing, finish_live_recording,
};
use results::*;
pub(crate) use results::{
    emit_processing_state, processing_job_from_row, processing_state_for_recording,
    visible_transcript_segments_for_recording, visible_visual_observation_rows_for_recording,
    visual_observations_for_recording, visual_observations_for_recording_with_regions,
};
pub use types::{
    ActivityEvent, ActivityInterval, AiBackendStatus, AiError, AiLocalTranscriptionStatus,
    AiModelAsset, AiModelAssetList, AiOcrStatus, AiProviderStatus, AiTranscriptionLanguageStatus,
    CancelAiModelDownloadInput, DeleteAiModelInput, DeleteAiProviderSecretInput,
    DownloadAiModelInput, GetRecordingTranscriptInput, ListProcessingJobsInput, OcrRegion,
    ProcessingChunk, ProcessingJob, ProcessingJobActionInput, ProcessingJobList, RecordingActivity,
    RecordingAiProcessingState, RecordingAiResultInput, RecordingTranscript,
    RecordingTranscriptSegment, RecordingTranscriptTrack, RecordingTranscriptWord,
    RecordingTranscriptionChunkState, RecordingTranscriptionTrackState,
    RecordingVisualObservations, SaveAiProviderSecretInput, SaveAiProviderSecretResult,
    SelfTestAiModelInput, StartRecordingAiProcessingInput, VisualFrameProcessingState,
    VisualRegion, VlmObservation, VlmState,
};

fn storage_error(error: rusqlite::Error) -> AiError {
    AiError {
        code: "ai_storage_failed".to_string(),
        message: format!("AI storage failed: {error}"),
    }
}

fn settings_error(error: settings::AppSettingsError) -> AiError {
    AiError {
        code: error.code,
        message: error.message,
    }
}

fn transcription_error(error: transcription::TranscriptionError) -> AiError {
    AiError {
        code: error.code,
        message: error.message,
    }
}

fn whisper_backend_error(error: whisper_backend::WhisperBackendError) -> AiError {
    AiError {
        code: error.code,
        message: error.message,
    }
}

fn file_error(error: std::io::Error) -> AiError {
    AiError {
        code: "ai_file_failed".to_string(),
        message: format!("AI file operation failed: {error}"),
    }
}

fn download_error(error: reqwest::Error) -> AiError {
    AiError {
        code: "ai_model_download_failed".to_string(),
        message: if let Some(status) = error.status() {
            format!("Model download failed with HTTP status {status}.")
        } else {
            "Model download failed before a response completed.".to_string()
        },
    }
}

fn http_error(status: StatusCode) -> AiError {
    AiError {
        code: "ai_model_download_http_failed".to_string(),
        message: format!("Model download failed with HTTP status {status}."),
    }
}

fn secret_error(error: secret_crypto::SecretCryptoError) -> AiError {
    AiError {
        code: error.code,
        message: error.message,
    }
}

fn ai_error(code: &str, message: &str) -> AiError {
    AiError {
        code: code.to_string(),
        message: message.to_string(),
    }
}

fn check_atomic_cancel(cancel: &AtomicBool) -> Result<(), AiError> {
    if cancel.load(Ordering::SeqCst) {
        return Err(ai_error(
            "ai_transcription_cancelled",
            "Transcription request was cancelled.",
        ));
    }
    Ok(())
}

fn check_processing_cancel(cancel: &AtomicBool) -> Result<(), AiError> {
    if cancel.load(Ordering::SeqCst) {
        return Err(ai_error(
            "ai_processing_cancelled",
            "AI processing was cancelled.",
        ));
    }
    Ok(())
}

fn seconds_to_ms(seconds: f64) -> i64 {
    (seconds * 1000.0).round().max(0.0) as i64
}

fn format_seconds(milliseconds: i64) -> String {
    format!("{:.3}", milliseconds.max(0) as f64 / 1000.0)
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default()
}

fn elapsed_ms(started: Instant) -> u64 {
    started.elapsed().as_millis().try_into().unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests;
