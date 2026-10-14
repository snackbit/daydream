use super::*;

struct PipeWirePreviewState {
    error: Option<RecorderError>,
    inputs_by_source_id: HashMap<String, Vec<SceneCompositorInput>>,
    demand: Arc<Mutex<Option<crate::recorder::types::RecorderPreviewDemand>>>,
    sender: FrameSender,
    last_captured_ms: u64,
}

struct PreviewEncoder {
    stopping: Arc<std::sync::atomic::AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
}
impl Drop for PreviewEncoder {
    fn drop(&mut self) {
        self.stopping
            .store(true, std::sync::atomic::Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

pub(super) fn run_pipewire_preview(
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
    use tauri::Manager;
    let demand = app
        .state::<crate::recorder::manager::RecorderManager>()
        .preview_demand
        .clone();
    let (sender, receiver) = crate::recorder::capture_runtime::latest_frame_channel(1);
    let (error_sender, error_receiver) = pw::channel::channel();
    let stopping = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let worker_stopping = stopping.clone();
    let worker_demand = demand.clone();
    let inputs = plan.inputs.clone();
    let encoder = thread::Builder::new()
        .name("daydream-preview-encoder".into())
        .spawn(move || {
            let mut sequence = 0_u64;
            let first_frame_started = std::time::Instant::now();
            while !worker_stopping.load(std::sync::atomic::Ordering::Acquire) {
                let frame = match receiver.recv_timeout(Duration::from_millis(67)) {
                    Ok(frame) => frame,
                    Err(mpsc::RecvTimeoutError::Timeout) => continue,
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                };
                let request = worker_demand.lock().ok().and_then(|demand| demand.clone());
                let input = inputs.iter().find(|input| {
                    input.source_id == frame.source_id
                        && request
                            .as_ref()
                            .is_none_or(|request| request.scene_source_id == input.scene_source_id)
                });
                let Some(input) = input else {
                    continue;
                };
                let crop = crate::recorder::types::CropRect {
                    left: 0.,
                    top: 0.,
                    right: 0.,
                    bottom: 0.,
                };
                let encoded = if let Some(request) = &request {
                    crate::recorder::preview::encode_preview_frame(
                        &frame,
                        &crop,
                        request.width,
                        request.height,
                        true,
                    )
                } else {
                    encode_cropped_preview_frame(&frame, &crop)
                };
                // Demand changes fence encoding already in progress without reopening the portal.
                if worker_demand
                    .lock()
                    .ok()
                    .is_none_or(|current| *current != request)
                {
                    continue;
                }
                let encoded = match encoded {
                    Ok(encoded) => encoded,
                    Err(error) => {
                        let _ = error_sender.send(error);
                        break;
                    }
                };
                sequence = sequence.saturating_add(1);
                if sequence == 1 {
                    tracing::debug!(
                        elapsed_ms = first_frame_started.elapsed().as_millis() as u64,
                        "preview_first_frame_ready"
                    );
                }
                let _ = app.emit(
                    RECORDER_PREVIEW_FRAME_EVENT,
                    RecorderPreviewFrame {
                        generation: request.as_ref().map_or(0, |request| request.generation),
                        sequence,
                        source_width: frame.width,
                        source_height: frame.height,
                        data_url: encoded.data_url,
                        height: encoded.height,
                        width: encoded.width,
                        scene_source_id: input.scene_source_id.clone(),
                        source_id: input.source_id.clone(),
                        timestamp_ms: frame.timestamp_ms,
                    },
                );
            }
        })
        .map_err(|_| capture_adapter_failed_error("Preview worker could not start."))?;
    let encoder = PreviewEncoder {
        stopping,
        worker: Some(encoder),
    };
    let state = Rc::new(RefCell::new(PipeWirePreviewState {
        error: None,
        inputs_by_source_id: inputs_by_source_id(&plan),
        demand,
        sender,
        last_captured_ms: 0,
    }));
    let _encoding_errors = error_receiver.attach(mainloop.loop_(), {
        let state = state.clone();
        let mainloop = mainloop.clone();
        move |error| {
            state.borrow_mut().error = Some(error);
            mainloop.quit();
        }
    });
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

        let param_bytes = raw_video_params(stream_config.node_id, 15.0)?;
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
    drop(encoder);

    if let Some(error) = state.borrow().error.clone() {
        return Err(error);
    }

    Ok(())
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

    let timestamp_ms = now_ms();
    let mut state = state.borrow_mut();
    if timestamp_ms.saturating_sub(state.last_captured_ms) < 66 {
        return Ok(());
    }
    let Ok(demand) = state.demand.try_lock() else {
        return Ok(());
    };
    if let Some(request) = demand.as_ref() {
        if !state
            .inputs_by_source_id
            .get(&user_data.source_id)
            .is_some_and(|inputs| {
                inputs
                    .iter()
                    .any(|input| input.scene_source_id == request.scene_source_id)
            })
        {
            return Ok(());
        }
    }
    drop(demand);
    // Three raw buffers (callback, pending, in flight), RGBA and encoded output
    // remain below 256 MiB. No resize, color conversion or publication here.
    if datas[0].chunk().size() as usize > 64 * 1024 * 1024 {
        return Ok(());
    }
    let frame =
        captured_frame_from_pipewire_data(&user_data.source_id, user_data.format, &mut datas[0])?;
    state.last_captured_ms = timestamp_ms;
    match state.sender.try_send(frame) {
        Ok(()) | Err(FrameTrySendError::Full(_)) => (),
        Err(FrameTrySendError::InvalidFrame(error)) => return Err(error),
        Err(FrameTrySendError::Disconnected) => {
            return Err(capture_adapter_failed_error("Preview worker stopped."))
        }
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
