use super::*;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct LiveCapturePreflight {
    pub(crate) plan: SceneCompositorPlan,
    pub(crate) runtime_sources: Vec<CaptureRuntimeSource>,
}

pub(crate) struct PreparedLiveCapture {
    pub(super) live_session: LivePortalSession,
    pub(super) preflight: LiveCapturePreflight,
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
            session: Box::new(PipeWirePreviewSession::new(shutdown, worker)),
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
