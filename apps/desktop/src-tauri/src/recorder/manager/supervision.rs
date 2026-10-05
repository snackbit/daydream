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
            .with_connection(|connection| {
                storage::update_recording_status(connection, recording_id, "failed")
            })
            .map_err(storage_error)?;
        self.status()
    }
}
