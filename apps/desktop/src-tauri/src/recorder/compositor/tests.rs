use std::collections::HashMap;

use uuid::Uuid;

use super::SceneCompositorPlan;
use crate::recorder::{
    capture_runtime::{
        CaptureRuntimeSource, CaptureRuntimeSourceHandle, CapturedFrame, CapturedFrameFormat,
    },
    types::{
        CanvasRect, CaptureSourceKind, CropRect, RecorderScene, RecorderSceneLayoutMode,
        RecorderSceneSource,
    },
};

#[test]
fn scene_source_ids_map_to_runtime_handles_in_z_index_order() {
    let scene = test_scene(vec![
        scene_source("front", "monitor-2", 20),
        scene_source("back", "monitor-1", 10),
    ]);
    let plan = SceneCompositorPlan::from_scene(
        &scene,
        &[
            runtime_source("monitor-1", "runtime-a"),
            runtime_source("monitor-2", "runtime-b"),
        ],
    )
    .expect("plan scene");

    assert_eq!(plan.inputs[0].scene_source_id, "back");
    assert_eq!(plan.inputs[0].source_id, "monitor-1");
    assert_eq!(
        plan.inputs[0].runtime_handle,
        CaptureRuntimeSourceHandle::BackendId("runtime-a".to_string()),
    );
    assert_eq!(plan.inputs[1].scene_source_id, "front");
    assert_eq!(plan.inputs[1].source_id, "monitor-2");
}

#[test]
fn missing_runtime_source_fails_before_segment_writing() {
    let scene = test_scene(vec![scene_source("missing", "monitor-missing", 0)]);
    let error = SceneCompositorPlan::from_scene(&scene, &[runtime_source("monitor-1", "one")])
        .expect_err("missing source should fail");

    assert_eq!(error.code, "recorder_capture_source_missing");
    assert!(error.message.contains("monitor-missing"));
}

#[test]
fn compositor_plan_pads_odd_canvas_dimensions_for_encoder_handoff() {
    let mut scene = test_scene(vec![scene_source("back", "monitor-1", 0)]);
    scene.canvas_width = 101;
    scene.canvas_height = 51;

    let plan = SceneCompositorPlan::from_scene(&scene, &[runtime_source("monitor-1", "one")])
        .expect("plan scene");

    assert_eq!(scene.canvas_width, 101);
    assert_eq!(scene.canvas_height, 51);
    assert_eq!(plan.canvas_width, 102);
    assert_eq!(plan.canvas_height, 52);
}

#[test]
fn composed_frame_handoff_requires_canvas_dimensions_and_supported_format() {
    let scene = test_scene(vec![scene_source("back", "monitor-1", 0)]);
    let plan = SceneCompositorPlan::from_scene(&scene, &[runtime_source("monitor-1", "one")])
        .expect("plan scene");
    let valid_frame = CapturedFrame::new(
        "composed",
        1,
        100,
        50,
        100,
        CapturedFrameFormat::Yuv420p,
        vec![0; 7_500],
    )
    .expect("valid composed frame");
    let invalid_frame = CapturedFrame::new(
        "composed",
        1,
        99,
        50,
        99,
        CapturedFrameFormat::Yuv444p,
        vec![0; 14_850],
    )
    .expect("valid raw frame with wrong canvas dimensions");

    plan.validate_composed_frame(&valid_frame)
        .expect("valid frame handoff");
    let error = plan
        .validate_composed_frame(&invalid_frame)
        .expect_err("wrong canvas dimensions should fail");
    assert_eq!(error.code, "recorder_capture_format_unsupported");
}

#[test]
fn cpu_compositor_overlays_sources_in_z_order() {
    let scene = test_scene(vec![
        positioned_scene_source("back", "monitor-1", 0, 0.0, 0.0, 2.0, 1.0),
        positioned_scene_source("front", "monitor-2", 10, 1.0, 0.0, 1.0, 1.0),
    ]);
    let plan = SceneCompositorPlan::from_scene(
        &scene,
        &[
            runtime_source("monitor-1", "runtime-a"),
            runtime_source("monitor-2", "runtime-b"),
        ],
    )
    .expect("plan scene");
    let frames = HashMap::from([
        (
            "monitor-1".to_string(),
            rgba_frame("monitor-1", [255, 0, 0, 255]),
        ),
        (
            "monitor-2".to_string(),
            rgba_frame("monitor-2", [0, 0, 255, 255]),
        ),
    ]);

    let composed = plan
        .compose_latest_frames(&frames, 10)
        .expect("compose latest frames");

    assert_eq!(composed.format, CapturedFrameFormat::Rgba);
    assert_eq!(composed.width, 100);
    assert_eq!(composed.height, 50);
    assert_eq!(composed.stride, 400);
    assert_eq!(composed.data.len(), 20_000);
    assert_eq!(&composed.data[0..4], &[255, 0, 0, 255]);
    assert_eq!(&composed.data[4..8], &[0, 0, 255, 255]);
}

#[test]
fn full_canvas_single_source_uses_passthrough_fast_path() {
    let scene = test_scene(vec![scene_source("front", "monitor-1", 0)]);
    let plan = SceneCompositorPlan::from_scene(&scene, &[runtime_source("monitor-1", "one")])
        .expect("plan scene");
    let frame = rgba_frame("monitor-1", [255, 0, 0, 255]);
    let frames = HashMap::from([("monitor-1".to_string(), frame.clone())]);

    let composed = plan
        .compose_latest_frames(&frames, 10)
        .expect("compose latest frames");

    assert_eq!(composed.format, CapturedFrameFormat::Rgba);
    assert_eq!(composed.timestamp_ms, 10);
    assert_eq!(composed.width, 100);
    assert_eq!(composed.height, 50);
    assert_eq!(composed.stride, 400);
    assert_eq!(composed.data, frame.data);
}

#[test]
fn cropped_single_source_uses_general_rgba_compositor() {
    let mut source = scene_source("front", "monitor-1", 0);
    source.crop.left = 1.0;
    let scene = test_scene(vec![source]);
    let plan = SceneCompositorPlan::from_scene(&scene, &[runtime_source("monitor-1", "one")])
        .expect("plan scene");
    let frames = HashMap::from([(
        "monitor-1".to_string(),
        rgba_frame("monitor-1", [255, 0, 0, 255]),
    )]);

    let composed = plan
        .compose_latest_frames(&frames, 10)
        .expect("compose latest frames");

    assert_eq!(composed.format, CapturedFrameFormat::Rgba);
    assert_eq!(composed.stride, 400);
    assert_eq!(composed.data.len(), 20_000);
}

fn runtime_source(source_id: &str, runtime_id: &str) -> CaptureRuntimeSource {
    CaptureRuntimeSource {
        display_name: source_id.to_string(),
        handle: CaptureRuntimeSourceHandle::BackendId(runtime_id.to_string()),
        height: Some(50),
        kind: CaptureSourceKind::Monitor,
        source_id: source_id.to_string(),
        width: Some(100),
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
    positioned_scene_source(id, source_id, z_index, 0.0, 0.0, 100.0, 50.0)
}

fn positioned_scene_source(
    id: &str,
    source_id: &str,
    z_index: i32,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> RecorderSceneSource {
    RecorderSceneSource {
        crop: CropRect {
            bottom: 0.0,
            left: 0.0,
            right: 0.0,
            top: 0.0,
        },
        display_name: id.to_string(),
        id: id.to_string(),
        preserve_aspect_ratio: true,
        rect: CanvasRect {
            height,
            width,
            x,
            y,
        },
        source_height: Some(50.0),
        source_id: source_id.to_string(),
        source_kind: CaptureSourceKind::Monitor,
        source_width: Some(100.0),
        z_index,
    }
}

fn rgba_frame(source_id: &str, pixel: [u8; 4]) -> CapturedFrame {
    let mut data = Vec::with_capacity(100 * 50 * 4);
    for _ in 0..(100 * 50) {
        data.extend(pixel);
    }

    CapturedFrame::new(source_id, 1, 100, 50, 400, CapturedFrameFormat::Rgba, data)
        .expect("valid rgba frame")
}
