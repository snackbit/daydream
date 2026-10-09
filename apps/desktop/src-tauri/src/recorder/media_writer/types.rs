use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct FragmentEvent {
    pub(crate) opened: bool,
    pub(crate) path: PathBuf,
    pub(crate) timestamp_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct MediaSegmentInput {
    pub(crate) video_sources: Vec<crate::storage::RecordingVideoSource>,
    pub(crate) benchmarks_dir: PathBuf,
    pub(crate) canvas_height: u32,
    pub(crate) canvas_width: u32,
    pub(crate) desktop_audio_source_id: Option<String>,
    pub(crate) fps: f64,
    pub(crate) microphone_audio_source_id: Option<String>,
    pub(crate) output_path: PathBuf,
    pub(crate) recording_id: String,
    pub(crate) segment_id: String,
    pub(crate) segment_index: u32,
    pub(crate) started_at_ms: u64,
    pub(crate) video_bitrate_kbps: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MediaWriterStreamIndexes {
    pub(crate) desktop_audio_stream_index: Option<i64>,
    pub(crate) microphone_audio_stream_index: Option<i64>,
    pub(crate) video_stream_index: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MediaSegmentProbe {
    #[serde(default)]
    pub(crate) videos: Vec<MediaSegmentVideoProbe>,
    pub(crate) audio: Vec<MediaSegmentAudioProbe>,
    pub(crate) available: bool,
    pub(crate) duration_ms: Option<u64>,
    pub(crate) error: Option<String>,
    pub(crate) probe_backend: String,
    pub(crate) streams: Vec<MediaSegmentStreamProbe>,
    pub(crate) video: Option<MediaSegmentVideoProbe>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MediaSegmentStreamProbe {
    pub(crate) codec_name: Option<String>,
    pub(crate) codec_type: Option<String>,
    pub(crate) duration_ms: Option<u64>,
    pub(crate) index: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MediaSegmentVideoProbe {
    pub(crate) codec_name: Option<String>,
    pub(crate) duration_ms: Option<u64>,
    pub(crate) estimated_frame_count: Option<u64>,
    pub(crate) height: Option<u32>,
    pub(crate) index: i64,
    pub(crate) pixel_format: Option<String>,
    pub(crate) width: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MediaSegmentAudioProbe {
    pub(crate) channels: Option<u32>,
    pub(crate) codec_name: Option<String>,
    pub(crate) duration_ms: Option<u64>,
    pub(crate) index: i64,
    pub(crate) sample_rate: Option<u32>,
}
