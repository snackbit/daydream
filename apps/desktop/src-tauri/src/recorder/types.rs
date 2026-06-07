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
#[serde(rename_all = "camelCase")]
pub struct StartRecorderPreviewInput {
    pub(crate) scene_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum RecorderPreviewState {
    Error,
    Idle,
    Live,
    Starting,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecorderPreviewStatus {
    pub(crate) error: Option<RecorderError>,
    pub(crate) scene_id: Option<String>,
    pub(crate) source_ids: Vec<String>,
    pub(crate) state: RecorderPreviewState,
    pub(crate) updated_at_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecorderPreviewFrame {
    pub(crate) data_url: String,
    pub(crate) height: u32,
    pub(crate) scene_source_id: String,
    pub(crate) source_id: String,
    pub(crate) timestamp_ms: u64,
    pub(crate) width: u32,
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
#[serde(rename_all = "kebab-case")]
pub enum RecorderFinalizationAction {
    Pause,
    Stop,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum RecorderFinalizationPhase {
    SavingSegment,
    StoppingCapture,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecorderFinalizationStatus {
    pub(crate) action: RecorderFinalizationAction,
    pub(crate) phase: RecorderFinalizationPhase,
    pub(crate) recording_id: String,
    pub(crate) segment_id: Option<String>,
    pub(crate) started_at_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecorderSegment {
    pub(crate) desktop_audio_stream_index: Option<i32>,
    pub(crate) duration_ms: Option<u64>,
    pub(crate) ended_at_ms: Option<u64>,
    pub(crate) error_code: Option<String>,
    pub(crate) writer_exit_code: Option<i32>,
    pub(crate) file_path: String,
    pub(crate) id: String,
    pub(crate) microphone_audio_stream_index: Option<i32>,
    pub(crate) recording_id: String,
    pub(crate) segment_index: u32,
    pub(crate) started_at_ms: u64,
    pub(crate) state: RecorderSegmentState,
    pub(crate) video_stream_index: Option<i32>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CaptureTimingDiagnostics {
    pub(crate) composed_frames: u64,
    pub(crate) frames_received_by_source: BTreeMap<String, u64>,
    pub(crate) frames_sent_to_writer: u64,
    pub(crate) last_composed_timestamp_ms: Option<u64>,
    pub(crate) max_compose_duration_ms: u64,
    pub(crate) reused_frame_compositions: u64,
    pub(crate) scheduler_missed_deadlines: u64,
    pub(crate) scheduler_ticks: u64,
    pub(crate) source_timestamp_max_ms: Option<u64>,
    pub(crate) source_timestamp_min_ms: Option<u64>,
    pub(crate) total_compose_duration_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecorderBackendComponentHealth {
    pub(crate) available: bool,
    pub(crate) error: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecorderMediaWriterHealth {
    pub(crate) available: bool,
    pub(crate) capabilities: RecorderMediaWriterCapabilities,
    pub(crate) error: Option<String>,
    pub(crate) version: Option<String>,
    pub(crate) writer: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecorderMediaWriterCapabilities {
    pub(crate) appsrc: bool,
    pub(crate) audio_convert: bool,
    pub(crate) audio_resample: bool,
    pub(crate) file_sink: bool,
    pub(crate) h264_parse: bool,
    pub(crate) matroska_muxer: bool,
    pub(crate) opus_encoder: bool,
    pub(crate) pulse_source: bool,
    pub(crate) queue: bool,
    pub(crate) video_convert: bool,
    pub(crate) x264_encoder: bool,
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
pub struct RecorderGpuCaptureHealth {
    pub(crate) available: bool,
    pub(crate) cleanup_error: Option<String>,
    pub(crate) dmabuf_import_available: bool,
    pub(crate) egl_available: bool,
    pub(crate) egl_client_apis: Vec<String>,
    pub(crate) egl_platform: Option<String>,
    pub(crate) egl_vendor: Option<String>,
    pub(crate) egl_version: Option<String>,
    pub(crate) error: Option<String>,
    pub(crate) modifier_import_available: Option<bool>,
    pub(crate) opengl_available: bool,
    pub(crate) renderer: Option<String>,
    pub(crate) vendor: Option<String>,
    pub(crate) version: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecorderBackendHealth {
    pub(crate) desktop_audio: RecorderAudioHealth,
    pub(crate) gpu_capture: RecorderGpuCaptureHealth,
    pub(crate) media_writer: RecorderMediaWriterHealth,
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
    pub(crate) finalization: Option<RecorderFinalizationStatus>,
    pub(crate) last_error: Option<RecorderError>,
    pub(crate) state: RecorderState,
    pub(crate) updated_at_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecorderPerformanceStats {
    pub(crate) desired_fps: Option<f64>,
    pub(crate) segment_started_at_ms: Option<u64>,
    pub(crate) state: RecorderState,
    pub(crate) timing: Option<CaptureTimingDiagnostics>,
    pub(crate) updated_at_ms: u64,
}
