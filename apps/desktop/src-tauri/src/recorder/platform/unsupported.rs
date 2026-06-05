use std::path::Path;

use crate::storage::AppStorage;

use super::super::{
    errors::unavailable_error,
    ffmpeg::FfmpegSegmentInput,
    manager::RunningSegmentMedia,
    manager::{unavailable_backend_health, RecorderManager},
    types::{
        AudioSourceList, CaptureSourceList, RecorderBackendHealth, RecorderError, RecorderScene,
    },
};

pub(crate) struct PreparedLiveCapture;

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

pub fn start_live_segment(
    _ffmpeg_path: &Path,
    _input: &FfmpegSegmentInput,
    _prepared: PreparedLiveCapture,
) -> Result<RunningSegmentMedia, RecorderError> {
    Err(unavailable_error())
}
