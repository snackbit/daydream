use gstreamer as gst;
use gstreamer::prelude::Cast;
use tracing::trace;

use super::super::{errors::dependency_error, types::RecorderError};
use super::{
    key_int_max, launch_value,
    types::{MediaSegmentInput, MediaWriterStreamIndexes},
};

const DESKTOP_AUDIO_BITRATE_BPS: u32 = 96_000;
const MICROPHONE_AUDIO_BITRATE_BPS: u32 = 64_000;
const LOW_LATENCY_QUEUE: &str = "queue max-size-time=0 max-size-bytes=0 max-size-buffers=0";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct AudioBranch {
    pub(super) source_name: &'static str,
    pub(super) cutoff_queue_name: &'static str,
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

#[cfg(test)]
pub(super) fn audio_source_names(input: &MediaSegmentInput) -> Vec<&'static str> {
    audio_branches(input)
        .into_iter()
        .map(|branch| branch.source_name)
        .collect()
}

pub(super) fn audio_branches(input: &MediaSegmentInput) -> Vec<AudioBranch> {
    let mut names = Vec::with_capacity(2);
    if input.desktop_audio_source_id.is_some() {
        names.push(AudioBranch {
            source_name: "desktop_audio_src",
            cutoff_queue_name: "desktop_audio_cutoff_queue",
        });
    }
    if input.microphone_audio_source_id.is_some() {
        names.push(AudioBranch {
            source_name: "microphone_audio_src",
            cutoff_queue_name: "microphone_audio_cutoff_queue",
        });
    }

    names
}

pub(super) fn build_pipeline(input: &MediaSegmentInput) -> Result<gst::Pipeline, RecorderError> {
    let launch = build_pipeline_description(input);
    trace!(
        output_path = %input.output_path.to_string_lossy(),
        launch = %launch,
        "media_writer_pipeline_description_built"
    );
    let element = gst::parse::launch(&launch).map_err(|error| {
        dependency_error(
            "recorder_media_writer_pipeline_failed",
            format!("Failed to build GStreamer pipeline: {error}."),
        )
    })?;
    element.downcast::<gst::Pipeline>().map_err(|_error| {
        dependency_error(
            "recorder_media_writer_pipeline_failed",
            "GStreamer launch description did not produce a pipeline.",
        )
    })
}

pub(super) fn build_pipeline_description(input: &MediaSegmentInput) -> String {
    let key_int_max = key_int_max(input.fps, 20);
    let mut description = format!(
        "appsrc name=video_src is-live=true format=time do-timestamp=false block=true \
         ! {queue} \
         ! videoconvert ! video/x-raw,format=I420 \
         ! x264enc bitrate={bitrate} key-int-max={key_int_max} speed-preset=medium \
             tune=zerolatency rc-lookahead=0 bframes=0 sync-lookahead=0 \
         ! h264parse ! {queue} ! mux. ",
        bitrate = input.video_bitrate_kbps,
        queue = LOW_LATENCY_QUEUE,
    );
    if let Some(source_id) = &input.desktop_audio_source_id {
        description.push_str(&audio_branch(
            source_id,
            DESKTOP_AUDIO_BITRATE_BPS,
            "desktop_audio_src",
        ));
    }
    if let Some(source_id) = &input.microphone_audio_source_id {
        description.push_str(&audio_branch(
            source_id,
            MICROPHONE_AUDIO_BITRATE_BPS,
            "microphone_audio_src",
        ));
    }
    description.push_str(&format!(
        "matroskamux name=mux ! filesink location={}",
        launch_value(input.output_path.as_os_str())
    ));

    description
}

fn audio_branch(source_id: &str, bitrate: u32, name: &str) -> String {
    let cutoff_queue_name = match name {
        "desktop_audio_src" => "desktop_audio_cutoff_queue",
        "microphone_audio_src" => "microphone_audio_cutoff_queue",
        _ => "audio_cutoff_queue",
    };
    format!(
        "pulsesrc name={name} device={device} ! {queue} ! audioconvert ! audioresample \
         ! audio/x-raw,rate=48000 ! opusenc bitrate={bitrate} ! {cutoff_queue} ! mux. ",
        name = name,
        device = launch_value(source_id),
        bitrate = bitrate,
        queue = LOW_LATENCY_QUEUE,
        cutoff_queue = low_latency_queue_with_name(cutoff_queue_name),
    )
}

fn low_latency_queue_with_name(name: &str) -> String {
    format!("queue name={name} max-size-time=0 max-size-bytes=0 max-size-buffers=0")
}
