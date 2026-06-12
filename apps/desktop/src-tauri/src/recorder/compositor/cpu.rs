use std::collections::HashMap;

use super::pixels::sample_bilinear_rgba;
use super::plan::{SceneCompositorInput, SceneCompositorPlan};
use crate::recorder::{
    capture_runtime::{CapturedFrame, CapturedFrameFormat},
    types::RecorderError,
};

pub(super) fn compose_cpu_frame(
    plan: &SceneCompositorPlan,
    frames_by_source_id: &HashMap<String, CapturedFrame>,
    timestamp_ms: u64,
) -> Result<CapturedFrame, RecorderError> {
    let width = plan.canvas_width as usize;
    let height = plan.canvas_height as usize;
    let mut canvas = vec![0_u8; width * height * 4];

    for input in &plan.inputs {
        let Some(frame) = frames_by_source_id.get(&input.source_id) else {
            continue;
        };
        frame.validate()?;
        composite_input(
            &mut canvas,
            plan.canvas_width,
            plan.canvas_height,
            input,
            frame,
        )?;
    }

    CapturedFrame::new(
        "composed",
        timestamp_ms,
        plan.canvas_width,
        plan.canvas_height,
        plan.canvas_width.saturating_mul(4),
        CapturedFrameFormat::Rgba,
        canvas,
    )
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
