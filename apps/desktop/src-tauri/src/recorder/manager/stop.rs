use super::*;
use tracing::warn;

impl RecorderManager {
    pub fn pause_internal(
        &self,
        storage: &AppStorage,
        timestamp_ms: u64,
    ) -> Result<RecorderStatus, RecorderError> {
        let active =
            self.lock_runtime()?.active.clone().ok_or_else(|| {
                lifecycle_error("recorder_not_recording", "No recording is active.")
            })?;
        let segment = active
            .active_segment
            .clone()
            .ok_or_else(|| lifecycle_error("recorder_not_recording", "No segment is recording."))?;

        storage
            .with_connection(|connection| {
                storage::finalize_recording_segment(
                    connection,
                    &FinalizeRecordingSegmentInput {
                        desktop_audio_stream_index: None,
                        duration_ms: duration_ms(segment.started_at_ms, timestamp_ms) as i64,
                        ended_at_ms: timestamp_ms as i64,
                        id: segment.id.clone(),
                        microphone_audio_stream_index: None,
                        probe_json: None,
                        updated_at_ms: timestamp_ms as i64,
                        video_stream_index: None,
                    },
                )?;
                storage::update_recording_status(connection, &active.recording_id, "paused")
            })
            .map_err(storage_error)?;

        let mut paused = active;
        paused.active_segment = None;
        let mut runtime = self.lock_runtime()?;
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

    pub fn pause_recording(
        &self,
        storage: &AppStorage,
        recording_id: &str,
        timestamp_ms: u64,
    ) -> Result<RecorderStatus, RecorderError> {
        let active =
            self.lock_runtime()?.active.clone().ok_or_else(|| {
                lifecycle_error("recorder_not_recording", "No recording is active.")
            })?;
        if active.recording_id != recording_id {
            return Err(lifecycle_error(
                "recorder_recording_mismatch",
                "The requested recording is not active.",
            ));
        }
        let segment = active
            .active_segment
            .clone()
            .ok_or_else(|| lifecycle_error("recorder_not_recording", "No segment is recording."))?;

        self.finalize_media_segment(storage, &active, &segment, timestamp_ms)?;
        mark_recording_paused(storage, &active.recording_id)?;

        let mut paused = active;
        paused.active_segment = None;
        let mut runtime = self.lock_runtime()?;
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

    pub fn begin_pause_recording(
        &self,
        recording_id: &str,
        timestamp_ms: u64,
    ) -> Result<RecorderStatus, RecorderError> {
        let mut runtime = self.lock_runtime()?;
        if status_has_pending_lifecycle(&runtime.status) {
            return Err(lifecycle_error(
                "recorder_already_active",
                "A recorder lifecycle transition is already in progress.",
            ));
        }
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
        if active.active_segment.is_none() {
            return Err(lifecycle_error(
                "recorder_not_recording",
                "No segment is recording.",
            ));
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

        Ok(status)
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
            self.ensure_error_status_after_completion_failure(&error, timestamp_ms)?;
            return Err(error);
        }
        if let Err(error) = mark_recording_paused(storage, &active.recording_id) {
            self.ensure_error_status_after_completion_failure(&error, timestamp_ms)?;
            return Err(error);
        }

        let mut paused = active;
        paused.active_segment = None;
        let mut runtime = self.lock_runtime()?;
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
    pub fn stop_internal(
        &self,
        storage: &AppStorage,
        timestamp_ms: u64,
    ) -> Result<RecorderStatus, RecorderError> {
        let Some(active) = self.lock_runtime()?.active.clone() else {
            let mut runtime = self.lock_runtime()?;
            let status = idle_status_with_backend_health(
                timestamp_ms,
                runtime.status.backend_health.clone(),
            );
            runtime.status = status.clone();
            return Ok(status);
        };

        storage
            .with_connection(|connection| {
                if let Some(segment) = &active.active_segment {
                    storage::finalize_recording_segment(
                        connection,
                        &FinalizeRecordingSegmentInput {
                            desktop_audio_stream_index: None,
                            duration_ms: duration_ms(segment.started_at_ms, timestamp_ms) as i64,
                            ended_at_ms: timestamp_ms as i64,
                            id: segment.id.clone(),
                            microphone_audio_stream_index: None,
                            probe_json: None,
                            updated_at_ms: timestamp_ms as i64,
                            video_stream_index: None,
                        },
                    )?;
                }

                storage::mark_recording_completed(
                    connection,
                    &active.recording_id,
                    timestamp_ms as i64,
                )
            })
            .map_err(storage_error)?;

        let mut runtime = self.lock_runtime()?;
        let status =
            idle_status_with_backend_health(timestamp_ms, runtime.status.backend_health.clone());
        runtime.active = None;
        runtime.status = status.clone();

        Ok(status)
    }

    pub fn stop_recording(
        &self,
        storage: &AppStorage,
        recording_id: &str,
        timestamp_ms: u64,
    ) -> Result<RecorderStatus, RecorderError> {
        let Some(active) = self.lock_runtime()?.active.clone() else {
            let mut runtime = self.lock_runtime()?;
            let status = idle_status_with_backend_health(
                timestamp_ms,
                runtime.status.backend_health.clone(),
            );
            runtime.status = status.clone();
            return Ok(status);
        };

        if active.recording_id != recording_id {
            return Err(lifecycle_error(
                "recorder_recording_mismatch",
                "The requested recording is not active.",
            ));
        }

        if let Some(segment) = &active.active_segment {
            self.finalize_media_segment(storage, &active, segment, timestamp_ms)?;
        }

        storage
            .with_connection(|connection| {
                storage::mark_recording_completed(
                    connection,
                    &active.recording_id,
                    timestamp_ms as i64,
                )
            })
            .map_err(storage_error)?;

        let mut runtime = self.lock_runtime()?;
        let status =
            idle_status_with_backend_health(timestamp_ms, runtime.status.backend_health.clone());
        runtime.active = None;
        runtime.status = status.clone();

        Ok(status)
    }

    pub fn begin_stop_recording(
        &self,
        recording_id: &str,
        timestamp_ms: u64,
    ) -> Result<RecorderStatus, RecorderError> {
        let mut runtime = self.lock_runtime()?;
        if status_has_pending_lifecycle(&runtime.status) {
            return Err(lifecycle_error(
                "recorder_already_active",
                "A recorder lifecycle transition is already in progress.",
            ));
        }
        let Some(active) = runtime.active.clone() else {
            let status = idle_status_with_backend_health(
                timestamp_ms,
                runtime.status.backend_health.clone(),
            );
            runtime.status = status.clone();
            return Ok(status);
        };

        if active.recording_id != recording_id {
            return Err(lifecycle_error(
                "recorder_recording_mismatch",
                "The requested recording is not active.",
            ));
        }

        let status = status_for_finalizing_stop(
            &active,
            timestamp_ms,
            runtime.status.backend_health.clone(),
        );
        runtime.status = status.clone();

        Ok(status)
    }

    pub fn complete_stop_recording(
        &self,
        storage: &AppStorage,
        recording_id: &str,
        timestamp_ms: u64,
    ) -> Result<RecorderStatus, RecorderError> {
        let active = self.active_for_completion(recording_id, RecorderFinalizationAction::Stop)?;

        if let Some(segment) = &active.active_segment {
            if let Err(error) = self.finalize_media_segment(storage, &active, segment, timestamp_ms)
            {
                self.ensure_error_status_after_completion_failure(&error, timestamp_ms)?;
                return Err(error);
            }
        }

        storage
            .with_connection(|connection| {
                storage::mark_recording_completed(
                    connection,
                    &active.recording_id,
                    timestamp_ms as i64,
                )
            })
            .map_err(storage_error)?;

        let mut runtime = self.lock_runtime()?;
        let status =
            idle_status_with_backend_health(timestamp_ms, runtime.status.backend_health.clone());
        runtime.active = None;
        runtime.status = status.clone();

        Ok(status)
    }

    pub fn cleanup_internal(
        &self,
        storage: &AppStorage,
        timestamp_ms: u64,
    ) -> Result<RecorderStatus, RecorderError> {
        self.stop_internal(storage, timestamp_ms)
    }
    fn finalize_media_segment(
        &self,
        storage: &AppStorage,
        active: &ActiveRecording,
        segment: &ActiveSegment,
        timestamp_ms: u64,
    ) -> Result<(), RecorderError> {
        if let Some(runtime) = &segment.runtime {
            let segment_runtime = runtime
                .lock()
                .map_err(|_error| {
                    lifecycle_error(
                        "recorder_state_unavailable",
                        "Segment media state lock is unavailable.",
                    )
                })?
                .take();

            if let Some(segment_runtime) = segment_runtime {
                if let Err(error) = segment_runtime.stop(timestamp_ms) {
                    self.fail_active_recording(
                        storage,
                        active,
                        &segment.id,
                        timestamp_ms,
                        &error,
                        None,
                    )?;
                    return Err(error);
                }
            } else {
                return Err(lifecycle_error(
                    "recorder_segment_not_active",
                    "The active segment media has already been consumed.",
                ));
            }
        }

        let probe = media_writer::probe_segment_best_effort(&segment.file_path);
        let wall_duration_ms = duration_ms(segment.started_at_ms, timestamp_ms);
        if let Some(probe_duration_ms) = probe.duration_ms {
            if probe_duration_ms > wall_duration_ms.saturating_add(100) {
                warn!(
                    recording_id = active.recording_id,
                    segment_id = segment.id,
                    probe_duration_ms,
                    wall_duration_ms,
                    "recorder_segment_probe_duration_exceeded_wall_time"
                );
            }
        }
        let duration_ms = wall_duration_ms;
        let probe_json = media_writer::probe_json_string(&probe);

        storage
            .with_connection(|connection| {
                storage::finalize_recording_segment(
                    connection,
                    &FinalizeRecordingSegmentInput {
                        desktop_audio_stream_index: segment
                            .stream_indexes
                            .desktop_audio_stream_index,
                        duration_ms: i64::try_from(duration_ms).unwrap_or(i64::MAX),
                        ended_at_ms: timestamp_ms as i64,
                        id: segment.id.clone(),
                        microphone_audio_stream_index: segment
                            .stream_indexes
                            .microphone_audio_stream_index,
                        probe_json: Some(probe_json),
                        updated_at_ms: timestamp_ms as i64,
                        video_stream_index: Some(segment.stream_indexes.video_stream_index),
                    },
                )
            })
            .map_err(storage_error)?;

        Ok(())
    }

    fn active_for_completion(
        &self,
        recording_id: &str,
        expected_action: RecorderFinalizationAction,
    ) -> Result<ActiveRecording, RecorderError> {
        let runtime = self.lock_runtime()?;
        let Some(finalization) = &runtime.status.finalization else {
            return Err(lifecycle_error(
                "recorder_invalid_transition",
                "The recorder is not waiting for that lifecycle completion.",
            ));
        };
        if finalization.action != expected_action {
            return Err(lifecycle_error(
                "recorder_invalid_transition",
                "The recorder is not waiting for that lifecycle completion.",
            ));
        }
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

        Ok(active)
    }

    fn ensure_error_status_after_completion_failure(
        &self,
        error: &RecorderError,
        timestamp_ms: u64,
    ) -> Result<(), RecorderError> {
        let mut runtime = self.lock_runtime()?;
        if runtime.status.state == RecorderState::Error {
            return Ok(());
        }

        let status = error_status_with_backend_health(
            timestamp_ms,
            runtime.status.backend_health.clone(),
            error.clone(),
        );
        runtime.active = None;
        runtime.status = status;

        Ok(())
    }

    pub(super) fn fail_active_recording(
        &self,
        storage: &AppStorage,
        active: &ActiveRecording,
        segment_id: &str,
        timestamp_ms: u64,
        error: &RecorderError,
        writer_exit_code: Option<i64>,
    ) -> Result<(), RecorderError> {
        fail_segment_for_error(storage, segment_id, timestamp_ms, error, writer_exit_code)?;
        storage
            .with_connection(|connection| {
                storage::update_recording_status(connection, &active.recording_id, "failed")
            })
            .map_err(storage_error)?;

        let mut runtime = self.lock_runtime()?;
        let status = error_status_with_backend_health(
            timestamp_ms,
            runtime.status.backend_health.clone(),
            error.clone(),
        );
        runtime.active = None;
        runtime.status = status;

        Ok(())
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
