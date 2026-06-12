use super::*;

pub(super) fn captured_frame_from_pipewire_data(
    source_id: &str,
    format: spa::param::video::VideoInfoRaw,
    data: &mut spa::buffer::Data,
) -> Result<CapturedFrame, RecorderError> {
    match captured_pipewire_frame_from_data(source_id, format, data)? {
        PipeWireCapturedFrame::Cpu(frame) => Ok(frame),
        PipeWireCapturedFrame::DmaBuf(_frame) => Err(CapturedFrameStorage::DmaBuf
            .ensure_cpu_readable(source_id)
            .expect_err("DMA-BUF preview is not CPU-readable")),
    }
}

pub(super) fn captured_pipewire_frame_from_data(
    source_id: &str,
    format: spa::param::video::VideoInfoRaw,
    data: &mut spa::buffer::Data,
) -> Result<PipeWireCapturedFrame, RecorderError> {
    let size = format.size();
    let frame_format = captured_frame_format_from_pipewire(format.format())?;
    let chunk = data.chunk();
    let stride: u32 = chunk.stride().try_into().map_err(|_error| {
        capture_format_unsupported_error(format!(
            "Capture source {source_id} reported invalid PipeWire stride {}.",
            chunk.stride()
        ))
    })?;
    let offset: usize = chunk.offset().try_into().map_err(|_error| {
        capture_format_unsupported_error(format!(
            "Capture source {source_id} reported invalid PipeWire frame offset."
        ))
    })?;
    let len: usize = chunk.size().try_into().map_err(|_error| {
        capture_format_unsupported_error(format!(
            "Capture source {source_id} reported invalid PipeWire frame byte size."
        ))
    })?;
    let storage = storage_from_pipewire_data_type(data.type_());
    match storage {
        CapturedFrameStorage::CpuReadable => {
            let mapped_bytes = data.data();
            let bytes =
                cpu_readable_pipewire_data_bytes(source_id, &storage, mapped_bytes.as_deref())?;
            captured_frame_from_cpu_readable_pipewire_bytes(CpuReadableFrameInput {
                source_id,
                storage,
                timestamp_ms: now_ms(),
                width: size.width,
                height: size.height,
                stride,
                frame_format,
                offset,
                len,
                bytes,
            })
            .map(PipeWireCapturedFrame::Cpu)
        }
        CapturedFrameStorage::DmaBuf => {
            captured_dmabuf_frame_from_pipewire_data(DmaBufFrameInput {
                source_id,
                timestamp_ms: now_ms(),
                width: size.width,
                height: size.height,
                stride,
                frame_format,
                modifier: format.modifier(),
                offset,
                data,
            })
            .map(PipeWireCapturedFrame::DmaBuf)
        }
        CapturedFrameStorage::Unknown(_) => Err(storage
            .ensure_cpu_readable(source_id)
            .expect_err("unsupported storage should fail")),
    }
}

pub(super) fn cpu_readable_pipewire_data_bytes<'a>(
    source_id: &str,
    storage: &CapturedFrameStorage,
    bytes: Option<&'a [u8]>,
) -> Result<&'a [u8], RecorderError> {
    storage.ensure_cpu_readable(source_id)?;
    bytes.ok_or_else(|| {
        capture_format_unsupported_error(format!(
            "Capture source {source_id} did not expose CPU-readable PipeWire frame bytes."
        ))
    })
}

pub(super) struct DmaBufFrameInput<'a> {
    pub(super) data: &'a spa::buffer::Data,
    pub(super) frame_format: CapturedFrameFormat,
    pub(super) height: u32,
    pub(super) modifier: u64,
    pub(super) offset: usize,
    pub(super) source_id: &'a str,
    pub(super) stride: u32,
    pub(super) timestamp_ms: u64,
    pub(super) width: u32,
}

fn captured_dmabuf_frame_from_pipewire_data(
    input: DmaBufFrameInput<'_>,
) -> Result<CapturedDmaBufFrame, RecorderError> {
    let fd = input.data.fd();
    if fd < 0 {
        return Err(capture_format_unsupported_error(format!(
            "Capture source {} reported an invalid DMA-BUF fd.",
            input.source_id
        )));
    }
    let fd = unsafe { BorrowedFd::borrow_raw(fd) }
        .try_clone_to_owned()
        .map_err(|error| {
            capture_adapter_failed_error(format!(
                "Failed to duplicate DMA-BUF fd for capture source {}: {error}.",
                input.source_id
            ))
        })?;
    let offset: u32 = input.offset.try_into().map_err(|_error| {
        capture_format_unsupported_error(format!(
            "Capture source {} reported invalid DMA-BUF frame offset.",
            input.source_id
        ))
    })?;
    let fourcc = drm_fourcc_from_frame_format(input.frame_format)?;

    CapturedDmaBufFrame::new(CapturedDmaBufFrameInput {
        fd,
        format: input.frame_format,
        fourcc,
        height: input.height,
        modifier: input.modifier,
        offset,
        source_id: input.source_id.to_string(),
        stride: input.stride,
        timestamp_ms: input.timestamp_ms,
        width: input.width,
    })
}

pub(super) struct CpuReadableFrameInput<'a> {
    pub(super) bytes: &'a [u8],
    pub(super) frame_format: CapturedFrameFormat,
    pub(super) height: u32,
    pub(super) len: usize,
    pub(super) offset: usize,
    pub(super) source_id: &'a str,
    pub(super) storage: CapturedFrameStorage,
    pub(super) stride: u32,
    pub(super) timestamp_ms: u64,
    pub(super) width: u32,
}

pub(super) fn captured_frame_from_cpu_readable_pipewire_bytes(
    input: CpuReadableFrameInput<'_>,
) -> Result<CapturedFrame, RecorderError> {
    input.storage.ensure_cpu_readable(input.source_id)?;

    let end = input.offset.checked_add(input.len).ok_or_else(|| {
        capture_format_unsupported_error(format!(
            "Capture source {} reported overflowing PipeWire frame bounds.",
            input.source_id
        ))
    })?;
    let frame_bytes = input.bytes.get(input.offset..end).ok_or_else(|| {
        capture_format_unsupported_error(format!(
            "Capture source {} reported PipeWire frame bounds outside mapped memory.",
            input.source_id
        ))
    })?;

    CapturedFrame::new(
        input.source_id,
        input.timestamp_ms,
        input.width,
        input.height,
        input.stride,
        input.frame_format,
        frame_bytes.to_vec(),
    )
}

const fn fourcc_code(a: u8, b: u8, c: u8, d: u8) -> u32 {
    a as u32 | ((b as u32) << 8) | ((c as u32) << 16) | ((d as u32) << 24)
}

pub(super) fn drm_fourcc_from_frame_format(
    format: CapturedFrameFormat,
) -> Result<u32, RecorderError> {
    match format {
        CapturedFrameFormat::Rgbx => Ok(fourcc_code(b'R', b'X', b'2', b'4')),
        CapturedFrameFormat::Rgba => Ok(fourcc_code(b'R', b'A', b'2', b'4')),
        CapturedFrameFormat::Xrgb => Ok(fourcc_code(b'X', b'R', b'2', b'4')),
        CapturedFrameFormat::Argb => Ok(fourcc_code(b'A', b'R', b'2', b'4')),
        CapturedFrameFormat::Bgrx => Ok(fourcc_code(b'B', b'X', b'2', b'4')),
        CapturedFrameFormat::Bgra => Ok(fourcc_code(b'B', b'A', b'2', b'4')),
        CapturedFrameFormat::Yuv444p => Err(capture_format_unsupported_error(
            "Planar yuv444p DMA-BUF import is not supported.",
        )),
        CapturedFrameFormat::Yuv420p => Err(capture_format_unsupported_error(
            "Planar yuv420p DMA-BUF import is not supported.",
        )),
    }
}

pub(super) fn storage_from_pipewire_data_type(
    data_type: spa::buffer::DataType,
) -> CapturedFrameStorage {
    if data_type == spa::buffer::DataType::MemPtr || data_type == spa::buffer::DataType::MemFd {
        CapturedFrameStorage::CpuReadable
    } else if data_type == spa::buffer::DataType::DmaBuf {
        CapturedFrameStorage::DmaBuf
    } else {
        CapturedFrameStorage::Unknown(format!("{data_type:?}"))
    }
}

pub(super) fn captured_frame_format_from_pipewire(
    format: spa::param::video::VideoFormat,
) -> Result<CapturedFrameFormat, RecorderError> {
    if format == spa::param::video::VideoFormat::RGBx {
        Ok(CapturedFrameFormat::Rgbx)
    } else if format == spa::param::video::VideoFormat::RGBA {
        Ok(CapturedFrameFormat::Rgba)
    } else if format == spa::param::video::VideoFormat::xRGB {
        Ok(CapturedFrameFormat::Xrgb)
    } else if format == spa::param::video::VideoFormat::ARGB {
        Ok(CapturedFrameFormat::Argb)
    } else if format == spa::param::video::VideoFormat::BGRx {
        Ok(CapturedFrameFormat::Bgrx)
    } else if format == spa::param::video::VideoFormat::BGRA {
        Ok(CapturedFrameFormat::Bgra)
    } else if format == spa::param::video::VideoFormat::Y444 {
        Ok(CapturedFrameFormat::Yuv444p)
    } else if format == spa::param::video::VideoFormat::I420 {
        Ok(CapturedFrameFormat::Yuv420p)
    } else {
        Err(capture_format_unsupported_error(format!(
            "PipeWire frame format {format:?} is not supported."
        )))
    }
}
