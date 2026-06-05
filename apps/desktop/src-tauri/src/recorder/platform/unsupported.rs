use crate::storage::AppStorage;

use super::super::{
    errors::unavailable_error,
    manager::{unavailable_backend_health, RecorderManager},
    types::{AudioSourceList, CaptureSourceList, RecorderBackendHealth, RecorderError},
};

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
