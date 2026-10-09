#[cfg(target_os = "linux")]
mod linux;
#[cfg(not(target_os = "linux"))]
mod unsupported;

use crate::storage::AppStorage;
use tauri::AppHandle;

use super::{
    manager::{RecorderManager, RecordingSegmentRuntime},
    media_writer::MediaSegmentInput,
    types::{
        AudioSourceList, CaptureSourceList, ProtectableWindowList, RecorderBackendHealth,
        RecorderError, RecorderScene, WindowMetadataSnapshot, WindowProtectionStatus,
    },
};

#[cfg(target_os = "linux")]
use linux as imp;
#[cfg(not(target_os = "linux"))]
use unsupported as imp;

pub(crate) use imp::PreparedLiveCapture;
pub(crate) use imp::RunningPreviewSession;

async fn run_capture_setup<T: Send + 'static>(
    setup: impl FnOnce() -> Result<T, RecorderError> + Send + 'static,
) -> Result<T, RecorderError> {
    // PipeWire/GStreamer setup waits synchronously for workers. Its error paths
    // also await portal closure through block_on; neither belongs on an async
    // executor thread. Await completion so lifecycle ownership stays with IPC.
    tauri::async_runtime::spawn_blocking(setup)
        .await
        .map_err(|_| {
            super::errors::capture_adapter_failed_error(
                "Capture setup stopped unexpectedly. Try starting again.",
            )
        })?
}

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

pub async fn start_live_segment(
    input: &MediaSegmentInput,
    prepared: PreparedLiveCapture,
) -> Result<RecordingSegmentRuntime, RecorderError> {
    let input = input.clone();
    run_capture_setup(move || imp::start_live_segment(&input, prepared)).await
}

pub fn list_protectable_windows() -> ProtectableWindowList {
    imp::list_protectable_windows()
}

pub fn window_protection_status() -> WindowProtectionStatus {
    imp::window_protection_status()
}

pub(crate) fn window_metadata_snapshot() -> WindowMetadataSnapshot {
    imp::window_metadata_snapshot()
}

#[cfg(test)]
mod startup_tests {
    use super::*;

    #[test]
    fn failed_capture_setup_can_await_portal_cleanup_from_an_async_command() {
        let error = tauri::async_runtime::block_on(async {
            tauri::async_runtime::spawn(async {
                run_capture_setup(|| {
                    // Session::Close is an async D-Bus boundary. Its completion
                    // must be awaited before the startup error reaches the UI.
                    tauri::async_runtime::block_on(async { tokio::task::yield_now().await });
                    Err::<(), _>(crate::recorder::errors::dependency_error(
                        "synthetic_media_start_failed",
                        "Synthetic capture startup failed.",
                    ))
                })
                .await
            })
            .await
            .expect("capture command must reply instead of panicking")
            .expect_err("original startup failure must be returned")
        });
        assert_eq!(error.code, "synthetic_media_start_failed");
    }
}
