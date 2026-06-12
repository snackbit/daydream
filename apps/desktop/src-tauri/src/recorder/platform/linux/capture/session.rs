use super::*;

struct PipeWireCaptureSession {
    scheduler_shutdown: mpsc::Sender<()>,
    scheduler_worker: RuntimeWorker,
    shutdown: pw::channel::Sender<PipeWireControl>,
    worker: RuntimeWorker,
}

pub(super) struct PipeWirePreviewSession {
    shutdown: pw::channel::Sender<PipeWireControl>,
    worker: RuntimeWorker,
}

#[derive(Debug)]
pub(crate) struct RunningPreviewSession {
    pub(crate) session: Box<dyn CapturePreviewSession>,
    pub(crate) source_ids: Vec<String>,
}

impl PipeWirePreviewSession {
    pub(super) fn new(
        shutdown: pw::channel::Sender<PipeWireControl>,
        worker: RuntimeWorker,
    ) -> Self {
        Self { shutdown, worker }
    }
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
pub(super) enum PipeWireControl {
    Shutdown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct PipeWireStreamConfig {
    pub(super) node_id: u32,
    pub(super) source_id: String,
}

#[derive(Debug)]
pub(super) struct PipeWireStreamData {
    pub(super) format: spa::param::video::VideoInfoRaw,
    pub(super) logged_negotiated_format: bool,
    pub(super) source_id: String,
}

struct PipeWireCaptureState {
    error: Option<RecorderError>,
    latest_frame_log: PipeWireLatestFrameLog,
    latest_frames: SharedPipeWireFrameSampler,
    timing: SharedCaptureTiming,
}

pub(super) struct PipeWireStartupNotifier {
    sender: Option<mpsc::Sender<Result<(), RecorderError>>>,
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

impl PipeWireStartupNotifier {
    pub(super) fn new(sender: mpsc::Sender<Result<(), RecorderError>>) -> Self {
        Self {
            sender: Some(sender),
        }
    }

    pub(super) fn ready(&mut self) {
        if let Some(sender) = self.sender.take() {
            let _ = sender.send(Ok(()));
        }
    }

    pub(super) fn error(&mut self, error: &RecorderError) {
        if let Some(sender) = self.sender.take() {
            let _ = sender.send(Err(error.clone()));
        }
    }
}

pub(super) fn stream_configs_from_plan(
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
