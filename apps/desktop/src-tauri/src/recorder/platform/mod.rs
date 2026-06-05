#[cfg(target_os = "linux")]
mod linux;
#[cfg(not(target_os = "linux"))]
mod unsupported;

use crate::storage::AppStorage;

use super::{
    manager::RecorderManager,
    types::{AudioSourceList, CaptureSourceList, RecorderBackendHealth, RecorderError},
};

#[cfg(target_os = "linux")]
use linux as imp;
#[cfg(not(target_os = "linux"))]
use unsupported as imp;

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
