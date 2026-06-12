use std::{path::Path, time::Duration};

use gstreamer as gst;
use gstreamer_pbutils as gst_pbutils;

use super::types::{
    MediaSegmentAudioProbe, MediaSegmentProbe, MediaSegmentStreamProbe, MediaSegmentVideoProbe,
};

const PROBE_TIMEOUT: Duration = Duration::from_secs(5);

pub(crate) fn probe_segment_best_effort(segment_path: &Path) -> MediaSegmentProbe {
    match probe_segment(segment_path) {
        Ok(probe) => probe,
        Err(error) => unavailable_segment_probe(error),
    }
}

pub(crate) fn probe_json_string(probe: &MediaSegmentProbe) -> String {
    serde_json::to_string(probe).unwrap_or_else(|error| {
        serde_json::to_string(&unavailable_segment_probe(error.to_string()))
            .expect("serialize unavailable media probe")
    })
}

fn probe_segment(segment_path: &Path) -> Result<MediaSegmentProbe, String> {
    gst::init().map_err(|error| format!("GStreamer initialization failed: {error}."))?;
    let discoverer =
        gst_pbutils::Discoverer::new(gst::ClockTime::from_seconds(PROBE_TIMEOUT.as_secs()))
            .map_err(|error| format!("Failed to create GStreamer discoverer: {error}."))?;
    let uri = gst::glib::filename_to_uri(segment_path, None::<&str>)
        .map_err(|error| format!("Failed to build file URI for media probe: {error}."))?;
    let info = discoverer
        .discover_uri(uri.as_ref())
        .map_err(|error| format!("GStreamer media discovery failed: {error}."))?;
    let duration_ms = info.duration().map(clock_time_ms);
    let video_streams = info.video_streams();
    let audio_streams = info.audio_streams();
    let video = video_streams
        .first()
        .map(|stream| video_probe(0, stream, duration_ms));
    let audio = audio_streams
        .iter()
        .enumerate()
        .map(|(index, stream)| audio_probe(index as i64 + 1, stream, duration_ms))
        .collect::<Vec<_>>();
    let mut streams = Vec::new();
    if let Some(video) = &video {
        streams.push(MediaSegmentStreamProbe {
            codec_name: video.codec_name.clone(),
            codec_type: Some("video".to_string()),
            duration_ms: video.duration_ms,
            index: video.index,
        });
    }
    streams.extend(audio.iter().map(|audio| MediaSegmentStreamProbe {
        codec_name: audio.codec_name.clone(),
        codec_type: Some("audio".to_string()),
        duration_ms: audio.duration_ms,
        index: audio.index,
    }));

    Ok(MediaSegmentProbe {
        audio,
        available: true,
        duration_ms,
        error: None,
        probe_backend: "gstreamer-discoverer".to_string(),
        streams,
        video,
    })
}

fn video_probe(
    index: i64,
    stream: &gst_pbutils::DiscovererVideoInfo,
    duration_ms: Option<u64>,
) -> MediaSegmentVideoProbe {
    MediaSegmentVideoProbe {
        codec_name: stream_caps_name(stream),
        duration_ms,
        estimated_frame_count: None,
        height: Some(stream.height()),
        index,
        pixel_format: stream_caps_field(stream, "format"),
        width: Some(stream.width()),
    }
}

fn audio_probe(
    index: i64,
    stream: &gst_pbutils::DiscovererAudioInfo,
    duration_ms: Option<u64>,
) -> MediaSegmentAudioProbe {
    MediaSegmentAudioProbe {
        channels: Some(stream.channels()),
        codec_name: stream_caps_name(stream),
        duration_ms,
        index,
        sample_rate: Some(stream.sample_rate()),
    }
}

fn stream_caps_name(stream: &impl gst_pbutils::prelude::DiscovererStreamInfoExt) -> Option<String> {
    stream.caps().and_then(|caps| {
        caps.structure(0)
            .map(|structure| structure.name().to_string())
    })
}

fn stream_caps_field(
    stream: &impl gst_pbutils::prelude::DiscovererStreamInfoExt,
    field: &str,
) -> Option<String> {
    stream.caps().and_then(|caps| {
        caps.structure(0)
            .and_then(|structure| structure.get::<String>(field).ok())
    })
}

fn unavailable_segment_probe(error: impl Into<String>) -> MediaSegmentProbe {
    MediaSegmentProbe {
        audio: Vec::new(),
        available: false,
        duration_ms: None,
        error: Some(error.into()),
        probe_backend: "gstreamer-discoverer".to_string(),
        streams: Vec::new(),
        video: None,
    }
}

fn clock_time_ms(value: gst::ClockTime) -> u64 {
    value.mseconds()
}
