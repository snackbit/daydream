mod audio;
mod capture;
mod fake;
mod gpu_compositor;
mod gpu_probe;
mod portal;
mod window_provider;

use tauri::AppHandle;

use crate::storage::AppStorage;

use super::super::{
    manager::RecorderManager,
    manager::RecordingSegmentRuntime,
    media_writer::{self, MediaSegmentInput},
    operation_timing::{run_blocking, OperationTimer},
    types::{
        AudioSourceList, ProtectableWindowList, RecorderAudioHealth,
        RecorderBackendComponentHealth, RecorderBackendHealth, RecorderError, RecorderScene,
        WindowProtectionStatus,
    },
};

pub(crate) use capture::RunningPreviewSession;

pub(crate) enum PreparedLiveCapture {
    Fake(fake::PreparedLiveCapture),
    Portal(capture::PreparedLiveCapture),
}

pub async fn collect_backend_health() -> RecorderBackendHealth {
    if fake_backend_enabled() {
        return fake::collect_backend_health();
    }

    let portal = portal::collect_portal_health().await;
    let blocking_portal = portal.clone();
    match run_blocking("capture_backend_probes", move || {
        Ok(collect_blocking_backend_health(blocking_portal))
    })
    .await
    {
        Ok(health) => health,
        Err(error) => {
            let mut health = super::super::manager::unavailable_backend_health();
            health.portal = portal;
            health.desktop_audio.error = Some(error.message.clone());
            health.microphone.error = Some(error.message.clone());
            health.pipewire.error = Some(error.message.clone());
            health.gpu_capture.error = Some(error.message.clone());
            health.media_writer.error = Some(error.message);
            health
        }
    }
}

fn collect_blocking_backend_health(
    portal: super::super::types::RecorderPortalHealth,
) -> RecorderBackendHealth {
    let _audio_timing = OperationTimer::start("audio_probe");
    let audio = audio::probe_audio_server();
    drop(_audio_timing);
    let pipewire = audio
        .server_name
        .as_deref()
        .map(pipewire_health_from_server_name)
        .unwrap_or_else(|| RecorderBackendComponentHealth {
            available: false,
            error: audio.error.clone(),
        });
    let desktop_audio = audio_health(audio.desktop_default_source_id.clone(), audio.error.clone());
    let microphone = audio_health(audio.microphone_default_source_id.clone(), audio.error);
    let gpu_timing = OperationTimer::start("gpu_probe");
    let gpu_capture = gpu_probe::collect_gpu_capture_health();
    drop(gpu_timing);
    let writer_timing = OperationTimer::start("media_writer_probe");
    let writer = media_writer::collect_media_writer_health();
    drop(writer_timing);

    RecorderBackendHealth {
        desktop_audio,
        gpu_capture,
        media_writer: writer,
        microphone,
        pipewire,
        portal,
    }
}

pub fn get_capture_sources(
    manager: &RecorderManager,
) -> Result<super::super::types::CaptureSourceList, RecorderError> {
    if fake_backend_enabled() {
        return fake::get_capture_sources(manager);
    }

    manager.capture_sources()
}

pub async fn select_capture_sources(
    storage: &AppStorage,
    manager: &RecorderManager,
    force_chooser: bool,
) -> Result<super::super::types::CaptureSourceList, RecorderError> {
    if fake_backend_enabled() {
        return fake::select_capture_sources(storage, manager, force_chooser).await;
    }

    portal::select_capture_sources(storage, manager, force_chooser).await
}

pub async fn get_audio_sources() -> Result<AudioSourceList, RecorderError> {
    if fake_backend_enabled() {
        return fake::get_audio_sources().await;
    }

    run_blocking("audio_sources", audio::get_audio_sources).await
}

pub async fn prepare_live_capture(
    storage: &AppStorage,
    scene: &RecorderScene,
) -> Result<PreparedLiveCapture, RecorderError> {
    if fake_backend_enabled() || fake::scene_uses_fake_backend(scene) {
        return fake::prepare_live_capture(storage, scene)
            .await
            .map(PreparedLiveCapture::Fake);
    }

    let live_session = portal::open_live_capture_session(storage).await?;
    capture::prepared_live_capture(live_session, scene)
        .await
        .map(PreparedLiveCapture::Portal)
}

pub async fn start_preview_session(
    app: AppHandle,
    storage: &AppStorage,
    scene: &RecorderScene,
) -> Result<RunningPreviewSession, RecorderError> {
    if fake_backend_enabled() || fake::scene_uses_fake_backend(scene) {
        return fake::start_preview_session(app, storage, scene).await;
    }

    let live_session = portal::open_live_capture_session(storage).await?;
    let prepared = capture::prepared_live_capture(live_session, scene).await?;

    super::run_capture_setup(move || capture::start_preview_session(app, prepared)).await
}

pub fn start_live_segment(
    input: &MediaSegmentInput,
    prepared: PreparedLiveCapture,
) -> Result<RecordingSegmentRuntime, RecorderError> {
    match prepared {
        PreparedLiveCapture::Fake(prepared) => fake::start_live_segment(input, prepared),
        PreparedLiveCapture::Portal(prepared) => capture::start_live_segment(input, prepared),
    }
}

pub fn list_protectable_windows() -> ProtectableWindowList {
    window_provider::list_protectable_windows()
}

pub fn window_protection_status() -> WindowProtectionStatus {
    window_provider::window_protection_status()
}

pub(crate) fn window_metadata_snapshot() -> super::super::types::WindowMetadataSnapshot {
    window_provider::window_metadata_snapshot()
}

fn fake_backend_enabled() -> bool {
    std::env::var("DAYDREAM_RECORDER_BACKEND").as_deref() == Ok("fake")
}

fn audio_health(default_source_id: Option<String>, error: Option<String>) -> RecorderAudioHealth {
    RecorderAudioHealth {
        available: default_source_id.is_some() && error.is_none(),
        default_source_id,
        error,
    }
}

fn pipewire_health_from_server_name(server_name: &str) -> RecorderBackendComponentHealth {
    let lower = server_name.to_lowercase();

    if lower.contains("pipewire") {
        RecorderBackendComponentHealth {
            available: true,
            error: None,
        }
    } else {
        RecorderBackendComponentHealth {
            available: false,
            error: Some(format!("Audio server is not PipeWire: {server_name}.")),
        }
    }
}
