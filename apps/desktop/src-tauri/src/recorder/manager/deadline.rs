//! Terminal failure remains readable even while a capture/storage owner is blocked.
use super::*;

impl RecorderManager {
    pub(crate) fn set_lifecycle_background_work(&self, active: bool) {
        self.lifecycle_background_work
            .store(active, std::sync::atomic::Ordering::SeqCst);
    }
    pub(crate) fn lifecycle_deadline(&self) -> std::time::Instant {
        self.lifecycle_deadline
            .lock()
            .ok()
            .and_then(|deadline| *deadline)
            .unwrap_or_else(|| std::time::Instant::now() + std::time::Duration::from_secs(20))
    }
    pub(super) fn begin_lifecycle_deadline(&self) {
        if let Ok(mut deadline) = self.lifecycle_deadline.lock() {
            *deadline = Some(std::time::Instant::now() + std::time::Duration::from_secs(20));
        }
    }
    pub(crate) fn force_lifecycle_failure(
        &self,
        recording_id: &str,
        backend_health: RecorderBackendHealth,
        error: RecorderError,
        timestamp_ms: u64,
    ) -> RecorderStatus {
        #[cfg(target_os = "linux")]
        super::super::media_worker::quarantine();
        let status = error_status_with_backend_health(timestamp_ms, backend_health, error);
        if let Ok(mut failure) = self.forced_failure.lock() {
            *failure = Some((recording_id.into(), status.clone()));
        }
        status
    }
    pub(crate) fn forced_lifecycle_error(&self, recording_id: &str) -> Option<RecorderError> {
        self.forced_failure.lock().ok().and_then(|failure| {
            failure
                .as_ref()
                .filter(|(id, _)| id == recording_id)
                .and_then(|(_, status)| status.last_error.clone())
        })
    }
}
