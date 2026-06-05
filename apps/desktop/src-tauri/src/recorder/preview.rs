use base64::{engine::general_purpose::STANDARD, Engine as _};

use super::{
    capture_runtime::CapturedFrame,
    compositor::sample_bilinear_rgba,
    errors::{capture_adapter_failed_error, capture_format_unsupported_error},
    types::{CropRect, RecorderError},
};

pub(crate) const PREVIEW_MAX_EDGE: u32 = 640;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct EncodedPreviewFrame {
    pub(crate) data_url: String,
    pub(crate) height: u32,
    pub(crate) width: u32,
}

pub(crate) fn encode_cropped_preview_frame(
    frame: &CapturedFrame,
    crop: &CropRect,
) -> Result<EncodedPreviewFrame, RecorderError> {
    frame.validate()?;
    let crop_area = crop_area(frame, crop)?;
    let (width, height) = preview_size(crop_area.width, crop_area.height);
    let rgba = preview_rgba(frame, crop_area, width, height)?;
    let png = encode_png_rgba(width, height, &rgba)?;

    Ok(EncodedPreviewFrame {
        data_url: format!("data:image/png;base64,{}", STANDARD.encode(png)),
        height,
        width,
    })
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct CropArea {
    height: f64,
    left: f64,
    top: f64,
    width: f64,
}

fn crop_area(frame: &CapturedFrame, crop: &CropRect) -> Result<CropArea, RecorderError> {
    let left = crop.left.max(0.0).min(frame.width as f64);
    let top = crop.top.max(0.0).min(frame.height as f64);
    let right = (frame.width as f64 - crop.right.max(0.0)).max(left);
    let bottom = (frame.height as f64 - crop.bottom.max(0.0)).max(top);
    let width = right - left;
    let height = bottom - top;

    if width <= 0.0 || height <= 0.0 {
        return Err(capture_format_unsupported_error(format!(
            "Capture source {} preview crop is empty for frame {}x{}.",
            frame.source_id, frame.width, frame.height
        )));
    }

    Ok(CropArea {
        height,
        left,
        top,
        width,
    })
}

fn preview_size(crop_width: f64, crop_height: f64) -> (u32, u32) {
    let scale = (PREVIEW_MAX_EDGE as f64 / crop_width.max(crop_height)).min(1.0);
    let width = (crop_width * scale)
        .round()
        .clamp(1.0, PREVIEW_MAX_EDGE as f64) as u32;
    let height = (crop_height * scale)
        .round()
        .clamp(1.0, PREVIEW_MAX_EDGE as f64) as u32;

    (width, height)
}

fn preview_rgba(
    frame: &CapturedFrame,
    crop: CropArea,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, RecorderError> {
    let mut output = vec![0_u8; width as usize * height as usize * 4];

    for y in 0..height {
        for x in 0..width {
            let local_x = (x as f64 + 0.5) / width as f64;
            let local_y = (y as f64 + 0.5) / height as f64;
            let source_x = crop.left + local_x * crop.width - 0.5;
            let source_y = crop.top + local_y * crop.height - 0.5;
            let pixel = sample_bilinear_rgba(frame, source_x, source_y)?;
            let index = ((y as usize * width as usize) + x as usize) * 4;
            output[index..index + 4].copy_from_slice(&pixel);
        }
    }

    Ok(output)
}

fn encode_png_rgba(width: u32, height: u32, rgba: &[u8]) -> Result<Vec<u8>, RecorderError> {
    let mut bytes = Vec::new();
    let mut encoder = png::Encoder::new(&mut bytes, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    {
        let mut writer = encoder.write_header().map_err(|error| {
            capture_adapter_failed_error(format!("Failed to encode recorder preview PNG: {error}."))
        })?;
        writer.write_image_data(rgba).map_err(|error| {
            capture_adapter_failed_error(format!("Failed to write recorder preview PNG: {error}."))
        })?;
    }

    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::encode_cropped_preview_frame;
    use crate::recorder::{
        capture_runtime::{CapturedFrame, CapturedFrameFormat},
        types::CropRect,
    };

    #[test]
    fn preview_encoder_crops_and_bounds_png_data_url() {
        let frame = rgba_frame(800, 400);
        let encoded = encode_cropped_preview_frame(
            &frame,
            &CropRect {
                bottom: 50.0,
                left: 100.0,
                right: 100.0,
                top: 50.0,
            },
        )
        .expect("encode preview");

        assert_eq!(encoded.width, 600);
        assert_eq!(encoded.height, 300);
        assert!(encoded.data_url.starts_with("data:image/png;base64,"));
    }

    #[test]
    fn preview_encoder_rejects_empty_crops_without_frame_bytes_in_error() {
        let frame = rgba_frame(2, 2);
        let error = encode_cropped_preview_frame(
            &frame,
            &CropRect {
                bottom: 2.0,
                left: 2.0,
                right: 0.0,
                top: 0.0,
            },
        )
        .expect_err("empty crop should fail");

        assert_eq!(error.code, "recorder_capture_format_unsupported");
        assert!(!format!("{error:?}").contains("[255"));
    }

    fn rgba_frame(width: u32, height: u32) -> CapturedFrame {
        let mut data = Vec::with_capacity(width as usize * height as usize * 4);
        for y in 0..height {
            for x in 0..width {
                data.extend([x as u8, y as u8, 128, 255]);
            }
        }

        CapturedFrame::new(
            "source-1",
            1,
            width,
            height,
            width * 4,
            CapturedFrameFormat::Rgba,
            data,
        )
        .expect("valid frame")
    }
}
