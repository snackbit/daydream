use tauri::AppHandle;

use crate::storage::AppStorage;

use super::super::{
    capture_runtime::CapturePreviewSession,
    errors::unavailable_error,
    manager::RecordingSegmentRuntime,
    manager::{unavailable_backend_health, RecorderManager},
    media_writer::MediaSegmentInput,
    types::{
        AudioSourceList, CaptureSourceList, RecorderBackendHealth, RecorderError, RecorderScene,
    },
};

pub(crate) struct PreparedLiveCapture;

#[derive(Debug)]
pub(crate) struct RunningPreviewSession {
    pub(crate) session: Box<dyn CapturePreviewSession>,
    pub(crate) source_ids: Vec<String>,
}

pub async fn collect_backend_health() -> RecorderBackendHealth {
    unavailable_backend_health()
}

pub fn get_capture_sources(manager: &RecorderManager) -> Result<CaptureSourceList, RecorderError> {
    manager.capture_sources()
}

pub async fn select_capture_sources(
    _storage: &AppStorage,
    _manager: &RecorderManager,
    _force_chooser: bool,
) -> Result<CaptureSourceList, RecorderError> {
    Err(unavailable_error())
}

pub async fn get_audio_sources() -> Result<AudioSourceList, RecorderError> {
    Ok(AudioSourceList {
        sources: Vec::new(),
    })
}

pub async fn prepare_live_capture(
    _storage: &AppStorage,
    _scene: &RecorderScene,
) -> Result<PreparedLiveCapture, RecorderError> {
    Err(unavailable_error())
}

pub async fn start_preview_session(
    _app: AppHandle,
    _storage: &AppStorage,
    _scene: &RecorderScene,
) -> Result<RunningPreviewSession, RecorderError> {
    Err(unavailable_error())
}

pub fn start_live_segment(
    _input: &MediaSegmentInput,
    _prepared: PreparedLiveCapture,
) -> Result<RecordingSegmentRuntime, RecorderError> {
    Err(unavailable_error())
}
