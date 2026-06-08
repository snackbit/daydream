use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{RecorderBackendHealth, RecorderError};

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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum CaptureFrameStorageKind {
    CpuReadable,
    DmaBuf,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CaptureTimingDiagnostics {
    pub(crate) composed_frames: u64,
    pub(crate) cpu_readable_frames_received: u64,
    pub(crate) dmabuf_frames_read_back_to_cpu: u64,
    pub(crate) dmabuf_frames_received: u64,
    pub(crate) frames_received_by_source: BTreeMap<String, u64>,
    pub(crate) frames_sent_to_writer: u64,
    pub(crate) last_composed_timestamp_ms: Option<u64>,
    pub(crate) latest_frame_storage_by_source: BTreeMap<String, CaptureFrameStorageKind>,
    pub(crate) max_compose_duration_ms: u64,
    pub(crate) reused_frame_compositions: u64,
    pub(crate) scheduler_missed_deadlines: u64,
    pub(crate) scheduler_ticks: u64,
    pub(crate) source_timestamp_max_ms: Option<u64>,
    pub(crate) source_timestamp_min_ms: Option<u64>,
    pub(crate) total_compose_duration_ms: u64,
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
