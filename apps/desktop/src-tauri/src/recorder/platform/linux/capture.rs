#![allow(dead_code)]

use std::{
    cell::RefCell,
    collections::HashMap,
    os::fd::{AsFd, BorrowedFd, OwnedFd},
    rc::Rc,
    sync::{mpsc, Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use pipewire as pw;
use pw::{properties::properties, spa};
use tauri::{AppHandle, Emitter};
use tracing::{debug, info, warn};

use super::portal::{close_portal_session_blocking, LivePortalSession, LivePortalStream};
use crate::recorder::{
    benchmark::{RecorderBenchmarkSession, RecorderBenchmarkStage},
    capture_runtime::{
        bounded_frame_channel, new_capture_timing, record_capture_frame, record_compose_duration,
        record_composed_frame, record_dmabuf_frame_read_back_to_cpu, record_scheduler_tick,
        CapturePreviewSession, CaptureRuntimeSource, CaptureRuntimeSourceHandle, CaptureSession,
        CapturedDmaBufFrame, CapturedFrame, CapturedFrameFormat, CapturedFrameStorage, FrameSender,
        FrameTrySendError, RuntimeWorker, SharedCaptureTiming,
    },
    commands::RECORDER_PREVIEW_FRAME_EVENT,
    compositor::{SceneCompositorInput, SceneCompositorPlan},
    errors::{
        capture_adapter_failed_error, capture_format_unsupported_error,
        capture_source_missing_error,
    },
    manager::{now_ms, RecordingSegmentRuntime},
    media_writer::{self, MediaSegmentInput},
    platform::linux::gpu_compositor::LinuxGpuFrameCompositor,
    preview::encode_cropped_preview_frame,
    types::{
        CaptureFrameStorageKind, CaptureSourceKind, RecorderError, RecorderPreviewFrame,
        RecorderScene,
    },
};

const PIPEWIRE_READY_TIMEOUT: Duration = Duration::from_secs(5);
const PIPEWIRE_LATEST_FRAME_LOG_INTERVAL: Duration = Duration::from_secs(5);

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct LiveCapturePreflight {
    pub(crate) plan: SceneCompositorPlan,
    pub(crate) runtime_sources: Vec<CaptureRuntimeSource>,
}

pub(crate) struct PipeWireCaptureSession {
    scheduler_shutdown: mpsc::Sender<()>,
    scheduler_worker: RuntimeWorker,
    shutdown: pw::channel::Sender<PipeWireControl>,
    worker: RuntimeWorker,
}

pub(crate) struct PipeWirePreviewSession {
    shutdown: pw::channel::Sender<PipeWireControl>,
    worker: RuntimeWorker,
}

pub(crate) struct PreparedLiveCapture {
    live_session: LivePortalSession,
    preflight: LiveCapturePreflight,
}

#[derive(Debug)]
pub(crate) struct RunningPreviewSession {
    pub(crate) session: Box<dyn CapturePreviewSession>,
    pub(crate) source_ids: Vec<String>,
}

impl std::fmt::Debug for PipeWireCaptureSession {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PipeWireCaptureSession")
            .field("worker", &self.worker)
            .field("scheduler_worker", &self.scheduler_worker)
            .finish_non_exhaustive()
    }
}

impl std::fmt::Debug for PipeWirePreviewSession {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PipeWirePreviewSession")
            .field("worker", &self.worker)
            .finish_non_exhaustive()
    }
}

#[derive(Debug)]
enum PipeWireControl {
    Shutdown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PipeWireStreamConfig {
    node_id: u32,
    source_id: String,
}

#[derive(Debug)]
struct PipeWireStreamData {
    format: spa::param::video::VideoInfoRaw,
    logged_negotiated_format: bool,
    source_id: String,
}

struct PipeWireCaptureState {
    error: Option<RecorderError>,
    latest_frame_log: PipeWireLatestFrameLog,
    latest_frames: SharedPipeWireFrameSampler,
    timing: SharedCaptureTiming,
}

struct PipeWireLatestFrameLog {
    frames_since_last_log: u64,
    last_logged_at: Instant,
    latest_timestamps_by_source: HashMap<String, u64>,
}

struct PipeWireLatestFrameLogSnapshot {
    frames_since_last_log: u64,
    latest_timestamps_by_source: HashMap<String, u64>,
    window_ms: u128,
}

struct PipeWireScheduledFrameLog {
    frames_since_last_log: u64,
    last_logged_at: Instant,
    last_timestamp_ms: u64,
    max_compose_duration_ms: u64,
    reused_frames_since_last_log: u64,
    total_compose_duration_ms: u64,
}

struct PipeWireScheduledFrameLogSnapshot {
    average_compose_duration_ms: u64,
    frames_since_last_log: u64,
    last_timestamp_ms: u64,
    max_compose_duration_ms: u64,
    reused_frames_since_last_log: u64,
    window_ms: u128,
}

type SharedPipeWireFrameSampler = Arc<Mutex<LatestPipeWireFrameSampler>>;

struct PipeWireCaptureSchedulerState {
    benchmark: Option<RecorderBenchmarkSession>,
    composed_frames: FrameSender,
    frame_interval_ms: u64,
    gpu_compositor: Option<LinuxGpuFrameCompositor>,
    latest_frames: SharedPipeWireFrameSampler,
    next_composed_at_ms: Option<u64>,
    plan: SceneCompositorPlan,
    scheduled_frame_log: PipeWireScheduledFrameLog,
    timing: SharedCaptureTiming,
}

struct PipeWirePreviewState {
    app: AppHandle,
    error: Option<RecorderError>,
    inputs_by_source_id: HashMap<String, Vec<SceneCompositorInput>>,
    last_emitted_by_scene_source_id: HashMap<String, u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ComposedFrameSendOutcome {
    Sent,
    Stopped,
}

struct PipeWireStartupNotifier {
    sender: Option<mpsc::Sender<Result<(), RecorderError>>>,
}

enum PipeWireCapturedFrame {
    Cpu(CapturedFrame),
    DmaBuf(CapturedDmaBufFrame),
}

struct LatestPipeWireFrameSampler {
    frame_interval_ms: u64,
    last_composed_at_ms: Option<u64>,
    latest_frames: HashMap<String, PipeWireCapturedFrame>,
}

impl LatestPipeWireFrameSampler {
    fn new(fps: f64) -> Self {
        Self {
            frame_interval_ms: frame_interval_ms(fps),
            last_composed_at_ms: None,
            latest_frames: HashMap::new(),
        }
    }

    fn push(&mut self, frame: PipeWireCapturedFrame) -> Result<(), RecorderError> {
        match &frame {
            PipeWireCapturedFrame::Cpu(frame) => frame.validate()?,
            PipeWireCapturedFrame::DmaBuf(frame) => frame.validate()?,
        }
        self.latest_frames
            .insert(frame.source_id().to_string(), frame);

        Ok(())
    }

    fn should_compose(&self, now_ms: u64) -> bool {
        match self.last_composed_at_ms {
            None => !self.latest_frames.is_empty(),
            Some(last_composed_at_ms) => {
                now_ms.saturating_sub(last_composed_at_ms) >= self.frame_interval_ms
                    && !self.latest_frames.is_empty()
            }
        }
    }

    fn mark_composed(&mut self, timestamp_ms: u64) {
        self.last_composed_at_ms = Some(timestamp_ms);
    }

    fn frame_interval(&self) -> Duration {
        Duration::from_millis(self.frame_interval_ms)
    }

    fn latest_frames(&self) -> &HashMap<String, PipeWireCapturedFrame> {
        &self.latest_frames
    }

    fn has_frames(&self) -> bool {
        !self.latest_frames.is_empty()
    }

    #[cfg(test)]
    fn latest_frame_count(&self) -> usize {
        self.latest_frames.len()
    }

    #[cfg(test)]
    fn has_latest_frame(&self, source_id: &str) -> bool {
        self.latest_frames.contains_key(source_id)
    }
}

impl PipeWireLatestFrameLog {
    fn new(now: Instant) -> Self {
        Self {
            frames_since_last_log: 0,
            last_logged_at: now,
            latest_timestamps_by_source: HashMap::new(),
        }
    }

    fn record_frame(
        &mut self,
        source_id: &str,
        frame_timestamp_ms: u64,
        now: Instant,
    ) -> Option<PipeWireLatestFrameLogSnapshot> {
        self.frames_since_last_log += 1;
        self.latest_timestamps_by_source
            .insert(source_id.to_string(), frame_timestamp_ms);
        let window = now.duration_since(self.last_logged_at);
        if window < PIPEWIRE_LATEST_FRAME_LOG_INTERVAL {
            return None;
        }

        let snapshot = PipeWireLatestFrameLogSnapshot {
            frames_since_last_log: self.frames_since_last_log,
            latest_timestamps_by_source: std::mem::take(&mut self.latest_timestamps_by_source),
            window_ms: window.as_millis(),
        };
        self.frames_since_last_log = 0;
        self.last_logged_at = now;

        Some(snapshot)
    }
}

impl PipeWireScheduledFrameLog {
    fn new(now: Instant) -> Self {
        Self {
            frames_since_last_log: 0,
            last_logged_at: now,
            last_timestamp_ms: 0,
            max_compose_duration_ms: 0,
            reused_frames_since_last_log: 0,
            total_compose_duration_ms: 0,
        }
    }

    fn record_frame(
        &mut self,
        timestamp_ms: u64,
        reused_latest_frame: bool,
        compose_duration_ms: u64,
        now: Instant,
    ) -> Option<PipeWireScheduledFrameLogSnapshot> {
        self.frames_since_last_log += 1;
        self.last_timestamp_ms = timestamp_ms;
        if reused_latest_frame {
            self.reused_frames_since_last_log += 1;
        }
        self.total_compose_duration_ms = self
            .total_compose_duration_ms
            .saturating_add(compose_duration_ms);
        self.max_compose_duration_ms = self.max_compose_duration_ms.max(compose_duration_ms);

        let window = now.duration_since(self.last_logged_at);
        if window < PIPEWIRE_LATEST_FRAME_LOG_INTERVAL {
            return None;
        }

        let frames_since_last_log = self.frames_since_last_log.max(1);
        let snapshot = PipeWireScheduledFrameLogSnapshot {
            average_compose_duration_ms: self.total_compose_duration_ms / frames_since_last_log,
            frames_since_last_log,
            last_timestamp_ms: self.last_timestamp_ms,
            max_compose_duration_ms: self.max_compose_duration_ms,
            reused_frames_since_last_log: self.reused_frames_since_last_log,
            window_ms: window.as_millis(),
        };
        self.frames_since_last_log = 0;
        self.last_logged_at = now;
        self.max_compose_duration_ms = 0;
        self.reused_frames_since_last_log = 0;
        self.total_compose_duration_ms = 0;

        Some(snapshot)
    }
}

fn frame_interval_ms(fps: f64) -> u64 {
    if fps.is_finite() && fps > 0.0 {
        Duration::from_secs_f64(1.0 / fps).as_millis().max(1) as u64
    } else {
        1000
    }
}

impl PipeWireCapturedFrame {
    fn source_id(&self) -> &str {
        match self {
            Self::Cpu(frame) => &frame.source_id,
            Self::DmaBuf(frame) => &frame.source_id,
        }
    }

    fn timestamp_ms(&self) -> u64 {
        match self {
            Self::Cpu(frame) => frame.timestamp_ms,
            Self::DmaBuf(frame) => frame.timestamp_ms,
        }
    }

    fn format_label(&self) -> &'static str {
        match self {
            Self::Cpu(frame) => frame.format.label(),
            Self::DmaBuf(frame) => frame.format.label(),
        }
    }

    fn try_clone(&self) -> Result<Self, RecorderError> {
        match self {
            Self::Cpu(frame) => Ok(Self::Cpu(frame.clone())),
            Self::DmaBuf(frame) => Ok(Self::DmaBuf(CapturedDmaBufFrame::new(
                frame.source_id.clone(),
                frame.timestamp_ms,
                frame.width,
                frame.height,
                frame.stride,
                frame.format,
                frame.fourcc,
                frame.modifier,
                frame.offset,
                frame.fd.as_fd().try_clone_to_owned().map_err(|error| {
                    capture_adapter_failed_error(format!(
                        "Failed to duplicate DMA-BUF fd for capture source {}: {error}.",
                        frame.source_id
                    ))
                })?,
            )?)),
        }
    }
}

pub(crate) fn preflight_live_capture(
    scene: &RecorderScene,
    live_streams: &[LivePortalStream],
) -> Result<LiveCapturePreflight, RecorderError> {
    let runtime_sources = match runtime_sources_for_scene(scene, live_streams) {
        Ok(runtime_sources) => runtime_sources,
        Err(error) => {
            warn!(
                scene_id = %scene.id,
                scene_source_count = scene.sources.len(),
                scene_source_ids = %scene
                    .sources
                    .iter()
                    .map(|source| source.source_id.as_str())
                    .collect::<Vec<_>>()
                    .join(","),
                live_stream_count = live_streams.len(),
                live_stream_source_ids = %live_streams
                    .iter()
                    .map(|stream| stream.source_id.as_str())
                    .collect::<Vec<_>>()
                    .join(","),
                error_code = %error.code,
                "live_capture_preflight_source_mapping_failed"
            );
            return Err(error);
        }
    };
    let plan = SceneCompositorPlan::from_scene(scene, &runtime_sources)?;

    Ok(LiveCapturePreflight {
        plan,
        runtime_sources,
    })
}

pub(crate) fn prepared_live_capture(
    live_session: LivePortalSession,
    scene: &RecorderScene,
) -> Result<PreparedLiveCapture, RecorderError> {
    let preflight = match preflight_live_capture(scene, &live_session.streams) {
        Ok(preflight) => preflight,
        Err(error) => {
            let _ = live_session.close_blocking("preflight");
            return Err(error);
        }
    };

    Ok(PreparedLiveCapture {
        live_session,
        preflight,
    })
}

pub(crate) fn start_live_segment(
    input: &MediaSegmentInput,
    prepared: PreparedLiveCapture,
) -> Result<RecordingSegmentRuntime, RecorderError> {
    let (sender, receiver) = bounded_frame_channel(3);
    let timing = new_capture_timing();
    let media_writer =
        match media_writer::start_segment_from_frames(input, receiver, Some(timing.clone())) {
            Ok(media_writer) => media_writer,
            Err(error) => {
                let _ = prepared
                    .live_session
                    .close_blocking("recording-media-start");
                return Err(error);
            }
        };
    let benchmark = media_writer.benchmark();
    let capture = match start_pipewire_capture_session(
        prepared.live_session,
        prepared.preflight,
        input.fps,
        sender,
        timing.clone(),
        benchmark,
    ) {
        Ok(capture) => capture,
        Err(error) => {
            let _ = media_writer.stop(input.started_at_ms);
            return Err(error);
        }
    };

    Ok(RecordingSegmentRuntime::new(
        media_writer,
        Some(capture),
        Some(timing),
    ))
}

pub(crate) fn start_preview_session(
    app: AppHandle,
    prepared: PreparedLiveCapture,
) -> Result<RunningPreviewSession, RecorderError> {
    let source_ids = prepared
        .preflight
        .runtime_sources
        .iter()
        .map(|source| source.source_id.clone())
        .collect::<Vec<_>>();
    let stream_configs = match stream_configs_from_plan(&prepared.preflight.plan) {
        Ok(stream_configs) => stream_configs,
        Err(error) => {
            let _ = prepared.live_session.close_blocking("preview-config");
            return Err(error);
        }
    };
    let (shutdown, shutdown_receiver) = pw::channel::channel();
    let (ready_sender, ready_receiver) = mpsc::channel();
    let worker = RuntimeWorker::spawn("pipewire-preview", move || {
        run_pipewire_preview(
            app,
            prepared.live_session,
            prepared.preflight.plan,
            stream_configs,
            shutdown_receiver,
            ready_sender,
        )
    });

    match ready_receiver.recv_timeout(PIPEWIRE_READY_TIMEOUT) {
        Ok(Ok(())) => Ok(RunningPreviewSession {
            session: Box::new(PipeWirePreviewSession { shutdown, worker }),
            source_ids,
        }),
        Ok(Err(error)) => {
            let _ = worker.join();
            Err(error)
        }
        Err(_error) => {
            let _ = shutdown.send(PipeWireControl::Shutdown);
            let _ = worker.join();
            Err(capture_adapter_failed_error(
                "PipeWire preview did not become ready before the startup timeout.",
            ))
        }
    }
}

fn runtime_sources_for_scene(
    scene: &RecorderScene,
    live_streams: &[LivePortalStream],
) -> Result<Vec<CaptureRuntimeSource>, RecorderError> {
    let mut claimed_streams = vec![false; live_streams.len()];
    let mut runtime_sources = Vec::with_capacity(scene.sources.len());

    for (scene_index, scene_source) in scene.sources.iter().enumerate() {
        let stream_index = match_live_stream_for_scene_source(
            scene_source,
            scene_index,
            live_streams,
            &claimed_streams,
        )
        .ok_or_else(|| capture_source_missing_error(&scene_source.source_id))?;
        claimed_streams[stream_index] = true;
        let stream = &live_streams[stream_index];

        runtime_sources.push(CaptureRuntimeSource {
            display_name: stream.display_name.clone(),
            handle: CaptureRuntimeSourceHandle::BackendId(format!(
                "pipewire-node-{}",
                stream.node_id
            )),
            height: stream.logical_height,
            kind: CaptureSourceKind::Monitor,
            source_id: scene_source.source_id.clone(),
            width: stream.logical_width,
        });
    }

    Ok(runtime_sources)
}

fn match_live_stream_for_scene_source(
    scene_source: &crate::recorder::types::RecorderSceneSource,
    scene_index: usize,
    live_streams: &[LivePortalStream],
    claimed_streams: &[bool],
) -> Option<usize> {
    if let Some(index) = live_streams
        .iter()
        .enumerate()
        .find(|(index, stream)| {
            !claimed_streams[*index] && stream.source_id == scene_source.source_id
        })
        .map(|(index, _stream)| index)
    {
        return Some(index);
    }

    if let Some(index) =
        unique_live_stream_by_display_name(scene_source, live_streams, claimed_streams)
    {
        return Some(index);
    }

    if !is_volatile_pipewire_source_id(&scene_source.source_id) {
        return None;
    }

    live_streams
        .get(scene_index)
        .filter(|_stream| {
            claimed_streams
                .get(scene_index)
                .is_some_and(|claimed| !claimed)
        })
        .map(|_stream| scene_index)
        .or_else(|| claimed_streams.iter().position(|claimed| !claimed))
}

fn unique_live_stream_by_display_name(
    scene_source: &crate::recorder::types::RecorderSceneSource,
    live_streams: &[LivePortalStream],
    claimed_streams: &[bool],
) -> Option<usize> {
    let mut matches = live_streams
        .iter()
        .enumerate()
        .filter(|(index, stream)| {
            !claimed_streams[*index] && stream.display_name == scene_source.display_name
        })
        .map(|(index, _stream)| index);
    let first = matches.next()?;

    matches.next().is_none().then_some(first)
}

fn is_volatile_pipewire_source_id(source_id: &str) -> bool {
    source_id
        .strip_prefix("pipewire-node-")
        .and_then(|node_id| node_id.parse::<u32>().ok())
        .is_some()
}

pub(crate) fn start_pipewire_capture_session(
    live_session: LivePortalSession,
    preflight: LiveCapturePreflight,
    fps: f64,
    composed_frames: FrameSender,
    timing: SharedCaptureTiming,
    benchmark: Option<RecorderBenchmarkSession>,
) -> Result<Box<dyn CaptureSession>, RecorderError> {
    let stream_configs = match stream_configs_from_plan(&preflight.plan) {
        Ok(stream_configs) => stream_configs,
        Err(error) => {
            let _ = live_session.close_blocking("recording-config");
            return Err(error);
        }
    };
    let (shutdown, shutdown_receiver) = pw::channel::channel();
    let (scheduler_shutdown, scheduler_shutdown_receiver) = mpsc::channel();
    let (ready_sender, ready_receiver) = mpsc::channel();
    let latest_frames = Arc::new(Mutex::new(LatestPipeWireFrameSampler::new(fps)));
    let scheduler_worker = RuntimeWorker::spawn("pipewire-capture-scheduler", {
        let latest_frames = Arc::clone(&latest_frames);
        let timing = timing.clone();
        let plan = preflight.plan.clone();
        let benchmark = benchmark.clone();
        move || {
            run_pipewire_capture_scheduler(
                plan,
                fps,
                composed_frames,
                timing,
                latest_frames,
                benchmark,
                scheduler_shutdown_receiver,
            )
        }
    });
    let worker = RuntimeWorker::spawn("pipewire-capture", move || {
        run_pipewire_capture(
            live_session,
            fps,
            timing,
            latest_frames,
            stream_configs,
            shutdown_receiver,
            ready_sender,
        )
    });

    match ready_receiver.recv_timeout(PIPEWIRE_READY_TIMEOUT) {
        Ok(Ok(())) => Ok(Box::new(PipeWireCaptureSession {
            scheduler_shutdown,
            scheduler_worker,
            shutdown,
            worker,
        })),
        Ok(Err(error)) => {
            let _ = scheduler_shutdown.send(());
            let _ = worker.join();
            let _ = scheduler_worker.join();
            Err(error)
        }
        Err(_error) => {
            let _ = shutdown.send(PipeWireControl::Shutdown);
            let _ = scheduler_shutdown.send(());
            let _ = worker.join();
            let _ = scheduler_worker.join();
            Err(capture_adapter_failed_error(
                "PipeWire capture did not become ready before the startup timeout.",
            ))
        }
    }
}

impl CaptureSession for PipeWireCaptureSession {
    fn stop(self: Box<Self>, stop_at_ms: u64) -> Result<(), RecorderError> {
        info!(stop_at_ms, "pipewire_capture_scheduler_stop_requested");
        let _ = self.scheduler_shutdown.send(());
        let _ = self.shutdown.send(PipeWireControl::Shutdown);
        let pipewire_result = self.worker.join();
        let scheduler_result = self.scheduler_worker.join();
        info!(stop_at_ms, "pipewire_capture_scheduler_stopped");

        pipewire_result.and(scheduler_result)
    }
}

impl CapturePreviewSession for PipeWirePreviewSession {
    fn stop(self: Box<Self>) -> Result<(), RecorderError> {
        let _ = self.shutdown.send(PipeWireControl::Shutdown);
        self.worker.join()
    }
}

fn run_pipewire_capture(
    live_session: LivePortalSession,
    fps: f64,
    timing: SharedCaptureTiming,
    latest_frames: SharedPipeWireFrameSampler,
    stream_configs: Vec<PipeWireStreamConfig>,
    shutdown_receiver: pw::channel::Receiver<PipeWireControl>,
    ready_sender: mpsc::Sender<Result<(), RecorderError>>,
) -> Result<(), RecorderError> {
    let (fd, portal_session) = live_session.into_fd_and_session();
    let mut startup = PipeWireStartupNotifier::new(ready_sender);
    let result = run_pipewire_capture_inner(
        fd,
        fps,
        timing,
        latest_frames,
        stream_configs,
        shutdown_receiver,
        &mut startup,
    );
    if let Err(error) = &result {
        startup.error(error);
    }
    let close_result = close_portal_session_blocking(portal_session, "recording");

    result.and(close_result)
}

fn run_pipewire_capture_inner(
    fd: OwnedFd,
    fps: f64,
    timing: SharedCaptureTiming,
    latest_frames: SharedPipeWireFrameSampler,
    stream_configs: Vec<PipeWireStreamConfig>,
    shutdown_receiver: pw::channel::Receiver<PipeWireControl>,
    startup: &mut PipeWireStartupNotifier,
) -> Result<(), RecorderError> {
    pw::init();

    let mainloop = pw::main_loop::MainLoopRc::new(None).map_err(pipewire_error)?;
    let _shutdown_receiver = shutdown_receiver.attach(mainloop.loop_(), {
        let mainloop = mainloop.clone();
        move |_message| mainloop.quit()
    });
    let context = pw::context::ContextRc::new(&mainloop, None).map_err(pipewire_error)?;
    let core = context.connect_fd_rc(fd, None).map_err(pipewire_error)?;
    let state = Rc::new(RefCell::new(PipeWireCaptureState {
        error: None,
        latest_frame_log: PipeWireLatestFrameLog::new(Instant::now()),
        latest_frames,
        timing,
    }));
    let frame_interval = Duration::from_millis(frame_interval_ms(fps));
    info!(
        fps,
        frame_interval_ms = frame_interval.as_millis(),
        source_count = stream_configs.len(),
        "pipewire_capture_scheduler_started"
    );
    let mut streams = Vec::with_capacity(stream_configs.len());
    let mut listeners = Vec::with_capacity(stream_configs.len());

    for stream_config in stream_configs {
        let stream = pw::stream::StreamBox::new(
            &core,
            "daydream-screen-capture",
            properties! {
                *pw::keys::MEDIA_TYPE => "Video",
                *pw::keys::MEDIA_CATEGORY => "Capture",
                *pw::keys::MEDIA_ROLE => "Screen",
            },
        )
        .map_err(pipewire_error)?;
        let data = PipeWireStreamData {
            format: Default::default(),
            logged_negotiated_format: false,
            source_id: stream_config.source_id.clone(),
        };
        let state_for_process = state.clone();
        let mainloop_for_process = mainloop.clone();
        let listener = stream
            .add_local_listener_with_user_data(data)
            .param_changed(|_, user_data, id, param| {
                let Some(param) = param else {
                    return;
                };
                if id != pw::spa::param::ParamType::Format.as_raw() {
                    return;
                }

                let Ok((media_type, media_subtype)) =
                    pw::spa::param::format_utils::parse_format(param)
                else {
                    return;
                };
                if media_type != pw::spa::param::format::MediaType::Video
                    || media_subtype != pw::spa::param::format::MediaSubtype::Raw
                {
                    return;
                }

                let _ = user_data.format.parse(param);
            })
            .process(move |stream, user_data| {
                let result = capture_stream_frame(stream, user_data, &state_for_process);
                if let Err(error) = result {
                    let mut state = state_for_process.borrow_mut();
                    if state.error.is_none() {
                        state.error = Some(error);
                    }
                    mainloop_for_process.quit();
                }
            })
            .register()
            .map_err(pipewire_error)?;

        let param_bytes = raw_video_params(stream_config.node_id, fps)?;
        let pod = spa::pod::Pod::from_bytes(&param_bytes).ok_or_else(|| {
            capture_adapter_failed_error(format!(
                "Failed to parse PipeWire raw video parameters for node {}.",
                stream_config.node_id
            ))
        })?;
        let mut params = [pod];
        stream
            .connect(
                spa::utils::Direction::Input,
                Some(stream_config.node_id),
                pw::stream::StreamFlags::AUTOCONNECT | pw::stream::StreamFlags::MAP_BUFFERS,
                &mut params,
            )
            .map_err(pipewire_error)?;
        listeners.push(listener);
        streams.push(stream);
    }

    startup.ready();
    mainloop.run();

    drop(listeners);
    drop(streams);

    if let Some(error) = state.borrow().error.clone() {
        return Err(error);
    }

    Ok(())
}

fn run_pipewire_preview(
    app: AppHandle,
    live_session: LivePortalSession,
    plan: SceneCompositorPlan,
    stream_configs: Vec<PipeWireStreamConfig>,
    shutdown_receiver: pw::channel::Receiver<PipeWireControl>,
    ready_sender: mpsc::Sender<Result<(), RecorderError>>,
) -> Result<(), RecorderError> {
    let (fd, portal_session) = live_session.into_fd_and_session();
    let mut startup = PipeWireStartupNotifier::new(ready_sender);
    let result = run_pipewire_preview_inner(
        app,
        fd,
        plan,
        stream_configs,
        shutdown_receiver,
        &mut startup,
    );
    if let Err(error) = &result {
        startup.error(error);
    }
    let close_result = close_portal_session_blocking(portal_session, "preview");

    result.and(close_result)
}

fn run_pipewire_preview_inner(
    app: AppHandle,
    fd: OwnedFd,
    plan: SceneCompositorPlan,
    stream_configs: Vec<PipeWireStreamConfig>,
    shutdown_receiver: pw::channel::Receiver<PipeWireControl>,
    startup: &mut PipeWireStartupNotifier,
) -> Result<(), RecorderError> {
    pw::init();

    let mainloop = pw::main_loop::MainLoopRc::new(None).map_err(pipewire_error)?;
    let _shutdown_receiver = shutdown_receiver.attach(mainloop.loop_(), {
        let mainloop = mainloop.clone();
        move |_message| mainloop.quit()
    });
    let context = pw::context::ContextRc::new(&mainloop, None).map_err(pipewire_error)?;
    let core = context.connect_fd_rc(fd, None).map_err(pipewire_error)?;
    let state = Rc::new(RefCell::new(PipeWirePreviewState {
        app,
        error: None,
        inputs_by_source_id: inputs_by_source_id(&plan),
        last_emitted_by_scene_source_id: HashMap::new(),
    }));
    let mut streams = Vec::with_capacity(stream_configs.len());
    let mut listeners = Vec::with_capacity(stream_configs.len());

    for stream_config in stream_configs {
        let stream = pw::stream::StreamBox::new(
            &core,
            "daydream-screen-preview",
            properties! {
                *pw::keys::MEDIA_TYPE => "Video",
                *pw::keys::MEDIA_CATEGORY => "Capture",
                *pw::keys::MEDIA_ROLE => "Screen",
            },
        )
        .map_err(pipewire_error)?;
        let data = PipeWireStreamData {
            format: Default::default(),
            logged_negotiated_format: false,
            source_id: stream_config.source_id.clone(),
        };
        let state_for_process = state.clone();
        let mainloop_for_process = mainloop.clone();
        let listener = stream
            .add_local_listener_with_user_data(data)
            .param_changed(|_, user_data, id, param| {
                let Some(param) = param else {
                    return;
                };
                if id != pw::spa::param::ParamType::Format.as_raw() {
                    return;
                }

                let Ok((media_type, media_subtype)) =
                    pw::spa::param::format_utils::parse_format(param)
                else {
                    return;
                };
                if media_type != pw::spa::param::format::MediaType::Video
                    || media_subtype != pw::spa::param::format::MediaSubtype::Raw
                {
                    return;
                }

                let _ = user_data.format.parse(param);
            })
            .process(move |stream, user_data| {
                let result = capture_preview_stream_frame(stream, user_data, &state_for_process);
                if let Err(error) = result {
                    let mut state = state_for_process.borrow_mut();
                    if state.error.is_none() {
                        state.error = Some(error);
                    }
                    mainloop_for_process.quit();
                }
            })
            .register()
            .map_err(pipewire_error)?;

        let param_bytes = raw_video_params(stream_config.node_id, 1.0)?;
        let pod = spa::pod::Pod::from_bytes(&param_bytes).ok_or_else(|| {
            capture_adapter_failed_error(format!(
                "Failed to parse PipeWire raw video parameters for node {}.",
                stream_config.node_id
            ))
        })?;
        let mut params = [pod];
        stream
            .connect(
                spa::utils::Direction::Input,
                Some(stream_config.node_id),
                pw::stream::StreamFlags::AUTOCONNECT | pw::stream::StreamFlags::MAP_BUFFERS,
                &mut params,
            )
            .map_err(pipewire_error)?;
        listeners.push(listener);
        streams.push(stream);
    }

    startup.ready();
    mainloop.run();

    drop(listeners);
    drop(streams);

    if let Some(error) = state.borrow().error.clone() {
        return Err(error);
    }

    Ok(())
}

impl PipeWireStartupNotifier {
    fn new(sender: mpsc::Sender<Result<(), RecorderError>>) -> Self {
        Self {
            sender: Some(sender),
        }
    }

    fn ready(&mut self) {
        if let Some(sender) = self.sender.take() {
            let _ = sender.send(Ok(()));
        }
    }

    fn error(&mut self, error: &RecorderError) {
        if let Some(sender) = self.sender.take() {
            let _ = sender.send(Err(error.clone()));
        }
    }
}

fn stream_configs_from_plan(
    plan: &SceneCompositorPlan,
) -> Result<Vec<PipeWireStreamConfig>, RecorderError> {
    plan.inputs
        .iter()
        .map(stream_config_from_input)
        .collect::<Result<Vec<_>, _>>()
}

fn stream_config_from_input(
    input: &SceneCompositorInput,
) -> Result<PipeWireStreamConfig, RecorderError> {
    let CaptureRuntimeSourceHandle::BackendId(handle) = &input.runtime_handle;
    let Some(node_id) = handle.strip_prefix("pipewire-node-") else {
        return Err(capture_source_missing_error(&input.source_id));
    };
    let node_id = node_id
        .parse::<u32>()
        .map_err(|_error| capture_source_missing_error(&input.source_id))?;

    Ok(PipeWireStreamConfig {
        node_id,
        source_id: input.source_id.clone(),
    })
}

fn capture_stream_frame(
    stream: &pw::stream::Stream,
    user_data: &mut PipeWireStreamData,
    state: &Rc<RefCell<PipeWireCaptureState>>,
) -> Result<(), RecorderError> {
    let Some(mut buffer) = stream.dequeue_buffer() else {
        return Ok(());
    };
    let datas = buffer.datas_mut();
    if datas.is_empty() {
        return Ok(());
    }

    log_negotiated_stream_format(user_data, &datas[0]);
    let frame =
        captured_pipewire_frame_from_data(&user_data.source_id, user_data.format, &mut datas[0])?;
    let (frame_source_id, frame_storage, frame_timestamp_ms) = match &frame {
        PipeWireCapturedFrame::Cpu(frame) => (
            frame.source_id.clone(),
            CaptureFrameStorageKind::CpuReadable,
            frame.timestamp_ms,
        ),
        PipeWireCapturedFrame::DmaBuf(frame) => (
            frame.source_id.clone(),
            CaptureFrameStorageKind::DmaBuf,
            frame.timestamp_ms,
        ),
    };
    let mut state = state.borrow_mut();
    record_capture_frame(
        &state.timing,
        &frame_source_id,
        frame_storage,
        frame_timestamp_ms,
    );
    state
        .latest_frames
        .lock()
        .map_err(|_error| capture_adapter_failed_error("PipeWire capture sampler lock poisoned."))?
        .push(frame)?;
    if let Some(log) =
        state
            .latest_frame_log
            .record_frame(&frame_source_id, frame_timestamp_ms, Instant::now())
    {
        debug!(
            frames_since_last_log = log.frames_since_last_log,
            source_count = log.latest_timestamps_by_source.len(),
            latest_timestamps_by_source = ?log.latest_timestamps_by_source,
            window_ms = log.window_ms,
            "pipewire_capture_latest_frame_updated"
        );
    }

    Ok(())
}

fn log_negotiated_stream_format(user_data: &mut PipeWireStreamData, data: &spa::buffer::Data) {
    if user_data.logged_negotiated_format {
        return;
    }
    let size = user_data.format.size();
    let Ok(frame_format) = captured_frame_format_from_pipewire(user_data.format.format()) else {
        return;
    };
    let stride = data.chunk().stride();
    let storage = storage_from_pipewire_data_type(data.type_());

    info!(
        source_id = %user_data.source_id,
        pipewire_format = ?user_data.format.format(),
        runtime_format = %frame_format.label(),
        storage = ?storage,
        width = size.width,
        height = size.height,
        stride,
        "pipewire_capture_stream_format_negotiated"
    );
    user_data.logged_negotiated_format = true;
}

fn run_pipewire_capture_scheduler(
    plan: SceneCompositorPlan,
    fps: f64,
    composed_frames: FrameSender,
    timing: SharedCaptureTiming,
    latest_frames: SharedPipeWireFrameSampler,
    benchmark: Option<RecorderBenchmarkSession>,
    shutdown_receiver: mpsc::Receiver<()>,
) -> Result<(), RecorderError> {
    let frame_interval_ms = frame_interval_ms(fps);
    let mut state = PipeWireCaptureSchedulerState {
        benchmark,
        composed_frames,
        frame_interval_ms,
        gpu_compositor: None,
        latest_frames,
        next_composed_at_ms: None,
        plan,
        scheduled_frame_log: PipeWireScheduledFrameLog::new(Instant::now()),
        timing,
    };
    let idle_sleep = Duration::from_millis(frame_interval_ms.clamp(1, 50));

    loop {
        if scheduler_shutdown_requested(&shutdown_receiver) {
            return Ok(());
        }

        let now = now_ms();
        let Some(timestamp_ms) = next_scheduler_timestamp(&mut state, now)? else {
            match shutdown_receiver.recv_timeout(idle_sleep) {
                Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => return Ok(()),
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
            }
        };
        if scheduler_shutdown_requested(&shutdown_receiver) {
            return Ok(());
        }
        if compose_latest_pipewire_frames(&mut state, timestamp_ms, &shutdown_receiver)?
            == ComposedFrameSendOutcome::Stopped
        {
            return Ok(());
        }
    }
}

fn scheduler_shutdown_requested(shutdown_receiver: &mpsc::Receiver<()>) -> bool {
    matches!(
        shutdown_receiver.try_recv(),
        Ok(()) | Err(mpsc::TryRecvError::Disconnected)
    )
}

fn next_scheduler_timestamp(
    state: &mut PipeWireCaptureSchedulerState,
    now_ms: u64,
) -> Result<Option<u64>, RecorderError> {
    let has_frames = state
        .latest_frames
        .lock()
        .map_err(|_error| capture_adapter_failed_error("PipeWire capture sampler lock poisoned."))?
        .has_frames();
    if !has_frames {
        return Ok(None);
    }

    let Some(next_composed_at_ms) = state.next_composed_at_ms else {
        state.next_composed_at_ms = Some(now_ms.saturating_add(state.frame_interval_ms));
        record_scheduler_tick(&state.timing, 0);
        return Ok(Some(now_ms));
    };
    if now_ms < next_composed_at_ms {
        return Ok(None);
    }

    let late_by_ms = now_ms.saturating_sub(next_composed_at_ms);
    let missed_deadlines = if late_by_ms > state.frame_interval_ms {
        late_by_ms / state.frame_interval_ms
    } else {
        0
    };
    let timestamp_ms = if late_by_ms > state.frame_interval_ms {
        now_ms
    } else {
        next_composed_at_ms
    };
    state.next_composed_at_ms = Some(timestamp_ms.saturating_add(state.frame_interval_ms));
    record_scheduler_tick(&state.timing, missed_deadlines);

    Ok(Some(timestamp_ms))
}

fn compose_latest_pipewire_frames(
    state: &mut PipeWireCaptureSchedulerState,
    timestamp_ms: u64,
    shutdown_receiver: &mpsc::Receiver<()>,
) -> Result<ComposedFrameSendOutcome, RecorderError> {
    if scheduler_shutdown_requested(shutdown_receiver) {
        return Ok(ComposedFrameSendOutcome::Stopped);
    }
    let snapshot_started = Instant::now();
    let latest_snapshot = latest_pipewire_frame_snapshot(&state.latest_frames)?;
    if scheduler_shutdown_requested(shutdown_receiver) {
        return Ok(ComposedFrameSendOutcome::Stopped);
    }
    if let Some(benchmark) = &state.benchmark {
        benchmark.record_stage(
            RecorderBenchmarkStage::Snapshot,
            snapshot_started.elapsed(),
            Some(timestamp_ms),
            Some(latest_snapshot.len()),
            latest_snapshot
                .values()
                .map(|frame| frame.format_label().to_string())
                .collect(),
            None,
        );
    }
    let reused_latest_frame = latest_snapshot
        .values()
        .any(|frame| frame.timestamp_ms() < timestamp_ms);
    let compose_started = Instant::now();
    let materialize_started = Instant::now();
    let latest_frames =
        materialized_latest_frames(&mut state.gpu_compositor, &state.timing, &latest_snapshot)?;
    if scheduler_shutdown_requested(shutdown_receiver) {
        return Ok(ComposedFrameSendOutcome::Stopped);
    }
    if let Some(benchmark) = &state.benchmark {
        benchmark.record_stage(
            RecorderBenchmarkStage::Materialize,
            materialize_started.elapsed(),
            Some(timestamp_ms),
            Some(latest_frames.len()),
            latest_frames
                .values()
                .map(|frame| frame.format.label().to_string())
                .collect(),
            Some(
                latest_frames
                    .values()
                    .map(|frame| frame.data.len() as u64)
                    .sum(),
            ),
        );
    }
    let compose_stage = if state.plan.is_full_canvas_passthrough(&latest_frames) {
        RecorderBenchmarkStage::Passthrough
    } else {
        RecorderBenchmarkStage::Compose
    };
    let compose_only_started = Instant::now();
    let composed = state
        .plan
        .compose_latest_frames(&latest_frames, timestamp_ms)?;
    if scheduler_shutdown_requested(shutdown_receiver) {
        return Ok(ComposedFrameSendOutcome::Stopped);
    }
    if let Some(benchmark) = &state.benchmark {
        benchmark.record_stage(
            compose_stage,
            compose_only_started.elapsed(),
            Some(timestamp_ms),
            Some(latest_frames.len()),
            vec![composed.format.label().to_string()],
            Some(composed.data.len().try_into().unwrap_or(u64::MAX)),
        );
    }
    state.plan.validate_composed_frame(&composed)?;
    let send_started = Instant::now();
    let send_outcome = send_composed_frame_with_backpressure(
        &state.composed_frames,
        composed,
        writer_backpressure_timeout(state.frame_interval_ms),
        shutdown_receiver,
    )?;
    if send_outcome == ComposedFrameSendOutcome::Stopped {
        return Ok(ComposedFrameSendOutcome::Stopped);
    }
    if let Some(benchmark) = &state.benchmark {
        benchmark.record_stage(
            RecorderBenchmarkStage::SendWait,
            send_started.elapsed(),
            Some(timestamp_ms),
            None,
            Vec::new(),
            None,
        );
    }
    let compose_duration_ms = compose_started
        .elapsed()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX);
    record_composed_frame(&state.timing, timestamp_ms, reused_latest_frame);
    record_compose_duration(&state.timing, compose_duration_ms);
    if let Some(log) = state.scheduled_frame_log.record_frame(
        timestamp_ms,
        reused_latest_frame,
        compose_duration_ms,
        Instant::now(),
    ) {
        debug!(
            average_compose_duration_ms = log.average_compose_duration_ms,
            frames_since_last_log = log.frames_since_last_log,
            last_timestamp_ms = log.last_timestamp_ms,
            max_compose_duration_ms = log.max_compose_duration_ms,
            reused_frames_since_last_log = log.reused_frames_since_last_log,
            window_ms = log.window_ms,
            "pipewire_capture_scheduled_frame_composed"
        );
    }

    Ok(ComposedFrameSendOutcome::Sent)
}

fn materialized_latest_frames(
    gpu_compositor: &mut Option<LinuxGpuFrameCompositor>,
    timing: &SharedCaptureTiming,
    latest_frames: &HashMap<String, PipeWireCapturedFrame>,
) -> Result<HashMap<String, CapturedFrame>, RecorderError> {
    if latest_frames
        .values()
        .any(|frame| matches!(frame, PipeWireCapturedFrame::DmaBuf(_)))
        && gpu_compositor.is_none()
    {
        *gpu_compositor = Some(LinuxGpuFrameCompositor::new()?);
    }

    let mut frames = HashMap::with_capacity(latest_frames.len());
    for (source_id, frame) in latest_frames {
        let frame = match frame {
            PipeWireCapturedFrame::Cpu(frame) => frame.clone(),
            PipeWireCapturedFrame::DmaBuf(frame) => {
                let frame = gpu_compositor
                    .as_ref()
                    .expect("GPU compositor initialized")
                    .read_frame_to_rgba(frame)?;
                record_dmabuf_frame_read_back_to_cpu(timing);
                frame
            }
        };
        frames.insert(source_id.clone(), frame);
    }

    Ok(frames)
}

fn latest_pipewire_frame_snapshot(
    latest_frames: &SharedPipeWireFrameSampler,
) -> Result<HashMap<String, PipeWireCapturedFrame>, RecorderError> {
    latest_frames
        .lock()
        .map_err(|_error| capture_adapter_failed_error("PipeWire capture sampler lock poisoned."))?
        .latest_frames()
        .iter()
        .map(|(source_id, frame)| Ok((source_id.clone(), frame.try_clone()?)))
        .collect()
}

fn send_composed_frame_with_backpressure(
    sender: &FrameSender,
    frame: CapturedFrame,
    timeout: Duration,
    shutdown_receiver: &mpsc::Receiver<()>,
) -> Result<ComposedFrameSendOutcome, RecorderError> {
    let started = Instant::now();
    let mut frame = frame;
    loop {
        if scheduler_shutdown_requested(shutdown_receiver) {
            return Ok(ComposedFrameSendOutcome::Stopped);
        }
        match sender.try_send(frame) {
            Ok(()) => return Ok(ComposedFrameSendOutcome::Sent),
            Err(FrameTrySendError::Full(returned_frame)) => {
                if started.elapsed() >= timeout {
                    return Err(capture_adapter_failed_error(format!(
                        "Timed out sending composed frame to the media writer after {}ms.",
                        timeout.as_millis()
                    )));
                }
                if scheduler_shutdown_requested(shutdown_receiver) {
                    return Ok(ComposedFrameSendOutcome::Stopped);
                }
                frame = returned_frame;
                thread::sleep(Duration::from_millis(5));
            }
            Err(FrameTrySendError::Disconnected) => {
                return Err(capture_adapter_failed_error(
                    "Capture frame receiver is no longer available.",
                ));
            }
            Err(FrameTrySendError::InvalidFrame(error)) => return Err(error),
        }
    }
}

fn writer_backpressure_timeout(frame_interval_ms: u64) -> Duration {
    Duration::from_millis(frame_interval_ms.saturating_mul(5).max(1_000))
}

fn capture_preview_stream_frame(
    stream: &pw::stream::Stream,
    user_data: &mut PipeWireStreamData,
    state: &Rc<RefCell<PipeWirePreviewState>>,
) -> Result<(), RecorderError> {
    let Some(mut buffer) = stream.dequeue_buffer() else {
        return Ok(());
    };
    let datas = buffer.datas_mut();
    if datas.is_empty() {
        return Ok(());
    }

    let frame =
        captured_frame_from_pipewire_data(&user_data.source_id, user_data.format, &mut datas[0])?;
    let timestamp_ms = now_ms();
    let mut state = state.borrow_mut();
    let Some(inputs) = state.inputs_by_source_id.get(&frame.source_id).cloned() else {
        return Ok(());
    };

    for input in inputs {
        let last_emitted_at_ms = state
            .last_emitted_by_scene_source_id
            .get(&input.scene_source_id)
            .copied();
        if last_emitted_at_ms.is_some_and(|last| timestamp_ms.saturating_sub(last) < 1000) {
            continue;
        }

        let encoded = encode_cropped_preview_frame(&frame, &input.crop)?;
        let payload = RecorderPreviewFrame {
            data_url: encoded.data_url,
            height: encoded.height,
            scene_source_id: input.scene_source_id.clone(),
            source_id: input.source_id.clone(),
            timestamp_ms,
            width: encoded.width,
        };
        state
            .app
            .emit(RECORDER_PREVIEW_FRAME_EVENT, payload)
            .map_err(|error| {
                capture_adapter_failed_error(format!(
                    "Failed to emit recorder preview frame for source {}: {error}.",
                    input.source_id
                ))
            })?;
        state
            .last_emitted_by_scene_source_id
            .insert(input.scene_source_id, timestamp_ms);
    }

    Ok(())
}

fn inputs_by_source_id(plan: &SceneCompositorPlan) -> HashMap<String, Vec<SceneCompositorInput>> {
    let mut inputs_by_source_id = HashMap::<String, Vec<SceneCompositorInput>>::new();
    for input in &plan.inputs {
        inputs_by_source_id
            .entry(input.source_id.clone())
            .or_default()
            .push(input.clone());
    }

    inputs_by_source_id
}

fn captured_frame_from_pipewire_data(
    source_id: &str,
    format: spa::param::video::VideoInfoRaw,
    data: &mut spa::buffer::Data,
) -> Result<CapturedFrame, RecorderError> {
    match captured_pipewire_frame_from_data(source_id, format, data)? {
        PipeWireCapturedFrame::Cpu(frame) => Ok(frame),
        PipeWireCapturedFrame::DmaBuf(_frame) => Err(CapturedFrameStorage::DmaBuf
            .ensure_cpu_readable(source_id)
            .expect_err("DMA-BUF preview is not CPU-readable")),
    }
}

fn captured_pipewire_frame_from_data(
    source_id: &str,
    format: spa::param::video::VideoInfoRaw,
    data: &mut spa::buffer::Data,
) -> Result<PipeWireCapturedFrame, RecorderError> {
    let size = format.size();
    let frame_format = captured_frame_format_from_pipewire(format.format())?;
    let chunk = data.chunk();
    let stride: u32 = chunk.stride().try_into().map_err(|_error| {
        capture_format_unsupported_error(format!(
            "Capture source {source_id} reported invalid PipeWire stride {}.",
            chunk.stride()
        ))
    })?;
    let offset: usize = chunk.offset().try_into().map_err(|_error| {
        capture_format_unsupported_error(format!(
            "Capture source {source_id} reported invalid PipeWire frame offset."
        ))
    })?;
    let len: usize = chunk.size().try_into().map_err(|_error| {
        capture_format_unsupported_error(format!(
            "Capture source {source_id} reported invalid PipeWire frame byte size."
        ))
    })?;
    let storage = storage_from_pipewire_data_type(data.type_());
    match storage {
        CapturedFrameStorage::CpuReadable => {
            let mapped_bytes = data.data();
            let bytes =
                cpu_readable_pipewire_data_bytes(source_id, &storage, mapped_bytes.as_deref())?;
            captured_frame_from_cpu_readable_pipewire_bytes(
                source_id,
                storage,
                now_ms(),
                size.width,
                size.height,
                stride,
                frame_format,
                offset,
                len,
                bytes,
            )
            .map(PipeWireCapturedFrame::Cpu)
        }
        CapturedFrameStorage::DmaBuf => captured_dmabuf_frame_from_pipewire_data(
            source_id,
            now_ms(),
            size.width,
            size.height,
            stride,
            frame_format,
            offset,
            format.modifier(),
            data,
        )
        .map(PipeWireCapturedFrame::DmaBuf),
        CapturedFrameStorage::Unknown(_) => Err(storage
            .ensure_cpu_readable(source_id)
            .expect_err("unsupported storage should fail")),
    }
}

fn cpu_readable_pipewire_data_bytes<'a>(
    source_id: &str,
    storage: &CapturedFrameStorage,
    bytes: Option<&'a [u8]>,
) -> Result<&'a [u8], RecorderError> {
    storage.ensure_cpu_readable(source_id)?;
    bytes.ok_or_else(|| {
        capture_format_unsupported_error(format!(
            "Capture source {source_id} did not expose CPU-readable PipeWire frame bytes."
        ))
    })
}

#[allow(clippy::too_many_arguments)]
fn captured_dmabuf_frame_from_pipewire_data(
    source_id: &str,
    timestamp_ms: u64,
    width: u32,
    height: u32,
    stride: u32,
    frame_format: CapturedFrameFormat,
    offset: usize,
    modifier: u64,
    data: &spa::buffer::Data,
) -> Result<CapturedDmaBufFrame, RecorderError> {
    let fd = data.fd();
    if fd < 0 {
        return Err(capture_format_unsupported_error(format!(
            "Capture source {source_id} reported an invalid DMA-BUF fd."
        )));
    }
    let fd = unsafe { BorrowedFd::borrow_raw(fd) }
        .try_clone_to_owned()
        .map_err(|error| {
            capture_adapter_failed_error(format!(
                "Failed to duplicate DMA-BUF fd for capture source {source_id}: {error}."
            ))
        })?;
    let offset: u32 = offset.try_into().map_err(|_error| {
        capture_format_unsupported_error(format!(
            "Capture source {source_id} reported invalid DMA-BUF frame offset."
        ))
    })?;
    let fourcc = drm_fourcc_from_frame_format(frame_format)?;

    CapturedDmaBufFrame::new(
        source_id,
        timestamp_ms,
        width,
        height,
        stride,
        frame_format,
        fourcc,
        modifier,
        offset,
        fd,
    )
}

#[allow(clippy::too_many_arguments)]
fn captured_frame_from_cpu_readable_pipewire_bytes(
    source_id: &str,
    storage: CapturedFrameStorage,
    timestamp_ms: u64,
    width: u32,
    height: u32,
    stride: u32,
    frame_format: CapturedFrameFormat,
    offset: usize,
    len: usize,
    bytes: &[u8],
) -> Result<CapturedFrame, RecorderError> {
    storage.ensure_cpu_readable(source_id)?;

    let end = offset.checked_add(len).ok_or_else(|| {
        capture_format_unsupported_error(format!(
            "Capture source {source_id} reported overflowing PipeWire frame bounds."
        ))
    })?;
    let frame_bytes = bytes.get(offset..end).ok_or_else(|| {
        capture_format_unsupported_error(format!(
            "Capture source {source_id} reported PipeWire frame bounds outside mapped memory."
        ))
    })?;

    CapturedFrame::new(
        source_id,
        timestamp_ms,
        width,
        height,
        stride,
        frame_format,
        frame_bytes.to_vec(),
    )
}

const fn fourcc_code(a: u8, b: u8, c: u8, d: u8) -> u32 {
    a as u32 | ((b as u32) << 8) | ((c as u32) << 16) | ((d as u32) << 24)
}

fn drm_fourcc_from_frame_format(format: CapturedFrameFormat) -> Result<u32, RecorderError> {
    match format {
        CapturedFrameFormat::Rgbx => Ok(fourcc_code(b'R', b'X', b'2', b'4')),
        CapturedFrameFormat::Rgba => Ok(fourcc_code(b'R', b'A', b'2', b'4')),
        CapturedFrameFormat::Xrgb => Ok(fourcc_code(b'X', b'R', b'2', b'4')),
        CapturedFrameFormat::Argb => Ok(fourcc_code(b'A', b'R', b'2', b'4')),
        CapturedFrameFormat::Bgrx => Ok(fourcc_code(b'B', b'X', b'2', b'4')),
        CapturedFrameFormat::Bgra => Ok(fourcc_code(b'B', b'A', b'2', b'4')),
        CapturedFrameFormat::Yuv444p => Err(capture_format_unsupported_error(
            "Planar yuv444p DMA-BUF import is not supported.",
        )),
        CapturedFrameFormat::Yuv420p => Err(capture_format_unsupported_error(
            "Planar yuv420p DMA-BUF import is not supported.",
        )),
    }
}

fn storage_from_pipewire_data_type(data_type: spa::buffer::DataType) -> CapturedFrameStorage {
    if data_type == spa::buffer::DataType::MemPtr || data_type == spa::buffer::DataType::MemFd {
        CapturedFrameStorage::CpuReadable
    } else if data_type == spa::buffer::DataType::DmaBuf {
        CapturedFrameStorage::DmaBuf
    } else {
        CapturedFrameStorage::Unknown(format!("{data_type:?}"))
    }
}

fn captured_frame_format_from_pipewire(
    format: spa::param::video::VideoFormat,
) -> Result<CapturedFrameFormat, RecorderError> {
    if format == spa::param::video::VideoFormat::RGBx {
        Ok(CapturedFrameFormat::Rgbx)
    } else if format == spa::param::video::VideoFormat::RGBA {
        Ok(CapturedFrameFormat::Rgba)
    } else if format == spa::param::video::VideoFormat::xRGB {
        Ok(CapturedFrameFormat::Xrgb)
    } else if format == spa::param::video::VideoFormat::ARGB {
        Ok(CapturedFrameFormat::Argb)
    } else if format == spa::param::video::VideoFormat::BGRx {
        Ok(CapturedFrameFormat::Bgrx)
    } else if format == spa::param::video::VideoFormat::BGRA {
        Ok(CapturedFrameFormat::Bgra)
    } else if format == spa::param::video::VideoFormat::Y444 {
        Ok(CapturedFrameFormat::Yuv444p)
    } else if format == spa::param::video::VideoFormat::I420 {
        Ok(CapturedFrameFormat::Yuv420p)
    } else {
        Err(capture_format_unsupported_error(format!(
            "PipeWire frame format {format:?} is not supported."
        )))
    }
}

fn raw_video_params(node_id: u32, fps: f64) -> Result<Vec<u8>, RecorderError> {
    let framerate = pipewire_framerate_fraction(fps);
    let obj = pw::spa::pod::object!(
        pw::spa::utils::SpaTypes::ObjectParamFormat,
        pw::spa::param::ParamType::EnumFormat,
        pw::spa::pod::property!(
            pw::spa::param::format::FormatProperties::MediaType,
            Id,
            pw::spa::param::format::MediaType::Video
        ),
        pw::spa::pod::property!(
            pw::spa::param::format::FormatProperties::MediaSubtype,
            Id,
            pw::spa::param::format::MediaSubtype::Raw
        ),
        pw::spa::pod::property!(
            pw::spa::param::format::FormatProperties::VideoFormat,
            Choice,
            Enum,
            Id,
            pw::spa::param::video::VideoFormat::RGBx,
            pw::spa::param::video::VideoFormat::RGBx,
            pw::spa::param::video::VideoFormat::RGBA,
            pw::spa::param::video::VideoFormat::xRGB,
            pw::spa::param::video::VideoFormat::ARGB,
            pw::spa::param::video::VideoFormat::BGRx,
            pw::spa::param::video::VideoFormat::BGRA,
            pw::spa::param::video::VideoFormat::I420,
            pw::spa::param::video::VideoFormat::Y444,
        ),
        pw::spa::pod::property!(
            pw::spa::param::format::FormatProperties::VideoSize,
            Choice,
            Range,
            Rectangle,
            pw::spa::utils::Rectangle {
                width: 1920,
                height: 1080
            },
            pw::spa::utils::Rectangle {
                width: 1,
                height: 1
            },
            pw::spa::utils::Rectangle {
                width: 8192,
                height: 8192
            }
        ),
        pw::spa::pod::property!(
            pw::spa::param::format::FormatProperties::VideoFramerate,
            Choice,
            Range,
            Fraction,
            framerate,
            pw::spa::utils::Fraction { num: 0, denom: 1 },
            pw::spa::utils::Fraction { num: 60, denom: 1 }
        ),
    );

    pw::spa::pod::serialize::PodSerializer::serialize(
        std::io::Cursor::new(Vec::new()),
        &pw::spa::pod::Value::Object(obj),
    )
    .map(|serialized| serialized.0.into_inner())
    .map_err(|error| {
        capture_adapter_failed_error(format!(
            "Failed to build PipeWire raw video format parameters for node {node_id}: {error}."
        ))
    })
}

fn pipewire_framerate_fraction(fps: f64) -> pw::spa::utils::Fraction {
    if !fps.is_finite() || fps <= 0.0 {
        return pw::spa::utils::Fraction { num: 1, denom: 1 };
    }
    if fps.fract() == 0.0 {
        return pw::spa::utils::Fraction {
            num: fps.round().clamp(1.0, 60.0) as u32,
            denom: 1,
        };
    }
    if (fps - 0.5).abs() < f64::EPSILON {
        return pw::spa::utils::Fraction { num: 1, denom: 2 };
    }
    if (fps - 0.2).abs() < f64::EPSILON {
        return pw::spa::utils::Fraction { num: 1, denom: 5 };
    }
    if (fps - 0.1).abs() < f64::EPSILON {
        return pw::spa::utils::Fraction { num: 1, denom: 10 };
    }

    pw::spa::utils::Fraction {
        num: (fps * 1000.0).round().clamp(1.0, 60_000.0) as u32,
        denom: 1000,
    }
}

fn pipewire_error(error: pw::Error) -> RecorderError {
    capture_adapter_failed_error(format!("PipeWire capture adapter failed: {error}."))
}

#[cfg(test)]
mod tests {
    use std::{
        fs::File,
        sync::{mpsc, Arc, Mutex},
        time::{Duration, Instant},
    };

    use pipewire as pw;
    use pw::spa::{
        pod::{deserialize::PodDeserializer, ChoiceValue, Value},
        utils::ChoiceEnum,
    };
    use uuid::Uuid;

    use super::{
        captured_frame_format_from_pipewire, captured_frame_from_cpu_readable_pipewire_bytes,
        compose_latest_pipewire_frames, cpu_readable_pipewire_data_bytes,
        drm_fourcc_from_frame_format, next_scheduler_timestamp, pipewire_framerate_fraction,
        preflight_live_capture, raw_video_params, stream_configs_from_plan,
        ComposedFrameSendOutcome, LatestPipeWireFrameSampler, PipeWireCaptureSchedulerState,
        PipeWireCapturedFrame, PipeWireLatestFrameLog, PipeWireScheduledFrameLog,
        PipeWireStartupNotifier,
    };
    use crate::recorder::{
        capture_runtime::{
            bounded_frame_channel, capture_timing_snapshot, new_capture_timing,
            CaptureRuntimeSourceHandle, CapturedDmaBufFrame, CapturedFrame, CapturedFrameFormat,
            CapturedFrameStorage,
        },
        errors::capture_adapter_failed_error,
        platform::linux::portal::LivePortalStream,
        types::{
            CanvasRect, CaptureSourceKind, CropRect, RecorderScene, RecorderSceneLayoutMode,
            RecorderSceneSource,
        },
    };

    #[test]
    fn pipewire_framerate_fraction_tracks_configured_fps() {
        assert_eq!(
            pipewire_framerate_fraction(1.0),
            pw::spa::utils::Fraction { num: 1, denom: 1 }
        );
        assert_eq!(
            pipewire_framerate_fraction(5.0),
            pw::spa::utils::Fraction { num: 5, denom: 1 }
        );
        assert_eq!(
            pipewire_framerate_fraction(0.5),
            pw::spa::utils::Fraction { num: 1, denom: 2 }
        );
        assert_eq!(
            pipewire_framerate_fraction(0.2),
            pw::spa::utils::Fraction { num: 1, denom: 5 }
        );
        assert_eq!(
            pipewire_framerate_fraction(0.1),
            pw::spa::utils::Fraction { num: 1, denom: 10 }
        );
        assert_eq!(
            pipewire_framerate_fraction(0.0),
            pw::spa::utils::Fraction { num: 1, denom: 1 }
        );
        assert_eq!(
            pipewire_framerate_fraction(-5.0),
            pw::spa::utils::Fraction { num: 1, denom: 1 }
        );
        assert_eq!(
            pipewire_framerate_fraction(f64::NAN),
            pw::spa::utils::Fraction { num: 1, denom: 1 }
        );
    }

    #[test]
    fn raw_video_params_use_configured_fps_as_default_pipewire_framerate() {
        let params = raw_video_params(42, 5.0).expect("raw video params");
        let framerate = raw_video_params_default_framerate(&params)
            .expect("video framerate property should be present");

        assert_eq!(framerate, pw::spa::utils::Fraction { num: 5, denom: 1 });
    }

    #[test]
    fn raw_video_params_prefer_dmabuf_importable_packed_rgb() {
        let params = raw_video_params(42, 5.0).expect("raw video params");
        let format =
            raw_video_params_default_format(&params).expect("video format property should exist");

        assert_eq!(format, pw::spa::param::video::VideoFormat::RGBx.as_raw());
    }

    fn raw_video_params_default_framerate(params: &[u8]) -> Option<pw::spa::utils::Fraction> {
        let (_, value) = PodDeserializer::deserialize_any_from(params).ok()?;
        let Value::Object(object) = value else {
            return None;
        };

        object.properties.into_iter().find_map(|property| {
            if property.key != pw::spa::param::format::FormatProperties::VideoFramerate.as_raw() {
                return None;
            }

            let Value::Choice(ChoiceValue::Fraction(choice)) = property.value else {
                return None;
            };

            match choice.1 {
                ChoiceEnum::Range { default, .. } => Some(default),
                _ => None,
            }
        })
    }

    fn raw_video_params_default_format(params: &[u8]) -> Option<u32> {
        let (_, value) = PodDeserializer::deserialize_any_from(params).ok()?;
        let Value::Object(object) = value else {
            return None;
        };

        object.properties.into_iter().find_map(|property| {
            if property.key != pw::spa::param::format::FormatProperties::VideoFormat.as_raw() {
                return None;
            }

            let Value::Choice(ChoiceValue::Id(choice)) = property.value else {
                return None;
            };

            match choice.1 {
                ChoiceEnum::Enum { default, .. } => Some(default.0),
                _ => None,
            }
        })
    }

    #[test]
    fn preflight_maps_stable_portal_ids_to_runtime_sources() {
        let scene = test_scene(vec![
            scene_source("front", "portal-monitor-b", 20),
            scene_source("back", "portal-monitor-a", 10),
        ]);
        let preflight = preflight_live_capture(
            &scene,
            &[
                live_stream("portal-monitor-a", 7),
                live_stream("portal-monitor-b", 8),
            ],
        )
        .expect("preflight live capture");

        assert_eq!(preflight.runtime_sources.len(), 2);
        assert_eq!(preflight.runtime_sources[0].source_id, "portal-monitor-b");
        assert_eq!(preflight.runtime_sources[1].source_id, "portal-monitor-a");
        assert_eq!(preflight.plan.inputs[0].source_id, "portal-monitor-a");
        assert_eq!(preflight.plan.inputs[1].source_id, "portal-monitor-b");
    }

    #[test]
    fn preflight_rejects_missing_stable_portal_id_before_recording_rows() {
        let scene = test_scene(vec![scene_source("missing", "portal-monitor-missing", 0)]);
        let error = preflight_live_capture(&scene, &[live_stream("portal-monitor-a", 7)])
            .expect_err("missing stream should fail");

        assert_eq!(error.code, "recorder_capture_source_missing");
        assert!(error.message.contains("portal-monitor-missing"));
    }

    #[test]
    fn preflight_remaps_changed_portal_id_by_unique_display_name() {
        let scene = test_scene(vec![scene_source_with_display(
            "saved-monitor",
            "portal-monitor-old",
            0,
            "Monitor 1",
        )]);
        let preflight = preflight_live_capture(
            &scene,
            &[live_stream_with_display(
                "portal-monitor-new",
                101,
                "Monitor 1",
            )],
        )
        .expect("changed portal id should remap to live stream");

        assert_eq!(preflight.runtime_sources[0].source_id, "portal-monitor-old");
        assert_eq!(
            preflight.runtime_sources[0].handle,
            CaptureRuntimeSourceHandle::BackendId("pipewire-node-101".to_string())
        );
        assert_eq!(preflight.plan.inputs[0].source_id, "portal-monitor-old");
    }

    #[test]
    fn preflight_rejects_ambiguous_changed_portal_display_name() {
        let scene = test_scene(vec![scene_source_with_display(
            "saved-monitor",
            "portal-monitor-old",
            0,
            "Monitor",
        )]);
        let error = preflight_live_capture(
            &scene,
            &[
                live_stream_with_display("portal-monitor-new-a", 101, "Monitor"),
                live_stream_with_display("portal-monitor-new-b", 102, "Monitor"),
            ],
        )
        .expect_err("ambiguous changed portal id should fail");

        assert_eq!(error.code, "recorder_capture_source_missing");
        assert!(error.message.contains("portal-monitor-old"));
    }

    #[test]
    fn preflight_maps_volatile_pipewire_ids_by_display_name() {
        let scene = test_scene(vec![scene_source_with_display(
            "saved-monitor",
            "pipewire-node-100",
            0,
            "Monitor 1",
        )]);
        let preflight = preflight_live_capture(
            &scene,
            &[live_stream_with_display(
                "pipewire-node-101",
                101,
                "Monitor 1",
            )],
        )
        .expect("volatile pipewire node should remap to live stream");

        assert_eq!(preflight.runtime_sources[0].source_id, "pipewire-node-100");
        assert_eq!(
            preflight.runtime_sources[0].handle,
            CaptureRuntimeSourceHandle::BackendId("pipewire-node-101".to_string())
        );
        assert_eq!(preflight.plan.inputs[0].source_id, "pipewire-node-100");
    }

    #[test]
    fn preflight_maps_volatile_pipewire_ids_by_scene_order() {
        let scene = test_scene(vec![
            scene_source_with_display("left", "pipewire-node-100", 0, "Saved left"),
            scene_source_with_display("right", "pipewire-node-200", 1, "Saved right"),
        ]);
        let preflight = preflight_live_capture(
            &scene,
            &[
                live_stream_with_display("pipewire-node-8", 8, "Live first"),
                live_stream_with_display("pipewire-node-9", 9, "Live second"),
            ],
        )
        .expect("volatile pipewire nodes should remap by order");

        assert_eq!(preflight.runtime_sources[0].source_id, "pipewire-node-100");
        assert_eq!(
            preflight.runtime_sources[0].handle,
            CaptureRuntimeSourceHandle::BackendId("pipewire-node-8".to_string())
        );
        assert_eq!(preflight.runtime_sources[1].source_id, "pipewire-node-200");
        assert_eq!(
            preflight.runtime_sources[1].handle,
            CaptureRuntimeSourceHandle::BackendId("pipewire-node-9".to_string())
        );
    }

    #[test]
    fn pipewire_stream_configs_use_backend_node_handles() {
        let scene = test_scene(vec![scene_source("front", "portal-monitor-a", 0)]);
        let preflight = preflight_live_capture(&scene, &[live_stream("portal-monitor-a", 42)])
            .expect("preflight live capture");
        let configs = stream_configs_from_plan(&preflight.plan).expect("stream configs");

        assert_eq!(configs[0].node_id, 42);
        assert_eq!(configs[0].source_id, "portal-monitor-a");
    }

    #[test]
    fn mixed_cpu_and_dmabuf_pipewire_frames_share_latest_sampler() {
        let mut sampler = LatestPipeWireFrameSampler::new(1.0);

        sampler
            .push(PipeWireCapturedFrame::Cpu(cpu_frame("cpu-monitor", 100)))
            .expect("push CPU frame");
        assert!(sampler.should_compose(100));
        sampler.mark_composed(100);

        sampler
            .push(PipeWireCapturedFrame::DmaBuf(dmabuf_frame(
                "dmabuf-monitor",
                200,
            )))
            .expect("push DMA-BUF frame");

        assert_eq!(sampler.latest_frame_count(), 2);
        assert!(sampler.has_latest_frame("cpu-monitor"));
        assert!(sampler.has_latest_frame("dmabuf-monitor"));
        assert!(!sampler.should_compose(999));
        assert!(sampler.should_compose(1100));
    }

    #[test]
    fn latest_frame_log_aggregates_until_five_second_window_elapses() {
        let started_at = Instant::now();
        let mut log = PipeWireLatestFrameLog::new(started_at);

        assert!(log
            .record_frame("portal-monitor-a", 100, started_at + Duration::from_secs(1))
            .is_none());
        assert!(log
            .record_frame("portal-monitor-a", 200, started_at + Duration::from_secs(2))
            .is_none());
        log.record_frame("portal-monitor-b", 300, started_at + Duration::from_secs(5))
            .map(|snapshot| {
                assert_eq!(snapshot.frames_since_last_log, 3);
                assert_eq!(snapshot.window_ms, 5_000);
                assert_eq!(
                    snapshot.latest_timestamps_by_source.get("portal-monitor-a"),
                    Some(&200),
                );
                assert_eq!(
                    snapshot.latest_timestamps_by_source.get("portal-monitor-b"),
                    Some(&300),
                );
            })
            .expect("five second window should emit summary");

        assert!(log
            .record_frame("portal-monitor-a", 400, started_at + Duration::from_secs(6))
            .is_none());
    }

    #[test]
    fn scheduled_frame_log_aggregates_until_five_second_window_elapses() {
        let started_at = Instant::now();
        let mut log = PipeWireScheduledFrameLog::new(started_at);

        assert!(log
            .record_frame(100, false, 10, started_at + Duration::from_secs(1))
            .is_none());
        assert!(log
            .record_frame(200, true, 20, started_at + Duration::from_secs(2))
            .is_none());
        log.record_frame(300, true, 40, started_at + Duration::from_secs(5))
            .map(|snapshot| {
                assert_eq!(snapshot.frames_since_last_log, 3);
                assert_eq!(snapshot.reused_frames_since_last_log, 2);
                assert_eq!(snapshot.average_compose_duration_ms, 23);
                assert_eq!(snapshot.max_compose_duration_ms, 40);
                assert_eq!(snapshot.last_timestamp_ms, 300);
                assert_eq!(snapshot.window_ms, 5_000);
            })
            .expect("five second window should emit summary");

        assert!(log
            .record_frame(400, false, 50, started_at + Duration::from_secs(6))
            .is_none());
    }

    #[test]
    fn capture_scheduler_can_compose_reused_frame_without_new_pipewire_callback() {
        let scene = test_scene(vec![scene_source("front", "portal-monitor-a", 0)]);
        let preflight = preflight_live_capture(&scene, &[live_stream("portal-monitor-a", 42)])
            .expect("preflight live capture");
        let (sender, receiver) = bounded_frame_channel(3);
        let timing = new_capture_timing();
        let latest_frames = Arc::new(Mutex::new(LatestPipeWireFrameSampler::new(5.0)));
        let mut state = PipeWireCaptureSchedulerState {
            benchmark: None,
            composed_frames: sender,
            gpu_compositor: None,
            frame_interval_ms: 200,
            latest_frames: Arc::clone(&latest_frames),
            next_composed_at_ms: None,
            plan: preflight.plan,
            scheduled_frame_log: PipeWireScheduledFrameLog::new(Instant::now()),
            timing: timing.clone(),
        };
        let (_shutdown_sender, shutdown_receiver) = mpsc::channel();

        latest_frames
            .lock()
            .expect("sampler lock")
            .push(PipeWireCapturedFrame::Cpu(sized_cpu_frame(
                "portal-monitor-a",
                100,
                100,
                50,
            )))
            .expect("push first source frame");
        compose_latest_pipewire_frames(&mut state, 100, &shutdown_receiver)
            .expect("compose first frame");
        compose_latest_pipewire_frames(&mut state, 300, &shutdown_receiver)
            .expect("compose reused frame");

        let first = receiver
            .recv_timeout(Duration::from_millis(10))
            .expect("first composed frame");
        let second = receiver
            .recv_timeout(Duration::from_millis(10))
            .expect("second composed frame");
        let snapshot = capture_timing_snapshot(&timing);

        assert_eq!(first.timestamp_ms, 100);
        assert_eq!(second.timestamp_ms, 300);
        assert_eq!(snapshot.composed_frames, 2);
        assert_eq!(snapshot.reused_frame_compositions, 1);
    }

    #[test]
    fn capture_scheduler_shutdown_does_not_compose_or_record_frame() {
        let scene = test_scene(vec![scene_source("front", "portal-monitor-a", 0)]);
        let preflight = preflight_live_capture(&scene, &[live_stream("portal-monitor-a", 42)])
            .expect("preflight live capture");
        let (sender, receiver) = bounded_frame_channel(3);
        let timing = new_capture_timing();
        let latest_frames = Arc::new(Mutex::new(LatestPipeWireFrameSampler::new(5.0)));
        let mut state = PipeWireCaptureSchedulerState {
            benchmark: None,
            composed_frames: sender,
            gpu_compositor: None,
            frame_interval_ms: 200,
            latest_frames: Arc::clone(&latest_frames),
            next_composed_at_ms: None,
            plan: preflight.plan,
            scheduled_frame_log: PipeWireScheduledFrameLog::new(Instant::now()),
            timing: timing.clone(),
        };
        let (shutdown_sender, shutdown_receiver) = mpsc::channel();

        latest_frames
            .lock()
            .expect("sampler lock")
            .push(PipeWireCapturedFrame::Cpu(sized_cpu_frame(
                "portal-monitor-a",
                100,
                100,
                50,
            )))
            .expect("push first source frame");
        shutdown_sender.send(()).expect("request shutdown");

        let outcome = compose_latest_pipewire_frames(&mut state, 100, &shutdown_receiver)
            .expect("compose should observe shutdown");
        let snapshot = capture_timing_snapshot(&timing);

        assert_eq!(outcome, ComposedFrameSendOutcome::Stopped);
        assert!(receiver.recv_timeout(Duration::from_millis(10)).is_err());
        assert_eq!(snapshot.composed_frames, 0);
        assert_eq!(snapshot.frames_sent_to_writer, 0);
    }

    #[test]
    fn capture_scheduler_timestamps_reused_frames_at_configured_fps() {
        let scene = test_scene(vec![scene_source("front", "portal-monitor-a", 0)]);
        let preflight = preflight_live_capture(&scene, &[live_stream("portal-monitor-a", 42)])
            .expect("preflight live capture");
        let (sender, receiver) = bounded_frame_channel(8);
        let latest_frames = Arc::new(Mutex::new(LatestPipeWireFrameSampler::new(5.0)));
        latest_frames
            .lock()
            .expect("sampler lock")
            .push(PipeWireCapturedFrame::Cpu(sized_cpu_frame(
                "portal-monitor-a",
                1_000,
                100,
                50,
            )))
            .expect("push source frame");
        let timing = new_capture_timing();
        let mut state = PipeWireCaptureSchedulerState {
            benchmark: None,
            composed_frames: sender,
            frame_interval_ms: 200,
            gpu_compositor: None,
            latest_frames,
            next_composed_at_ms: None,
            plan: preflight.plan,
            scheduled_frame_log: PipeWireScheduledFrameLog::new(Instant::now()),
            timing: timing.clone(),
        };
        let (_shutdown_sender, shutdown_receiver) = mpsc::channel();

        for now_ms in [1_000, 1_200, 1_400, 1_600, 1_800] {
            let timestamp_ms =
                next_scheduler_timestamp(&mut state, now_ms).expect("scheduler timestamp");
            compose_latest_pipewire_frames(
                &mut state,
                timestamp_ms.expect("due frame"),
                &shutdown_receiver,
            )
            .expect("compose scheduled frame");
        }

        let timestamps = (0..5)
            .map(|_index| {
                receiver
                    .recv_timeout(Duration::from_millis(10))
                    .expect("scheduled frame")
                    .timestamp_ms
            })
            .collect::<Vec<_>>();
        let snapshot = capture_timing_snapshot(&timing);

        assert_eq!(timestamps, vec![1_000, 1_200, 1_400, 1_600, 1_800]);
        assert_eq!(snapshot.composed_frames, 5);
        assert_eq!(snapshot.reused_frame_compositions, 4);
    }

    #[test]
    fn pipewire_video_formats_map_to_runtime_formats() {
        assert_eq!(
            captured_frame_format_from_pipewire(pipewire::spa::param::video::VideoFormat::RGBx)
                .expect("rgbx"),
            crate::recorder::capture_runtime::CapturedFrameFormat::Rgbx
        );
        assert_eq!(
            captured_frame_format_from_pipewire(pipewire::spa::param::video::VideoFormat::BGRA)
                .expect("bgra"),
            crate::recorder::capture_runtime::CapturedFrameFormat::Bgra
        );
        assert_eq!(
            captured_frame_format_from_pipewire(pipewire::spa::param::video::VideoFormat::Y444)
                .expect("yuv444p"),
            crate::recorder::capture_runtime::CapturedFrameFormat::Yuv444p
        );
        assert_eq!(
            captured_frame_format_from_pipewire(pipewire::spa::param::video::VideoFormat::I420)
                .expect("i420"),
            crate::recorder::capture_runtime::CapturedFrameFormat::Yuv420p
        );
    }

    #[test]
    fn pipewire_cpu_readable_frame_copies_owned_bytes() {
        let mut mapped_bytes = vec![9, 8, 7, 6, 5, 4, 3, 2];
        let frame = captured_frame_from_cpu_readable_pipewire_bytes(
            "portal-monitor-a",
            CapturedFrameStorage::CpuReadable,
            1,
            1,
            1,
            4,
            CapturedFrameFormat::Rgba,
            2,
            4,
            &mapped_bytes,
        )
        .expect("captured frame");

        mapped_bytes[2..6].copy_from_slice(&[0, 0, 0, 0]);

        assert_eq!(frame.data, vec![7, 6, 5, 4]);
    }

    #[test]
    fn pipewire_dmabuf_frame_reports_gate_h2_requirement() {
        let error = captured_frame_from_cpu_readable_pipewire_bytes(
            "portal-monitor-a",
            CapturedFrameStorage::DmaBuf,
            1,
            1,
            1,
            4,
            CapturedFrameFormat::Rgba,
            0,
            4,
            &[1, 2, 3, 4],
        )
        .expect_err("dmabuf should not be CPU-readable");

        assert_eq!(error.code, "recorder_capture_dmabuf_required");
        assert!(!format!("{error:?}").contains("[1, 2, 3, 4]"));
    }

    #[test]
    fn pipewire_dmabuf_without_mapped_bytes_reports_gate_h2_requirement() {
        let error = cpu_readable_pipewire_data_bytes(
            "portal-monitor-a",
            &CapturedFrameStorage::DmaBuf,
            None,
        )
        .expect_err("dmabuf should not request CPU-readable bytes");

        assert_eq!(error.code, "recorder_capture_dmabuf_required");
        assert!(!error.message.contains("CPU-readable PipeWire frame bytes"));
    }

    #[test]
    fn dmabuf_fourcc_mapping_tracks_negotiated_pipewire_format() {
        assert_eq!(
            drm_fourcc_from_frame_format(CapturedFrameFormat::Rgbx).expect("rgbx fourcc"),
            u32::from_le_bytes(*b"RX24")
        );
        assert_eq!(
            drm_fourcc_from_frame_format(CapturedFrameFormat::Bgra).expect("bgra fourcc"),
            u32::from_le_bytes(*b"BA24")
        );
        let error = drm_fourcc_from_frame_format(CapturedFrameFormat::Yuv444p)
            .expect_err("planar yuv DMA-BUF import should stay unsupported");

        assert_eq!(error.code, "recorder_capture_format_unsupported");
    }

    #[test]
    fn pipewire_startup_notifier_sends_ready_once() {
        let (sender, receiver) = mpsc::channel();
        let mut notifier = PipeWireStartupNotifier::new(sender);

        notifier.ready();
        notifier.error(&capture_adapter_failed_error(
            "late capture failure should not change startup readiness",
        ));

        assert!(receiver
            .recv_timeout(Duration::from_millis(10))
            .unwrap()
            .is_ok());
        assert!(receiver.recv_timeout(Duration::from_millis(10)).is_err());
    }

    #[test]
    fn pipewire_startup_notifier_sends_setup_error_before_ready() {
        let (sender, receiver) = mpsc::channel();
        let mut notifier = PipeWireStartupNotifier::new(sender);

        notifier.error(&capture_adapter_failed_error("PipeWire setup failed."));

        let error = receiver
            .recv_timeout(Duration::from_millis(10))
            .expect("startup result")
            .expect_err("setup should fail");
        assert_eq!(error.code, "recorder_capture_adapter_failed");
    }

    fn live_stream(source_id: &str, node_id: u32) -> LivePortalStream {
        live_stream_with_display(source_id, node_id, source_id)
    }

    fn live_stream_with_display(
        source_id: &str,
        node_id: u32,
        display_name: &str,
    ) -> LivePortalStream {
        LivePortalStream {
            display_name: display_name.to_string(),
            logical_height: Some(50),
            logical_width: Some(100),
            node_id,
            source_id: source_id.to_string(),
        }
    }

    fn test_scene(sources: Vec<RecorderSceneSource>) -> RecorderScene {
        RecorderScene {
            canvas_height: 50,
            canvas_width: 100,
            desktop_audio_source_id: None,
            id: Uuid::now_v7().to_string(),
            layout_mode: RecorderSceneLayoutMode::Custom,
            microphone_audio_source_id: None,
            name: "Test scene".to_string(),
            sources,
        }
    }

    fn scene_source(id: &str, source_id: &str, z_index: i32) -> RecorderSceneSource {
        scene_source_with_display(id, source_id, z_index, id)
    }

    fn scene_source_with_display(
        id: &str,
        source_id: &str,
        z_index: i32,
        display_name: &str,
    ) -> RecorderSceneSource {
        RecorderSceneSource {
            crop: CropRect {
                bottom: 0.0,
                left: 0.0,
                right: 0.0,
                top: 0.0,
            },
            display_name: display_name.to_string(),
            id: id.to_string(),
            preserve_aspect_ratio: true,
            rect: CanvasRect {
                height: 50.0,
                width: 100.0,
                x: 0.0,
                y: 0.0,
            },
            source_height: Some(50.0),
            source_id: source_id.to_string(),
            source_kind: CaptureSourceKind::Monitor,
            source_width: Some(100.0),
            z_index,
        }
    }

    fn cpu_frame(source_id: &str, timestamp_ms: u64) -> CapturedFrame {
        sized_cpu_frame(source_id, timestamp_ms, 1, 1)
    }

    fn sized_cpu_frame(
        source_id: &str,
        timestamp_ms: u64,
        width: u32,
        height: u32,
    ) -> CapturedFrame {
        let pixel_count = width as usize * height as usize;
        let mut data = Vec::with_capacity(pixel_count * 4);
        for _ in 0..pixel_count {
            data.extend_from_slice(&[255, 0, 0, 255]);
        }

        CapturedFrame::new(
            source_id,
            timestamp_ms,
            width,
            height,
            width * 4,
            CapturedFrameFormat::Rgba,
            data,
        )
        .expect("valid CPU frame")
    }

    fn dmabuf_frame(source_id: &str, timestamp_ms: u64) -> CapturedDmaBufFrame {
        let fd = File::open("/dev/null").expect("open fd for DMA-BUF sampler test");

        CapturedDmaBufFrame::new(
            source_id,
            timestamp_ms,
            1,
            1,
            4,
            CapturedFrameFormat::Rgbx,
            drm_fourcc_from_frame_format(CapturedFrameFormat::Rgbx).expect("rgbx fourcc"),
            0,
            0,
            fd.into(),
        )
        .expect("valid DMA-BUF metadata")
    }
}
