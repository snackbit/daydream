use super::*;

impl RecorderManager {
    pub(crate) fn preview_status(&self) -> Result<RecorderPreviewStatus, RecorderError> {
        Ok(self.lock_runtime()?.preview_status.clone())
    }

    pub(crate) fn begin_preview_start(
        &self,
        scene_id: &str,
        timestamp_ms: u64,
    ) -> Result<PreviewStartDecision, RecorderError> {
        let _lifecycle = self.lock_preview_lifecycle()?;
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
        let _lifecycle = self.lock_preview_lifecycle()?;
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

    pub(crate) fn cancel_preview_start(
        &self,
        scene_id: &str,
        generation: u64,
        timestamp_ms: u64,
    ) -> Result<RecorderPreviewStatus, RecorderError> {
        let _lifecycle = self.lock_preview_lifecycle()?;
        let mut runtime = self.lock_runtime()?;
        if runtime.preview_start_generation == generation
            && runtime.preview_status.state == RecorderPreviewState::Starting
            && runtime.preview_status.scene_id.as_deref() == Some(scene_id)
        {
            runtime.preview_start_generation = runtime.preview_start_generation.saturating_add(1);
            runtime.preview_status = idle_preview_status(timestamp_ms);
        }
        Ok(runtime.preview_status.clone())
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
        let _lifecycle = self.lock_preview_lifecycle()?;
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
            if let Err(error) = preview.session.stop() {
                let mut runtime = self.lock_runtime()?;
                runtime.preview_status = RecorderPreviewStatus {
                    error: Some(error.clone()),
                    scene_id: preview_scene_id,
                    source_ids: Vec::new(),
                    state: RecorderPreviewState::Error,
                    updated_at_ms: timestamp_ms,
                };
                return Err(error);
            }
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

    fn lock_preview_lifecycle(&self) -> Result<std::sync::MutexGuard<'_, ()>, RecorderError> {
        self.preview_lifecycle.lock().map_err(|_| {
            lifecycle_error(
                "recorder_state_unavailable",
                "Preview lifecycle is unavailable.",
            )
        })
    }
}
