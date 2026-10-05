use super::*;

impl RecorderManager {
    pub(crate) fn begin_selecting(
        &self,
        scene_id: &str,
        timestamp_ms: u64,
    ) -> Result<RecorderStatus, RecorderError> {
        let mut runtime = self.lock_runtime()?;
        if runtime.active.is_some() || status_has_pending_lifecycle(&runtime.status) {
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
        self.begin_resume_selecting_with_authority(recording_id, timestamp_ms, None)
    }

    #[cfg(test)]
    pub(crate) fn begin_auto_resume_selecting(
        &self,
        recording_id: &str,
        timestamp_ms: u64,
    ) -> Result<ActiveRecording, RecorderError> {
        let generation = self
            .automatic_resume_authority(recording_id)?
            .ok_or_else(|| {
                lifecycle_error(
                    "recorder_manual_pause",
                    "Automatic resume cannot override a manual pause.",
                )
            })?;
        self.begin_auto_resume_with_authority(recording_id, timestamp_ms, generation)
    }

    pub(crate) fn automatic_resume_authority(
        &self,
        recording_id: &str,
    ) -> Result<Option<u64>, RecorderError> {
        let runtime = self.lock_runtime()?;
        Ok((runtime.pause_owner == Some(PauseOwner::Automatic)
            && runtime
                .active
                .as_ref()
                .is_some_and(|active| active.recording_id == recording_id))
        .then_some(runtime.pause_generation))
    }

    pub(crate) fn begin_auto_resume_with_authority(
        &self,
        recording_id: &str,
        timestamp_ms: u64,
        generation: u64,
    ) -> Result<ActiveRecording, RecorderError> {
        self.begin_resume_selecting_with_authority(recording_id, timestamp_ms, Some(generation))
    }

    fn begin_resume_selecting_with_authority(
        &self,
        recording_id: &str,
        timestamp_ms: u64,
        automatic_generation: Option<u64>,
    ) -> Result<ActiveRecording, RecorderError> {
        let mut runtime = self.lock_runtime()?;
        if automatic_generation.is_some() && runtime.pause_owner != Some(PauseOwner::Automatic) {
            return Err(lifecycle_error(
                "recorder_manual_pause",
                "Automatic resume cannot override a manual pause.",
            ));
        }
        if automatic_generation.is_some_and(|generation| generation != runtime.pause_generation) {
            return Err(lifecycle_error(
                "recorder_stale_automatic_resume",
                "A newer pause canceled automatic resume.",
            ));
        }
        if status_has_pending_lifecycle(&runtime.status) {
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
        if automatic_generation.is_none() {
            runtime.pause_owner = None;
        }
        runtime.automatic_resume_pending = automatic_generation.is_some();
        runtime.cancel_pending_resume = false;

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
        storage: &AppStorage,
        active: ActiveRecording,
        timestamp_ms: u64,
    ) -> Result<RecorderStatus, RecorderError> {
        let mut runtime = self.lock_runtime()?;
        if let Some(error) = runtime.pending_failure.take() {
            drop(runtime);
            return self.abort_recording(storage, &active.recording_id, error, timestamp_ms);
        }
        let status = status_from_active(
            RecorderState::Paused,
            &active,
            timestamp_ms,
            runtime.status.backend_health.clone(),
            None,
        );
        runtime.active = Some(active);
        runtime.automatic_resume_pending = false;
        runtime.cancel_pending_resume = false;
        runtime.status = status.clone();

        Ok(status)
    }
}
