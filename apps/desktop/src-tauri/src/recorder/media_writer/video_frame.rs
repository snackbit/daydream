use gstreamer as gst;

use super::{
    super::{
        capture_runtime::{CapturedFrame, CapturedFrameFormat},
        errors::dependency_error,
        types::RecorderError,
    },
    gst_framerate_fraction,
};

#[derive(Debug)]
pub(super) struct PreparedVideoFrame {
    pub(super) data: Vec<u8>,
    pub(super) format: CapturedFrameFormat,
    pub(super) height: u32,
    pub(super) width: u32,
}

#[cfg(test)]
pub(super) fn black_yuv420p_frame(
    canvas_width: u32,
    canvas_height: u32,
) -> Result<Vec<u8>, String> {
    if !canvas_width.is_multiple_of(2) || !canvas_height.is_multiple_of(2) {
        return Err("I420 frame dimensions must be even.".to_string());
    }
    let plane_len = (canvas_width as usize)
        .checked_mul(canvas_height as usize)
        .ok_or_else(|| "Frame dimensions overflow.".to_string())?;
    let chroma_len = (canvas_width as usize / 2)
        .checked_mul(canvas_height as usize / 2)
        .ok_or_else(|| "Frame dimensions overflow.".to_string())?;
    let frame_len = plane_len
        .checked_add(chroma_len * 2)
        .ok_or_else(|| "Frame dimensions overflow.".to_string())?;
    let mut frame = vec![0; frame_len];
    frame[plane_len..].fill(128);

    Ok(frame)
}

pub(super) fn prepare_video_frame_for_gstreamer(
    frame: CapturedFrame,
) -> Result<PreparedVideoFrame, RecorderError> {
    frame.validate()?;
    if frame_is_tightly_packed_for_gstreamer(&frame)? {
        return Ok(PreparedVideoFrame {
            data: frame.data,
            format: frame.format,
            height: frame.height,
            width: frame.width,
        });
    }

    repack_video_frame_for_gstreamer(frame)
}

pub(super) fn frame_is_tightly_packed_for_gstreamer(
    frame: &CapturedFrame,
) -> Result<bool, RecorderError> {
    Ok(match frame.format {
        CapturedFrameFormat::Yuv444p => frame.stride == frame.width,
        CapturedFrameFormat::Yuv420p => frame.stride == frame.width,
        CapturedFrameFormat::Rgbx
        | CapturedFrameFormat::Rgba
        | CapturedFrameFormat::Xrgb
        | CapturedFrameFormat::Argb
        | CapturedFrameFormat::Bgrx
        | CapturedFrameFormat::Bgra => {
            frame.stride
                == frame
                    .width
                    .checked_mul(4)
                    .ok_or_else(|| "Packed video frame stride overflow.".to_string())
                    .map_err(|message| {
                        dependency_error("recorder_media_writer_frame_write_failed", message)
                    })?
        }
    })
}

fn repack_video_frame_for_gstreamer(
    frame: CapturedFrame,
) -> Result<PreparedVideoFrame, RecorderError> {
    let data = match frame.format {
        CapturedFrameFormat::Yuv444p => repack_yuv444p_frame(&frame),
        CapturedFrameFormat::Yuv420p => repack_yuv420p_frame(&frame),
        CapturedFrameFormat::Rgbx
        | CapturedFrameFormat::Rgba
        | CapturedFrameFormat::Xrgb
        | CapturedFrameFormat::Argb
        | CapturedFrameFormat::Bgrx
        | CapturedFrameFormat::Bgra => repack_packed_rgba_frame(&frame),
    }?;
    Ok(PreparedVideoFrame {
        data,
        format: frame.format,
        height: frame.height,
        width: frame.width,
    })
}

fn repack_packed_rgba_frame(frame: &CapturedFrame) -> Result<Vec<u8>, RecorderError> {
    let width_bytes = frame.width.checked_mul(4).ok_or_else(|| {
        dependency_error(
            "recorder_media_writer_frame_write_failed",
            "Packed video frame width overflow.",
        )
    })? as usize;
    let mut output = vec![0; width_bytes * frame.height as usize];
    for row in 0..frame.height as usize {
        let source_start = row * frame.stride as usize;
        let output_start = row * width_bytes;
        output[output_start..output_start + width_bytes]
            .copy_from_slice(&frame.data[source_start..source_start + width_bytes]);
    }
    Ok(output)
}

fn repack_yuv444p_frame(frame: &CapturedFrame) -> Result<Vec<u8>, RecorderError> {
    let width = frame.width as usize;
    let height = frame.height as usize;
    let source_plane_len = frame.stride as usize * height;
    let output_plane_len = width * height;
    let mut output = vec![0; output_plane_len * 3];
    for plane in 0..3 {
        for row in 0..height {
            let source_start = plane * source_plane_len + row * frame.stride as usize;
            let output_start = plane * output_plane_len + row * width;
            output[output_start..output_start + width]
                .copy_from_slice(&frame.data[source_start..source_start + width]);
        }
    }
    Ok(output)
}

fn repack_yuv420p_frame(frame: &CapturedFrame) -> Result<Vec<u8>, RecorderError> {
    let width = frame.width as usize;
    let height = frame.height as usize;
    let y_source_stride = frame.stride as usize;
    let uv_source_stride = y_source_stride / 2;
    let y_output_len = width * height;
    let uv_width = width / 2;
    let uv_height = height / 2;
    let uv_output_len = uv_width * uv_height;
    let y_source_len = y_source_stride * height;
    let uv_source_len = uv_source_stride * uv_height;
    let mut output = vec![0; y_output_len + uv_output_len * 2];
    for row in 0..height {
        let source_start = row * y_source_stride;
        let output_start = row * width;
        output[output_start..output_start + width]
            .copy_from_slice(&frame.data[source_start..source_start + width]);
    }
    for plane in 0..2 {
        for row in 0..uv_height {
            let source_start = y_source_len + plane * uv_source_len + row * uv_source_stride;
            let output_start = y_output_len + plane * uv_output_len + row * uv_width;
            output[output_start..output_start + uv_width]
                .copy_from_slice(&frame.data[source_start..source_start + uv_width]);
        }
    }
    Ok(output)
}

pub(super) fn video_caps(frame: &PreparedVideoFrame, fps: f64) -> Result<gst::Caps, String> {
    let format = gst_video_format(frame.format);
    let mut builder = gst::Caps::builder("video/x-raw")
        .field("format", format)
        .field("framerate", gst_framerate_fraction(fps));
    if frame.width > 0 {
        builder = builder.field("width", frame.width as i32);
    }
    if frame.height > 0 {
        builder = builder.field("height", frame.height as i32);
    }
    Ok(builder.build())
}

pub(super) fn gst_video_format(format: CapturedFrameFormat) -> &'static str {
    match format {
        CapturedFrameFormat::Yuv444p => "Y444",
        CapturedFrameFormat::Yuv420p => "I420",
        CapturedFrameFormat::Rgbx => "RGBx",
        CapturedFrameFormat::Rgba => "RGBA",
        CapturedFrameFormat::Xrgb => "xRGB",
        CapturedFrameFormat::Argb => "ARGB",
        CapturedFrameFormat::Bgrx => "BGRx",
        CapturedFrameFormat::Bgra => "BGRA",
    }
}
