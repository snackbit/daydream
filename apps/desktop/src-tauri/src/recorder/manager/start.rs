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
        let ffmpeg_probe = ffmpeg::probe_selected_ffmpeg()
            .map_err(|error| dependency_error("recorder_ffmpeg_unavailable", error))?;
        let audio_selected =
            input.desktop_audio_source_id.is_some() || input.microphone_audio_source_id.is_some();
        ffmpeg::validate_segment_capabilities(&ffmpeg_probe.capabilities, audio_selected)?;

        let recording_id = Uuid::now_v7().to_string();
        let segment_id = Uuid::now_v7().to_string();
        let recording_dir = self.recordings_root().join(&recording_id);
        let segments_dir = recording_dir.join("segments");
        let segment_path = segments_dir.join(segment_file_name(1));
        let segment_input = FfmpegSegmentInput {
            canvas_height: scene.canvas_height.try_into().map_err(|_error| {
                lifecycle_error(
                    "recorder_invalid_scene",
                    "Recorder scene canvas height is invalid.",
                )
            })?,
            canvas_width: scene.canvas_width.try_into().map_err(|_error| {
                lifecycle_error(
                    "recorder_invalid_scene",
                    "Recorder scene canvas width is invalid.",
                )
            })?,
            desktop_audio_source_id: input.desktop_audio_source_id.clone(),
            fps: input.fps,
            microphone_audio_source_id: input.microphone_audio_source_id.clone(),
            output_path: segment_path.clone(),
        };
        let segment = ActiveSegment {
            file_path: segment_path.clone(),
            id: segment_id,
            media: None,
            started_at_ms: timestamp_ms,
            stream_indexes: ffmpeg::stream_indexes(&segment_input),
            timing: None,
        };
        let active = ActiveRecording {
            active_segment: Some(segment.clone()),
            canvas_height: segment_input.canvas_height,
            canvas_width: segment_input.canvas_width,
            desktop_audio_source_id: input.desktop_audio_source_id.clone(),
            ffmpeg_path: ffmpeg_probe.path.clone(),
            fps: input.fps,
            microphone_audio_source_id: input.microphone_audio_source_id.clone(),
            recording_id: recording_id.clone(),
            scene_id: input.scene_id.clone(),
            segments_dir,
            next_segment_index: 2,
        };

        Ok(PendingRecordingStart {
            active,
            recording_dir,
            recording_id,
            segment,
            segment_path,
            segment_index: 1,
            segment_input,
            ffmpeg_path: ffmpeg_probe.path,
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
                        default_fps: pending.input.fps,
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
        media: RunningSegmentMedia,
        timestamp_ms: u64,
    ) -> Result<RecorderStatus, RecorderError> {
        pending.segment.timing = media.capture_timing();
        pending.segment.media = Some(Arc::new(Mutex::new(Some(media))));
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

    pub fn start_internal(
        &self,
        storage: &AppStorage,
        input: StartRecordingInput,
        timestamp_ms: u64,
    ) -> Result<RecorderStatus, RecorderError> {
        let mut runtime = self.lock_runtime()?;
        if runtime.active.is_some() {
            return Err(lifecycle_error(
                "recorder_already_active",
                "A recording is already active.",
            ));
        }

        let scene = storage
            .with_connection(|connection| storage::get_recorder_scene(connection, &input.scene_id))
            .map_err(|error| scene_or_storage_error(error, &input.scene_id))?;

        let recording_id = Uuid::now_v7().to_string();
        let segment_id = Uuid::now_v7().to_string();
        let recording_dir = self.recordings_root().join(&recording_id);
        let segments_dir = recording_dir.join("segments");
        fs::create_dir_all(&segments_dir).map_err(filesystem_error)?;

        let segment_path = segments_dir.join(segment_file_name(1));
        storage
            .with_connection(|connection| {
                storage::create_recording(
                    connection,
                    &CreateRecordingInput {
                        id: recording_id.clone(),
                        active_scene_id: Some(input.scene_id.clone()),
                        created_at_ms: timestamp_ms as i64,
                        default_fps: input.fps,
                        notes: None,
                        output_root: Some(path_to_string(&self.recordings_root())),
                        quality_label: quality_preset_as_storage_label(&input.quality_preset)
                            .to_string(),
                        storage_dir: path_to_string(&recording_dir),
                    },
                )?;
                storage::mark_recording_started(connection, &recording_id, timestamp_ms as i64)?;
                storage::create_recording_segment(
                    connection,
                    &CreateRecordingSegmentInput {
                        file_path: path_to_string(&segment_path),
                        id: segment_id.clone(),
                        recording_id: recording_id.clone(),
                        segment_index: 1,
                        started_at_ms: timestamp_ms as i64,
                    },
                )
            })
            .map_err(storage_error)?;

        let active_segment = ActiveSegment {
            file_path: segment_path.clone(),
            id: segment_id.clone(),
            media: None,
            started_at_ms: timestamp_ms,
            stream_indexes: FfmpegStreamIndexes {
                desktop_audio_stream_index: None,
                microphone_audio_stream_index: None,
                video_stream_index: 0,
            },
            timing: None,
        };
        let active = ActiveRecording {
            active_segment: Some(active_segment),
            canvas_height: scene.canvas_height.try_into().unwrap_or(0),
            canvas_width: scene.canvas_width.try_into().unwrap_or(0),
            desktop_audio_source_id: input.desktop_audio_source_id.clone(),
            ffmpeg_path: String::new(),
            fps: input.fps,
            microphone_audio_source_id: input.microphone_audio_source_id.clone(),
            recording_id: recording_id.clone(),
            scene_id: input.scene_id.clone(),
            segments_dir,
            next_segment_index: 2,
        };
        let status = status_from_active(
            RecorderState::Recording,
            &active,
            timestamp_ms,
            runtime.status.backend_health.clone(),
            None,
        );

        runtime.active = Some(active);
        runtime.status = status.clone();

        Ok(status)
    }

    pub fn start_recording(
        &self,
        storage: &AppStorage,
        input: StartRecordingInput,
        timestamp_ms: u64,
    ) -> Result<RecorderStatus, RecorderError> {
        let mut runtime = self.lock_runtime()?;
        if runtime.active.is_some() {
            return Err(lifecycle_error(
                "recorder_already_active",
                "A recording is already active.",
            ));
        }

        let scene = storage
            .with_connection(|connection| storage::get_recorder_scene(connection, &input.scene_id))
            .map_err(|error| scene_or_storage_error(error, &input.scene_id))?;
        let ffmpeg_probe = ffmpeg::probe_selected_ffmpeg()
            .map_err(|error| dependency_error("recorder_ffmpeg_unavailable", error))?;
        let audio_selected =
            input.desktop_audio_source_id.is_some() || input.microphone_audio_source_id.is_some();
        ffmpeg::validate_segment_capabilities(&ffmpeg_probe.capabilities, audio_selected)?;

        let recording_id = Uuid::now_v7().to_string();
        let segment_id = Uuid::now_v7().to_string();
        let recording_dir = self.recordings_root().join(&recording_id);
        let segments_dir = recording_dir.join("segments");
        fs::create_dir_all(&segments_dir).map_err(filesystem_error)?;

        let segment_path = segments_dir.join(segment_file_name(1));
        let segment_input = FfmpegSegmentInput {
            canvas_height: scene.canvas_height.try_into().map_err(|_error| {
                lifecycle_error(
                    "recorder_invalid_scene",
                    "Recorder scene canvas height is invalid.",
                )
            })?,
            canvas_width: scene.canvas_width.try_into().map_err(|_error| {
                lifecycle_error(
                    "recorder_invalid_scene",
                    "Recorder scene canvas width is invalid.",
                )
            })?,
            desktop_audio_source_id: input.desktop_audio_source_id.clone(),
            fps: input.fps,
            microphone_audio_source_id: input.microphone_audio_source_id.clone(),
            output_path: segment_path.clone(),
        };
        let stream_indexes = ffmpeg::stream_indexes(&segment_input);

        storage
            .with_connection(|connection| {
                storage::create_recording(
                    connection,
                    &CreateRecordingInput {
                        id: recording_id.clone(),
                        active_scene_id: Some(input.scene_id.clone()),
                        created_at_ms: timestamp_ms as i64,
                        default_fps: input.fps,
                        notes: None,
                        output_root: Some(path_to_string(&self.recordings_root())),
                        quality_label: quality_preset_as_storage_label(&input.quality_preset)
                            .to_string(),
                        storage_dir: path_to_string(&recording_dir),
                    },
                )?;
                storage::mark_recording_started(connection, &recording_id, timestamp_ms as i64)?;
                storage::create_recording_segment(
                    connection,
                    &CreateRecordingSegmentInput {
                        file_path: path_to_string(&segment_path),
                        id: segment_id.clone(),
                        recording_id: recording_id.clone(),
                        segment_index: 1,
                        started_at_ms: timestamp_ms as i64,
                    },
                )
            })
            .map_err(storage_error)?;

        let ffmpeg_segment =
            match ffmpeg::start_synthetic_segment(Path::new(&ffmpeg_probe.path), &segment_input) {
                Ok(segment) => segment,
                Err(error) => {
                    fail_segment_for_error(storage, &segment_id, timestamp_ms, &error, None)?;
                    let _ = storage.with_connection(|connection| {
                        storage::update_recording_status(connection, &recording_id, "failed")
                    });
                    return Err(error);
                }
            };

        let active_segment = ActiveSegment {
            file_path: segment_path.clone(),
            id: segment_id.clone(),
            media: Some(Arc::new(Mutex::new(Some(RunningSegmentMedia::new(
                ffmpeg_segment,
                None,
                None,
            ))))),
            started_at_ms: timestamp_ms,
            stream_indexes,
            timing: None,
        };
        let active = ActiveRecording {
            active_segment: Some(active_segment),
            canvas_height: segment_input.canvas_height,
            canvas_width: segment_input.canvas_width,
            desktop_audio_source_id: input.desktop_audio_source_id.clone(),
            ffmpeg_path: ffmpeg_probe.path,
            fps: input.fps,
            microphone_audio_source_id: input.microphone_audio_source_id.clone(),
            recording_id: recording_id.clone(),
            scene_id: input.scene_id.clone(),
            segments_dir,
            next_segment_index: 2,
        };
        let status = status_from_active(
            RecorderState::Recording,
            &active,
            timestamp_ms,
            runtime.status.backend_health.clone(),
            None,
        );

        runtime.active = Some(active);
        runtime.status = status.clone();

        Ok(status)
    }
}
