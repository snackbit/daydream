use std::{
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use tracing::info;
use uuid::Uuid;

use crate::storage::{
    self, AppStorage, CreateRecordingInput, CreateRecordingSegmentInput,
    FinalizeRecordingSegmentInput,
};
use crate::{settings, storage_limits};

use super::capture_runtime::{capture_timing_snapshot, CapturePreviewSession};
use super::compositor::even_canvas_dimension;
use super::errors::{
    filesystem_error, lifecycle_error, scene_or_storage_error, storage_error, storage_error_message,
};
use super::media_writer::{self, MediaSegmentInput};
use super::storage_mapping::quality_preset_as_storage_label;
use super::storage_mapping::scene_from_storage;
use super::types::{
    CaptureSource, CaptureSourceList, CaptureTimingDiagnostics, RecorderBackendHealth,
    RecorderError, RecorderFinalizationAction, RecorderGpuCaptureHealth, RecorderPerformanceStats,
    RecorderPreviewState, RecorderPreviewStatus, RecorderState, RecorderStatus,
    StartRecordingInput,
};

mod capture_provenance;
mod deadline;
#[cfg(all(test, target_os = "linux"))]
mod deadline_tests;
mod finalization;
#[cfg(test)]
#[path = "tests/fragments.rs"]
mod fragment_tests;
mod fragments;
mod paths;
mod pause;
mod preference_cache;
mod recording_state;
mod resume;
mod runtime;
mod segment_runtime;
mod selection;
mod start;
mod status;
mod stop;
mod supervision;

pub(crate) use recording_state::{
    ActiveRecording, ActiveSegment, PendingRecordingStart, PendingSegmentStart,
};
pub(crate) use runtime::PreviewStartDecision;
pub(crate) use segment_runtime::RecordingSegmentRuntime;

use finalization::fail_segment_for_error;
use paths::{duration_ms, path_to_string, segment_file_name};
pub(crate) use runtime::PauseOwner;
use runtime::{ActivePreview, RecorderRuntime};
use status::{
    error_status_with_backend_health, idle_preview_status, idle_status,
    idle_status_with_backend_health, selecting_status, status_for_finalizing_pause,
    status_for_finalizing_stop, status_from_active, status_has_pending_lifecycle,
};

#[derive(Debug)]
pub struct RecorderManager {
    pub(crate) preview_demand: Arc<Mutex<Option<super::types::RecorderPreviewDemand>>>,
    app_data_dir: PathBuf,
    inner: Mutex<RecorderRuntime>,
    forced_failure: Mutex<Option<(String, RecorderStatus)>>,
    lifecycle_background_work: std::sync::atomic::AtomicBool,
    lifecycle_deadline: Mutex<Option<std::time::Instant>>,
}

impl RecorderManager {
    pub fn new(app_data_dir: PathBuf) -> Self {
        Self {
            preview_demand: Arc::new(Mutex::new(None)),
            app_data_dir,
            forced_failure: Mutex::new(None),
            lifecycle_background_work: std::sync::atomic::AtomicBool::new(false),
            lifecycle_deadline: Mutex::new(None),
            inner: Mutex::new(RecorderRuntime {
                active: None,
                capture_sources: Vec::new(),
                preview: None,
                preview_start_generation: 0,
                preview_status: idle_preview_status(now_ms()),
                status: idle_status(now_ms()),
                pause_owner: None,
                protection_monitor: None,
                pending_failure: None,
                automatic_resume_pending: false,
                cancel_pending_resume: false,
                pause_generation: 0,
            }),
        }
    }

    pub fn status(&self) -> Result<RecorderStatus, RecorderError> {
        if let Some((_, status)) = self
            .forced_failure
            .lock()
            .map_err(|_| {
                lifecycle_error(
                    "recorder_state_unavailable",
                    "Recorder state is unavailable.",
                )
            })?
            .as_ref()
        {
            return Ok(status.clone());
        }
        let mut runtime = self.lock_runtime()?;
        let disposition = status::cleanup_disposition();
        if runtime.status.cleanup_disposition != disposition {
            runtime.status.cleanup_disposition = disposition;
            runtime.status.status_revision = status::next_status_revision();
        }
        Ok(runtime.status.clone())
    }

    pub(crate) fn performance_stats(
        &self,
        timestamp_ms: u64,
    ) -> Result<RecorderPerformanceStats, RecorderError> {
        let runtime = self.lock_runtime()?;
        let active_segment = runtime
            .active
            .as_ref()
            .and_then(|active| active.active_segment.as_ref());

        Ok(RecorderPerformanceStats {
            desired_fps: runtime.active.as_ref().map(|active| active.fps),
            segment_started_at_ms: active_segment.map(|segment| segment.capture_started_at_ms),
            state: runtime.status.state.clone(),
            timing: active_segment
                .and_then(|segment| segment.timing.as_ref())
                .map(capture_timing_snapshot),
            updated_at_ms: timestamp_ms,
        })
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

    pub(crate) fn preview_start_is_current(&self, scene_id: &str, generation: u64) -> bool {
        self.lock_runtime()
            .map(|runtime| {
                runtime.preview_start_generation == generation
                    && runtime.preview_status.state == RecorderPreviewState::Starting
                    && runtime.preview_status.scene_id.as_deref() == Some(scene_id)
                    && runtime.active.is_none()
                    && !status_has_pending_lifecycle(&runtime.status)
            })
            .unwrap_or(false)
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
        let preview_scene_id = preview.as_ref().map(|preview| preview.scene_id.clone());
        info!(
            scene_id = preview_scene_id.as_deref().unwrap_or(""),
            had_live_preview = preview.is_some(),
            "recorder_preview_stop_started"
        );
        if let Some(preview) = preview {
            preview.session.stop()?;
        }

        let mut runtime = self.lock_runtime()?;
        let status = idle_preview_status(timestamp_ms);
        runtime.preview_status = status.clone();
        info!(
            scene_id = preview_scene_id.as_deref().unwrap_or(""),
            "recorder_preview_stop_succeeded"
        );

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
        runtime.status.status_revision = status::next_status_revision();

        Ok(runtime.status.clone())
    }

    pub fn capture_sources(&self) -> Result<CaptureSourceList, RecorderError> {
        Ok(CaptureSourceList {
            sources: self.lock_runtime()?.capture_sources.clone(),
        })
    }

    pub(crate) fn configuration_is_locked(&self) -> Result<bool, RecorderError> {
        let runtime = self.lock_runtime()?;

        Ok(runtime.active.is_some() || status_has_pending_lifecycle(&runtime.status))
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

#[cfg(any(test, not(target_os = "linux")))]
pub(crate) fn unavailable_backend_health() -> RecorderBackendHealth {
    status::unavailable_backend_health()
}

pub(crate) fn unavailable_gpu_capture_health(error: impl Into<String>) -> RecorderGpuCaptureHealth {
    status::unavailable_gpu_capture_health(error)
}
