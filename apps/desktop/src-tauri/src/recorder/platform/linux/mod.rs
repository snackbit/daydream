mod audio;
mod portal;

use std::{env, path::Path, process::Command};

use crate::storage::AppStorage;

use super::super::{
    manager::RecorderManager,
    types::{
        AudioSourceList, RecorderAudioHealth, RecorderBackendComponentHealth,
        RecorderBackendHealth, RecorderError, RecorderFfmpegHealth,
    },
};

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
        ffmpeg: collect_ffmpeg_health(),
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

fn audio_health(default_source_id: Option<String>, error: Option<String>) -> RecorderAudioHealth {
    RecorderAudioHealth {
        available: default_source_id.is_some() && error.is_none(),
        default_source_id,
        error,
    }
}

fn collect_ffmpeg_health() -> RecorderFfmpegHealth {
    let Ok(path) = env::var("DAYDREAM_FFMPEG_PATH") else {
        return RecorderFfmpegHealth {
            available: false,
            error: Some("DAYDREAM_FFMPEG_PATH is not configured.".to_string()),
            path: None,
        };
    };

    let trimmed_path = path.trim();

    if trimmed_path.is_empty() {
        return RecorderFfmpegHealth {
            available: false,
            error: Some("DAYDREAM_FFMPEG_PATH is empty.".to_string()),
            path: None,
        };
    }

    match Command::new(Path::new(trimmed_path))
        .arg("-version")
        .output()
    {
        Ok(output) if output.status.success() => RecorderFfmpegHealth {
            available: true,
            error: None,
            path: Some(trimmed_path.to_string()),
        },
        Ok(output) => RecorderFfmpegHealth {
            available: false,
            error: Some(format!("FFmpeg exited with {}.", output.status)),
            path: Some(trimmed_path.to_string()),
        },
        Err(error) => RecorderFfmpegHealth {
            available: false,
            error: Some(format!("Failed to run FFmpeg: {error}")),
            path: Some(trimmed_path.to_string()),
        },
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
