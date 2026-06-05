mod audio;
mod capture;
mod portal;

use std::path::Path;
use tauri::AppHandle;

use crate::storage::AppStorage;

use super::super::{
    ffmpeg,
    ffmpeg::FfmpegSegmentInput,
    manager::RecorderManager,
    manager::RunningSegmentMedia,
    types::{
        AudioSourceList, RecorderAudioHealth, RecorderBackendComponentHealth,
        RecorderBackendHealth, RecorderError, RecorderScene,
    },
};

pub(crate) use capture::{PreparedLiveCapture, RunningPreviewSession};

pub async fn collect_backend_health() -> RecorderBackendHealth {
    let portal = portal::collect_portal_health().await;
    let audio = audio::probe_audio_server();
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

    RecorderBackendHealth {
        desktop_audio,
        ffmpeg: ffmpeg::collect_ffmpeg_health(),
        microphone,
        pipewire,
        portal,
    }
}

pub fn get_capture_sources(
    manager: &RecorderManager,
) -> Result<super::super::types::CaptureSourceList, RecorderError> {
    manager.capture_sources()
}

pub async fn select_capture_sources(
    storage: &AppStorage,
    manager: &RecorderManager,
    force_chooser: bool,
) -> Result<super::super::types::CaptureSourceList, RecorderError> {
    portal::select_capture_sources(storage, manager, force_chooser).await
}

pub async fn get_audio_sources() -> Result<AudioSourceList, RecorderError> {
    audio::get_audio_sources()
}

pub async fn prepare_live_capture(
    storage: &AppStorage,
    scene: &RecorderScene,
) -> Result<PreparedLiveCapture, RecorderError> {
    let live_session = portal::open_live_capture_session(storage).await?;
    capture::prepared_live_capture(live_session, scene)
}

pub async fn start_preview_session(
    app: AppHandle,
    storage: &AppStorage,
    scene: &RecorderScene,
) -> Result<RunningPreviewSession, RecorderError> {
    let live_session = portal::open_live_capture_session(storage).await?;
    let prepared = capture::prepared_live_capture(live_session, scene)?;

    capture::start_preview_session(app, prepared)
}

pub fn start_live_segment(
    ffmpeg_path: &Path,
    input: &FfmpegSegmentInput,
    prepared: PreparedLiveCapture,
) -> Result<RunningSegmentMedia, RecorderError> {
    capture::start_live_segment(ffmpeg_path, input, prepared)
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
