#[cfg(test)]
use super::super::media_writer::InProcessMediaWriterSegment;
#[cfg(test)]
#[path = "test_hooks.rs"]
mod test_hooks;

use super::{
    super::{
        benchmark::RecorderBenchmarkSession,
        capture_runtime::{FrameReceiver, SharedCaptureTiming},
        errors::dependency_error,
        media_writer::{FinalizedSegmentMedia, FragmentEvent, MediaSegmentInput},
        types::RecorderError,
    },
    protocol::{self, Request, Response},
};
use std::{
    io,
    net::Shutdown,
    os::unix::net::UnixStream,
    sync::{
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};

static GENERATION: AtomicU64 = AtomicU64::new(1);
static ACTIVE: AtomicUsize = AtomicUsize::new(0);
static QUARANTINED: AtomicBool = AtomicBool::new(false);
pub(crate) fn is_quiescent() -> bool {
    ACTIVE.load(Ordering::SeqCst) == 0 && !QUARANTINED.load(Ordering::SeqCst)
}
pub(crate) fn is_quarantined() -> bool {
    QUARANTINED.load(Ordering::SeqCst)
}
pub(crate) fn quarantine() {
    QUARANTINED.store(true, Ordering::SeqCst);
}
fn failure(code: &str) -> RecorderError {
    dependency_error(
        code,
        "The media worker could not finish safely. Saved recording files have been preserved.",
    )
}
pub(super) fn acquire() -> Result<(), RecorderError> {
    if is_quarantined()
        || ACTIVE
            .compare_exchange(0, 1, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
    {
        return Err(failure("recorder_media_cleanup_pending"));
    }
    Ok(())
}
pub(super) fn release() {
    ACTIVE.fetch_sub(1, Ordering::SeqCst);
}

#[derive(Debug)]
pub(crate) enum MediaWriterSegment {
    #[cfg(test)]
    Local(InProcessMediaWriterSegment),
    Process(MediaProcess),
}
#[derive(Debug)]
pub(crate) struct MediaProcess {
    child: Option<super::process::WorkerChild>,
    control: UnixStream,
    frames: UnixStream,
    generation: u64,
    responses: mpsc::Receiver<Response>,
    fragments: Vec<FragmentEvent>,
    transport_stop: mpsc::Sender<()>,
    transport_done: mpsc::Receiver<io::Result<()>>,
    error: Option<RecorderError>,
}

pub(crate) fn start_segment_from_frames(
    input: &MediaSegmentInput,
    frames: FrameReceiver,
    timing: Option<SharedCaptureTiming>,
) -> Result<MediaWriterSegment, RecorderError> {
    #[cfg(test)]
    if std::env::var_os("DAYDREAM_MEDIA_WORKER_BINARY").is_none() {
        return super::super::media_writer::start_in_process_segment(input, frames, timing)
            .map(MediaWriterSegment::Local);
    }
    start_process_segment_with_timing(input, frames, timing)
}
#[cfg(test)]
pub(super) fn start_process_segment(
    input: &MediaSegmentInput,
    frames: FrameReceiver,
) -> Result<MediaWriterSegment, RecorderError> {
    start_process_segment_with_timing(input, frames, None)
}
fn start_process_segment_with_timing(
    input: &MediaSegmentInput,
    frames: FrameReceiver,
    timing: Option<SharedCaptureTiming>,
) -> Result<MediaWriterSegment, RecorderError> {
    protocol::validate_input(input)
        .map_err(|_| failure("recorder_media_transport_budget_exceeded"))?;
    acquire()?;
    let (child, control, frame_stream, reader, transport) =
        super::process::spawn().map_err(|error| {
            if error.kind() == io::ErrorKind::TimedOut {
                quarantine();
            }
            #[cfg(test)]
            eprintln!(
                "synthetic media fixture spawn error: {:?}",
                error.raw_os_error()
            );
            release();
            failure("recorder_media_worker_start_failed")
        })?;
    let generation = GENERATION.fetch_add(1, Ordering::SeqCst);
    let (sender, responses) = mpsc::sync_channel(64);
    thread::spawn(move || {
        let mut reader = reader;
        while let Ok(response) = protocol::receive(&mut reader) {
            if sender.send(response).is_err() {
                break;
            }
        }
    });
    let sources = input
        .video_sources
        .iter()
        .map(|s| s.source_id.clone())
        .collect::<Vec<_>>();
    let (transport_stop, stop) = mpsc::channel();
    let (finished, transport_done) = mpsc::channel();
    thread::spawn(move || {
        let mut transport = transport;
        let mut sequences = vec![0u64; sources.len().max(1)];
        let mut stopping = false;
        let result = (|| {
            loop {
                stopping |= stop.try_recv().is_ok();
                let frame = match frames.recv_timeout(if stopping {
                    Duration::ZERO
                } else {
                    Duration::from_millis(25)
                }) {
                    Ok(frame) => frame,
                    Err(mpsc::RecvTimeoutError::Timeout) if stopping => break,
                    Err(mpsc::RecvTimeoutError::Timeout) => continue,
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                };
                let source = if sources.is_empty() {
                    0
                } else {
                    sources
                        .iter()
                        .position(|id| id == &frame.source_id)
                        .ok_or_else(|| io::Error::other("Unknown media source."))?
                };
                sequences[source] += 1;
                protocol::write_frame(
                    &mut transport,
                    generation,
                    source as u32,
                    sequences[source],
                    frame,
                )?;
                if let Some(timing) = &timing {
                    super::super::capture_runtime::record_frame_sent_to_writer(timing);
                }
            }
            Ok(())
        })();
        let _ = transport.shutdown(Shutdown::Write);
        let _ = finished.send(result);
    });
    let mut process = MediaProcess {
        child: Some(child),
        control,
        frames: frame_stream,
        generation,
        responses,
        fragments: vec![],
        transport_stop,
        transport_done,
        error: None,
    };
    protocol::send(
        &mut process.control,
        &Request::Start {
            version: protocol::VERSION,
            generation,
            input: input.clone(),
        },
    )
    .map_err(|_| failure("recorder_media_worker_start_failed"))?;
    match process.responses.recv_timeout(Duration::from_secs(10)) {
        Ok(Response::Ready { generation: g }) if g == generation => {
            Ok(MediaWriterSegment::Process(process))
        }
        Ok(Response::Failed { error, .. }) => Err(error),
        _other => {
            #[cfg(test)]
            eprintln!("synthetic media fixture initial response: {_other:?}");
            Err(failure("recorder_media_worker_start_failed"))
        }
    }
}

impl MediaWriterSegment {
    pub(crate) fn apply_cutoff(&mut self, timestamp_ms: u64) -> Result<(), RecorderError> {
        match self {
            #[cfg(test)]
            Self::Local(writer) => writer.apply_cutoff(timestamp_ms),
            Self::Process(process) => protocol::send(
                &mut process.control,
                &Request::Cutoff {
                    generation: process.generation,
                    timestamp_ms,
                },
            )
            .map_err(|_| failure("recorder_media_worker_failed")),
        }
    }
    pub(crate) fn poll_error(&mut self) -> Option<RecorderError> {
        match self {
            #[cfg(test)]
            Self::Local(writer) => writer.poll_error(),
            Self::Process(process) => {
                process.collect();
                process.error.clone()
            }
        }
    }
    pub(crate) fn benchmark(&self) -> Option<RecorderBenchmarkSession> {
        match self {
            #[cfg(test)]
            Self::Local(writer) => writer.benchmark(),
            Self::Process(_) => None,
        }
    }
    pub(crate) fn drain_fragments(&mut self) -> Vec<FragmentEvent> {
        match self {
            #[cfg(test)]
            Self::Local(writer) => writer.drain_fragments(),
            Self::Process(process) => {
                process.collect();
                std::mem::take(&mut process.fragments)
            }
        }
    }
    pub(crate) fn stop(self, timestamp_ms: u64) -> Result<Vec<FragmentEvent>, RecorderError> {
        self.stop_until(timestamp_ms, Instant::now() + Duration::from_secs(20))
    }
    pub(crate) fn stop_until(
        self,
        timestamp_ms: u64,
        deadline: Instant,
    ) -> Result<Vec<FragmentEvent>, RecorderError> {
        self.stop_with_probe_until(timestamp_ms, deadline)
            .map(|media| media.events)
    }
    pub(crate) fn stop_with_probe_until(
        self,
        timestamp_ms: u64,
        deadline: Instant,
    ) -> Result<FinalizedSegmentMedia, RecorderError> {
        match self {
            #[cfg(test)]
            Self::Local(writer) => writer
                .stop(timestamp_ms)
                .map(|events| FinalizedSegmentMedia {
                    events,
                    probe: None,
                }),
            Self::Process(mut process) => process.stop(timestamp_ms, deadline),
        }
    }
    pub(crate) fn abort(self) {
        match self {
            #[cfg(test)]
            Self::Local(writer) => writer.abort(),
            Self::Process(mut process) => {
                let _ = protocol::send(
                    &mut process.control,
                    &Request::Abort {
                        generation: process.generation,
                    },
                );
                process.kill_and_reap(Instant::now() + Duration::from_secs(3));
            }
        }
    }
}
impl MediaProcess {
    fn collect(&mut self) {
        if !matches!(
            self.transport_done.try_recv(),
            Err(mpsc::TryRecvError::Empty)
        ) {
            self.error
                .get_or_insert_with(|| failure("recorder_media_transport_failed"));
        }
        while let Ok(response) = self.responses.try_recv() {
            match response {
                Response::Fragments { generation, events } if generation == self.generation => {
                    self.fragments.extend(events)
                }
                Response::Failed { generation, error } if generation == self.generation => {
                    self.error = Some(error)
                }
                _ => {}
            }
        }
        if self
            .child
            .as_mut()
            .is_some_and(|child| child.try_wait().ok().flatten().is_some())
        {
            self.error
                .get_or_insert_with(|| failure("recorder_media_worker_failed"));
        }
    }
    fn stop(
        &mut self,
        timestamp_ms: u64,
        deadline: Instant,
    ) -> Result<FinalizedSegmentMedia, RecorderError> {
        let normal = deadline - Duration::from_secs(5);
        let _ = self.transport_stop.send(());
        if !matches!(
            self.transport_done.recv_timeout(Duration::from_secs(2)),
            Ok(Ok(()))
        ) {
            self.kill_and_reap(deadline);
            return Err(failure("recorder_media_transport_failed"));
        }
        if protocol::send(
            &mut self.control,
            &Request::Stop {
                generation: self.generation,
                timestamp_ms,
            },
        )
        .is_err()
        {
            self.kill_and_reap(deadline);
            return Err(failure("recorder_media_worker_failed"));
        }
        let mut result = None;
        let mut terminal_probe = None;
        while let Ok(response) = self
            .responses
            .recv_timeout(normal.saturating_duration_since(Instant::now()))
        {
            match response {
                Response::Fragments { generation, events } if generation == self.generation => {
                    self.fragments.extend(events)
                }
                Response::Finished {
                    generation,
                    result: finished,
                    probe,
                } if generation == self.generation => {
                    result = Some(finished);
                    terminal_probe = probe;
                    break;
                }
                Response::Failed { generation, error } if generation == self.generation => {
                    result = Some(Err(error));
                    break;
                }
                _ => {}
            }
        }
        if result.is_none() {
            let _ = protocol::send(
                &mut self.control,
                &Request::Flush {
                    generation: self.generation,
                },
            );
            let flush = deadline - Duration::from_secs(3);
            while Instant::now() < flush {
                if let Ok(Response::Finished { .. }) = self
                    .responses
                    .recv_timeout(flush.saturating_duration_since(Instant::now()))
                {
                    break;
                }
            }
        }
        self.kill_and_reap(deadline);
        if is_quarantined() {
            return Err(failure("recorder_media_cleanup_pending"));
        }
        match result.unwrap_or_else(|| Err(failure("recorder_media_worker_stop_timeout"))) {
            Ok(events) => {
                self.fragments.extend(events);
                Ok(FinalizedSegmentMedia {
                    events: std::mem::take(&mut self.fragments),
                    probe: terminal_probe,
                })
            }
            Err(error) => Err(error),
        }
    }
    fn kill_and_reap(&mut self, deadline: Instant) {
        let _ = self.transport_stop.send(());
        let _ = self.frames.shutdown(Shutdown::Both);
        let _ = self.control.shutdown(Shutdown::Both);
        let Some(mut child) = self.child.take() else {
            return;
        };
        let _ = child.kill();
        loop {
            match child.try_wait() {
                Ok(Some(_)) => {
                    ACTIVE.fetch_sub(1, Ordering::SeqCst);
                    return;
                }
                _ => {
                    if Instant::now() >= deadline {
                        QUARANTINED.store(true, Ordering::SeqCst);
                        return;
                    }
                    thread::sleep(Duration::from_millis(10));
                }
            }
        }
    }
}
impl Drop for MediaProcess {
    fn drop(&mut self) {
        self.kill_and_reap(Instant::now() + Duration::from_secs(3));
    }
}
