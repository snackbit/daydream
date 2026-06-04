use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

use rusqlite::Error as SqliteError;
use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

use crate::storage::{
    self, AppStorage, CreateRecordingInput, CreateRecordingSegmentInput,
    FinalizeRecordingSegmentInput, RecorderSceneRow, RecorderSceneSourceRow,
    SaveRecorderSceneInput as StorageSaveRecorderSceneInput,
    SaveRecorderSceneSourceInput as StorageSaveRecorderSceneSourceInput,
};

const RECORDER_BACKEND_NOT_IMPLEMENTED: &str = "recorder_backend_not_implemented";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum RecorderState {
    Error,
    Idle,
    Paused,
    Recording,
    Selecting,
    Stopping,
    Unavailable,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum RecorderErrorCategory {
    Audio,
    Dependency,
    Encoder,
    Filesystem,
    Lifecycle,
    Permission,
    Unavailable,
    Video,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecorderError {
    category: RecorderErrorCategory,
    code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<BTreeMap<String, serde_json::Value>>,
    message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CanvasRect {
    height: f64,
    width: f64,
    x: f64,
    y: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CropRect {
    bottom: f64,
    left: f64,
    right: f64,
    top: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum CaptureSourceKind {
    Monitor,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CaptureSource {
    display_name: String,
    height: u32,
    id: String,
    kind: CaptureSourceKind,
    scale_factor: Option<f64>,
    width: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CaptureSourceList {
    sources: Vec<CaptureSource>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CaptureSourceSelection {
    source_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum AudioSourceRole {
    DesktopAudio,
    Microphone,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum AudioSourceKind {
    Input,
    Monitor,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AudioSource {
    display_name: String,
    id: String,
    is_default: bool,
    kind: AudioSourceKind,
    roles: Vec<AudioSourceRole>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AudioSourceList {
    sources: Vec<AudioSource>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum RecorderSceneLayoutMode {
    Custom,
    Horizontal,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecorderSceneSource {
    crop: CropRect,
    display_name: String,
    id: String,
    preserve_aspect_ratio: bool,
    rect: CanvasRect,
    source_id: String,
    source_kind: CaptureSourceKind,
    z_index: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecorderScene {
    canvas_height: u32,
    canvas_width: u32,
    id: String,
    layout_mode: RecorderSceneLayoutMode,
    name: String,
    sources: Vec<RecorderSceneSource>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SaveRecorderSceneInput {
    scene: RecorderScene,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum RecorderQualityPreset {
    BalancedOcr,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StartRecordingInput {
    desktop_audio_source_id: Option<String>,
    fps: f64,
    microphone_audio_source_id: Option<String>,
    quality_preset: RecorderQualityPreset,
    scene_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PauseRecordingInput {
    recording_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResumeRecordingInput {
    recording_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StopRecordingInput {
    recording_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum RecorderSegmentState {
    Failed,
    Finalized,
    Recording,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecorderSegment {
    desktop_audio_stream_index: Option<i32>,
    duration_ms: Option<u64>,
    ended_at_ms: Option<u64>,
    error_code: Option<String>,
    ffmpeg_exit_code: Option<i32>,
    file_path: String,
    id: String,
    microphone_audio_stream_index: Option<i32>,
    recording_id: String,
    segment_index: u32,
    started_at_ms: u64,
    state: RecorderSegmentState,
    video_stream_index: Option<i32>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecorderBackendComponentHealth {
    available: bool,
    error: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecorderFfmpegHealth {
    available: bool,
    error: Option<String>,
    path: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecorderAudioHealth {
    available: bool,
    default_source_id: Option<String>,
    error: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecorderPortalHealth {
    available: bool,
    cursor_modes: Vec<String>,
    error: Option<String>,
    screen_cast_version: Option<u32>,
    source_types: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecorderBackendHealth {
    desktop_audio: RecorderAudioHealth,
    ffmpeg: RecorderFfmpegHealth,
    microphone: RecorderAudioHealth,
    pipewire: RecorderBackendComponentHealth,
    portal: RecorderPortalHealth,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecorderStatus {
    active_recording_id: Option<String>,
    active_scene_id: Option<String>,
    active_segment_id: Option<String>,
    backend_health: RecorderBackendHealth,
    last_error: Option<RecorderError>,
    state: RecorderState,
    updated_at_ms: u64,
}

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
        if self.lock_runtime()?.active.is_some() {
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
                        quality_label: input.quality_preset.as_storage_label().to_string(),
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

        let mut runtime = self.lock_runtime()?;
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

#[tauri::command(rename = "getRecorderStatus")]
pub fn get_recorder_status(
    manager: State<'_, RecorderManager>,
) -> Result<RecorderStatus, RecorderError> {
    manager.status()
}

#[tauri::command(rename = "getCaptureSources")]
pub fn get_capture_sources() -> Result<CaptureSourceList, RecorderError> {
    Err(unavailable_error())
}

#[tauri::command(rename = "getAudioSources")]
pub fn get_audio_sources() -> Result<AudioSourceList, RecorderError> {
    Err(unavailable_error())
}

#[tauri::command(rename = "saveRecorderScene")]
pub fn save_recorder_scene(
    input: SaveRecorderSceneInput,
    storage: State<'_, AppStorage>,
) -> Result<RecorderScene, RecorderError> {
    save_recorder_scene_for_storage(&storage, input, now_ms())
}

#[tauri::command(rename = "startRecording")]
pub fn start_recording(_input: StartRecordingInput) -> Result<RecorderStatus, RecorderError> {
    Err(unavailable_error())
}

#[tauri::command(rename = "pauseRecording")]
pub fn pause_recording(_input: PauseRecordingInput) -> Result<RecorderStatus, RecorderError> {
    Err(unavailable_error())
}

#[tauri::command(rename = "resumeRecording")]
pub fn resume_recording(_input: ResumeRecordingInput) -> Result<RecorderStatus, RecorderError> {
    Err(unavailable_error())
}

#[tauri::command(rename = "stopRecording")]
pub fn stop_recording(_input: StopRecordingInput) -> Result<RecorderStatus, RecorderError> {
    Err(unavailable_error())
}

fn unavailable_error() -> RecorderError {
    RecorderError {
        category: RecorderErrorCategory::Unavailable,
        code: RECORDER_BACKEND_NOT_IMPLEMENTED.to_string(),
        details: None,
        message: "Recorder backend is not implemented yet.".to_string(),
    }
}

fn save_recorder_scene_for_storage(
    storage: &AppStorage,
    input: SaveRecorderSceneInput,
    timestamp_ms: u64,
) -> Result<RecorderScene, RecorderError> {
    let storage_input = storage_scene_input_from_recorder(input.scene, timestamp_ms)?;
    let scene = storage
        .with_connection_mut(|connection| storage::save_recorder_scene(connection, &storage_input))
        .map_err(storage_error)?;

    scene_from_storage(scene)
}

fn storage_scene_input_from_recorder(
    scene: RecorderScene,
    timestamp_ms: u64,
) -> Result<StorageSaveRecorderSceneInput, RecorderError> {
    Ok(StorageSaveRecorderSceneInput {
        canvas_height: scene.canvas_height.into(),
        canvas_width: scene.canvas_width.into(),
        id: scene.id,
        layout_mode: scene.layout_mode.as_storage_label().to_string(),
        name: scene.name,
        sources: scene
            .sources
            .into_iter()
            .map(storage_scene_source_input_from_recorder)
            .collect::<Result<Vec<_>, _>>()?,
        timestamp_ms: timestamp_ms as i64,
    })
}

fn storage_scene_source_input_from_recorder(
    source: RecorderSceneSource,
) -> Result<StorageSaveRecorderSceneSourceInput, RecorderError> {
    Ok(StorageSaveRecorderSceneSourceInput {
        crop_bottom: pixel_value(source.crop.bottom, "crop.bottom")?,
        crop_left: pixel_value(source.crop.left, "crop.left")?,
        crop_right: pixel_value(source.crop.right, "crop.right")?,
        crop_top: pixel_value(source.crop.top, "crop.top")?,
        display_name: source.display_name,
        height: pixel_value(source.rect.height, "rect.height")?,
        id: source.id,
        preserve_aspect_ratio: source.preserve_aspect_ratio,
        source_id: source.source_id,
        source_kind: source.source_kind.as_storage_label().to_string(),
        width: pixel_value(source.rect.width, "rect.width")?,
        x: pixel_value(source.rect.x, "rect.x")?,
        y: pixel_value(source.rect.y, "rect.y")?,
        z_index: source.z_index.into(),
    })
}

fn scene_from_storage(scene: RecorderSceneRow) -> Result<RecorderScene, RecorderError> {
    Ok(RecorderScene {
        canvas_height: u32_value(scene.canvas_height, "canvasHeight")?,
        canvas_width: u32_value(scene.canvas_width, "canvasWidth")?,
        id: scene.id,
        layout_mode: RecorderSceneLayoutMode::from_storage_label(&scene.layout_mode)?,
        name: scene.name,
        sources: scene
            .sources
            .into_iter()
            .map(scene_source_from_storage)
            .collect::<Result<Vec<_>, _>>()?,
    })
}

fn scene_source_from_storage(
    source: RecorderSceneSourceRow,
) -> Result<RecorderSceneSource, RecorderError> {
    Ok(RecorderSceneSource {
        crop: CropRect {
            bottom: source.crop_bottom as f64,
            left: source.crop_left as f64,
            right: source.crop_right as f64,
            top: source.crop_top as f64,
        },
        display_name: source.display_name,
        id: source.id,
        preserve_aspect_ratio: source.preserve_aspect_ratio,
        rect: CanvasRect {
            height: source.height as f64,
            width: source.width as f64,
            x: source.x as f64,
            y: source.y as f64,
        },
        source_id: source.source_id,
        source_kind: CaptureSourceKind::from_storage_label(&source.source_kind)?,
        z_index: source.z_index as i32,
    })
}

impl CaptureSourceKind {
    fn as_storage_label(&self) -> &'static str {
        match self {
            Self::Monitor => "monitor",
        }
    }

    fn from_storage_label(value: &str) -> Result<Self, RecorderError> {
        match value {
            "monitor" => Ok(Self::Monitor),
            _ => Err(storage_error_message("Unknown capture source kind.")),
        }
    }
}

impl RecorderSceneLayoutMode {
    fn as_storage_label(&self) -> &'static str {
        match self {
            Self::Custom => "custom",
            Self::Horizontal => "horizontal",
        }
    }

    fn from_storage_label(value: &str) -> Result<Self, RecorderError> {
        match value {
            "custom" => Ok(Self::Custom),
            "horizontal" => Ok(Self::Horizontal),
            _ => Err(storage_error_message("Unknown scene layout mode.")),
        }
    }
}

impl RecorderQualityPreset {
    fn as_storage_label(&self) -> &'static str {
        match self {
            Self::BalancedOcr => "balanced-ocr",
        }
    }
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

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

fn pixel_value(value: f64, field: &str) -> Result<i64, RecorderError> {
    if !value.is_finite() {
        return Err(invalid_scene_error(field));
    }

    Ok(value.round() as i64)
}

fn u32_value(value: i64, field: &str) -> Result<u32, RecorderError> {
    value
        .try_into()
        .map_err(|_error| invalid_scene_error(field))
}

fn scene_or_storage_error(error: SqliteError, scene_id: &str) -> RecorderError {
    match error {
        SqliteError::QueryReturnedNoRows => lifecycle_error(
            "recorder_scene_not_found",
            &format!("Recorder scene {scene_id} was not found."),
        ),
        other => storage_error(other),
    }
}

fn lifecycle_error(code: &str, message: &str) -> RecorderError {
    RecorderError {
        category: RecorderErrorCategory::Lifecycle,
        code: code.to_string(),
        details: None,
        message: message.to_string(),
    }
}

fn filesystem_error(error: std::io::Error) -> RecorderError {
    RecorderError {
        category: RecorderErrorCategory::Filesystem,
        code: "recorder_storage_unavailable".to_string(),
        details: None,
        message: format!("Recorder storage is unavailable: {error}"),
    }
}

fn storage_error(error: SqliteError) -> RecorderError {
    RecorderError {
        category: RecorderErrorCategory::Dependency,
        code: "recorder_storage_error".to_string(),
        details: None,
        message: format!("Recorder storage failed: {error}"),
    }
}

fn storage_error_message(message: &str) -> RecorderError {
    RecorderError {
        category: RecorderErrorCategory::Dependency,
        code: "recorder_storage_error".to_string(),
        details: None,
        message: message.to_string(),
    }
}

fn invalid_scene_error(field: &str) -> RecorderError {
    RecorderError {
        category: RecorderErrorCategory::Lifecycle,
        code: "recorder_invalid_scene".to_string(),
        details: None,
        message: format!("Recorder scene field {field} is invalid."),
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use serde_json::json;
    use uuid::Uuid;

    use super::{
        save_recorder_scene_for_storage, start_recording, unavailable_error, CanvasRect,
        CaptureSourceKind, CropRect, PauseRecordingInput, RecorderErrorCategory, RecorderManager,
        RecorderQualityPreset, RecorderScene, RecorderSceneLayoutMode, RecorderSceneSource,
        RecorderState, ResumeRecordingInput, SaveRecorderSceneInput, StartRecordingInput,
        StopRecordingInput,
    };
    use crate::storage;

    #[test]
    fn unavailable_error_uses_locked_gate_a_shape() {
        let error = unavailable_error();

        assert_eq!(error.category, RecorderErrorCategory::Unavailable);
        assert_eq!(error.code, "recorder_backend_not_implemented");
        assert_eq!(error.message, "Recorder backend is not implemented yet.");
        assert_eq!(error.details, None);

        let serialized = serde_json::to_value(error).expect("serialize recorder error");
        assert_eq!(
            serialized,
            json!({
                "category": "unavailable",
                "code": "recorder_backend_not_implemented",
                "message": "Recorder backend is not implemented yet.",
            }),
        );
    }

    #[test]
    fn enum_serialization_uses_kebab_case() {
        assert_eq!(
            serde_json::to_value(RecorderState::Unavailable).expect("serialize recorder state"),
            json!("unavailable"),
        );
        assert_eq!(
            serde_json::to_value(RecorderQualityPreset::BalancedOcr)
                .expect("serialize quality preset"),
            json!("balanced-ocr"),
        );
    }

    #[test]
    fn struct_serialization_uses_camel_case_fields() {
        let value = serde_json::to_value(CanvasRect {
            height: 1440.0,
            width: 2560.0,
            x: 0.0,
            y: 0.0,
        })
        .expect("serialize canvas rect");

        assert_eq!(
            value,
            json!({
                "height": 1440.0,
                "width": 2560.0,
                "x": 0.0,
                "y": 0.0,
            }),
        );
    }

    #[test]
    fn manager_initial_status_is_idle() {
        let manager = RecorderManager::new(test_app_data_dir());
        let status = manager.status().expect("read status");

        assert_eq!(status.state, RecorderState::Idle);
        assert_eq!(status.active_recording_id, None);
        assert_eq!(status.active_segment_id, None);
        assert!(!status.backend_health.ffmpeg.available);
    }

    #[test]
    fn save_scene_persists_through_app_storage() {
        let storage = test_storage();
        let scene_id = Uuid::now_v7().to_string();

        let scene = save_recorder_scene_for_storage(
            &storage,
            SaveRecorderSceneInput {
                scene: test_scene(&scene_id),
            },
            10,
        )
        .expect("save scene");

        assert_eq!(scene.id, scene_id);
        assert_eq!(scene.sources.len(), 1);
        assert_eq!(scene.sources[0].source_id, "monitor-1");
    }

    #[test]
    fn manager_start_creates_recording_directory_metadata_and_segment() {
        let app_data_dir = test_app_data_dir();
        let manager = RecorderManager::new(app_data_dir.clone());
        let storage = test_storage();
        let scene_id = saved_test_scene(&storage);

        let status = manager
            .start_internal(&storage, start_input(&scene_id), 100)
            .expect("start recording");
        let recording_id = status
            .active_recording_id
            .clone()
            .expect("active recording id");
        let segments = storage
            .with_connection(|connection| {
                storage::list_recording_segments(connection, &recording_id)
            })
            .expect("list segments");

        assert_eq!(status.state, RecorderState::Recording);
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].segment_index, 1);
        assert!(segments[0].file_path.ends_with("segments/0001.mkv"));
        assert!(PathBuf::from(&segments[0].file_path)
            .parent()
            .expect("segment parent")
            .exists());
        assert!(!PathBuf::from(&segments[0].file_path).exists());

        let _ = fs::remove_dir_all(app_data_dir);
    }

    #[test]
    fn manager_pause_resume_and_stop_persist_segments_in_order() {
        let app_data_dir = test_app_data_dir();
        let manager = RecorderManager::new(app_data_dir.clone());
        let storage = test_storage();
        let scene_id = saved_test_scene(&storage);

        let started = manager
            .start_internal(&storage, start_input(&scene_id), 10)
            .expect("start recording");
        let recording_id = started
            .active_recording_id
            .clone()
            .expect("active recording id");

        let paused = manager.pause_internal(&storage, 25).expect("pause");
        assert_eq!(paused.state, RecorderState::Paused);
        assert_eq!(paused.active_segment_id, None);

        let resumed = manager.resume_internal(&storage, 30).expect("resume");
        assert_eq!(resumed.state, RecorderState::Recording);
        assert!(resumed.active_segment_id.is_some());

        let stopped = manager.stop_internal(&storage, 42).expect("stop");
        assert_eq!(stopped.state, RecorderState::Idle);
        assert_eq!(stopped.active_recording_id, None);

        let recording = storage
            .with_connection(|connection| storage::get_recording(connection, &recording_id))
            .expect("read recording");
        let segments = storage
            .with_connection(|connection| {
                storage::list_recording_segments(connection, &recording_id)
            })
            .expect("list segments");

        assert_eq!(recording.status, "completed");
        assert_eq!(recording.started_at_ms, Some(10));
        assert_eq!(recording.ended_at_ms, Some(42));
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].state, "finalized");
        assert_eq!(segments[0].duration_ms, Some(15));
        assert!(segments[0].file_path.ends_with("0001.mkv"));
        assert_eq!(segments[1].state, "finalized");
        assert_eq!(segments[1].duration_ms, Some(12));
        assert!(segments[1].file_path.ends_with("0002.mkv"));

        let _ = fs::remove_dir_all(app_data_dir);
    }

    #[test]
    fn manager_stop_from_paused_does_not_create_new_segment() {
        let app_data_dir = test_app_data_dir();
        let manager = RecorderManager::new(app_data_dir.clone());
        let storage = test_storage();
        let scene_id = saved_test_scene(&storage);

        let started = manager
            .start_internal(&storage, start_input(&scene_id), 10)
            .expect("start recording");
        let recording_id = started
            .active_recording_id
            .clone()
            .expect("active recording id");
        manager.pause_internal(&storage, 20).expect("pause");
        manager.stop_internal(&storage, 30).expect("stop");

        let segments = storage
            .with_connection(|connection| {
                storage::list_recording_segments(connection, &recording_id)
            })
            .expect("list segments");

        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].state, "finalized");

        let _ = fs::remove_dir_all(app_data_dir);
    }

    #[test]
    fn manager_rejects_invalid_transitions() {
        let app_data_dir = test_app_data_dir();
        let manager = RecorderManager::new(app_data_dir.clone());
        let storage = test_storage();
        let scene_id = saved_test_scene(&storage);

        let pause_error = manager
            .pause_internal(&storage, 1)
            .expect_err("pause while idle should fail");
        let resume_error = manager
            .resume_internal(&storage, 1)
            .expect_err("resume while idle should fail");

        manager
            .start_internal(&storage, start_input(&scene_id), 10)
            .expect("start recording");
        let start_error = manager
            .start_internal(&storage, start_input(&scene_id), 11)
            .expect_err("start while active should fail");

        assert_eq!(pause_error.code, "recorder_not_recording");
        assert_eq!(resume_error.code, "recorder_not_paused");
        assert_eq!(start_error.code, "recorder_already_active");

        let _ = fs::remove_dir_all(app_data_dir);
    }

    #[test]
    fn manager_rejects_missing_scene_before_creating_recording() {
        let app_data_dir = test_app_data_dir();
        let manager = RecorderManager::new(app_data_dir.clone());
        let storage = test_storage();
        let scene_id = Uuid::now_v7().to_string();

        let error = manager
            .start_internal(&storage, start_input(&scene_id), 10)
            .expect_err("missing scene should fail");
        let recording_count: i64 = storage
            .with_connection(|connection| {
                connection.query_row("SELECT COUNT(*) FROM recordings", [], |row| row.get(0))
            })
            .expect("count recordings");

        assert_eq!(error.code, "recorder_scene_not_found");
        assert_eq!(recording_count, 0);
        assert!(!app_data_dir.join("recordings").exists());
    }

    #[test]
    fn public_lifecycle_command_still_returns_unavailable() {
        let scene_id = Uuid::now_v7().to_string();
        let start_error = start_recording(start_input(&scene_id))
            .expect_err("public start should remain unavailable");
        let pause_error = super::pause_recording(PauseRecordingInput {
            recording_id: Uuid::now_v7().to_string(),
        })
        .expect_err("public pause should remain unavailable");
        let resume_error = super::resume_recording(ResumeRecordingInput {
            recording_id: Uuid::now_v7().to_string(),
        })
        .expect_err("public resume should remain unavailable");
        let stop_error = super::stop_recording(StopRecordingInput {
            recording_id: Uuid::now_v7().to_string(),
        })
        .expect_err("public stop should remain unavailable");

        assert_eq!(start_error.code, "recorder_backend_not_implemented");
        assert_eq!(pause_error.code, "recorder_backend_not_implemented");
        assert_eq!(resume_error.code, "recorder_backend_not_implemented");
        assert_eq!(stop_error.code, "recorder_backend_not_implemented");
    }

    fn test_storage() -> storage::AppStorage {
        storage::test_storage(PathBuf::from("/tmp/daydream-test.sqlite3"))
    }

    fn test_app_data_dir() -> PathBuf {
        std::env::temp_dir().join(format!("daydream-recorder-{}", Uuid::now_v7()))
    }

    fn saved_test_scene(storage: &storage::AppStorage) -> String {
        let scene_id = Uuid::now_v7().to_string();
        save_recorder_scene_for_storage(
            storage,
            SaveRecorderSceneInput {
                scene: test_scene(&scene_id),
            },
            1,
        )
        .expect("save test scene");

        scene_id
    }

    fn start_input(scene_id: &str) -> StartRecordingInput {
        StartRecordingInput {
            desktop_audio_source_id: None,
            fps: 1.0,
            microphone_audio_source_id: None,
            quality_preset: RecorderQualityPreset::BalancedOcr,
            scene_id: scene_id.to_string(),
        }
    }

    fn test_scene(scene_id: &str) -> RecorderScene {
        RecorderScene {
            canvas_height: 1440,
            canvas_width: 2560,
            id: scene_id.to_string(),
            layout_mode: RecorderSceneLayoutMode::Horizontal,
            name: "Test scene".to_string(),
            sources: vec![RecorderSceneSource {
                crop: CropRect {
                    bottom: 0.0,
                    left: 0.0,
                    right: 0.0,
                    top: 0.0,
                },
                display_name: "Display 1".to_string(),
                id: Uuid::now_v7().to_string(),
                preserve_aspect_ratio: true,
                rect: CanvasRect {
                    height: 1440.0,
                    width: 2560.0,
                    x: 0.0,
                    y: 0.0,
                },
                source_id: "monitor-1".to_string(),
                source_kind: CaptureSourceKind::Monitor,
                z_index: 0,
            }],
        }
    }
}
