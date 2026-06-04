use std::{
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

use uuid::Uuid;

use crate::storage::{
    self, AppStorage, CreateRecordingInput, CreateRecordingSegmentInput,
    FinalizeRecordingSegmentInput,
};

use super::errors::{filesystem_error, lifecycle_error, scene_or_storage_error, storage_error};
use super::storage_mapping::quality_preset_as_storage_label;
use super::types::{
    RecorderAudioHealth, RecorderBackendComponentHealth, RecorderBackendHealth, RecorderError,
    RecorderFfmpegHealth, RecorderPortalHealth, RecorderState, RecorderStatus, StartRecordingInput,
};

#[derive(Debug)]
pub struct RecorderManager {
    app_data_dir: PathBuf,
    inner: Mutex<RecorderRuntime>,
}

#[derive(Debug)]
struct RecorderRuntime {
    active: Option<ActiveRecording>,
    status: RecorderStatus,
}

#[derive(Clone, Debug)]
struct ActiveRecording {
    active_segment: Option<ActiveSegment>,
    recording_id: String,
    scene_id: String,
    segments_dir: PathBuf,
    next_segment_index: u32,
}

#[derive(Clone, Debug)]
struct ActiveSegment {
    id: String,
    started_at_ms: u64,
}

impl RecorderManager {
    pub fn new(app_data_dir: PathBuf) -> Self {
        Self {
            app_data_dir,
            inner: Mutex::new(RecorderRuntime {
                active: None,
                status: idle_status(now_ms()),
            }),
        }
    }

    pub fn status(&self) -> Result<RecorderStatus, RecorderError> {
        Ok(self.lock_runtime()?.status.clone())
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

        storage
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
            id: segment_id.clone(),
            started_at_ms: timestamp_ms,
        };
        let active = ActiveRecording {
            active_segment: Some(active_segment),
            recording_id: recording_id.clone(),
            scene_id: input.scene_id.clone(),
            segments_dir,
            next_segment_index: 2,
        };
        let status = status_from_active(RecorderState::Recording, &active, timestamp_ms, None);

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
        let status = status_from_active(RecorderState::Paused, &paused, timestamp_ms, None);
        let mut runtime = self.lock_runtime()?;
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
            id: segment_id,
            started_at_ms: timestamp_ms,
        });
        resumed.next_segment_index += 1;

        let status = status_from_active(RecorderState::Recording, &resumed, timestamp_ms, None);
        let mut runtime = self.lock_runtime()?;
        runtime.active = Some(resumed);
        runtime.status = status.clone();

        Ok(status)
    }

    pub fn stop_internal(
        &self,
        storage: &AppStorage,
        timestamp_ms: u64,
    ) -> Result<RecorderStatus, RecorderError> {
        let Some(active) = self.lock_runtime()?.active.clone() else {
            let status = idle_status(timestamp_ms);
            self.lock_runtime()?.status = status.clone();
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

        let status = idle_status(timestamp_ms);
        let mut runtime = self.lock_runtime()?;
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
    last_error: Option<RecorderError>,
) -> RecorderStatus {
    RecorderStatus {
        active_recording_id: Some(active.recording_id.clone()),
        active_scene_id: Some(active.scene_id.clone()),
        active_segment_id: active
            .active_segment
            .as_ref()
            .map(|segment| segment.id.clone()),
        backend_health: unavailable_backend_health(),
        last_error,
        state,
        updated_at_ms,
    }
}

fn idle_status(updated_at_ms: u64) -> RecorderStatus {
    RecorderStatus {
        active_recording_id: None,
        active_scene_id: None,
        active_segment_id: None,
        backend_health: unavailable_backend_health(),
        last_error: None,
        state: RecorderState::Idle,
        updated_at_ms,
    }
}

fn unavailable_backend_health() -> RecorderBackendHealth {
    RecorderBackendHealth {
        desktop_audio: RecorderAudioHealth {
            available: false,
            default_source_id: None,
            error: Some("Recorder backend is not implemented yet.".to_string()),
        },
        ffmpeg: RecorderFfmpegHealth {
            available: false,
            error: Some("Recorder backend is not implemented yet.".to_string()),
            path: None,
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
