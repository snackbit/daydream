use super::*;

impl RecorderManager {
    pub(crate) fn prepare_recording_start(
        &self,
        storage: &AppStorage,
        input: StartRecordingInput,
        timestamp_ms: u64,
    ) -> Result<PendingRecordingStart, RecorderError> {
        let scene = storage
            .with_connection(|connection| storage::get_recorder_scene(connection, &input.scene_id))
            .map_err(|error| scene_or_storage_error(error, &input.scene_id))
            .and_then(scene_from_storage)?;
        let app_settings = settings::load_app_settings(storage)
            .map_err(|error| storage_error_message(&error.message))?;
        let media_writer_health = media_writer::collect_media_writer_health();

        let recording_id = Uuid::now_v7().to_string();
        let segment_id = Uuid::now_v7().to_string();
        let recording_dir = self.recordings_root().join(&recording_id);
        let segments_dir = recording_dir.join("segments");
        let benchmarks_dir = recording_dir.join("benchmarks");
        let segment_path = segments_dir.join(segment_file_name(1));
        let segment_input = MediaSegmentInput {
            benchmarks_dir,
            canvas_height: even_canvas_dimension(scene.canvas_height),
            canvas_width: even_canvas_dimension(scene.canvas_width),
            desktop_audio_source_id: input.desktop_audio_source_id.clone(),
            fps: app_settings.record.fps,
            microphone_audio_source_id: input.microphone_audio_source_id.clone(),
            output_path: segment_path.clone(),
            recording_id: recording_id.clone(),
            segment_id: segment_id.clone(),
            segment_index: 1,
            started_at_ms: timestamp_ms,
            video_bitrate_kbps: app_settings.record.video_bitrate_kbps,
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
        let active = ActiveRecording {
            active_segment: Some(segment.clone()),
            canvas_height: segment_input.canvas_height,
            canvas_width: segment_input.canvas_width,
            desktop_audio_source_id: input.desktop_audio_source_id.clone(),
            fps: app_settings.record.fps,
            media_writer: media_writer_health
                .writer
                .clone()
                .unwrap_or_else(|| "gstreamer".to_string()),
            microphone_audio_source_id: input.microphone_audio_source_id.clone(),
            recording_id: recording_id.clone(),
            scene_id: input.scene_id.clone(),
            segments_dir,
            next_segment_index: 2,
            video_bitrate_kbps: app_settings.record.video_bitrate_kbps,
        };

        Ok(PendingRecordingStart {
            active,
            recording_dir,
            recording_id,
            segment,
            segment_path,
            segment_index: 1,
            segment_input,
            input,
            scene,
        })
    }

    pub(crate) fn create_pending_recording_rows(
        &self,
        storage: &AppStorage,
        pending: &PendingRecordingStart,
    ) -> Result<(), RecorderError> {
        fs::create_dir_all(
            pending
                .segment_path
                .parent()
                .ok_or_else(|| filesystem_error(std::io::Error::other("missing segment parent")))?,
        )
        .map_err(filesystem_error)?;

        storage
            .with_connection(|connection| {
                storage::create_recording(
                    connection,
                    &CreateRecordingInput {
                        id: pending.recording_id.clone(),
                        active_scene_id: Some(pending.input.scene_id.clone()),
                        created_at_ms: pending.segment.started_at_ms as i64,
                        default_fps: pending.segment_input.fps,
                        notes: None,
                        output_root: Some(path_to_string(&self.recordings_root())),
                        quality_label: quality_preset_as_storage_label(
                            &pending.input.quality_preset,
                        )
                        .to_string(),
                        storage_dir: path_to_string(&pending.recording_dir),
                    },
                )?;
                storage::mark_recording_started(
                    connection,
                    &pending.recording_id,
                    pending.segment.started_at_ms as i64,
                )?;
                storage::create_recording_segment(
                    connection,
                    &CreateRecordingSegmentInput {
                        file_path: path_to_string(&pending.segment_path),
                        id: pending.segment.id.clone(),
                        recording_id: pending.recording_id.clone(),
                        segment_index: pending.segment_index as i64,
                        started_at_ms: pending.segment.started_at_ms as i64,
                    },
                )?;
                Ok(())
            })
            .map_err(storage_error)
    }

    pub(crate) fn activate_pending_recording(
        &self,
        mut pending: PendingRecordingStart,
        runtime: RecordingSegmentRuntime,
        timestamp_ms: u64,
    ) -> Result<RecorderStatus, RecorderError> {
        pending.segment.timing = runtime.capture_timing();
        pending.segment.runtime = Some(Arc::new(Mutex::new(Some(runtime))));
        pending.active.active_segment = Some(pending.segment);

        let mut runtime = self.lock_runtime()?;
        let status = status_from_active(
            RecorderState::Recording,
            &pending.active,
            timestamp_ms,
            runtime.status.backend_health.clone(),
            None,
        );
        runtime.active = Some(pending.active);
        runtime.status = status.clone();

        Ok(status)
    }

    #[cfg(test)]
    pub(crate) fn activate_pending_recording_without_runtime_for_test(
        &self,
        mut pending: PendingRecordingStart,
        timestamp_ms: u64,
    ) -> Result<RecorderStatus, RecorderError> {
        pending.active.active_segment = Some(pending.segment);

        let mut runtime = self.lock_runtime()?;
        let status = status_from_active(
            RecorderState::Recording,
            &pending.active,
            timestamp_ms,
            runtime.status.backend_health.clone(),
            None,
        );
        runtime.active = Some(pending.active);
        runtime.status = status.clone();

        Ok(status)
    }

    pub(crate) fn fail_pending_recording(
        &self,
        storage: &AppStorage,
        pending: &PendingRecordingStart,
        timestamp_ms: u64,
        error: &RecorderError,
    ) -> Result<(), RecorderError> {
        fail_segment_for_error(storage, &pending.segment.id, timestamp_ms, error, None)?;
        storage
            .with_connection(|connection| {
                storage::update_recording_status(connection, &pending.recording_id, "failed")
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
