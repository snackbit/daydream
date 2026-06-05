#![allow(dead_code)]

use std::collections::HashMap;

use super::{
    capture_runtime::{CaptureRuntimeSourceHandle, CapturedFrame, CapturedFrameFormat},
    errors::{capture_format_unsupported_error, capture_source_missing_error},
    types::{CanvasRect, CropRect, RecorderError, RecorderScene},
};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SceneCompositorPlan {
    pub(crate) canvas_height: u32,
    pub(crate) canvas_width: u32,
    pub(crate) inputs: Vec<SceneCompositorInput>,
    pub(crate) output_format: CapturedFrameFormat,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SceneCompositorInput {
    pub(crate) crop: CropRect,
    pub(crate) rect: CanvasRect,
    pub(crate) runtime_handle: CaptureRuntimeSourceHandle,
    pub(crate) scene_source_id: String,
    pub(crate) source_id: String,
    pub(crate) z_index: i32,
}

impl SceneCompositorPlan {
    pub(crate) fn from_scene(
        scene: &RecorderScene,
        runtime_sources: &[super::capture_runtime::CaptureRuntimeSource],
    ) -> Result<Self, RecorderError> {
        let source_handles = runtime_sources
            .iter()
            .map(|source| (source.source_id.as_str(), source.handle.clone()))
            .collect::<HashMap<_, _>>();
        let mut inputs = Vec::with_capacity(scene.sources.len());

        for source in &scene.sources {
            let runtime_handle = source_handles
                .get(source.source_id.as_str())
                .cloned()
                .ok_or_else(|| capture_source_missing_error(&source.source_id))?;
            inputs.push(SceneCompositorInput {
                crop: source.crop.clone(),
                rect: source.rect.clone(),
                runtime_handle,
                scene_source_id: source.id.clone(),
                source_id: source.source_id.clone(),
                z_index: source.z_index,
            });
        }

        inputs.sort_by_key(|input| input.z_index);

        Ok(Self {
            canvas_height: scene.canvas_height,
            canvas_width: scene.canvas_width,
            inputs,
            output_format: CapturedFrameFormat::Yuv444p,
        })
    }

    pub(crate) fn validate_composed_frame(
        &self,
        frame: &CapturedFrame,
    ) -> Result<(), RecorderError> {
        frame.validate()?;
        if frame.width != self.canvas_width || frame.height != self.canvas_height {
            return Err(capture_format_unsupported_error(format!(
                "Composed frame has dimensions {}x{}, expected {}x{}.",
                frame.width, frame.height, self.canvas_width, self.canvas_height
            )));
        }
        frame.ffmpeg_handoff_plan()?;

        Ok(())
    }

    pub(crate) fn compose_latest_frames(
        &self,
        frames_by_source_id: &HashMap<String, CapturedFrame>,
        timestamp_ms: u64,
    ) -> Result<CapturedFrame, RecorderError> {
        let width = self.canvas_width as usize;
        let height = self.canvas_height as usize;
        let mut canvas = vec![0_u8; width * height * 4];

        for input in &self.inputs {
            let Some(frame) = frames_by_source_id.get(&input.source_id) else {
                continue;
            };
            frame.validate()?;
            composite_input(
                &mut canvas,
                self.canvas_width,
                self.canvas_height,
                input,
                frame,
            )?;
        }

        CapturedFrame::new(
            "composed",
            timestamp_ms,
            self.canvas_width,
            self.canvas_height,
            self.canvas_width,
            CapturedFrameFormat::Yuv444p,
            rgba_to_yuv444p(&canvas, self.canvas_width, self.canvas_height),
        )
    }
}

fn composite_input(
    canvas: &mut [u8],
    canvas_width: u32,
    canvas_height: u32,
    input: &SceneCompositorInput,
    frame: &CapturedFrame,
) -> Result<(), RecorderError> {
    let dest_left = input.rect.x.max(0.0).floor() as i32;
    let dest_top = input.rect.y.max(0.0).floor() as i32;
    let dest_right = (input.rect.x + input.rect.width)
        .min(canvas_width as f64)
        .ceil() as i32;
    let dest_bottom = (input.rect.y + input.rect.height)
        .min(canvas_height as f64)
        .ceil() as i32;
    if dest_left >= dest_right || dest_top >= dest_bottom {
        return Ok(());
    }

    let crop_left = input.crop.left.max(0.0).min(frame.width as f64);
    let crop_top = input.crop.top.max(0.0).min(frame.height as f64);
    let crop_right = (frame.width as f64 - input.crop.right.max(0.0)).max(crop_left);
    let crop_bottom = (frame.height as f64 - input.crop.bottom.max(0.0)).max(crop_top);
    let crop_width = crop_right - crop_left;
    let crop_height = crop_bottom - crop_top;
    if crop_width <= 0.0
        || crop_height <= 0.0
        || input.rect.width <= 0.0
        || input.rect.height <= 0.0
    {
        return Ok(());
    }

    for y in dest_top..dest_bottom {
        for x in dest_left..dest_right {
            let local_x = (x as f64 + 0.5 - input.rect.x) / input.rect.width;
            let local_y = (y as f64 + 0.5 - input.rect.y) / input.rect.height;
            if !(0.0..=1.0).contains(&local_x) || !(0.0..=1.0).contains(&local_y) {
                continue;
            }

            let source_x = crop_left + local_x * crop_width - 0.5;
            let source_y = crop_top + local_y * crop_height - 0.5;
            let [red, green, blue, alpha] = sample_bilinear_rgba(frame, source_x, source_y)?;
            let canvas_index = ((y as usize * canvas_width as usize) + x as usize) * 4;
            alpha_blend_pixel(
                &mut canvas[canvas_index..canvas_index + 4],
                red,
                green,
                blue,
                alpha,
            );
        }
    }

    Ok(())
}

fn alpha_blend_pixel(target: &mut [u8], red: u8, green: u8, blue: u8, alpha: u8) {
    if alpha == 255 {
        target.copy_from_slice(&[red, green, blue, 255]);
        return;
    }

    let alpha = alpha as u16;
    let inverse = 255 - alpha;
    target[0] = (((red as u16 * alpha) + (target[0] as u16 * inverse)) / 255) as u8;
    target[1] = (((green as u16 * alpha) + (target[1] as u16 * inverse)) / 255) as u8;
    target[2] = (((blue as u16 * alpha) + (target[2] as u16 * inverse)) / 255) as u8;
    target[3] = 255;
}

fn sample_bilinear_rgba(
    frame: &CapturedFrame,
    source_x: f64,
    source_y: f64,
) -> Result<[u8; 4], RecorderError> {
    let x0 = source_x.floor().clamp(0.0, (frame.width - 1) as f64) as u32;
    let y0 = source_y.floor().clamp(0.0, (frame.height - 1) as f64) as u32;
    let x1 = (x0 + 1).min(frame.width - 1);
    let y1 = (y0 + 1).min(frame.height - 1);
    let x_weight = (source_x - x0 as f64).clamp(0.0, 1.0);
    let y_weight = (source_y - y0 as f64).clamp(0.0, 1.0);
    let top_left = pixel_rgba(frame, x0, y0)?;
    let top_right = pixel_rgba(frame, x1, y0)?;
    let bottom_left = pixel_rgba(frame, x0, y1)?;
    let bottom_right = pixel_rgba(frame, x1, y1)?;

    let mut result = [0_u8; 4];
    for channel in 0..4 {
        let top =
            top_left[channel] as f64 * (1.0 - x_weight) + top_right[channel] as f64 * x_weight;
        let bottom = bottom_left[channel] as f64 * (1.0 - x_weight)
            + bottom_right[channel] as f64 * x_weight;
        result[channel] = (top * (1.0 - y_weight) + bottom * y_weight)
            .round()
            .clamp(0.0, 255.0) as u8;
    }

    Ok(result)
}

fn pixel_rgba(frame: &CapturedFrame, x: u32, y: u32) -> Result<[u8; 4], RecorderError> {
    match frame.format {
        CapturedFrameFormat::Rgbx => packed_pixel(frame, x, y, [0, 1, 2, 3], false),
        CapturedFrameFormat::Rgba => packed_pixel(frame, x, y, [0, 1, 2, 3], true),
        CapturedFrameFormat::Xrgb => packed_pixel(frame, x, y, [1, 2, 3, 0], false),
        CapturedFrameFormat::Argb => packed_pixel(frame, x, y, [1, 2, 3, 0], true),
        CapturedFrameFormat::Bgrx => packed_pixel(frame, x, y, [2, 1, 0, 3], false),
        CapturedFrameFormat::Bgra => packed_pixel(frame, x, y, [2, 1, 0, 3], true),
        CapturedFrameFormat::Yuv444p => yuv_pixel(frame, x, y),
    }
}

fn packed_pixel(
    frame: &CapturedFrame,
    x: u32,
    y: u32,
    channels: [usize; 4],
    has_alpha: bool,
) -> Result<[u8; 4], RecorderError> {
    let index = y as usize * frame.stride as usize + x as usize * 4;
    let alpha = if has_alpha {
        frame.data[index + channels[3]]
    } else {
        255
    };

    Ok([
        frame.data[index + channels[0]],
        frame.data[index + channels[1]],
        frame.data[index + channels[2]],
        alpha,
    ])
}

fn yuv_pixel(frame: &CapturedFrame, x: u32, y: u32) -> Result<[u8; 4], RecorderError> {
    let index = y as usize * frame.stride as usize + x as usize;
    let plane_len = frame.stride as usize * frame.height as usize;
    let y_value = frame.data[index] as i32;
    let u_value = frame.data[plane_len + index] as i32 - 128;
    let v_value = frame.data[plane_len * 2 + index] as i32 - 128;
    let red = (y_value + ((359 * v_value) >> 8)).clamp(0, 255) as u8;
    let green = (y_value - ((88 * u_value + 183 * v_value) >> 8)).clamp(0, 255) as u8;
    let blue = (y_value + ((454 * u_value) >> 8)).clamp(0, 255) as u8;

    Ok([red, green, blue, 255])
}

fn rgba_to_yuv444p(rgba: &[u8], width: u32, height: u32) -> Vec<u8> {
    let plane_len = width as usize * height as usize;
    let mut output = vec![0; plane_len * 3];

    for index in 0..plane_len {
        let rgba_index = index * 4;
        let (y, u, v) = rgb_to_yuv(rgba[rgba_index], rgba[rgba_index + 1], rgba[rgba_index + 2]);
        output[index] = y;
        output[plane_len + index] = u;
        output[plane_len * 2 + index] = v;
    }

    output
}

fn rgb_to_yuv(red: u8, green: u8, blue: u8) -> (u8, u8, u8) {
    let red = red as i32;
    let green = green as i32;
    let blue = blue as i32;
    let y = ((77 * red + 150 * green + 29 * blue) >> 8).clamp(0, 255);
    let u = (((-43 * red - 85 * green + 128 * blue) >> 8) + 128).clamp(0, 255);
    let v = (((128 * red - 107 * green - 21 * blue) >> 8) + 128).clamp(0, 255);

    (y as u8, u as u8, v as u8)
}

#[cfg(test)]
mod tests {
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
            CapturedFrameFormat::Yuv444p,
            vec![0; 15_000],
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

        assert_eq!(composed.format, CapturedFrameFormat::Yuv444p);
        assert_eq!(composed.width, 100);
        assert_eq!(composed.height, 50);
        assert_eq!(composed.data.len(), 15_000);
        assert_eq!(composed.data[0], 76);
        assert_eq!(composed.data[1], 28);
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
}
