use super::*;

impl RecorderManager {
    pub(crate) fn begin_selecting(
        &self,
        scene_id: &str,
        timestamp_ms: u64,
    ) -> Result<RecorderStatus, RecorderError> {
        let mut runtime = self.lock_runtime()?;
        if runtime.active.is_some() || is_pending_lifecycle_state(&runtime.status.state) {
            return Err(lifecycle_error(
                "recorder_already_active",
                "A recording is already active.",
            ));
        }

        runtime.status = selecting_status(
            timestamp_ms,
            runtime.status.backend_health.clone(),
            None,
            Some(scene_id.to_string()),
            None,
        );

        Ok(runtime.status.clone())
    }

    pub(crate) fn begin_resume_selecting(
        &self,
        recording_id: &str,
        timestamp_ms: u64,
    ) -> Result<ActiveRecording, RecorderError> {
        let mut runtime = self.lock_runtime()?;
        if is_pending_lifecycle_state(&runtime.status.state) {
            return Err(lifecycle_error(
                "recorder_already_active",
                "A recorder lifecycle transition is already in progress.",
            ));
        }
        let active = runtime
            .active
            .clone()
            .ok_or_else(|| lifecycle_error("recorder_not_paused", "No recording is paused."))?;

        if active.recording_id != recording_id {
            return Err(lifecycle_error(
                "recorder_recording_mismatch",
                "The requested recording is not active.",
            ));
        }
        if active.active_segment.is_some() {
            return Err(lifecycle_error(
                "recorder_not_paused",
                "The active recording is not paused.",
            ));
        }

        runtime.status = status_from_active(
            RecorderState::Selecting,
            &active,
            timestamp_ms,
            runtime.status.backend_health.clone(),
            None,
        );

        Ok(active)
    }

    pub(crate) fn clear_selecting_to_idle(
        &self,
        timestamp_ms: u64,
    ) -> Result<RecorderStatus, RecorderError> {
        let mut runtime = self.lock_runtime()?;
        let status =
            idle_status_with_backend_health(timestamp_ms, runtime.status.backend_health.clone());
        runtime.status = status.clone();

        Ok(status)
    }

    pub(crate) fn restore_paused_after_selecting(
        &self,
        active: ActiveRecording,
        timestamp_ms: u64,
    ) -> Result<RecorderStatus, RecorderError> {
        let mut runtime = self.lock_runtime()?;
        let status = status_from_active(
            RecorderState::Paused,
            &active,
            timestamp_ms,
            runtime.status.backend_health.clone(),
            None,
        );
        runtime.active = Some(active);
        runtime.status = status.clone();

        Ok(status)
    }
}
