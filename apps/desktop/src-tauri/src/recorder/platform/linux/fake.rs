use std::{
    sync::mpsc,
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use tauri::AppHandle;

use crate::storage::AppStorage;

use super::super::super::{
    capture_runtime::{
        bounded_frame_channel, CapturePreviewSession, CaptureSession, CapturedFrame,
        CapturedFrameFormat, FrameSender,
    },
    errors::permission_error,
    manager::{RecorderManager, RecordingSegmentRuntime},
    media_writer::{self, MediaSegmentInput},
    types::{
        AudioSource, AudioSourceKind, AudioSourceList, AudioSourceRole, CaptureSource,
        CaptureSourceKind, CaptureSourceList, RecorderAudioHealth, RecorderBackendComponentHealth,
        RecorderBackendHealth, RecorderError, RecorderGpuCaptureHealth, RecorderPortalHealth,
        RecorderScene,
    },
};

const FAKE_MONITOR_ID: &str = "fake-monitor-1";
const FAKE_DESKTOP_AUDIO_ID: &str = "fake-desktop-audio";
const FAKE_MIC_ID: &str = "fake-mic";

#[derive(Debug)]
pub(crate) struct PreparedLiveCapture {
    source_ids: Vec<String>,
}

#[derive(Debug)]
pub(crate) struct NoopPreviewSession;

#[derive(Debug)]
struct FakeCaptureSession {
    shutdown: mpsc::Sender<()>,
    worker: thread::JoinHandle<Result<(), RecorderError>>,
}

pub(crate) fn scene_uses_fake_backend(scene: &RecorderScene) -> bool {
    scene
        .sources
        .iter()
        .any(|source| source.source_id.starts_with("fake-"))
}

pub(crate) fn collect_backend_health() -> RecorderBackendHealth {
    let mut media_writer = media_writer::collect_media_writer_health();
    if let Some(error) = media_writer.error.clone() {
        media_writer.available = false;
        media_writer.error = Some(error);
    }

    RecorderBackendHealth {
        desktop_audio: RecorderAudioHealth {
            available: true,
            default_source_id: Some(FAKE_DESKTOP_AUDIO_ID.to_string()),
            error: None,
        },
        gpu_capture: RecorderGpuCaptureHealth {
            available: true,
            cleanup_error: None,
            dmabuf_import_available: true,
            egl_available: true,
            egl_client_apis: vec!["OpenGL".to_string()],
            egl_platform: Some("fake".to_string()),
            egl_vendor: Some("Daydream".to_string()),
            egl_version: Some("1.5".to_string()),
            error: None,
            modifier_import_available: Some(true),
            opengl_available: true,
            renderer: Some("Daydream fake renderer".to_string()),
            vendor: Some("Daydream".to_string()),
            version: Some("fake".to_string()),
        },
        media_writer,
        microphone: RecorderAudioHealth {
            available: true,
            default_source_id: Some(FAKE_MIC_ID.to_string()),
            error: None,
        },
        pipewire: RecorderBackendComponentHealth {
            available: true,
            error: None,
        },
        portal: RecorderPortalHealth {
            available: true,
            cursor_modes: vec!["embedded".to_string()],
            error: None,
            screen_cast_version: Some(5),
            source_types: vec!["monitor".to_string()],
        },
    }
}

pub(crate) fn get_capture_sources(
    manager: &RecorderManager,
) -> Result<CaptureSourceList, RecorderError> {
    manager.capture_sources()
}

pub(crate) async fn select_capture_sources(
    _storage: &AppStorage,
    manager: &RecorderManager,
    _force_chooser: bool,
) -> Result<CaptureSourceList, RecorderError> {
    manager.set_capture_sources(vec![fake_capture_source()])
}

pub(crate) async fn get_audio_sources() -> Result<AudioSourceList, RecorderError> {
    Ok(AudioSourceList {
        sources: vec![
            AudioSource {
                display_name: "Fake desktop audio".to_string(),
                id: FAKE_DESKTOP_AUDIO_ID.to_string(),
                is_default: true,
                kind: AudioSourceKind::Monitor,
                roles: vec![AudioSourceRole::DesktopAudio],
            },
            AudioSource {
                display_name: "Fake microphone".to_string(),
                id: FAKE_MIC_ID.to_string(),
                is_default: true,
                kind: AudioSourceKind::Input,
                roles: vec![AudioSourceRole::Microphone],
            },
        ],
    })
}

pub(crate) async fn prepare_live_capture(
    _storage: &AppStorage,
    scene: &RecorderScene,
) -> Result<PreparedLiveCapture, RecorderError> {
    match fake_token_mode() {
        FakeTokenMode::Valid => {}
        FakeTokenMode::Missing => {
            return Err(permission_error(
                "recorder_portal_restore_token_missing",
                "Screen permission is missing. Reselect monitor sources.",
            ));
        }
        FakeTokenMode::Stale => {
            return Err(permission_error(
                "recorder_portal_restore_token_invalid",
                "Screen permission expired. Reselect monitor sources.",
            ));
        }
    }

    if scene.sources.is_empty() {
        return Err(permission_error(
            "recorder_portal_restore_token_missing",
            "Screen permission is missing. Reselect monitor sources.",
        ));
    }

    Ok(PreparedLiveCapture {
        source_ids: scene
            .sources
            .iter()
            .map(|source| source.source_id.clone())
            .collect(),
    })
}

pub(crate) async fn start_preview_session(
    _app: AppHandle,
    storage: &AppStorage,
    scene: &RecorderScene,
) -> Result<super::capture::RunningPreviewSession, RecorderError> {
    let prepared = prepare_live_capture(storage, scene).await?;

    Ok(super::capture::RunningPreviewSession {
        session: Box::new(NoopPreviewSession),
        source_ids: prepared.source_ids,
    })
}

pub(crate) fn start_live_segment(
    input: &MediaSegmentInput,
    prepared: PreparedLiveCapture,
) -> Result<RecordingSegmentRuntime, RecorderError> {
    let (sender, receiver) = bounded_frame_channel(3);
    let media_writer = media_writer::start_segment_from_frames(input, receiver, None)?;
    let capture = match start_fake_capture_session(input, prepared, sender) {
        Ok(capture) => capture,
        Err(error) => {
            let _ = media_writer.stop(input.started_at_ms);
            return Err(error);
        }
    };

    Ok(RecordingSegmentRuntime::new(
        media_writer,
        Some(capture),
        None,
    ))
}

impl CapturePreviewSession for NoopPreviewSession {
    fn stop(self: Box<Self>) -> Result<(), RecorderError> {
        Ok(())
    }
}

impl CaptureSession for FakeCaptureSession {
    fn poll_error(&self) -> Option<RecorderError> {
        self.worker
            .is_finished()
            .then(|| fake_capture_error("Fake capture stopped unexpectedly."))
    }
    fn stop(self: Box<Self>, _stop_at_ms: u64) -> Result<(), RecorderError> {
        let _ = self.shutdown.send(());
        self.worker
            .join()
            .unwrap_or_else(|_error| Err(fake_capture_error("Fake capture worker panicked.")))
    }
}

fn start_fake_capture_session(
    input: &MediaSegmentInput,
    prepared: PreparedLiveCapture,
    frames: FrameSender,
) -> Result<Box<dyn CaptureSession>, RecorderError> {
    let source_id = prepared
        .source_ids
        .first()
        .cloned()
        .unwrap_or_else(|| FAKE_MONITOR_ID.to_string());
    let canvas_width = input.canvas_width;
    let canvas_height = input.canvas_height;
    let frame_interval = fake_frame_interval(input.fps);
    let (shutdown, receiver) = mpsc::channel();
    let worker = thread::spawn(move || {
        let mut frame_index = 0_u64;
        loop {
            let frame = fake_frame(&source_id, canvas_width, canvas_height, frame_index)?;
            frames.send(frame)?;
            frame_index += 1;

            match receiver.recv_timeout(frame_interval) {
                Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => return Ok(()),
                Err(mpsc::RecvTimeoutError::Timeout) => {}
            }
        }
    });

    Ok(Box::new(FakeCaptureSession { shutdown, worker }))
}

fn fake_capture_source() -> CaptureSource {
    CaptureSource {
        display_name: "Fake monitor".to_string(),
        height: Some(180),
        id: FAKE_MONITOR_ID.to_string(),
        kind: CaptureSourceKind::Monitor,
        logical_height: Some(180),
        logical_width: Some(320),
        scale_factor: Some(1.0),
        width: Some(320),
    }
}

fn fake_frame(
    source_id: &str,
    width: u32,
    height: u32,
    frame_index: u64,
) -> Result<CapturedFrame, RecorderError> {
    let plane_len = (width as usize)
        .checked_mul(height as usize)
        .ok_or_else(|| fake_capture_error("Fake capture frame dimensions overflow."))?;
    let frame_len = plane_len
        .checked_mul(3)
        .ok_or_else(|| fake_capture_error("Fake capture frame dimensions overflow."))?;
    let mut data = vec![0; frame_len];
    let luma = u8::try_from(frame_index % 220)
        .unwrap_or(0)
        .saturating_add(16);
    data[..plane_len].fill(luma);
    data[plane_len..].fill(128);

    CapturedFrame::new(
        source_id,
        timestamp_ms(),
        width,
        height,
        width,
        CapturedFrameFormat::Yuv444p,
        data,
    )
}

fn fake_frame_interval(fps: f64) -> Duration {
    if fps.is_finite() && fps > 0.0 {
        Duration::from_secs_f64(1.0 / fps).min(Duration::from_millis(100))
    } else {
        Duration::from_millis(100)
    }
}

fn timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

fn fake_capture_error(message: impl Into<String>) -> RecorderError {
    super::super::super::errors::capture_adapter_failed_error(message)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FakeTokenMode {
    Valid,
    Missing,
    Stale,
}

fn fake_token_mode() -> FakeTokenMode {
    fake_token_mode_from_value(std::env::var("DAYDREAM_FAKE_PORTAL_TOKEN").ok().as_deref())
}

fn fake_token_mode_from_value(value: Option<&str>) -> FakeTokenMode {
    match value {
        Some("missing") => FakeTokenMode::Missing,
        Some("stale") => FakeTokenMode::Stale,
        _ => FakeTokenMode::Valid,
    }
}

#[cfg(test)]
mod tests {
    use super::{fake_frame_interval, fake_token_mode_from_value, FakeTokenMode};

    #[test]
    fn fake_frame_interval_is_capped_for_fast_tests() {
        assert_eq!(
            fake_frame_interval(1.0),
            std::time::Duration::from_millis(100)
        );
    }

    #[test]
    fn fake_token_mode_parser_defaults_to_valid() {
        assert_eq!(fake_token_mode_from_value(None), FakeTokenMode::Valid);
        assert_eq!(
            fake_token_mode_from_value(Some("valid")),
            FakeTokenMode::Valid
        );
        assert_eq!(
            fake_token_mode_from_value(Some("missing")),
            FakeTokenMode::Missing
        );
        assert_eq!(
            fake_token_mode_from_value(Some("stale")),
            FakeTokenMode::Stale
        );
    }
}
