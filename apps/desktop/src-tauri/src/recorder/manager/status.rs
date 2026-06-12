use super::super::types::{
    RecorderAudioHealth, RecorderBackendComponentHealth, RecorderBackendHealth, RecorderError,
    RecorderFinalizationAction, RecorderFinalizationPhase, RecorderFinalizationStatus,
    RecorderGpuCaptureHealth, RecorderMediaWriterHealth, RecorderPortalHealth,
    RecorderPreviewState, RecorderPreviewStatus, RecorderState, RecorderStatus,
};
use super::{ActiveRecording, ActiveSegment};

pub(super) fn status_from_active(
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

pub(super) fn idle_status(updated_at_ms: u64) -> RecorderStatus {
    idle_status_with_backend_health(updated_at_ms, unavailable_backend_health())
}

pub(super) fn idle_preview_status(updated_at_ms: u64) -> RecorderPreviewStatus {
    RecorderPreviewStatus {
        error: None,
        scene_id: None,
        source_ids: Vec::new(),
        state: RecorderPreviewState::Idle,
        updated_at_ms,
    }
}

pub(super) fn idle_status_with_backend_health(
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

pub(super) fn selecting_status(
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

pub(super) fn error_status_with_backend_health(
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

pub(super) fn status_for_finalizing_pause(
    active: &ActiveRecording,
    segment: &ActiveSegment,
    timestamp_ms: u64,
    backend_health: RecorderBackendHealth,
) -> RecorderStatus {
    RecorderStatus {
        active_recording_id: Some(active.recording_id.clone()),
        active_scene_id: Some(active.scene_id.clone()),
        active_segment_id: Some(segment.id.clone()),
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

pub(super) fn status_for_finalizing_stop(
    active: &ActiveRecording,
    timestamp_ms: u64,
    backend_health: RecorderBackendHealth,
) -> RecorderStatus {
    RecorderStatus {
        active_recording_id: None,
        active_scene_id: None,
        active_segment_id: active
            .active_segment
            .as_ref()
            .map(|segment| segment.id.clone()),
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

pub(super) fn status_has_pending_lifecycle(status: &RecorderStatus) -> bool {
    status.finalization.is_some() || is_pending_lifecycle_state(&status.state)
}

pub(super) fn is_pending_lifecycle_state(state: &RecorderState) -> bool {
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
        gpu_capture: unavailable_gpu_capture_health("Recorder backend is not implemented yet."),
        media_writer: RecorderMediaWriterHealth {
            available: false,
            capabilities: Default::default(),
            error: Some("Recorder backend is not implemented yet.".to_string()),
            version: None,
            writer: None,
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

pub(crate) fn unavailable_gpu_capture_health(error: impl Into<String>) -> RecorderGpuCaptureHealth {
    RecorderGpuCaptureHealth {
        available: false,
        cleanup_error: None,
        dmabuf_import_available: false,
        egl_available: false,
        egl_client_apis: Vec::new(),
        egl_platform: None,
        egl_vendor: None,
        egl_version: None,
        error: Some(error.into()),
        modifier_import_available: None,
        opengl_available: false,
        renderer: None,
        vendor: None,
        version: None,
    }
}
