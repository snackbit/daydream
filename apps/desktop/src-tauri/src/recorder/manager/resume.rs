use super::*;

impl RecorderManager {
    pub(crate) fn prepare_resume_segment_start(
        &self,
        storage: &AppStorage,
        active: ActiveRecording,
        timestamp_ms: u64,
    ) -> Result<PendingSegmentStart, RecorderError> {
        let media_writer_health = media_writer::collect_media_writer_health();
        let scene = storage
            .with_connection(|connection| storage::get_recorder_scene(connection, &active.scene_id))
            .map_err(|error| scene_or_storage_error(error, &active.scene_id))
            .and_then(scene_from_storage)?;

        let segment_index = active.next_segment_index;
        let segment_id = Uuid::now_v7().to_string();
        let segment_path = active.segments_dir.join(segment_file_name(segment_index));
        let benchmarks_dir = active
            .segments_dir
            .parent()
            .map(|recording_dir| recording_dir.join("benchmarks"))
            .unwrap_or_else(|| active.segments_dir.join("benchmarks"));
        let segment_input = MediaSegmentInput {
            benchmarks_dir,
            canvas_height: active.canvas_height,
            canvas_width: active.canvas_width,
            desktop_audio_source_id: active.desktop_audio_source_id.clone(),
            fps: active.fps,
            microphone_audio_source_id: active.microphone_audio_source_id.clone(),
            output_path: segment_path.clone(),
            recording_id: active.recording_id.clone(),
            segment_id: segment_id.clone(),
            segment_index,
            started_at_ms: timestamp_ms,
            video_bitrate_kbps: active.video_bitrate_kbps,
        };
        media_writer::validate_segment_capabilities(
            &media_writer_health.capabilities,
            &segment_input,
        )?;
        let segment = ActiveSegment {
            file_path: segment_path.clone(),
            id: segment_id,
            runtime: None,
            started_at_ms: timestamp_ms,
            stream_indexes: media_writer::stream_indexes(&segment_input),
            timing: None,
        };

        let mut active = active;
        active.media_writer = media_writer_health
            .writer
            .clone()
            .unwrap_or_else(|| "gstreamer".to_string());

        Ok(PendingSegmentStart {
            active,
            segment,
            segment_path,
            segment_index,
            segment_input,
            scene,
        })
    }

    pub(crate) fn create_pending_segment_rows(
        &self,
        storage: &AppStorage,
        pending: &PendingSegmentStart,
    ) -> Result<(), RecorderError> {
        fs::create_dir_all(
            pending
                .segment_path
                .parent()
                .ok_or_else(|| filesystem_error(std::io::Error::other("missing segment parent")))?,
        )
        .map_err(filesystem_error)?;

        storage
            .with_connection_mut(|connection| {
                let transaction = connection.transaction()?;
                storage::update_recording_status(
                    &transaction,
                    &pending.active.recording_id,
                    "recording",
                )?;
                storage::create_recording_segment(
                    &transaction,
                    &CreateRecordingSegmentInput {
                        file_path: path_to_string(&pending.segment_path),
                        id: pending.segment.id.clone(),
                        recording_id: pending.active.recording_id.clone(),
                        segment_index: pending.segment_index as i64,
                        started_at_ms: pending.segment.started_at_ms as i64,
                    },
                )?;
                transaction.commit()
            })
            .map_err(storage_error)
    }

    pub(crate) fn activate_pending_segment(
        &self,
        storage: &AppStorage,
        mut pending: PendingSegmentStart,
        runtime: RecordingSegmentRuntime,
        timestamp_ms: u64,
    ) -> Result<RecorderStatus, RecorderError> {
        let mut manager_runtime = self.lock_runtime()?;
        if let Some(error) = manager_runtime.pending_failure.take() {
            drop(manager_runtime);
            runtime.abort(timestamp_ms);
            return self.abort_recording(
                storage,
                &pending.active.recording_id,
                error,
                timestamp_ms,
            );
        }
        pending.segment.timing = runtime.capture_timing();
        if manager_runtime.automatic_resume_pending && manager_runtime.cancel_pending_resume {
            drop(manager_runtime);
            runtime.abort(timestamp_ms);
            let error = lifecycle_error(
                "recorder_resume_canceled",
                "A newer pause canceled automatic resume.",
            );
            let persisted =
                fail_segment_for_error(storage, &pending.segment.id, timestamp_ms, &error, None)
                    .and_then(|_| {
                        storage
                            .with_connection(|connection| {
                                storage::update_recording_status(
                                    connection,
                                    &pending.active.recording_id,
                                    "paused",
                                )
                            })
                            .map(|_| ())
                            .map_err(storage_error)
                    });
            if let Err(error) = persisted {
                return self.abort_recording(
                    storage,
                    &pending.active.recording_id,
                    error,
                    timestamp_ms,
                );
            }
            pending.active.next_segment_index += 1;
            return self.restore_paused_after_selecting(storage, pending.active, timestamp_ms);
        }
        pending.segment.runtime = Some(Arc::new(Mutex::new(Some(runtime))));
        pending.active.active_segment = Some(pending.segment);
        pending.active.next_segment_index += 1;

        let runtime = &mut manager_runtime;
        let status = status_from_active(
            RecorderState::Recording,
            &pending.active,
            timestamp_ms,
            runtime.status.backend_health.clone(),
            None,
        );
        runtime.active = Some(pending.active);
        runtime.pause_owner = None;
        runtime.automatic_resume_pending = false;
        runtime.cancel_pending_resume = false;
        runtime.status = status.clone();

        Ok(status)
    }

    #[cfg(test)]
    pub(crate) fn activate_pending_segment_without_runtime_for_test(
        &self,
        mut pending: PendingSegmentStart,
        timestamp_ms: u64,
    ) -> Result<RecorderStatus, RecorderError> {
        pending.active.active_segment = Some(pending.segment);
        pending.active.next_segment_index += 1;

        let mut runtime = self.lock_runtime()?;
        let status = status_from_active(
            RecorderState::Recording,
            &pending.active,
            timestamp_ms,
            runtime.status.backend_health.clone(),
            None,
        );
        runtime.active = Some(pending.active);
        runtime.pause_owner = None;
        runtime.status = status.clone();

        Ok(status)
    }

    pub(crate) fn fail_pending_segment(
        &self,
        storage: &AppStorage,
        pending: &PendingSegmentStart,
        timestamp_ms: u64,
        error: &RecorderError,
    ) -> Result<(), RecorderError> {
        self.fail_active_recording(
            storage,
            &pending.active,
            &pending.segment.id,
            timestamp_ms,
            error,
            None,
        )
    }
}
