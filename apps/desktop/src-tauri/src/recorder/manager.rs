use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use uuid::Uuid;

use crate::storage::{
    self, AppStorage, CreateRecordingInput, CreateRecordingSegmentInput, FailRecordingSegmentInput,
    FinalizeRecordingSegmentInput,
};

use super::capture_runtime::{
    capture_timing_snapshot, CapturePreviewSession, CaptureSession, SharedCaptureTiming,
};
use super::errors::{
    dependency_error, filesystem_error, lifecycle_error, scene_or_storage_error, storage_error,
};
use super::ffmpeg::{self, FfmpegSegmentInput, FfmpegStreamIndexes, RunningFfmpegSegment};
use super::storage_mapping::quality_preset_as_storage_label;
use super::storage_mapping::scene_from_storage;
use super::types::{
    CaptureSource, CaptureSourceList, CaptureTimingDiagnostics, RecorderAudioHealth,
    RecorderBackendComponentHealth, RecorderBackendHealth, RecorderError, RecorderFfmpegHealth,
    RecorderFinalizationAction, RecorderFinalizationPhase, RecorderFinalizationStatus,
    RecorderPortalHealth, RecorderPreviewState, RecorderPreviewStatus, RecorderState,
    RecorderStatus, StartRecordingInput,
};

mod resume;
mod selection;
mod start;
mod stop;

#[derive(Debug)]
pub struct RecorderManager {
    app_data_dir: PathBuf,
    inner: Mutex<RecorderRuntime>,
}

#[derive(Debug)]
struct RecorderRuntime {
    active: Option<ActiveRecording>,
    capture_sources: Vec<CaptureSource>,
    preview: Option<ActivePreview>,
    preview_start_generation: u64,
    preview_status: RecorderPreviewStatus,
    status: RecorderStatus,
}

#[derive(Debug)]
struct ActivePreview {
    scene_id: String,
    session: Box<dyn CapturePreviewSession>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum PreviewStartDecision {
    AlreadyCurrent(RecorderPreviewStatus),
    Pending {
        generation: u64,
        status: RecorderPreviewStatus,
    },
}

#[derive(Clone, Debug)]
pub(crate) struct ActiveRecording {
    active_segment: Option<ActiveSegment>,
    canvas_height: u32,
    canvas_width: u32,
    desktop_audio_source_id: Option<String>,
    ffmpeg_path: String,
    fps: f64,
    microphone_audio_source_id: Option<String>,
    recording_id: String,
    scene_id: String,
    segments_dir: PathBuf,
    next_segment_index: u32,
}

#[derive(Clone, Debug)]
pub(crate) struct ActiveSegment {
    file_path: PathBuf,
    id: String,
    media: Option<SharedSegmentMedia>,
    started_at_ms: u64,
    stream_indexes: FfmpegStreamIndexes,
    timing: Option<SharedCaptureTiming>,
}

#[derive(Debug)]
pub(crate) struct RunningSegmentMedia {
    capture: Option<Box<dyn CaptureSession>>,
    ffmpeg: RunningFfmpegSegment,
    timing: Option<SharedCaptureTiming>,
}

type SharedSegmentMedia = Arc<Mutex<Option<RunningSegmentMedia>>>;

#[derive(Clone, Debug)]
pub(crate) struct PendingRecordingStart {
    pub(crate) active: ActiveRecording,
    pub(crate) recording_dir: PathBuf,
    pub(crate) recording_id: String,
    pub(crate) segment: ActiveSegment,
    pub(crate) segment_path: PathBuf,
    pub(crate) segment_index: u32,
    pub(crate) segment_input: FfmpegSegmentInput,
    pub(crate) ffmpeg_path: String,
    pub(crate) input: StartRecordingInput,
    pub(crate) scene: super::types::RecorderScene,
}

#[derive(Clone, Debug)]
pub(crate) struct PendingSegmentStart {
    pub(crate) active: ActiveRecording,
    pub(crate) segment: ActiveSegment,
    pub(crate) segment_path: PathBuf,
    pub(crate) segment_index: u32,
    pub(crate) segment_input: FfmpegSegmentInput,
    pub(crate) ffmpeg_path: String,
    pub(crate) scene: super::types::RecorderScene,
}

impl RunningSegmentMedia {
    pub(crate) fn new(
        ffmpeg: RunningFfmpegSegment,
        capture: Option<Box<dyn CaptureSession>>,
        timing: Option<SharedCaptureTiming>,
    ) -> Self {
        Self {
            capture,
            ffmpeg,
            timing,
        }
    }

    pub(crate) fn capture_timing(&self) -> Option<SharedCaptureTiming> {
        self.timing.clone()
    }

    fn stop(mut self) -> Result<std::process::ExitStatus, RecorderError> {
        let capture_error = self.capture.take().and_then(|capture| capture.stop().err());
        let ffmpeg_result = self.ffmpeg.stop();

        if let Some(error) = capture_error {
            return Err(error);
        }

        ffmpeg_result
    }
}

impl RecorderManager {
    pub fn new(app_data_dir: PathBuf) -> Self {
        Self {
            app_data_dir,
            inner: Mutex::new(RecorderRuntime {
                active: None,
                capture_sources: Vec::new(),
                preview: None,
                preview_start_generation: 0,
                preview_status: idle_preview_status(now_ms()),
                status: idle_status(now_ms()),
            }),
        }
    }

    pub fn status(&self) -> Result<RecorderStatus, RecorderError> {
        Ok(self.lock_runtime()?.status.clone())
    }

    pub(crate) fn preview_status(&self) -> Result<RecorderPreviewStatus, RecorderError> {
        Ok(self.lock_runtime()?.preview_status.clone())
    }

    pub(crate) fn begin_preview_start(
        &self,
        scene_id: &str,
        timestamp_ms: u64,
    ) -> Result<PreviewStartDecision, RecorderError> {
        let (preview_to_stop, generation) = {
            let mut runtime = self.lock_runtime()?;
            if runtime.active.is_some() || status_has_pending_lifecycle(&runtime.status) {
                return Err(lifecycle_error(
                    "recorder_preview_blocked",
                    "Recorder preview cannot start while recording is active.",
                ));
            }
            if runtime
                .preview
                .as_ref()
                .is_some_and(|preview| preview.scene_id == scene_id)
            {
                return Ok(PreviewStartDecision::AlreadyCurrent(
                    runtime.preview_status.clone(),
                ));
            }
            if runtime.preview_status.state == RecorderPreviewState::Starting
                && runtime.preview_status.scene_id.as_deref() == Some(scene_id)
            {
                return Ok(PreviewStartDecision::AlreadyCurrent(
                    runtime.preview_status.clone(),
                ));
            }

            runtime.preview_start_generation = runtime.preview_start_generation.saturating_add(1);
            (runtime.preview.take(), runtime.preview_start_generation)
        };
        if let Some(preview) = preview_to_stop {
            preview.session.stop()?;
        }

        let mut runtime = self.lock_runtime()?;
        let status = RecorderPreviewStatus {
            error: None,
            scene_id: Some(scene_id.to_string()),
            source_ids: Vec::new(),
            state: RecorderPreviewState::Starting,
            updated_at_ms: timestamp_ms,
        };
        runtime.preview_status = status.clone();

        Ok(PreviewStartDecision::Pending { generation, status })
    }

    pub(crate) fn activate_preview(
        &self,
        scene_id: String,
        generation: u64,
        source_ids: Vec<String>,
        session: Box<dyn CapturePreviewSession>,
        timestamp_ms: u64,
    ) -> Result<RecorderPreviewStatus, RecorderError> {
        let mut session = Some(session);
        let inactive_status = {
            let mut runtime = self.lock_runtime()?;
            let start_is_current = runtime.preview_start_generation == generation
                && runtime.preview_status.state == RecorderPreviewState::Starting
                && runtime.preview_status.scene_id.as_deref() == Some(scene_id.as_str());
            if !start_is_current {
                Some(runtime.preview_status.clone())
            } else if runtime.active.is_some() || status_has_pending_lifecycle(&runtime.status) {
                runtime.preview_start_generation =
                    runtime.preview_start_generation.saturating_add(1);
                let status = idle_preview_status(timestamp_ms);
                runtime.preview_status = status.clone();

                Some(status)
            } else {
                let status = RecorderPreviewStatus {
                    error: None,
                    scene_id: Some(scene_id.clone()),
                    source_ids: source_ids.clone(),
                    state: RecorderPreviewState::Live,
                    updated_at_ms: timestamp_ms,
                };
                runtime.preview = Some(ActivePreview {
                    scene_id,
                    session: session.take().expect("session is available"),
                });
                runtime.preview_status = status.clone();

                return Ok(status);
            }
        };

        if let Some(session) = session {
            session.stop()?;
        }

        Ok(inactive_status.expect("inactive status is available"))
    }

    pub(crate) fn fail_preview_start(
        &self,
        scene_id: String,
        generation: u64,
        error: RecorderError,
        timestamp_ms: u64,
    ) -> Result<RecorderPreviewStatus, RecorderError> {
        let mut runtime = self.lock_runtime()?;
        let start_is_current = runtime.preview_start_generation == generation
            && runtime.preview_status.state == RecorderPreviewState::Starting
            && runtime.preview_status.scene_id.as_deref() == Some(scene_id.as_str());
        if !start_is_current {
            return Ok(runtime.preview_status.clone());
        }
        if runtime.active.is_some() || status_has_pending_lifecycle(&runtime.status) {
            runtime.preview_start_generation = runtime.preview_start_generation.saturating_add(1);
            let status = idle_preview_status(timestamp_ms);
            runtime.preview_status = status.clone();

            return Ok(status);
        }

        let status = RecorderPreviewStatus {
            error: Some(error),
            scene_id: Some(scene_id),
            source_ids: Vec::new(),
            state: RecorderPreviewState::Error,
            updated_at_ms: timestamp_ms,
        };
        runtime.preview_status = status.clone();

        Ok(status)
    }

    pub(crate) fn stop_preview(
        &self,
        timestamp_ms: u64,
    ) -> Result<RecorderPreviewStatus, RecorderError> {
        let preview = {
            let mut runtime = self.lock_runtime()?;
            runtime.preview_start_generation = runtime.preview_start_generation.saturating_add(1);
            runtime.preview.take()
        };
        if let Some(preview) = preview {
            preview.session.stop()?;
        }

        let mut runtime = self.lock_runtime()?;
        let status = idle_preview_status(timestamp_ms);
        runtime.preview_status = status.clone();

        Ok(status)
    }

    pub fn status_with_backend_health(
        &self,
        backend_health: RecorderBackendHealth,
        timestamp_ms: u64,
    ) -> Result<RecorderStatus, RecorderError> {
        let mut runtime = self.lock_runtime()?;
        runtime.status.backend_health = backend_health;
        runtime.status.updated_at_ms = timestamp_ms;

        Ok(runtime.status.clone())
    }

    pub fn capture_sources(&self) -> Result<CaptureSourceList, RecorderError> {
        Ok(CaptureSourceList {
            sources: self.lock_runtime()?.capture_sources.clone(),
        })
    }

    pub fn set_capture_sources(
        &self,
        sources: Vec<CaptureSource>,
    ) -> Result<CaptureSourceList, RecorderError> {
        let mut runtime = self.lock_runtime()?;
        runtime.capture_sources = sources.clone();

        Ok(CaptureSourceList { sources })
    }

    pub(crate) fn active_capture_timing(
        &self,
    ) -> Result<Option<CaptureTimingDiagnostics>, RecorderError> {
        let runtime = self.lock_runtime()?;
        Ok(runtime
            .active
            .as_ref()
            .and_then(|active| active.active_segment.as_ref())
            .and_then(|segment| segment.timing.as_ref())
            .map(capture_timing_snapshot))
    }

    fn recordings_root(&self) -> PathBuf {
        self.app_data_dir.join("recordings")
    }

    fn lock_runtime(&self) -> Result<std::sync::MutexGuard<'_, RecorderRuntime>, RecorderError> {
        self.inner.lock().map_err(|_error| {
            lifecycle_error(
                "recorder_state_unavailable",
                "Recorder state lock is unavailable.",
            )
        })
    }
}

pub(crate) fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

fn status_from_active(
    state: RecorderState,
    active: &ActiveRecording,
    updated_at_ms: u64,
    backend_health: RecorderBackendHealth,
    last_error: Option<RecorderError>,
) -> RecorderStatus {
    RecorderStatus {
        active_recording_id: Some(active.recording_id.clone()),
        active_scene_id: Some(active.scene_id.clone()),
        active_segment_id: active
            .active_segment
            .as_ref()
            .map(|segment| segment.id.clone()),
        backend_health,
        finalization: None,
        last_error,
        state,
        updated_at_ms,
    }
}

fn idle_status(updated_at_ms: u64) -> RecorderStatus {
    idle_status_with_backend_health(updated_at_ms, unavailable_backend_health())
}

fn idle_preview_status(updated_at_ms: u64) -> RecorderPreviewStatus {
    RecorderPreviewStatus {
        error: None,
        scene_id: None,
        source_ids: Vec::new(),
        state: RecorderPreviewState::Idle,
        updated_at_ms,
    }
}

fn idle_status_with_backend_health(
    updated_at_ms: u64,
    backend_health: RecorderBackendHealth,
) -> RecorderStatus {
    RecorderStatus {
        active_recording_id: None,
        active_scene_id: None,
        active_segment_id: None,
        backend_health,
        finalization: None,
        last_error: None,
        state: RecorderState::Idle,
        updated_at_ms,
    }
}

fn selecting_status(
    updated_at_ms: u64,
    backend_health: RecorderBackendHealth,
    active_recording_id: Option<String>,
    active_scene_id: Option<String>,
    active_segment_id: Option<String>,
) -> RecorderStatus {
    RecorderStatus {
        active_recording_id,
        active_scene_id,
        active_segment_id,
        backend_health,
        finalization: None,
        last_error: None,
        state: RecorderState::Selecting,
        updated_at_ms,
    }
}

fn error_status_with_backend_health(
    updated_at_ms: u64,
    backend_health: RecorderBackendHealth,
    error: RecorderError,
) -> RecorderStatus {
    RecorderStatus {
        active_recording_id: None,
        active_scene_id: None,
        active_segment_id: None,
        backend_health,
        finalization: None,
        last_error: Some(error),
        state: RecorderState::Error,
        updated_at_ms,
    }
}

fn status_for_finalizing_pause(
    active: &ActiveRecording,
    segment: &ActiveSegment,
    timestamp_ms: u64,
    backend_health: RecorderBackendHealth,
) -> RecorderStatus {
    RecorderStatus {
        active_recording_id: Some(active.recording_id.clone()),
        active_scene_id: Some(active.scene_id.clone()),
        active_segment_id: None,
        backend_health,
        finalization: Some(RecorderFinalizationStatus {
            action: RecorderFinalizationAction::Pause,
            phase: RecorderFinalizationPhase::StoppingCapture,
            recording_id: active.recording_id.clone(),
            segment_id: Some(segment.id.clone()),
            started_at_ms: timestamp_ms,
        }),
        last_error: None,
        state: RecorderState::Paused,
        updated_at_ms: timestamp_ms,
    }
}

fn status_for_finalizing_stop(
    active: &ActiveRecording,
    timestamp_ms: u64,
    backend_health: RecorderBackendHealth,
) -> RecorderStatus {
    RecorderStatus {
        active_recording_id: None,
        active_scene_id: None,
        active_segment_id: None,
        backend_health,
        finalization: Some(RecorderFinalizationStatus {
            action: RecorderFinalizationAction::Stop,
            phase: RecorderFinalizationPhase::StoppingCapture,
            recording_id: active.recording_id.clone(),
            segment_id: active
                .active_segment
                .as_ref()
                .map(|segment| segment.id.clone()),
            started_at_ms: timestamp_ms,
        }),
        last_error: None,
        state: RecorderState::Idle,
        updated_at_ms: timestamp_ms,
    }
}

fn status_has_pending_lifecycle(status: &RecorderStatus) -> bool {
    status.finalization.is_some() || is_pending_lifecycle_state(&status.state)
}

fn is_pending_lifecycle_state(state: &RecorderState) -> bool {
    matches!(
        state,
        RecorderState::Pausing | RecorderState::Selecting | RecorderState::Stopping
    )
}

pub(crate) fn unavailable_backend_health() -> RecorderBackendHealth {
    RecorderBackendHealth {
        desktop_audio: RecorderAudioHealth {
            available: false,
            default_source_id: None,
            error: Some("Recorder backend is not implemented yet.".to_string()),
        },
        ffmpeg: RecorderFfmpegHealth {
            available: false,
            capabilities: Default::default(),
            error: Some("Recorder backend is not implemented yet.".to_string()),
            path: None,
            version_line: None,
        },
        microphone: RecorderAudioHealth {
            available: false,
            default_source_id: None,
            error: Some("Recorder backend is not implemented yet.".to_string()),
        },
        pipewire: RecorderBackendComponentHealth {
            available: false,
            error: Some("Recorder backend is not implemented yet.".to_string()),
        },
        portal: RecorderPortalHealth {
            available: false,
            cursor_modes: Vec::new(),
            error: Some("Recorder backend is not implemented yet.".to_string()),
            screen_cast_version: None,
            source_types: Vec::new(),
        },
    }
}

fn duration_ms(started_at_ms: u64, ended_at_ms: u64) -> u64 {
    ended_at_ms.saturating_sub(started_at_ms)
}

fn segment_file_name(segment_index: u32) -> String {
    format!("{segment_index:04}.mkv")
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

fn fail_segment_for_error(
    storage: &AppStorage,
    segment_id: &str,
    timestamp_ms: u64,
    error: &RecorderError,
    ffmpeg_exit_code: Option<i64>,
) -> Result<(), RecorderError> {
    storage
        .with_connection(|connection| {
            storage::fail_recording_segment(
                connection,
                &FailRecordingSegmentInput {
                    ended_at_ms: timestamp_ms as i64,
                    error_code: error.code.clone(),
                    ffmpeg_exit_code,
                    id: segment_id.to_string(),
                    updated_at_ms: timestamp_ms as i64,
                },
            )
        })
        .map(|_segment| ())
        .map_err(storage_error)
}
