use std::fmt;

use super::super::{errors::capture_format_unsupported_error, types::RecorderError};
use super::{CapturedFrameFormat, WriterFrameHandoffPlan};

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct CapturedFrame {
    pub(crate) data: Vec<u8>,
    pub(crate) format: CapturedFrameFormat,
    pub(crate) height: u32,
    pub(crate) source_id: String,
    pub(crate) stride: u32,
    pub(crate) timestamp_ms: u64,
    pub(crate) width: u32,
}

impl CapturedFrame {
    pub(crate) fn new(
        source_id: impl Into<String>,
        timestamp_ms: u64,
        width: u32,
        height: u32,
        stride: u32,
        format: CapturedFrameFormat,
        data: Vec<u8>,
    ) -> Result<Self, RecorderError> {
        let frame = Self {
            data,
            format,
            height,
            source_id: source_id.into(),
            stride,
            timestamp_ms,
            width,
        };
        frame.validate()?;

        Ok(frame)
    }

    pub(crate) fn validate(&self) -> Result<(), RecorderError> {
        if self.timestamp_ms == 0 {
            return Err(capture_format_unsupported_error(format!(
                "Capture frame from source {} has an invalid timestamp.",
                self.source_id
            )));
        }
        if self.width == 0 || self.height == 0 || self.stride == 0 {
            return Err(capture_format_unsupported_error(format!(
                "Capture frame from source {} has invalid dimensions {}x{} and stride {}.",
                self.source_id, self.width, self.height, self.stride
            )));
        }

        let minimum_len = self.minimum_data_len()?;
        if self.data.len() < minimum_len {
            return Err(capture_format_unsupported_error(format!(
                "Capture frame from source {} has {} bytes, but {} bytes are required for {} {}x{} stride {}.",
                self.source_id,
                self.data.len(),
                minimum_len,
                self.format.label(),
                self.width,
                self.height,
                self.stride
            )));
        }

        Ok(())
    }

    pub(crate) fn writer_handoff_plan(&self) -> Result<WriterFrameHandoffPlan, RecorderError> {
        self.validate()?;

        match self.format {
            CapturedFrameFormat::Yuv420p => Ok(WriterFrameHandoffPlan::PassThroughYuv420p),
            CapturedFrameFormat::Yuv444p => Ok(WriterFrameHandoffPlan::ConvertToYuv420p {
                source_format: self.format,
            }),
            CapturedFrameFormat::Rgbx
            | CapturedFrameFormat::Rgba
            | CapturedFrameFormat::Xrgb
            | CapturedFrameFormat::Argb
            | CapturedFrameFormat::Bgrx
            | CapturedFrameFormat::Bgra => Ok(WriterFrameHandoffPlan::ConvertToYuv420p {
                source_format: self.format,
            }),
        }
    }

    fn minimum_data_len(&self) -> Result<usize, RecorderError> {
        let height = self.height as usize;
        let stride = self.stride as usize;

        match self.format {
            CapturedFrameFormat::Yuv444p => stride
                .checked_mul(height)
                .and_then(|plane_len| plane_len.checked_mul(3))
                .ok_or_else(|| {
                    capture_format_unsupported_error(format!(
                        "Capture frame from source {} has overflowing yuv444p dimensions {}x{} stride {}.",
                        self.source_id, self.width, self.height, self.stride
                    ))
                }),
            CapturedFrameFormat::Yuv420p => {
                if !self.width.is_multiple_of(2)
                    || !self.height.is_multiple_of(2)
                    || !self.stride.is_multiple_of(2)
                {
                    return Err(capture_format_unsupported_error(format!(
                        "Capture frame from source {} has odd yuv420p dimensions {}x{} or stride {}.",
                        self.source_id, self.width, self.height, self.stride
                    )));
                }
                let y_len = stride.checked_mul(height).ok_or_else(|| {
                    capture_format_unsupported_error(format!(
                        "Capture frame from source {} has overflowing yuv420p dimensions {}x{} stride {}.",
                        self.source_id, self.width, self.height, self.stride
                    ))
                })?;
                let chroma_len = (stride / 2)
                    .checked_mul(height / 2)
                    .and_then(|plane_len| plane_len.checked_mul(2))
                    .ok_or_else(|| {
                        capture_format_unsupported_error(format!(
                            "Capture frame from source {} has overflowing yuv420p chroma dimensions {}x{} stride {}.",
                            self.source_id, self.width, self.height, self.stride
                        ))
                    })?;
                y_len.checked_add(chroma_len).ok_or_else(|| {
                    capture_format_unsupported_error(format!(
                        "Capture frame from source {} has overflowing yuv420p frame size.",
                        self.source_id
                    ))
                })
            }
            format => {
                let Some(bytes_per_pixel) = format.bytes_per_pixel() else {
                    return Err(capture_format_unsupported_error(format!(
                        "Capture frame format {} is not supported.",
                        format.label()
                    )));
                };
                let minimum_stride = self.width.checked_mul(bytes_per_pixel).ok_or_else(|| {
                    capture_format_unsupported_error(format!(
                        "Capture frame from source {} has overflowing dimensions {}x{}.",
                        self.source_id, self.width, self.height
                    ))
                })?;
                if self.stride < minimum_stride {
                    return Err(capture_format_unsupported_error(format!(
                        "Capture frame from source {} has stride {} below required stride {} for {}.",
                        self.source_id,
                        self.stride,
                        minimum_stride,
                        format.label()
                    )));
                }

                stride.checked_mul(height).ok_or_else(|| {
                    capture_format_unsupported_error(format!(
                        "Capture frame from source {} has overflowing dimensions {}x{} stride {}.",
                        self.source_id, self.width, self.height, self.stride
                    ))
                })
            }
        }
    }
}

impl fmt::Debug for CapturedFrame {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CapturedFrame")
            .field("source_id", &self.source_id)
            .field("timestamp_ms", &self.timestamp_ms)
            .field("width", &self.width)
            .field("height", &self.height)
            .field("stride", &self.stride)
            .field("format", &self.format)
            .field("data_len", &self.data.len())
            .finish()
    }
}
