use base64::{engine::general_purpose::STANDARD, Engine as _};

use super::{
    capture_runtime::CapturedFrame,
    compositor::sample_bilinear_rgba,
    errors::{capture_adapter_failed_error, capture_format_unsupported_error},
    types::{CropRect, RecorderError},
};

pub(crate) const PREVIEW_MAX_EDGE: u32 = 3840;
pub(crate) const PREVIEW_MAX_PIXELS: u64 = 4_000_000;
pub(crate) const PREVIEW_MAX_ENCODED_BYTES: usize = 4 * 1024 * 1024;

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
    encode_preview_frame(frame, crop, 1280, 720, false)
}

pub(crate) fn encode_preview_frame(
    frame: &CapturedFrame,
    crop: &CropRect,
    requested_width: u32,
    requested_height: u32,
    jpeg: bool,
) -> Result<EncodedPreviewFrame, RecorderError> {
    frame.validate()?;
    let crop_area = crop_area(frame, crop)?;
    let (width, height) = preview_size(
        crop_area.width,
        crop_area.height,
        requested_width,
        requested_height,
    );
    let rgba = preview_rgba(frame, crop_area, width, height)?;
    let (bytes, mime) = if jpeg {
        let mut bytes = Vec::new();
        let mut encoder = jpeg_encoder::Encoder::new(&mut bytes, 90);
        encoder.set_sampling_factor(jpeg_encoder::SamplingFactor::F_1_1);
        encoder
            .encode(
                &rgba,
                width as u16,
                height as u16,
                jpeg_encoder::ColorType::Rgba,
            )
            .map_err(|_| capture_adapter_failed_error("Preview image could not be encoded."))?;
        (bytes, "jpeg")
    } else {
        (encode_png_rgba(width, height, &rgba)?, "png")
    };
    if bytes.len() > PREVIEW_MAX_ENCODED_BYTES {
        return Err(capture_adapter_failed_error(
            "Preview image exceeds its memory limit.",
        ));
    }

    Ok(EncodedPreviewFrame {
        data_url: format!("data:image/{mime};base64,{}", STANDARD.encode(bytes)),
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

fn preview_size(crop_width: f64, crop_height: f64, width: u32, height: u32) -> (u32, u32) {
    let scale = (PREVIEW_MAX_EDGE as f64 / crop_width.max(crop_height))
        .min(1.0)
        .min(f64::from(width.max(1)) / crop_width)
        .min(f64::from(height.max(1)) / crop_height)
        .min((PREVIEW_MAX_PIXELS as f64 / (crop_width * crop_height)).sqrt());
    let width = (crop_width * scale)
        .floor()
        .clamp(1.0, PREVIEW_MAX_EDGE as f64) as u32;
    let height = (crop_height * scale)
        .floor()
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
    use base64::Engine as _;

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
    #[test]
    fn jpeg_preview_uses_full_chroma_and_respects_source_and_memory_limits() {
        let encoded = super::encode_preview_frame(
            &rgba_frame(800, 400),
            &CropRect {
                left: 0.0,
                top: 0.0,
                right: 0.0,
                bottom: 0.0,
            },
            3840,
            3840,
            true,
        )
        .unwrap();
        assert_eq!((encoded.width, encoded.height), (800, 400));
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(
                encoded
                    .data_url
                    .strip_prefix("data:image/jpeg;base64,")
                    .unwrap(),
            )
            .unwrap();
        assert!(bytes.len() <= super::PREVIEW_MAX_ENCODED_BYTES);
        let sof = bytes
            .windows(2)
            .position(|pair| pair == [0xff, 0xc0])
            .unwrap();
        assert_eq!(
            [bytes[sof + 11], bytes[sof + 14], bytes[sof + 17]],
            [0x11; 3]
        );
        let (width, height) = super::preview_size(5120.0, 1440.0, 5120, 1440);
        assert!(width.max(height) <= super::PREVIEW_MAX_EDGE);
        assert!(u64::from(width) * u64::from(height) <= super::PREVIEW_MAX_PIXELS);
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
