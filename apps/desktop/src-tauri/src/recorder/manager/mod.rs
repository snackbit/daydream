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
mod preview;
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
    // Serializes resource cleanup without holding the status/runtime lock.
    preview_lifecycle: Mutex<()>,
    forced_failure: Mutex<Option<(String, RecorderStatus)>>,
    lifecycle_background_work: std::sync::atomic::AtomicBool,
    lifecycle_deadline: Mutex<Option<std::time::Instant>>,
}

impl RecorderManager {
    pub fn new(app_data_dir: PathBuf) -> Self {
        Self {
            preview_demand: Arc::new(Mutex::new(None)),
            app_data_dir,
            preview_lifecycle: Mutex::new(()),
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

pub(crate) fn unavailable_backend_health() -> RecorderBackendHealth {
    status::unavailable_backend_health()
}

pub(crate) fn unavailable_gpu_capture_health(error: impl Into<String>) -> RecorderGpuCaptureHealth {
    status::unavailable_gpu_capture_health(error)
}
