//! Forget capture selections only after maintenance has drained their owners.
use super::*;

impl RecorderManager {
    pub(crate) fn reset_preferences_cache(&self) -> Result<(), RecorderError> {
        let mut runtime = self.inner.try_lock().map_err(|_| lifecycle_error(
            "recorder_preferences_cache_busy",
            "Capture setup is still owned by another operation. Restart Daydream before retrying preference reset.",
        ))?;
        if runtime.active.is_some()
            || runtime.preview.is_some()
            || !matches!(
                runtime.status.state,
                RecorderState::Idle | RecorderState::Error | RecorderState::Unavailable
            )
            || runtime.status.finalization.is_some()
            || !matches!(
                runtime.preview_status.state,
                RecorderPreviewState::Idle | RecorderPreviewState::Error
            )
        {
            return Err(lifecycle_error(
                "recorder_preferences_cache_active",
                "Stop recording and preview before retrying preference reset.",
            ));
        }
        runtime.capture_sources.clear();
        runtime.preview_start_generation = runtime.preview_start_generation.saturating_add(1);
        runtime.preview_status = status::idle_preview_status(now_ms());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn resetting_capture_preferences_forgets_selection_but_preserves_status() {
        let manager = RecorderManager::new("/tmp/synthetic-preference-cache".into());
        manager
            .set_capture_sources(vec![CaptureSource {
                display_name: "Synthetic chosen display".into(),
                height: Some(64),
                id: "synthetic-selected".into(),
                kind: super::super::super::CaptureSourceKind::Monitor,
                logical_height: None,
                logical_width: None,
                scale_factor: None,
                width: Some(64),
            }])
            .unwrap();
        let status = manager.status().unwrap();
        manager.reset_preferences_cache().unwrap();
        assert!(manager.capture_sources().unwrap().sources.is_empty());
        assert_eq!(manager.status().unwrap(), status);
        assert_eq!(
            manager.preview_status().unwrap().state,
            RecorderPreviewState::Idle
        );
    }
    #[test]
    fn unexpected_pending_or_locked_capture_owner_cannot_be_silently_reset() {
        let manager = RecorderManager::new("/tmp/synthetic-preference-cache-pending".into());
        let mut runtime = manager.inner.lock().unwrap();
        assert!(
            manager.reset_preferences_cache().is_err(),
            "Busy runtime must fail immediately without waiting on its lock"
        );
        runtime.status.state = RecorderState::Paused;
        drop(runtime);
        assert!(manager.reset_preferences_cache().is_err());
    }
}
