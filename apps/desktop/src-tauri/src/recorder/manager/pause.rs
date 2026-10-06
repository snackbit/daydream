use super::*;

impl RecorderManager {
    pub fn begin_pause_recording(
        &self,
        recording_id: &str,
        timestamp_ms: u64,
    ) -> Result<RecorderStatus, RecorderError> {
        self.begin_pause_with_owner(recording_id, timestamp_ms, PauseOwner::Manual)
            .map(|(status, _)| status)
    }

    pub(crate) fn begin_pause_with_owner(
        &self,
        recording_id: &str,
        timestamp_ms: u64,
        owner: PauseOwner,
    ) -> Result<(RecorderStatus, bool), RecorderError> {
        let mut runtime = self.lock_runtime()?;
        let active = runtime
            .active
            .clone()
            .ok_or_else(|| lifecycle_error("recorder_not_recording", "No recording is active."))?;
        if active.recording_id != recording_id {
            return Err(lifecycle_error(
                "recorder_recording_mismatch",
                "The requested recording is not active.",
            ));
        }
        if runtime
            .status
            .finalization
            .as_ref()
            .is_some_and(|pending| pending.action == RecorderFinalizationAction::Pause)
        {
            if owner == PauseOwner::Manual {
                runtime.pause_owner = Some(owner);
            }
            runtime.pause_generation = runtime.pause_generation.saturating_add(1);
            return Ok((runtime.status.clone(), false));
        }
        if runtime.automatic_resume_pending {
            if owner == PauseOwner::Manual || runtime.pause_owner != Some(PauseOwner::Manual) {
                runtime.pause_owner = Some(owner);
            }
            runtime.pause_generation = runtime.pause_generation.saturating_add(1);
            runtime.cancel_pending_resume = true;
            return Ok((runtime.status.clone(), false));
        }
        if status_has_pending_lifecycle(&runtime.status) {
            return Err(lifecycle_error(
                "recorder_already_active",
                "A recorder lifecycle transition is already in progress.",
            ));
        }
        if active.active_segment.is_none() {
            if owner == PauseOwner::Manual {
                runtime.pause_owner = Some(owner);
            }
            runtime.pause_generation = runtime.pause_generation.saturating_add(1);
            return Ok((runtime.status.clone(), false));
        }

        let segment = active
            .active_segment
            .as_ref()
            .ok_or_else(|| lifecycle_error("recorder_not_recording", "No segment is recording."))?;
        let status = status_for_finalizing_pause(
            &active,
            segment,
            timestamp_ms,
            runtime.status.backend_health.clone(),
        );
        runtime.status = status.clone();
        runtime.pause_owner = Some(owner);
        runtime.pause_generation = runtime.pause_generation.saturating_add(1);

        Ok((status, true))
    }

    pub fn complete_pause_recording(
        &self,
        storage: &AppStorage,
        recording_id: &str,
        timestamp_ms: u64,
    ) -> Result<RecorderStatus, RecorderError> {
        let active = self.active_for_completion(recording_id, RecorderFinalizationAction::Pause)?;
        let segment = active
            .active_segment
            .clone()
            .ok_or_else(|| lifecycle_error("recorder_not_recording", "No segment is recording."))?;

        if let Err(error) = self.finalize_media_segment(storage, &active, &segment, timestamp_ms) {
            let _ = self.abort_recording(storage, recording_id, error.clone(), timestamp_ms);
            return Err(error);
        }
        let mut paused = active;
        paused.active_segment = None;
        paused.next_segment_index = storage
            .with_connection(|connection| {
                storage::list_recording_segments(connection, recording_id)
            })
            .map_err(storage_error)?
            .iter()
            .map(|segment| segment.segment_index.max(0) as u32 + 1)
            .max()
            .unwrap_or(paused.next_segment_index);
        let mut runtime = self.lock_runtime()?;
        if let Some(error) = runtime.pending_failure.take() {
            drop(runtime);
            return self.abort_recording(storage, recording_id, error, timestamp_ms);
        }
        if let Err(error) = mark_recording_paused(storage, &paused.recording_id) {
            drop(runtime);
            let _ = self.abort_recording(storage, recording_id, error.clone(), timestamp_ms);
            return Err(error);
        }
        let status = status_from_active(
            RecorderState::Paused,
            &paused,
            timestamp_ms,
            runtime.status.backend_health.clone(),
            None,
        );
        runtime.active = Some(paused);
        runtime.status = status.clone();

        Ok(status)
    }
}

fn mark_recording_paused(storage: &AppStorage, recording_id: &str) -> Result<(), RecorderError> {
    storage
        .with_connection(|connection| {
            storage::update_recording_status(connection, recording_id, "paused")
        })
        .map(|_row| ())
        .map_err(storage_error)
}
