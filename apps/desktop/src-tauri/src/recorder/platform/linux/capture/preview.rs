use super::*;

struct PipeWirePreviewState {
    app: AppHandle,
    error: Option<RecorderError>,
    inputs_by_source_id: HashMap<String, Vec<SceneCompositorInput>>,
    last_emitted_by_scene_source_id: HashMap<String, u64>,
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

        let encoded = encode_cropped_preview_frame(
            &frame,
            &crate::recorder::types::CropRect {
                left: 0.0,
                top: 0.0,
                right: 0.0,
                bottom: 0.0,
            },
        )?;
        let payload = RecorderPreviewFrame {
            source_width: frame.width,
            source_height: frame.height,
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
