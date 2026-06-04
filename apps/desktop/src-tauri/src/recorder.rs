use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

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

#[tauri::command(rename = "getRecorderStatus")]
pub fn get_recorder_status() -> Result<RecorderStatus, RecorderError> {
    Err(unavailable_error())
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
pub fn save_recorder_scene(_input: SaveRecorderSceneInput) -> Result<RecorderScene, RecorderError> {
    Err(unavailable_error())
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

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        unavailable_error, CanvasRect, RecorderErrorCategory, RecorderQualityPreset, RecorderState,
    };

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
}
