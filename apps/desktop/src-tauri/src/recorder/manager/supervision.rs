use super::super::window_protection::WindowProtectionMonitor;
use super::*;

impl RecorderManager {
    pub(crate) fn attach_protection_monitor(
        &self,
        recording_id: &str,
        monitor: WindowProtectionMonitor,
    ) -> Result<(), RecorderError> {
        let mut runtime = self.lock_runtime()?;
        if runtime
            .active
            .as_ref()
            .is_some_and(|active| active.recording_id == recording_id)
            && !status_has_pending_lifecycle(&runtime.status)
        {
            runtime.protection_monitor = Some(monitor);
        } else {
            drop(runtime);
            monitor.stop();
        }
        Ok(())
    }

    pub(crate) fn stop_protection_monitor(&self) -> Result<(), RecorderError> {
        let monitor = self.lock_runtime()?.protection_monitor.take();
        if let Some(monitor) = monitor {
            monitor.stop();
        }
        Ok(())
    }

    pub(crate) fn active_runtime_error(
        &self,
        recording_id: &str,
    ) -> Result<Option<RecorderError>, RecorderError> {
        let segment_runtime = {
            let runtime = self.lock_runtime()?;
            if status_has_pending_lifecycle(&runtime.status) {
                return Ok(None);
            }
            runtime
                .active
                .as_ref()
                .filter(|active| active.recording_id == recording_id)
                .and_then(|active| active.active_segment.as_ref())
                .and_then(|segment| segment.runtime.clone())
        };
        let Some(segment_runtime) = segment_runtime else {
            return Ok(None);
        };
        let mut segment = segment_runtime.lock().map_err(|_| {
            lifecycle_error(
                "recorder_state_unavailable",
                "Recorder runtime lock is unavailable.",
            )
        })?;
        Ok(segment
            .as_mut()
            .and_then(RecordingSegmentRuntime::poll_error))
    }

    #[cfg(test)]
    pub(crate) fn report_active_failure(
        &self,
        storage: &AppStorage,
        recording_id: &str,
        error: RecorderError,
        timestamp_ms: u64,
    ) -> Result<RecorderStatus, RecorderError> {
        let (status, abort) = self.mark_recording_failure(recording_id, &error, timestamp_ms)?;
        if abort {
            self.abort_recording(storage, recording_id, error, timestamp_ms)
        } else {
            Ok(status)
        }
    }

    pub(crate) fn mark_recording_failure(
        &self,
        recording_id: &str,
        error: &RecorderError,
        timestamp_ms: u64,
    ) -> Result<(RecorderStatus, bool), RecorderError> {
        let mut runtime = self.lock_runtime()?;
        if runtime
            .active
            .as_ref()
            .is_none_or(|active| active.recording_id != recording_id)
        {
            return Ok((runtime.status.clone(), false));
        }
        if status_has_pending_lifecycle(&runtime.status) {
            runtime.pending_failure = Some(error.clone());
            runtime.status.last_error = Some(error.clone());
            return Ok((runtime.status.clone(), false));
        }
        runtime.status = error_status_with_backend_health(
            timestamp_ms,
            runtime.status.backend_health.clone(),
            error.clone(),
        );
        Ok((runtime.status.clone(), true))
    }

    pub(crate) fn abort_recording(
        &self,
        storage: &AppStorage,
        recording_id: &str,
        error: RecorderError,
        timestamp_ms: u64,
    ) -> Result<RecorderStatus, RecorderError> {
        let active = {
            let mut runtime = self.lock_runtime()?;
            if runtime
                .active
                .as_ref()
                .is_none_or(|active| active.recording_id != recording_id)
            {
                return Ok(runtime.status.clone());
            }
            let active = runtime.active.take().expect("active recording checked");
            runtime.status = error_status_with_backend_health(
                timestamp_ms,
                runtime.status.backend_health.clone(),
                error.clone(),
            );
            runtime.pause_owner = None;
            runtime.pending_failure = None;
            runtime.automatic_resume_pending = false;
            runtime.cancel_pending_resume = false;
            active
        };
        // Change visible state before joining workers. No resume can claim this recording.
        self.stop_protection_monitor()?;
        if let Some(segment) = &active.active_segment {
            if let Some(shared) = &segment.runtime {
                if let Some(media) = shared
                    .lock()
                    .map_err(|_| {
                        lifecycle_error(
                            "recorder_state_unavailable",
                            "Recorder runtime lock is unavailable.",
                        )
                    })?
                    .take()
                {
                    media.abort(timestamp_ms);
                }
            }
            let _ = fail_segment_for_error(storage, &segment.id, timestamp_ms, &error, None);
        }
        storage
            .with_connection_mut(|connection| {
                connection.execute("UPDATE processing_job_runtime SET input_open=0 WHERE job_id IN(SELECT id FROM processing_jobs WHERE recording_id=?1)",[recording_id])?;
                storage::update_recording_status(connection, recording_id, "failed")?;
                let transaction = connection.transaction()?;
                let sealed = crate::recording_privacy::seal_interrupted_capture(&transaction,recording_id,timestamp_ms as i64);
                if sealed.is_ok() { transaction.commit()?; } else {
                    // Keep unresolved spans intact when creating their masks fails.
                    transaction.rollback()?;
                    tracing::warn!(recording_id,"recorder_window_protection_recovery_pending");
                }
                storage::get_recording(connection,recording_id)
            })
            .map_err(storage_error)?;
        self.status()
    }
}
