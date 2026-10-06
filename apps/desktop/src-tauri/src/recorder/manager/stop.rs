use super::*;
use tracing::warn;

impl RecorderManager {
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
                let _ = self.abort_recording(storage, recording_id, error.clone(), timestamp_ms);
                return Err(error);
            }
        }

        // Capture is stopped before spans are closed. Completion never publishes
        // processable metadata before every automatic protection span is durable.
        self.stop_protection_monitor()?;
        let protection_barrier = storage
            .with_connection(|connection| storage::get_recording(connection, recording_id))
            .and_then(|recording| {
                let started = recording
                    .started_at_ms
                    .ok_or(rusqlite::Error::InvalidQuery)?;
                super::super::window_protection::close_open_protection_spans(
                    storage,
                    recording_id,
                    (timestamp_ms as i64).saturating_sub(started).max(0),
                    timestamp_ms as i64,
                )
            });
        if protection_barrier.is_err() {
            return self.abort_recording(storage, recording_id, super::super::errors::dependency_error("recorder_window_protection_storage_failed", "Protected spans could not be saved. Recording stopped and AI processing was blocked."), timestamp_ms);
        }

        let mut runtime = self.lock_runtime()?;
        if let Some(error) = runtime.pending_failure.take() {
            drop(runtime);
            return self.abort_recording(storage, recording_id, error, timestamp_ms);
        }
        let completion = storage
            .with_connection(|connection| {
                storage::mark_recording_completed(
                    connection,
                    &active.recording_id,
                    timestamp_ms as i64,
                )?;
                let recording=storage::get_recording(connection,&active.recording_id)?;
                connection.execute("UPDATE recording_live_state SET classified_through_ms=MAX(classified_through_ms,?2) WHERE recording_id=?1",rusqlite::params![active.recording_id,timestamp_ms as i64-recording.started_at_ms.unwrap_or(timestamp_ms as i64)])?;
                storage::get_recording(connection,&active.recording_id)
            })
            .map_err(storage_error);
        if let Err(error) = completion {
            drop(runtime);
            let _ = self.abort_recording(storage, recording_id, error.clone(), timestamp_ms);
            return Err(error);
        }

        let status =
            idle_status_with_backend_health(timestamp_ms, runtime.status.backend_health.clone());
        runtime.active = None;
        runtime.pause_owner = None;
        runtime.automatic_resume_pending = false;
        runtime.status = status.clone();
        drop(runtime);

        match settings::load_app_settings(storage) {
            Ok(app_settings) => {
                let cleanup = storage_limits::enforce_storage_cleanup_at(
                    storage,
                    &self.recordings_root(),
                    &app_settings.storage,
                    timestamp_ms as i64,
                );
                if cleanup.failed_recording_count > 0 {
                    warn!(
                        failed_recording_count = cleanup.failed_recording_count,
                        "Storage cleanup after recording stop had failures."
                    );
                }
            }
            Err(_error) => warn!(
                "Storage cleanup after recording stop skipped because settings failed to load."
            ),
        }

        Ok(status)
    }

    pub(super) fn finalize_media_segment(
        &self,
        storage: &AppStorage,
        active: &ActiveRecording,
        segment: &ActiveSegment,
        timestamp_ms: u64,
    ) -> Result<(), RecorderError> {
        let mut final_segment = segment.clone();
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
                match segment_runtime.stop(timestamp_ms) {
                    Ok(events) => {
                        final_segment =
                            self.save_fragment_events(storage, active, segment, &events)?;
                    }
                    Err(error) => {
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
                }
            } else {
                return Err(lifecycle_error(
                    "recorder_segment_not_active",
                    "The active segment media has already been consumed.",
                ));
            }
        }

        let segment = &final_segment;
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

    pub(super) fn active_for_completion(
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

    pub(super) fn fail_active_recording(
        &self,
        storage: &AppStorage,
        active: &ActiveRecording,
        segment_id: &str,
        timestamp_ms: u64,
        error: &RecorderError,
        writer_exit_code: Option<i64>,
    ) -> Result<(), RecorderError> {
        let segment_failure =
            fail_segment_for_error(storage, segment_id, timestamp_ms, error, writer_exit_code);
        let recording_failure = self
            .abort_recording(storage, &active.recording_id, error.clone(), timestamp_ms)
            .map(|_| ());
        segment_failure.and(recording_failure)
    }
}
