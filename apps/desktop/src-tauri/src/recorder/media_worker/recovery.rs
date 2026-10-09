use super::{
    super::{
        errors::dependency_error,
        media_writer::{self, MediaSegmentProbe},
        types::RecorderError,
    },
    protocol::{self, Request, Response},
};
use gstreamer::{self as gst, prelude::*};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    net::Shutdown,
    path::{Path, PathBuf},
    sync::{mpsc, Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct RecoveredMedia {
    pub(crate) original_probe: MediaSegmentProbe,
    pub(crate) probe: MediaSegmentProbe,
    pub(crate) start_pts_ms: i64,
    pub(crate) end_pts_ms: i64,
    pub(crate) streams: Vec<RecoveredStream>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct RecoveredStream {
    pub(crate) original_index: i64,
    pub(crate) output_index: i64,
    pub(crate) start_pts_ms: i64,
    pub(crate) end_pts_ms: i64,
}
fn error() -> RecorderError {
    dependency_error("recorder_recovery_media_failed","The recording fragment could not be recovered safely. The original file has been preserved.")
}

pub(crate) fn recover_media(input: &Path, output: &Path) -> Result<RecoveredMedia, RecorderError> {
    if input == output || output.exists() {
        return Err(error());
    }
    super::parent::acquire()?;
    let (mut child, mut control, frames, mut reader, _) =
        super::process::spawn().map_err(|failure| {
            if failure.kind() == std::io::ErrorKind::TimedOut {
                super::parent::quarantine();
            }
            super::parent::release();
            error()
        })?;
    let deadline = Instant::now() + Duration::from_secs(20);
    let (sender, result) = mpsc::channel();
    thread::spawn(move || {
        let _ = sender.send(protocol::receive::<Response>(&mut reader));
    });
    let response = protocol::send(
        &mut control,
        &Request::Recover {
            version: protocol::VERSION,
            input: input.into(),
            output: output.into(),
        },
    )
    .and_then(|_| {
        result
            .recv_timeout(Duration::from_secs(17))
            .map_err(std::io::Error::other)
    })
    .and_then(|r| r);
    let _ = control.shutdown(Shutdown::Both);
    let _ = frames.shutdown(Shutdown::Both);
    let _ = child.kill();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => {
                super::parent::release();
                break;
            }
            _ => {
                if Instant::now() >= deadline {
                    super::parent::quarantine();
                    return Err(error());
                }
                thread::sleep(Duration::from_millis(10));
            }
        }
    }
    match response {
        Ok(Response::Recovered { result }) => result,
        _ => Err(error()),
    }
}

pub(super) fn remux(input: PathBuf, output: PathBuf) -> Result<RecoveredMedia, RecorderError> {
    if input == output || output.exists() {
        return Err(error());
    }
    gst::init().map_err(|_| error())?;
    let original = media_writer::probe_segment_best_effort(&input);
    if !original.available || original.videos.is_empty() {
        return Err(error());
    }
    let pipeline = gst::Pipeline::new();
    let source = gst::ElementFactory::make("filesrc")
        .property("location", input.to_string_lossy().as_ref())
        .build()
        .map_err(|_| error())?;
    let demux = gst::ElementFactory::make("matroskademux")
        .build()
        .map_err(|_| error())?;
    let mux = gst::ElementFactory::make("matroskamux")
        .property("offset-to-zero", false)
        .build()
        .map_err(|_| error())?;
    let sink = gst::ElementFactory::make("filesink")
        .property("location", output.to_string_lossy().as_ref())
        .build()
        .map_err(|_| error())?;
    pipeline
        .add_many([&source, &demux, &mux, &sink])
        .map_err(|_| error())?;
    source.link(&demux).map_err(|_| error())?;
    mux.link(&sink).map_err(|_| error())?;
    let coverage = Arc::new(Mutex::new(BTreeMap::<i64, RecoveredStream>::new()));
    let coverage_for_pad = coverage.clone();
    let video_count = original.videos.len() as i64;
    let expected_count = original.streams.len();
    let weak = pipeline.downgrade();
    let output_pads = (0..expected_count)
        .map(|index| {
            mux.request_pad_simple(if index < (video_count as usize) {
                "video_%u"
            } else {
                "audio_%u"
            })
            .ok_or_else(error)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let invalid = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let failed = invalid.clone();
    demux.connect_pad_added(move |_, pad| {
        let result = (|| -> Result<(), RecorderError> {
            let name = pad.name();
            let (kind, index) = name.rsplit_once('_').ok_or_else(error)?;
            let index = index.parse::<i64>().map_err(|_| error())?;
            let (original_index, parser_name) = match kind {
                "video" => (index, "h264parse"),
                "audio" => (index + video_count, "opusparse"),
                _ => return Err(error()),
            };
            let pipeline = weak.upgrade().ok_or_else(error)?;
            let queue = gst::ElementFactory::make("queue")
                .property("max-size-bytes", 1_048_576u32)
                .build()
                .map_err(|_| error())?;
            let parser = gst::ElementFactory::make(parser_name)
                .build()
                .map_err(|_| error())?;
            pipeline.add_many([&queue, &parser]).map_err(|_| error())?;
            queue.link(&parser).map_err(|_| error())?;
            let sink_pad = output_pads
                .get(usize::try_from(original_index).map_err(|_| error())?)
                .ok_or_else(error)?;
            let mut streams = coverage_for_pad.lock().map_err(|_| error())?;
            let output_index = original_index;
            if streams
                .insert(
                    original_index,
                    RecoveredStream {
                        original_index,
                        output_index,
                        start_pts_ms: i64::MAX,
                        end_pts_ms: 0,
                    },
                )
                .is_some()
            {
                return Err(error());
            }
            drop(streams);
            let times = coverage_for_pad.clone();
            pad.add_probe(gst::PadProbeType::BUFFER, move |_, info| {
                if let Some(buffer) = info.buffer() {
                    if let Some(pts) = buffer.pts() {
                        if let Ok(mut streams) = times.lock() {
                            if let Some(stream) = streams.get_mut(&original_index) {
                                stream.start_pts_ms =
                                    stream.start_pts_ms.min(pts.mseconds() as i64);
                                stream.end_pts_ms = stream.end_pts_ms.max(
                                    pts.saturating_add(
                                        buffer.duration().unwrap_or(gst::ClockTime::ZERO),
                                    )
                                    .mseconds() as i64,
                                );
                            }
                        }
                    }
                }
                gst::PadProbeReturn::Ok
            });
            parser
                .static_pad("src")
                .ok_or_else(error)?
                .link(sink_pad)
                .map_err(|_| error())?;
            queue.sync_state_with_parent().map_err(|_| error())?;
            parser.sync_state_with_parent().map_err(|_| error())?;
            pad.link(&queue.static_pad("sink").ok_or_else(error)?)
                .map_err(|_| error())?;
            Ok(())
        })();
        if result.is_err() {
            failed.store(true, std::sync::atomic::Ordering::SeqCst);
        }
    });
    let bus = pipeline.bus().ok_or_else(error)?;
    pipeline
        .set_state(gst::State::Playing)
        .map_err(|_| error())?;
    let terminal = bus.timed_pop_filtered(
        gst::ClockTime::from_seconds(10),
        &[gst::MessageType::Eos, gst::MessageType::Error],
    );
    let success =
        terminal.is_some_and(|message| matches!(message.view(), gst::MessageView::Eos(_)));
    let _ = pipeline.set_state(gst::State::Null);
    if !success || invalid.load(std::sync::atomic::Ordering::SeqCst) {
        return Err(error());
    }
    let streams = coverage
        .lock()
        .map_err(|_| error())?
        .values()
        .cloned()
        .collect::<Vec<_>>();
    if streams.len() != expected_count
        || streams
            .iter()
            .any(|s| s.start_pts_ms == i64::MAX || s.end_pts_ms < s.start_pts_ms)
    {
        return Err(error());
    }
    let probe = media_writer::probe_segment_best_effort(&output);
    if !probe.available || probe.streams.len() != streams.len() {
        return Err(error());
    }
    Ok(RecoveredMedia {
        original_probe: original,
        probe,
        start_pts_ms: streams
            .iter()
            .map(|s| s.start_pts_ms)
            .min()
            .ok_or_else(error)?,
        end_pts_ms: streams
            .iter()
            .map(|s| s.end_pts_ms)
            .max()
            .ok_or_else(error)?,
        streams,
    })
}
