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
    compose_latest_pipewire_frames, cpu_readable_pipewire_data_bytes, drm_fourcc_from_frame_format,
    next_scheduler_timestamp, pipewire_framerate_fraction, preflight_live_capture,
    raw_video_params, stream_configs_from_plan, ComposedFrameSendOutcome, CpuReadableFrameInput,
    LatestPipeWireFrameSampler, PipeWireCaptureSchedulerState, PipeWireCapturedFrame,
    PipeWireLatestFrameLog, PipeWireScheduledFrameLog, PipeWireStartupNotifier,
};
use crate::recorder::{
    capture_runtime::{
        bounded_frame_channel, capture_timing_snapshot, new_capture_timing,
        CaptureRuntimeSourceHandle, CapturedDmaBufFrame, CapturedDmaBufFrameInput, CapturedFrame,
        CapturedFrameFormat, CapturedFrameStorage,
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

    sampler
        .push(PipeWireCapturedFrame::DmaBuf(dmabuf_frame(
            "dmabuf-monitor",
            200,
        )))
        .expect("push DMA-BUF frame");

    assert_eq!(sampler.latest_frame_count(), 2);
    assert!(sampler.has_latest_frame("cpu-monitor"));
    assert!(sampler.has_latest_frame("dmabuf-monitor"));
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
    let frame = captured_frame_from_cpu_readable_pipewire_bytes(CpuReadableFrameInput {
        bytes: &mapped_bytes,
        frame_format: CapturedFrameFormat::Rgba,
        height: 1,
        len: 4,
        offset: 2,
        source_id: "portal-monitor-a",
        storage: CapturedFrameStorage::CpuReadable,
        stride: 4,
        timestamp_ms: 1,
        width: 1,
    })
    .expect("captured frame");

    mapped_bytes[2..6].copy_from_slice(&[0, 0, 0, 0]);

    assert_eq!(frame.data, vec![7, 6, 5, 4]);
}

#[test]
fn pipewire_dmabuf_frame_reports_gate_h2_requirement() {
    let error = captured_frame_from_cpu_readable_pipewire_bytes(CpuReadableFrameInput {
        bytes: &[1, 2, 3, 4],
        frame_format: CapturedFrameFormat::Rgba,
        height: 1,
        len: 4,
        offset: 0,
        source_id: "portal-monitor-a",
        storage: CapturedFrameStorage::DmaBuf,
        stride: 4,
        timestamp_ms: 1,
        width: 1,
    })
    .expect_err("dmabuf should not be CPU-readable");

    assert_eq!(error.code, "recorder_capture_dmabuf_required");
    assert!(!format!("{error:?}").contains("[1, 2, 3, 4]"));
}

#[test]
fn pipewire_dmabuf_without_mapped_bytes_reports_gate_h2_requirement() {
    let error =
        cpu_readable_pipewire_data_bytes("portal-monitor-a", &CapturedFrameStorage::DmaBuf, None)
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

fn live_stream_with_display(source_id: &str, node_id: u32, display_name: &str) -> LivePortalStream {
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

fn sized_cpu_frame(source_id: &str, timestamp_ms: u64, width: u32, height: u32) -> CapturedFrame {
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

    CapturedDmaBufFrame::new(CapturedDmaBufFrameInput {
        fd: fd.into(),
        format: CapturedFrameFormat::Rgbx,
        fourcc: drm_fourcc_from_frame_format(CapturedFrameFormat::Rgbx).expect("rgbx fourcc"),
        height: 1,
        modifier: 0,
        offset: 0,
        source_id: source_id.to_string(),
        stride: 4,
        timestamp_ms,
        width: 1,
    })
    .expect("valid DMA-BUF metadata")
}
