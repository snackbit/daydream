use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use uuid::Uuid;

use crate::storage::{
    self, AppStorage, CreateRecordingInput, CreateRecordingSegmentInput, FailRecordingSegmentInput,
    FinalizeRecordingSegmentInput,
};

use super::errors::{
    dependency_error, filesystem_error, lifecycle_error, scene_or_storage_error, storage_error,
};
use super::ffmpeg::{self, FfmpegSegmentInput, FfmpegStreamIndexes, RunningFfmpegSegment};
use super::storage_mapping::quality_preset_as_storage_label;
use super::types::{
    CaptureSource, CaptureSourceList, RecorderAudioHealth, RecorderBackendComponentHealth,
    RecorderBackendHealth, RecorderError, RecorderFfmpegHealth, RecorderPortalHealth,
    RecorderState, RecorderStatus, StartRecordingInput,
};

#[derive(Debug)]
pub struct RecorderManager {
    app_data_dir: PathBuf,
    inner: Mutex<RecorderRuntime>,
}

#[derive(Debug)]
struct RecorderRuntime {
    active: Option<ActiveRecording>,
    capture_sources: Vec<CaptureSource>,
    status: RecorderStatus,
}

#[derive(Clone, Debug)]
struct ActiveRecording {
    active_segment: Option<ActiveSegment>,
    canvas_height: u32,
    canvas_width: u32,
    desktop_audio_source_id: Option<String>,
    fps: f64,
    microphone_audio_source_id: Option<String>,
    recording_id: String,
    scene_id: String,
    segments_dir: PathBuf,
    next_segment_index: u32,
}

#[derive(Clone, Debug)]
struct ActiveSegment {
    ffmpeg: Option<SharedFfmpegSegment>,
    id: String,
    started_at_ms: u64,
    stream_indexes: FfmpegStreamIndexes,
}

type SharedFfmpegSegment = Arc<Mutex<Option<RunningFfmpegSegment>>>;

impl RecorderManager {
    pub fn new(app_data_dir: PathBuf) -> Self {
        Self {
            app_data_dir,
            inner: Mutex::new(RecorderRuntime {
                active: None,
                capture_sources: Vec::new(),
                status: idle_status(now_ms()),
            }),
        }
    }

    pub fn status(&self) -> Result<RecorderStatus, RecorderError> {
        Ok(self.lock_runtime()?.status.clone())
    }

    pub fn status_with_backend_health(
        &self,
        backend_health: RecorderBackendHealth,
        timestamp_ms: u64,
    ) -> Result<RecorderStatus, RecorderError> {
        let mut runtime = self.lock_runtime()?;
        runtime.status.backend_health = backend_health;
        runtime.status.updated_at_ms = timestamp_ms;

        Ok(runtime.status.clone())
    }

    pub fn capture_sources(&self) -> Result<CaptureSourceList, RecorderError> {
        Ok(CaptureSourceList {
            sources: self.lock_runtime()?.capture_sources.clone(),
        })
    }

    pub fn set_capture_sources(
        &self,
        sources: Vec<CaptureSource>,
    ) -> Result<CaptureSourceList, RecorderError> {
        let mut runtime = self.lock_runtime()?;
        runtime.capture_sources = sources.clone();

        Ok(CaptureSourceList { sources })
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
            ffmpeg: None,
            id: segment_id.clone(),
            started_at_ms: timestamp_ms,
            stream_indexes: FfmpegStreamIndexes {
                desktop_audio_stream_index: None,
                microphone_audio_stream_index: None,
                video_stream_index: 0,
            },
        };
        let active = ActiveRecording {
            active_segment: Some(active_segment),
            canvas_height: scene.canvas_height.try_into().unwrap_or(0),
            canvas_width: scene.canvas_width.try_into().unwrap_or(0),
            desktop_audio_source_id: input.desktop_audio_source_id.clone(),
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
        let ffmpeg_probe = ffmpeg::probe_selected_ffmpeg().map_err(|error| {
            super::errors::dependency_error("recorder_ffmpeg_unavailable", error)
        })?;
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
            ffmpeg: Some(Arc::new(Mutex::new(Some(ffmpeg_segment)))),
            id: segment_id.clone(),
            started_at_ms: timestamp_ms,
            stream_indexes,
        };
        let active = ActiveRecording {
            active_segment: Some(active_segment),
            canvas_height: segment_input.canvas_height,
            canvas_width: segment_input.canvas_width,
            desktop_audio_source_id: input.desktop_audio_source_id.clone(),
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

        self.finalize_ffmpeg_segment(storage, &active, &segment, timestamp_ms)?;

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
            ffmpeg: None,
            id: segment_id,
            started_at_ms: timestamp_ms,
            stream_indexes: FfmpegStreamIndexes {
                desktop_audio_stream_index: None,
                microphone_audio_stream_index: None,
                video_stream_index: 0,
            },
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

        self.start_next_ffmpeg_segment(storage, active, timestamp_ms)
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
            self.finalize_ffmpeg_segment(storage, &active, segment, timestamp_ms)?;
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

    fn start_next_ffmpeg_segment(
        &self,
        storage: &AppStorage,
        active: ActiveRecording,
        timestamp_ms: u64,
    ) -> Result<RecorderStatus, RecorderError> {
        let ffmpeg_probe = ffmpeg::probe_selected_ffmpeg()
            .map_err(|error| dependency_error("recorder_ffmpeg_unavailable", error))?;
        let audio_selected =
            active.desktop_audio_source_id.is_some() || active.microphone_audio_source_id.is_some();
        ffmpeg::validate_segment_capabilities(&ffmpeg_probe.capabilities, audio_selected)?;

        let segment_index = active.next_segment_index;
        let segment_id = Uuid::now_v7().to_string();
        let segment_path = active.segments_dir.join(segment_file_name(segment_index));
        let segment_input = FfmpegSegmentInput {
            canvas_height: active.canvas_height,
            canvas_width: active.canvas_width,
            desktop_audio_source_id: active.desktop_audio_source_id.clone(),
            fps: active.fps,
            microphone_audio_source_id: active.microphone_audio_source_id.clone(),
            output_path: segment_path.clone(),
        };
        let stream_indexes = ffmpeg::stream_indexes(&segment_input);

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

        let ffmpeg_segment =
            match ffmpeg::start_synthetic_segment(Path::new(&ffmpeg_probe.path), &segment_input) {
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
        resumed.active_segment = Some(ActiveSegment {
            ffmpeg: Some(Arc::new(Mutex::new(Some(ffmpeg_segment)))),
            id: segment_id,
            started_at_ms: timestamp_ms,
            stream_indexes,
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

    fn finalize_ffmpeg_segment(
        &self,
        storage: &AppStorage,
        active: &ActiveRecording,
        segment: &ActiveSegment,
        timestamp_ms: u64,
    ) -> Result<(), RecorderError> {
        if let Some(ffmpeg) = &segment.ffmpeg {
            let ffmpeg_segment = ffmpeg
                .lock()
                .map_err(|_error| {
                    lifecycle_error(
                        "recorder_state_unavailable",
                        "FFmpeg state lock is unavailable.",
                    )
                })?
                .take();

            if let Some(ffmpeg_segment) = ffmpeg_segment {
                let exit_status = match ffmpeg_segment.stop() {
                    Ok(exit_status) => exit_status,
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
                };

                if !exit_status.success() {
                    let error = dependency_error(
                        "recorder_ffmpeg_failed",
                        format!("FFmpeg exited with {exit_status}."),
                    );
                    self.fail_active_recording(
                        storage,
                        active,
                        &segment.id,
                        timestamp_ms,
                        &error,
                        exit_status.code().map(i64::from),
                    )?;
                    return Err(error);
                }
            } else {
                return Err(lifecycle_error(
                    "recorder_segment_not_active",
                    "The active FFmpeg segment has already been consumed.",
                ));
            }
        }

        storage
            .with_connection(|connection| {
                storage::finalize_recording_segment(
                    connection,
                    &FinalizeRecordingSegmentInput {
                        desktop_audio_stream_index: segment
                            .stream_indexes
                            .desktop_audio_stream_index,
                        duration_ms: duration_ms(segment.started_at_ms, timestamp_ms) as i64,
                        ended_at_ms: timestamp_ms as i64,
                        id: segment.id.clone(),
                        microphone_audio_stream_index: segment
                            .stream_indexes
                            .microphone_audio_stream_index,
                        updated_at_ms: timestamp_ms as i64,
                        video_stream_index: Some(segment.stream_indexes.video_stream_index),
                    },
                )?;
                storage::update_recording_status(connection, &active.recording_id, "paused")
            })
            .map_err(storage_error)?;

        Ok(())
    }

    fn fail_active_recording(
        &self,
        storage: &AppStorage,
        active: &ActiveRecording,
        segment_id: &str,
        timestamp_ms: u64,
        error: &RecorderError,
        ffmpeg_exit_code: Option<i64>,
    ) -> Result<(), RecorderError> {
        fail_segment_for_error(storage, segment_id, timestamp_ms, error, ffmpeg_exit_code)?;
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

    fn recordings_root(&self) -> PathBuf {
        self.app_data_dir.join("recordings")
    }

    fn lock_runtime(&self) -> Result<std::sync::MutexGuard<'_, RecorderRuntime>, RecorderError> {
        self.inner.lock().map_err(|_error| {
            lifecycle_error(
                "recorder_state_unavailable",
                "Recorder state lock is unavailable.",
            )
        })
    }
}

pub(crate) fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

fn status_from_active(
    state: RecorderState,
    active: &ActiveRecording,
    updated_at_ms: u64,
    backend_health: RecorderBackendHealth,
    last_error: Option<RecorderError>,
) -> RecorderStatus {
    RecorderStatus {
        active_recording_id: Some(active.recording_id.clone()),
        active_scene_id: Some(active.scene_id.clone()),
        active_segment_id: active
            .active_segment
            .as_ref()
            .map(|segment| segment.id.clone()),
        backend_health,
        last_error,
        state,
        updated_at_ms,
    }
}

fn idle_status(updated_at_ms: u64) -> RecorderStatus {
    idle_status_with_backend_health(updated_at_ms, unavailable_backend_health())
}

fn idle_status_with_backend_health(
    updated_at_ms: u64,
    backend_health: RecorderBackendHealth,
) -> RecorderStatus {
    RecorderStatus {
        active_recording_id: None,
        active_scene_id: None,
        active_segment_id: None,
        backend_health,
        last_error: None,
        state: RecorderState::Idle,
        updated_at_ms,
    }
}

fn error_status_with_backend_health(
    updated_at_ms: u64,
    backend_health: RecorderBackendHealth,
    error: RecorderError,
) -> RecorderStatus {
    RecorderStatus {
        active_recording_id: None,
        active_scene_id: None,
        active_segment_id: None,
        backend_health,
        last_error: Some(error),
        state: RecorderState::Error,
        updated_at_ms,
    }
}

pub(crate) fn unavailable_backend_health() -> RecorderBackendHealth {
    RecorderBackendHealth {
        desktop_audio: RecorderAudioHealth {
            available: false,
            default_source_id: None,
            error: Some("Recorder backend is not implemented yet.".to_string()),
        },
        ffmpeg: RecorderFfmpegHealth {
            available: false,
            capabilities: Default::default(),
            error: Some("Recorder backend is not implemented yet.".to_string()),
            path: None,
            version_line: None,
        },
        microphone: RecorderAudioHealth {
            available: false,
            default_source_id: None,
            error: Some("Recorder backend is not implemented yet.".to_string()),
        },
        pipewire: RecorderBackendComponentHealth {
            available: false,
            error: Some("Recorder backend is not implemented yet.".to_string()),
        },
        portal: RecorderPortalHealth {
            available: false,
            cursor_modes: Vec::new(),
            error: Some("Recorder backend is not implemented yet.".to_string()),
            screen_cast_version: None,
            source_types: Vec::new(),
        },
    }
}

fn duration_ms(started_at_ms: u64, ended_at_ms: u64) -> u64 {
    ended_at_ms.saturating_sub(started_at_ms)
}

fn segment_file_name(segment_index: u32) -> String {
    format!("{segment_index:04}.mkv")
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

fn fail_segment_for_error(
    storage: &AppStorage,
    segment_id: &str,
    timestamp_ms: u64,
    error: &RecorderError,
    ffmpeg_exit_code: Option<i64>,
) -> Result<(), RecorderError> {
    storage
        .with_connection(|connection| {
            storage::fail_recording_segment(
                connection,
                &FailRecordingSegmentInput {
                    ended_at_ms: timestamp_ms as i64,
                    error_code: error.code.clone(),
                    ffmpeg_exit_code,
                    id: segment_id.to_string(),
                    updated_at_ms: timestamp_ms as i64,
                },
            )
        })
        .map(|_segment| ())
        .map_err(storage_error)
}
