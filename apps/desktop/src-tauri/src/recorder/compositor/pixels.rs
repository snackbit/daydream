use crate::recorder::{
    capture_runtime::{CapturedFrame, CapturedFrameFormat},
    types::RecorderError,
};

pub(crate) fn sample_bilinear_rgba(
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
        CapturedFrameFormat::Yuv420p => yuv420p_pixel(frame, x, y),
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

fn yuv420p_pixel(frame: &CapturedFrame, x: u32, y: u32) -> Result<[u8; 4], RecorderError> {
    let y_index = y as usize * frame.stride as usize + x as usize;
    let y_plane_len = frame.stride as usize * frame.height as usize;
    let chroma_stride = frame.stride as usize / 2;
    let chroma_index = (y as usize / 2) * chroma_stride + (x as usize / 2);
    let chroma_plane_len = chroma_stride * (frame.height as usize / 2);
    let y_value = frame.data[y_index] as i32;
    let u_value = frame.data[y_plane_len + chroma_index] as i32 - 128;
    let v_value = frame.data[y_plane_len + chroma_plane_len + chroma_index] as i32 - 128;
    let red = (y_value + ((359 * v_value) >> 8)).clamp(0, 255) as u8;
    let green = (y_value - ((88 * u_value + 183 * v_value) >> 8)).clamp(0, 255) as u8;
    let blue = (y_value + ((454 * u_value) >> 8)).clamp(0, 255) as u8;

    Ok([red, green, blue, 255])
}
