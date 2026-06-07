use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::{
    capture_runtime::{FrameReceiver, SharedCaptureTiming},
    errors::dependency_error,
    types::{RecorderError, RecorderMediaWriterCapabilities, RecorderMediaWriterHealth},
};

const UNSUPPORTED_MESSAGE: &str = "Recorder media writing is only implemented on Linux.";

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MediaSegmentInput {
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

#[derive(Debug)]
pub(crate) struct MediaWriterSegment;

impl MediaWriterSegment {
    pub(crate) fn benchmark(&self) -> Option<super::benchmark::RecorderBenchmarkSession> {
        None
    }
}

pub(crate) fn collect_media_writer_health() -> RecorderMediaWriterHealth {
    RecorderMediaWriterHealth {
        available: false,
        capabilities: RecorderMediaWriterCapabilities::default(),
        error: Some(UNSUPPORTED_MESSAGE.to_string()),
        version: None,
        writer: None,
    }
}

pub(crate) fn validate_segment_capabilities(
    _capabilities: &RecorderMediaWriterCapabilities,
    _input: &MediaSegmentInput,
) -> Result<(), RecorderError> {
    Err(unavailable_error())
}

pub(crate) fn start_synthetic_segment(
    _input: &MediaSegmentInput,
) -> Result<MediaWriterSegment, RecorderError> {
    Err(unavailable_error())
}

pub(crate) fn start_segment_from_frames(
    _input: &MediaSegmentInput,
    _frames: FrameReceiver,
    _timing: Option<SharedCaptureTiming>,
) -> Result<MediaWriterSegment, RecorderError> {
    Err(unavailable_error())
}

impl MediaWriterSegment {
    pub(crate) fn stop(self, _stop_at_ms: u64) -> Result<(), RecorderError> {
        Err(unavailable_error())
    }
}

pub(crate) fn stream_indexes(input: &MediaSegmentInput) -> MediaWriterStreamIndexes {
    let mut next_stream_index = 1;
    let desktop_audio_stream_index = input.desktop_audio_source_id.as_ref().map(|_source_id| {
        let index = next_stream_index;
        next_stream_index += 1;
        index
    });
    let microphone_audio_stream_index =
        input.microphone_audio_source_id.as_ref().map(|_source_id| {
            let index = next_stream_index;
            next_stream_index += 1;
            index
        });

    MediaWriterStreamIndexes {
        desktop_audio_stream_index,
        microphone_audio_stream_index,
        video_stream_index: 0,
    }
}

pub(crate) fn probe_segment_best_effort(_segment_path: &Path) -> MediaSegmentProbe {
    unavailable_segment_probe(UNSUPPORTED_MESSAGE)
}

pub(crate) fn probe_json_string(probe: &MediaSegmentProbe) -> String {
    serde_json::to_string(probe).unwrap_or_else(|error| {
        serde_json::to_string(&unavailable_segment_probe(error.to_string()))
            .expect("serialize unavailable media probe")
    })
}

fn unavailable_segment_probe(error: impl Into<String>) -> MediaSegmentProbe {
    MediaSegmentProbe {
        audio: Vec::new(),
        available: false,
        duration_ms: None,
        error: Some(error.into()),
        probe_backend: "unsupported".to_string(),
        streams: Vec::new(),
        video: None,
    }
}

fn unavailable_error() -> RecorderError {
    dependency_error(
        "recorder_media_writer_unavailable",
        UNSUPPORTED_MESSAGE.to_string(),
    )
}
