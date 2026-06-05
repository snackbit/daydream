use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum RecorderState {
    Error,
    Idle,
    Pausing,
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
    pub(crate) category: RecorderErrorCategory,
    pub(crate) code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) details: Option<BTreeMap<String, serde_json::Value>>,
    pub(crate) message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CanvasRect {
    pub(crate) height: f64,
    pub(crate) width: f64,
    pub(crate) x: f64,
    pub(crate) y: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CropRect {
    pub(crate) bottom: f64,
    pub(crate) left: f64,
    pub(crate) right: f64,
    pub(crate) top: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum CaptureSourceKind {
    Monitor,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CaptureSource {
    pub(crate) display_name: String,
    pub(crate) height: Option<u32>,
    pub(crate) id: String,
    pub(crate) kind: CaptureSourceKind,
    pub(crate) logical_height: Option<u32>,
    pub(crate) logical_width: Option<u32>,
    pub(crate) scale_factor: Option<f64>,
    pub(crate) width: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CaptureSourceList {
    pub(crate) sources: Vec<CaptureSource>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CaptureSourceSelection {
    pub(crate) source_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SelectCaptureSourcesInput {
    #[serde(default)]
    pub(crate) force_chooser: bool,
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
    pub(crate) display_name: String,
    pub(crate) id: String,
    pub(crate) is_default: bool,
    pub(crate) kind: AudioSourceKind,
    pub(crate) roles: Vec<AudioSourceRole>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AudioSourceList {
    pub(crate) sources: Vec<AudioSource>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum RecorderSceneLayoutMode {
    #[serde(rename = "auto-16-9")]
    Auto169,
    Custom,
    Horizontal,
    Vertical,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecorderSceneSource {
    pub(crate) crop: CropRect,
    pub(crate) display_name: String,
    pub(crate) id: String,
    pub(crate) preserve_aspect_ratio: bool,
    pub(crate) rect: CanvasRect,
    pub(crate) source_id: String,
    pub(crate) source_height: Option<f64>,
    pub(crate) source_kind: CaptureSourceKind,
    pub(crate) source_width: Option<f64>,
    pub(crate) z_index: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecorderScene {
    pub(crate) canvas_height: u32,
    pub(crate) canvas_width: u32,
    pub(crate) desktop_audio_source_id: Option<String>,
    pub(crate) id: String,
    pub(crate) layout_mode: RecorderSceneLayoutMode,
    pub(crate) microphone_audio_source_id: Option<String>,
    pub(crate) name: String,
    pub(crate) sources: Vec<RecorderSceneSource>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SaveRecorderSceneInput {
    pub(crate) scene: RecorderScene,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum RecorderQualityPreset {
    BalancedOcr,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StartRecordingInput {
    pub(crate) desktop_audio_source_id: Option<String>,
    pub(crate) fps: f64,
    pub(crate) microphone_audio_source_id: Option<String>,
    pub(crate) quality_preset: RecorderQualityPreset,
    pub(crate) scene_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PauseRecordingInput {
    pub(crate) recording_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResumeRecordingInput {
    pub(crate) recording_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StopRecordingInput {
    pub(crate) recording_id: String,
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
    pub(crate) desktop_audio_stream_index: Option<i32>,
    pub(crate) duration_ms: Option<u64>,
    pub(crate) ended_at_ms: Option<u64>,
    pub(crate) error_code: Option<String>,
    pub(crate) ffmpeg_exit_code: Option<i32>,
    pub(crate) file_path: String,
    pub(crate) id: String,
    pub(crate) microphone_audio_stream_index: Option<i32>,
    pub(crate) recording_id: String,
    pub(crate) segment_index: u32,
    pub(crate) started_at_ms: u64,
    pub(crate) state: RecorderSegmentState,
    pub(crate) video_stream_index: Option<i32>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecorderBackendComponentHealth {
    pub(crate) available: bool,
    pub(crate) error: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecorderFfmpegHealth {
    pub(crate) available: bool,
    pub(crate) capabilities: RecorderFfmpegCapabilities,
    pub(crate) error: Option<String>,
    pub(crate) path: Option<String>,
    pub(crate) version_line: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecorderFfmpegCapabilities {
    pub(crate) libopus_encoder: bool,
    pub(crate) libx264_encoder: bool,
    pub(crate) matroska_muxer: bool,
    pub(crate) pulse_input: bool,
    pub(crate) rawvideo_demuxer: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecorderAudioHealth {
    pub(crate) available: bool,
    pub(crate) default_source_id: Option<String>,
    pub(crate) error: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecorderPortalHealth {
    pub(crate) available: bool,
    pub(crate) cursor_modes: Vec<String>,
    pub(crate) error: Option<String>,
    pub(crate) screen_cast_version: Option<u32>,
    pub(crate) source_types: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecorderBackendHealth {
    pub(crate) desktop_audio: RecorderAudioHealth,
    pub(crate) ffmpeg: RecorderFfmpegHealth,
    pub(crate) microphone: RecorderAudioHealth,
    pub(crate) pipewire: RecorderBackendComponentHealth,
    pub(crate) portal: RecorderPortalHealth,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecorderStatus {
    pub(crate) active_recording_id: Option<String>,
    pub(crate) active_scene_id: Option<String>,
    pub(crate) active_segment_id: Option<String>,
    pub(crate) backend_health: RecorderBackendHealth,
    pub(crate) last_error: Option<RecorderError>,
    pub(crate) state: RecorderState,
    pub(crate) updated_at_ms: u64,
}
