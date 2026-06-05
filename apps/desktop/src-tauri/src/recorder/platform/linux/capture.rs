#![allow(dead_code)]

use std::{cell::RefCell, os::fd::OwnedFd, rc::Rc, sync::mpsc, time::Duration};

use pipewire as pw;
use pw::{properties::properties, spa};

use super::portal::{LivePortalSession, LivePortalStream};
use crate::recorder::{
    capture_runtime::{
        bounded_frame_channel, CaptureRuntimeSource, CaptureRuntimeSourceHandle, CaptureSession,
        CapturedFrame, CapturedFrameFormat, CapturedFrameStorage, FrameSender, LatestFrameSampler,
        RuntimeWorker,
    },
    compositor::{SceneCompositorInput, SceneCompositorPlan},
    errors::{
        capture_adapter_failed_error, capture_format_unsupported_error,
        capture_source_missing_error,
    },
    ffmpeg::{self, FfmpegSegmentInput},
    manager::{now_ms, RunningSegmentMedia},
    types::{CaptureSourceKind, RecorderError, RecorderScene},
};

const PIPEWIRE_READY_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct LiveCapturePreflight {
    pub(crate) plan: SceneCompositorPlan,
    pub(crate) runtime_sources: Vec<CaptureRuntimeSource>,
}

pub(crate) struct PipeWireCaptureSession {
    shutdown: pw::channel::Sender<PipeWireControl>,
    worker: RuntimeWorker,
}

pub(crate) struct PreparedLiveCapture {
    live_session: LivePortalSession,
    preflight: LiveCapturePreflight,
}

impl std::fmt::Debug for PipeWireCaptureSession {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PipeWireCaptureSession")
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
    source_id: String,
}

struct PipeWireCaptureState {
    composed_frames: FrameSender,
    error: Option<RecorderError>,
    plan: SceneCompositorPlan,
    sampler: LatestFrameSampler,
}

struct PipeWireStartupNotifier {
    sender: Option<mpsc::Sender<Result<(), RecorderError>>>,
}

pub(crate) fn preflight_live_capture(
    scene: &RecorderScene,
    live_streams: &[LivePortalStream],
) -> Result<LiveCapturePreflight, RecorderError> {
    let runtime_sources = runtime_sources_for_scene(scene, live_streams)?;
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
    let preflight = preflight_live_capture(scene, &live_session.streams)?;

    Ok(PreparedLiveCapture {
        live_session,
        preflight,
    })
}

pub(crate) fn start_live_segment(
    ffmpeg_path: &std::path::Path,
    input: &FfmpegSegmentInput,
    prepared: PreparedLiveCapture,
) -> Result<RunningSegmentMedia, RecorderError> {
    let (sender, receiver) = bounded_frame_channel(3);
    let ffmpeg = ffmpeg::start_segment_from_frames(ffmpeg_path, input, receiver)?;
    let capture = match start_pipewire_capture_session(
        prepared.live_session,
        prepared.preflight,
        input.fps,
        sender,
    ) {
        Ok(capture) => capture,
        Err(error) => {
            let _ = ffmpeg.stop();
            return Err(error);
        }
    };

    Ok(RunningSegmentMedia::new(ffmpeg, Some(capture)))
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

    if !is_volatile_pipewire_source_id(&scene_source.source_id) {
        return None;
    }

    if let Some(index) = live_streams
        .iter()
        .enumerate()
        .find(|(index, stream)| {
            !claimed_streams[*index] && stream.display_name == scene_source.display_name
        })
        .map(|(index, _stream)| index)
    {
        return Some(index);
    }

    live_streams
        .get(scene_index)
        .filter(|_stream| !claimed_streams[scene_index])
        .map(|_stream| scene_index)
        .or_else(|| claimed_streams.iter().position(|claimed| !claimed))
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
) -> Result<Box<dyn CaptureSession>, RecorderError> {
    let stream_configs = stream_configs_from_plan(&preflight.plan)?;
    let (shutdown, shutdown_receiver) = pw::channel::channel();
    let (ready_sender, ready_receiver) = mpsc::channel();
    let worker = RuntimeWorker::spawn("pipewire-capture", move || {
        run_pipewire_capture(
            live_session,
            preflight.plan,
            fps,
            composed_frames,
            stream_configs,
            shutdown_receiver,
            ready_sender,
        )
    });

    match ready_receiver.recv_timeout(PIPEWIRE_READY_TIMEOUT) {
        Ok(Ok(())) => Ok(Box::new(PipeWireCaptureSession { shutdown, worker })),
        Ok(Err(error)) => {
            let _ = worker.join();
            Err(error)
        }
        Err(_error) => {
            let _ = shutdown.send(PipeWireControl::Shutdown);
            let _ = worker.join();
            Err(capture_adapter_failed_error(
                "PipeWire capture did not become ready before the startup timeout.",
            ))
        }
    }
}

impl CaptureSession for PipeWireCaptureSession {
    fn stop(self: Box<Self>) -> Result<(), RecorderError> {
        let _ = self.shutdown.send(PipeWireControl::Shutdown);
        self.worker.join()
    }
}

fn run_pipewire_capture(
    live_session: LivePortalSession,
    plan: SceneCompositorPlan,
    fps: f64,
    composed_frames: FrameSender,
    stream_configs: Vec<PipeWireStreamConfig>,
    shutdown_receiver: pw::channel::Receiver<PipeWireControl>,
    ready_sender: mpsc::Sender<Result<(), RecorderError>>,
) -> Result<(), RecorderError> {
    let mut startup = PipeWireStartupNotifier::new(ready_sender);
    let result = run_pipewire_capture_inner(
        live_session.fd,
        plan,
        fps,
        composed_frames,
        stream_configs,
        shutdown_receiver,
        &mut startup,
    );
    if let Err(error) = &result {
        startup.error(error);
    }

    result
}

fn run_pipewire_capture_inner(
    fd: OwnedFd,
    plan: SceneCompositorPlan,
    fps: f64,
    composed_frames: FrameSender,
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
        composed_frames,
        error: None,
        plan,
        sampler: LatestFrameSampler::new(fps),
    }));
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

        let param_bytes = raw_video_params(stream_config.node_id)?;
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

    let frame =
        captured_frame_from_pipewire_data(&user_data.source_id, user_data.format, &mut datas[0])?;
    let mut state = state.borrow_mut();
    state.sampler.push(frame)?;
    let timestamp_ms = now_ms();
    if !state.sampler.should_compose(timestamp_ms) {
        return Ok(());
    }

    let composed = state
        .plan
        .compose_latest_frames(state.sampler.latest_frames(), timestamp_ms)?;
    state.plan.validate_composed_frame(&composed)?;
    state.composed_frames.send(composed)?;
    state.sampler.mark_composed(timestamp_ms);

    Ok(())
}

fn captured_frame_from_pipewire_data(
    source_id: &str,
    format: spa::param::video::VideoInfoRaw,
    data: &mut spa::buffer::Data,
) -> Result<CapturedFrame, RecorderError> {
    storage_from_pipewire_data_type(data.type_()).ensure_cpu_readable(source_id)?;

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
    let bytes = data.data().ok_or_else(|| {
        capture_format_unsupported_error(format!(
            "Capture source {source_id} did not expose CPU-readable PipeWire frame bytes."
        ))
    })?;
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
        now_ms(),
        size.width,
        size.height,
        stride,
        frame_format,
        frame_bytes.to_vec(),
    )
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
    } else {
        Err(capture_format_unsupported_error(format!(
            "PipeWire frame format {format:?} is not supported."
        )))
    }
}

fn raw_video_params(node_id: u32) -> Result<Vec<u8>, RecorderError> {
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
            pw::spa::utils::Fraction { num: 1, denom: 1 },
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

fn pipewire_error(error: pw::Error) -> RecorderError {
    capture_adapter_failed_error(format!("PipeWire capture adapter failed: {error}."))
}

#[cfg(test)]
mod tests {
    use std::{sync::mpsc, time::Duration};

    use uuid::Uuid;

    use super::{
        captured_frame_format_from_pipewire, preflight_live_capture, stream_configs_from_plan,
        PipeWireStartupNotifier,
    };
    use crate::recorder::{
        capture_runtime::CaptureRuntimeSourceHandle,
        errors::capture_adapter_failed_error,
        platform::linux::portal::LivePortalStream,
        types::{
            CanvasRect, CaptureSourceKind, CropRect, RecorderScene, RecorderSceneLayoutMode,
            RecorderSceneSource,
        },
    };

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
        let error =
            captured_frame_format_from_pipewire(pipewire::spa::param::video::VideoFormat::I420)
                .expect_err("unsupported format");

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
}
