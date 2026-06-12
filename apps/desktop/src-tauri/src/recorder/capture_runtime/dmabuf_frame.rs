use std::{
    fmt,
    os::fd::{AsRawFd, OwnedFd},
};

use super::super::{errors::capture_format_unsupported_error, types::RecorderError};
use super::CapturedFrameFormat;

pub(crate) struct CapturedDmaBufFrame {
    pub(crate) fd: OwnedFd,
    pub(crate) format: CapturedFrameFormat,
    pub(crate) fourcc: u32,
    pub(crate) height: u32,
    pub(crate) modifier: u64,
    pub(crate) offset: u32,
    pub(crate) source_id: String,
    pub(crate) stride: u32,
    pub(crate) timestamp_ms: u64,
    pub(crate) width: u32,
}

pub(crate) struct CapturedDmaBufFrameInput {
    pub(crate) fd: OwnedFd,
    pub(crate) format: CapturedFrameFormat,
    pub(crate) fourcc: u32,
    pub(crate) height: u32,
    pub(crate) modifier: u64,
    pub(crate) offset: u32,
    pub(crate) source_id: String,
    pub(crate) stride: u32,
    pub(crate) timestamp_ms: u64,
    pub(crate) width: u32,
}

impl CapturedDmaBufFrame {
    pub(crate) fn new(input: CapturedDmaBufFrameInput) -> Result<Self, RecorderError> {
        let frame = Self {
            fd: input.fd,
            format: input.format,
            fourcc: input.fourcc,
            height: input.height,
            modifier: input.modifier,
            offset: input.offset,
            source_id: input.source_id,
            stride: input.stride,
            timestamp_ms: input.timestamp_ms,
            width: input.width,
        };
        frame.validate()?;

        Ok(frame)
    }

    pub(crate) fn validate(&self) -> Result<(), RecorderError> {
        if self.timestamp_ms == 0 {
            return Err(capture_format_unsupported_error(format!(
                "Capture DMA-BUF frame from source {} has an invalid timestamp.",
                self.source_id
            )));
        }
        if self.width == 0 || self.height == 0 || self.stride == 0 {
            return Err(capture_format_unsupported_error(format!(
                "Capture DMA-BUF frame from source {} has invalid dimensions {}x{} and stride {}.",
                self.source_id, self.width, self.height, self.stride
            )));
        }
        let Some(bytes_per_pixel) = self.format.bytes_per_pixel() else {
            return Err(capture_format_unsupported_error(format!(
                "Capture DMA-BUF frame from source {} uses unsupported import format {}.",
                self.source_id,
                self.format.label()
            )));
        };
        let minimum_stride = self.width.checked_mul(bytes_per_pixel).ok_or_else(|| {
            capture_format_unsupported_error(format!(
                "Capture DMA-BUF frame from source {} has overflowing dimensions {}x{}.",
                self.source_id, self.width, self.height
            ))
        })?;
        if self.stride < minimum_stride {
            return Err(capture_format_unsupported_error(format!(
                "Capture DMA-BUF frame from source {} has stride {} below required stride {} for {}.",
                self.source_id,
                self.stride,
                minimum_stride,
                self.format.label()
            )));
        }
        if self.fourcc == 0 {
            return Err(capture_format_unsupported_error(format!(
                "Capture DMA-BUF frame from source {} has an invalid DRM format.",
                self.source_id
            )));
        }

        Ok(())
    }
}

impl fmt::Debug for CapturedDmaBufFrame {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CapturedDmaBufFrame")
            .field("source_id", &self.source_id)
            .field("timestamp_ms", &self.timestamp_ms)
            .field("width", &self.width)
            .field("height", &self.height)
            .field("stride", &self.stride)
            .field("offset", &self.offset)
            .field("format", &self.format)
            .field("fourcc", &self.fourcc)
            .field("modifier", &self.modifier)
            .field("fd", &self.fd.as_raw_fd())
            .finish()
    }
}
