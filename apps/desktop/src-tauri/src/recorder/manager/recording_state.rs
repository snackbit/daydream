use std::path::PathBuf;

use super::super::{
    capture_runtime::SharedCaptureTiming,
    media_writer::{MediaSegmentInput, MediaWriterStreamIndexes},
    types::StartRecordingInput,
};

#[derive(Clone, Debug)]
pub(crate) struct ActiveRecording {
    pub(super) capture_run_id: String,
    pub(super) video_sources: Vec<crate::storage::RecordingVideoSource>,
    pub(super) active_segment: Option<ActiveSegment>,
    pub(super) canvas_height: u32,
    pub(super) canvas_width: u32,
    pub(super) desktop_audio_source_id: Option<String>,
    pub(super) fps: f64,
    pub(super) media_writer: String,
    pub(super) microphone_audio_source_id: Option<String>,
    pub(super) recording_id: String,
    pub(super) scene_id: String,
    pub(super) segments_dir: PathBuf,
    pub(super) next_segment_index: u32,
    pub(super) video_bitrate_kbps: u32,
}

#[derive(Clone, Debug)]
pub(crate) struct ActiveSegment {
    pub(super) capture_started_at_ms: u64,
    pub(super) file_path: PathBuf,
    pub(super) id: String,
    pub(super) runtime: Option<SharedSegmentRuntime>,
    pub(super) started_at_ms: u64,
    pub(super) stream_indexes: MediaWriterStreamIndexes,
    pub(super) timing: Option<SharedCaptureTiming>,
}

pub(crate) type SharedSegmentRuntime =
    std::sync::Arc<std::sync::Mutex<Option<super::RecordingSegmentRuntime>>>;

#[derive(Clone, Debug)]
pub(crate) struct PendingRecordingStart {
    pub(crate) active: ActiveRecording,
    pub(crate) recording_dir: PathBuf,
    pub(crate) recording_id: String,
    pub(crate) segment: ActiveSegment,
    pub(crate) segment_path: PathBuf,
    pub(crate) segment_index: u32,
    pub(crate) segment_input: MediaSegmentInput,
    pub(crate) input: StartRecordingInput,
    pub(crate) scene: super::super::types::RecorderScene,
}

#[derive(Clone, Debug)]
pub(crate) struct PendingSegmentStart {
    pub(crate) active: ActiveRecording,
    pub(crate) segment: ActiveSegment,
    pub(crate) segment_path: PathBuf,
    pub(crate) segment_index: u32,
    pub(crate) segment_input: MediaSegmentInput,
    pub(crate) scene: super::super::types::RecorderScene,
}
