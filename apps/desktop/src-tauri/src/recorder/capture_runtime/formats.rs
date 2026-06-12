use super::super::{
    errors::{capture_dmabuf_required_error, capture_format_unsupported_error},
    types::RecorderError,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CapturedFrameFormat {
    Yuv444p,
    Yuv420p,
    Rgbx,
    Rgba,
    Xrgb,
    Argb,
    Bgrx,
    Bgra,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WriterFrameHandoffPlan {
    PassThroughYuv420p,
    ConvertToYuv420p { source_format: CapturedFrameFormat },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CapturedFrameStorage {
    CpuReadable,
    DmaBuf,
    Unknown(String),
}

impl CapturedFrameFormat {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Yuv444p => "yuv444p",
            Self::Yuv420p => "yuv420p",
            Self::Rgbx => "rgbx",
            Self::Rgba => "rgba",
            Self::Xrgb => "xrgb",
            Self::Argb => "argb",
            Self::Bgrx => "bgrx",
            Self::Bgra => "bgra",
        }
    }

    pub(crate) fn bytes_per_pixel(self) -> Option<u32> {
        match self {
            Self::Yuv444p | Self::Yuv420p => None,
            Self::Rgbx | Self::Rgba | Self::Xrgb | Self::Argb | Self::Bgrx | Self::Bgra => Some(4),
        }
    }
}

impl CapturedFrameStorage {
    pub(crate) fn ensure_cpu_readable(&self, source_id: &str) -> Result<(), RecorderError> {
        match self {
            Self::CpuReadable => Ok(()),
            Self::DmaBuf => Err(capture_dmabuf_required_error(format!(
                "Capture source {source_id} requires DMA-BUF import before it can be recorded."
            ))),
            Self::Unknown(label) => Err(capture_format_unsupported_error(format!(
                "Capture source {source_id} uses unsupported PipeWire buffer storage {label}."
            ))),
        }
    }
}
