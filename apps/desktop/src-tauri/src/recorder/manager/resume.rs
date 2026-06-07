use super::*;

impl RecorderManager {
    pub fn resume_internal(
        &self,
        storage: &AppStorage,
        timestamp_ms: u64,
    ) -> Result<RecorderStatus, RecorderError> {
        let active = self
            .lock_runtime()?
            .active
            .clone()
            .ok_or_else(|| lifecycle_error("recorder_not_paused", "No recording is paused."))?;

        if active.active_segment.is_some() {
            return Err(lifecycle_error(
                "recorder_not_paused",
                "The active recording is not paused.",
            ));
        }

        let segment_index = active.next_segment_index;
        let segment_id = Uuid::now_v7().to_string();
        let segment_path = active.segments_dir.join(segment_file_name(segment_index));

        storage
            .with_connection(|connection| {
                storage::update_recording_status(connection, &active.recording_id, "recording")?;
                storage::create_recording_segment(
                    connection,
                    &CreateRecordingSegmentInput {
                        file_path: path_to_string(&segment_path),
                        id: segment_id.clone(),
                        recording_id: active.recording_id.clone(),
                        segment_index: segment_index as i64,
                        started_at_ms: timestamp_ms as i64,
                    },
                )
            })
            .map_err(storage_error)?;

        let mut resumed = active;
        resumed.active_segment = Some(ActiveSegment {
            file_path: segment_path,
            id: segment_id,
            runtime: None,
            started_at_ms: timestamp_ms,
            stream_indexes: MediaWriterStreamIndexes {
                desktop_audio_stream_index: None,
                microphone_audio_stream_index: None,
                video_stream_index: 0,
            },
            timing: None,
        });
        resumed.next_segment_index += 1;

        let mut runtime = self.lock_runtime()?;
        let status = status_from_active(
            RecorderState::Recording,
            &resumed,
            timestamp_ms,
            runtime.status.backend_health.clone(),
            None,
        );
        runtime.active = Some(resumed);
        runtime.status = status.clone();

        Ok(status)
    }

    pub fn resume_recording(
        &self,
        storage: &AppStorage,
        recording_id: &str,
        timestamp_ms: u64,
    ) -> Result<RecorderStatus, RecorderError> {
        let active = self
            .lock_runtime()?
            .active
            .clone()
            .ok_or_else(|| lifecycle_error("recorder_not_paused", "No recording is paused."))?;

        if active.recording_id != recording_id {
            return Err(lifecycle_error(
                "recorder_recording_mismatch",
                "The requested recording is not active.",
            ));
        }
        if active.active_segment.is_some() {
            return Err(lifecycle_error(
                "recorder_not_paused",
                "The active recording is not paused.",
            ));
        }

        self.start_next_media_segment(storage, active, timestamp_ms)
    }

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
            .with_connection(|connection| {
                storage::update_recording_status(
                    connection,
                    &pending.active.recording_id,
                    "recording",
                )?;
                storage::create_recording_segment(
                    connection,
                    &CreateRecordingSegmentInput {
                        file_path: path_to_string(&pending.segment_path),
                        id: pending.segment.id.clone(),
                        recording_id: pending.active.recording_id.clone(),
                        segment_index: pending.segment_index as i64,
                        started_at_ms: pending.segment.started_at_ms as i64,
                    },
                )?;
                Ok(())
            })
            .map_err(storage_error)
    }

    pub(crate) fn activate_pending_segment(
        &self,
        mut pending: PendingSegmentStart,
        runtime: RecordingSegmentRuntime,
        timestamp_ms: u64,
    ) -> Result<RecorderStatus, RecorderError> {
        pending.segment.timing = runtime.capture_timing();
        pending.segment.runtime = Some(Arc::new(Mutex::new(Some(runtime))));
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

    fn start_next_media_segment(
        &self,
        storage: &AppStorage,
        active: ActiveRecording,
        timestamp_ms: u64,
    ) -> Result<RecorderStatus, RecorderError> {
        let media_writer_health = media_writer::collect_media_writer_health();

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
        };
        media_writer::validate_segment_capabilities(
            &media_writer_health.capabilities,
            &segment_input,
        )?;
        let stream_indexes = media_writer::stream_indexes(&segment_input);

        storage
            .with_connection(|connection| {
                storage::update_recording_status(connection, &active.recording_id, "recording")?;
                storage::create_recording_segment(
                    connection,
                    &CreateRecordingSegmentInput {
                        file_path: path_to_string(&segment_path),
                        id: segment_id.clone(),
                        recording_id: active.recording_id.clone(),
                        segment_index: segment_index as i64,
                        started_at_ms: timestamp_ms as i64,
                    },
                )
            })
            .map_err(storage_error)?;

        let media_segment = match media_writer::start_synthetic_segment(&segment_input) {
            Ok(segment) => segment,
            Err(error) => {
                self.fail_active_recording(
                    storage,
                    &active,
                    &segment_id,
                    timestamp_ms,
                    &error,
                    None,
                )?;
                return Err(error);
            }
        };

        let mut resumed = active;
        resumed.media_writer = media_writer_health
            .writer
            .clone()
            .unwrap_or_else(|| "gstreamer".to_string());
        resumed.active_segment = Some(ActiveSegment {
            file_path: segment_path,
            id: segment_id,
            runtime: Some(Arc::new(Mutex::new(Some(RecordingSegmentRuntime::new(
                media_segment,
                None,
                None,
            ))))),
            started_at_ms: timestamp_ms,
            stream_indexes,
            timing: None,
        });
        resumed.next_segment_index += 1;

        let mut runtime = self.lock_runtime()?;
        let status = status_from_active(
            RecorderState::Recording,
            &resumed,
            timestamp_ms,
            runtime.status.backend_health.clone(),
            None,
        );
        runtime.active = Some(resumed);
        runtime.status = status.clone();

        Ok(status)
    }
}
