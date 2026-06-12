use std::sync::{
    atomic::{AtomicU64, Ordering},
    mpsc, Arc,
};
use std::{thread, time::Duration};

use gstreamer as gst;
use gstreamer::prelude::*;
use tracing::{error, info, trace};

use super::super::{errors::dependency_error, types::RecorderError};
use super::{duration_to_clock_time, pipeline::AudioBranch};

const SOURCE_EOS_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Debug)]
pub(super) struct AudioCutoffProbe {
    source_name: &'static str,
    dropped_buffers: Arc<AtomicU64>,
}

pub(super) fn send_audio_source_eos(
    pipeline: &gst::Pipeline,
    audio_branches: &[AudioBranch],
) -> Result<(), RecorderError> {
    for branch in audio_branches {
        let source_name = branch.source_name;
        let source = pipeline.by_name(source_name).ok_or_else(|| {
            dependency_error(
                "recorder_media_writer_finalize_failed",
                format!("GStreamer audio source {source_name} is missing from the pipeline."),
            )
        })?;
        trace!(
            source = source_name,
            "media_writer_audio_source_eos_requested"
        );
        send_element_eos_with_timeout(source, source_name)?;
        trace!(source = source_name, "media_writer_audio_source_eos_sent");
    }

    Ok(())
}

pub(super) fn install_audio_cutoff_probes(
    pipeline: &gst::Pipeline,
    audio_branches: &[AudioBranch],
    started_at_ms: u64,
    stop_at_ms: u64,
) -> Result<Vec<AudioCutoffProbe>, RecorderError> {
    let cutoff_pts = duration_to_clock_time(Duration::from_millis(
        stop_at_ms.saturating_sub(started_at_ms),
    ));
    audio_branches
        .iter()
        .map(|branch| {
            let queue = pipeline.by_name(branch.cutoff_queue_name).ok_or_else(|| {
                dependency_error(
                    "recorder_media_writer_finalize_failed",
                    format!(
                        "GStreamer audio cutoff queue {} is missing from the pipeline.",
                        branch.cutoff_queue_name
                    ),
                )
            })?;
            let pad = queue.static_pad("src").ok_or_else(|| {
                dependency_error(
                    "recorder_media_writer_finalize_failed",
                    format!(
                        "GStreamer audio cutoff queue {} has no src pad.",
                        branch.cutoff_queue_name
                    ),
                )
            })?;
            let dropped_buffers = Arc::new(AtomicU64::new(0));
            let dropped_for_probe = dropped_buffers.clone();
            pad.add_probe(gst::PadProbeType::BUFFER, move |_pad, info| {
                if let Some(buffer) = info.buffer() {
                    if audio_buffer_is_after_cutoff(buffer, cutoff_pts) {
                        dropped_for_probe.fetch_add(1, Ordering::Relaxed);
                        return gst::PadProbeReturn::Drop;
                    }
                }
                gst::PadProbeReturn::Ok
            });

            Ok(AudioCutoffProbe {
                source_name: branch.source_name,
                dropped_buffers,
            })
        })
        .collect()
}

pub(super) fn log_audio_cutoff_results(probes: &[AudioCutoffProbe]) {
    for probe in probes {
        info!(
            source = probe.source_name,
            dropped_buffers = probe.dropped_buffers.load(Ordering::Relaxed),
            "media_writer_audio_cutoff_applied"
        );
    }
}

pub(super) fn audio_buffer_is_after_cutoff(
    buffer: &gst::BufferRef,
    cutoff_pts: gst::ClockTime,
) -> bool {
    buffer.pts().is_some_and(|pts| pts > cutoff_pts)
}

fn send_element_eos_with_timeout(
    element: gst::Element,
    source_name: &'static str,
) -> Result<(), RecorderError> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let sent = element.send_event(gst::event::Eos::new());
        let _ = sender.send(sent);
    });

    match receiver.recv_timeout(SOURCE_EOS_TIMEOUT) {
        Ok(true) => Ok(()),
        Ok(false) => Err(dependency_error(
            "recorder_media_writer_finalize_failed",
            format!("GStreamer audio source {source_name} rejected EOS."),
        )),
        Err(mpsc::RecvTimeoutError::Timeout) => {
            error!(
                source = source_name,
                "media_writer_audio_source_eos_timeout"
            );
            Err(dependency_error(
                "recorder_media_writer_finalize_failed",
                format!("Timed out sending EOS to GStreamer audio source {source_name}."),
            ))
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => Err(dependency_error(
            "recorder_media_writer_finalize_failed",
            format!("GStreamer audio source {source_name} EOS sender stopped unexpectedly."),
        )),
    }
}
