#[cfg(target_os = "linux")]
mod linux;
#[cfg(not(target_os = "linux"))]
mod unsupported;

use crate::storage::AppStorage;
use std::path::Path;
use tauri::AppHandle;

use super::{
    ffmpeg::FfmpegSegmentInput,
    manager::{RecorderManager, RunningSegmentMedia},
    types::{
        AudioSourceList, CaptureSourceList, RecorderBackendHealth, RecorderError, RecorderScene,
    },
};

#[cfg(target_os = "linux")]
use linux as imp;
#[cfg(not(target_os = "linux"))]
use unsupported as imp;

pub(crate) use imp::PreparedLiveCapture;
pub(crate) use imp::RunningPreviewSession;

pub async fn collect_backend_health() -> RecorderBackendHealth {
    imp::collect_backend_health().await
}

pub fn get_capture_sources(manager: &RecorderManager) -> Result<CaptureSourceList, RecorderError> {
    imp::get_capture_sources(manager)
}

pub async fn select_capture_sources(
    storage: &AppStorage,
    manager: &RecorderManager,
    force_chooser: bool,
) -> Result<CaptureSourceList, RecorderError> {
    imp::select_capture_sources(storage, manager, force_chooser).await
}

pub async fn get_audio_sources() -> Result<AudioSourceList, RecorderError> {
    imp::get_audio_sources().await
}

pub async fn prepare_live_capture(
    storage: &AppStorage,
    scene: &RecorderScene,
) -> Result<PreparedLiveCapture, RecorderError> {
    imp::prepare_live_capture(storage, scene).await
}

pub async fn start_preview_session(
    app: AppHandle,
    storage: &AppStorage,
    scene: &RecorderScene,
) -> Result<RunningPreviewSession, RecorderError> {
    imp::start_preview_session(app, storage, scene).await
}

pub fn start_live_segment(
    ffmpeg_path: &Path,
    input: &FfmpegSegmentInput,
    prepared: PreparedLiveCapture,
) -> Result<RunningSegmentMedia, RecorderError> {
    imp::start_live_segment(ffmpeg_path, input, prepared)
}
