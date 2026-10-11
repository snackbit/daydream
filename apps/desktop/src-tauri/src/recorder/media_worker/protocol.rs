use super::super::{
    capture_runtime::{CapturedFrame, CapturedFrameFormat},
    media_writer::{FragmentEvent, MediaSegmentInput},
    types::RecorderError,
};
use serde::{Deserialize, Serialize};
use std::io::{self, Read, Write};

pub(super) const VERSION: u32 = 1;
pub(super) const FRAME_BUDGET: usize = 512 * 1024 * 1024;
const CONTROL_LIMIT: usize = 1024 * 1024;
const MAGIC: u32 = 0x44445246;

#[derive(Debug, Serialize, Deserialize)]
pub(super) enum Request {
    #[cfg(test)]
    TestStall {
        generation: u64,
    },
    #[cfg(test)]
    TestLegacyEos {
        generation: u64,
        timestamp_ms: u64,
    },
    #[cfg(test)]
    TestParentDeathWait,
    #[cfg(test)]
    TestDescriptorHandoff,
    #[cfg(test)]
    TestHoldAudioPad {
        generation: u64,
    },
    #[cfg(test)]
    TestNull {
        generation: u64,
    },
    Recover {
        version: u32,
        input: std::path::PathBuf,
        output: std::path::PathBuf,
    },
    Start {
        version: u32,
        generation: u64,
        input: MediaSegmentInput,
    },
    Stop {
        generation: u64,
        timestamp_ms: u64,
    },
    Cutoff {
        generation: u64,
        timestamp_ms: u64,
    },
    Abort {
        generation: u64,
    },
    Flush {
        generation: u64,
    },
}
#[derive(Debug, Serialize, Deserialize)]
pub(super) enum Response {
    #[cfg(test)]
    TestAudioPadHeld {
        generation: u64,
    },
    #[cfg(test)]
    TestNullStarted {
        generation: u64,
    },
    #[cfg(test)]
    TestNullReturned {
        generation: u64,
    },
    #[cfg(test)]
    TestLegacyEos {
        generation: u64,
        result: Result<(), RecorderError>,
    },
    Recovered {
        result: Result<super::recovery::RecoveredMedia, RecorderError>,
    },
    Ready {
        generation: u64,
    },
    Fragments {
        generation: u64,
        events: Vec<FragmentEvent>,
    },
    Finished {
        generation: u64,
        result: Result<Vec<FragmentEvent>, RecorderError>,
        probe: Option<(
            std::path::PathBuf,
            super::super::media_writer::MediaSegmentProbe,
        )>,
    },
    Failed {
        generation: u64,
        error: RecorderError,
    },
}

pub(super) fn send<T: Serialize>(stream: &mut impl Write, value: &T) -> io::Result<()> {
    let bytes = serde_json::to_vec(value).map_err(io::Error::other)?;
    if bytes.len() > CONTROL_LIMIT {
        return Err(invalid());
    }
    stream.write_all(&(bytes.len() as u32).to_le_bytes())?;
    stream.write_all(&bytes)
}
pub(super) fn receive<T: for<'de> Deserialize<'de>>(stream: &mut impl Read) -> io::Result<T> {
    let mut size = [0; 4];
    stream.read_exact(&mut size)?;
    let size = u32::from_le_bytes(size) as usize;
    if size == 0 || size > CONTROL_LIMIT {
        return Err(invalid());
    }
    let mut bytes = vec![0; size];
    stream.read_exact(&mut bytes)?;
    serde_json::from_slice(&bytes).map_err(io::Error::other)
}
fn invalid() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "Invalid media worker protocol.")
}

pub(super) fn validate_input(input: &MediaSegmentInput) -> io::Result<()> {
    if !input.fps.is_finite() || input.fps <= 0.0 || input.fps > 5.0 {
        return Err(invalid());
    }
    let dimensions = if input.video_sources.is_empty() {
        vec![(input.canvas_width, input.canvas_height)]
    } else {
        input
            .video_sources
            .iter()
            .map(|s| (s.width, s.height))
            .collect()
    };
    // Pending capture, transport in flight, child decode, child queue and
    // appsrc handoff. Reserve two extra frame allocations and socket capacity.
    let bytes = dimensions
        .into_iter()
        .try_fold(0usize, |total, (w, h)| {
            if w == 0 || h == 0 {
                return None;
            }
            total.checked_add(
                (w as usize)
                    .checked_mul(h as usize)?
                    .checked_mul(4)?
                    .checked_mul(7)?,
            )
        })
        .ok_or_else(invalid)?;
    if bytes > FRAME_BUDGET - 2 * 1024 * 1024 {
        return Err(invalid());
    }
    Ok(())
}

pub(super) fn write_frame(
    stream: &mut impl Write,
    generation: u64,
    source: u32,
    sequence: u64,
    frame: CapturedFrame,
) -> io::Result<()> {
    frame.validate().map_err(|_| invalid())?;
    let format = match frame.format {
        CapturedFrameFormat::Yuv444p => 0,
        CapturedFrameFormat::Yuv420p => 1,
        CapturedFrameFormat::Rgbx => 2,
        CapturedFrameFormat::Rgba => 3,
        CapturedFrameFormat::Xrgb => 4,
        CapturedFrameFormat::Argb => 5,
        CapturedFrameFormat::Bgrx => 6,
        CapturedFrameFormat::Bgra => 7,
    };
    for value in [
        MAGIC,
        VERSION,
        source,
        format,
        frame.width,
        frame.height,
        frame.stride,
    ] {
        stream.write_all(&value.to_le_bytes())?;
    }
    for value in [
        generation,
        sequence,
        frame.timestamp_ms,
        frame.data.len() as u64,
    ] {
        stream.write_all(&value.to_le_bytes())?;
    }
    stream.write_all(&frame.data)
}
pub(super) fn read_frame(
    stream: &mut impl Read,
    generation: u64,
    input: &MediaSegmentInput,
    sequences: &mut [u64],
) -> io::Result<CapturedFrame> {
    let mut header = [0u8; 60];
    stream.read_exact(&mut header[..1])?;
    stream.read_exact(&mut header[1..]).map_err(|_| invalid())?;
    let u32_at = |i| u32::from_le_bytes(header[i..i + 4].try_into().expect("header field"));
    let u64_at = |i| u64::from_le_bytes(header[i..i + 8].try_into().expect("header field"));
    let source = u32_at(8) as usize;
    let sequence = u64_at(36);
    if u32_at(0) != MAGIC
        || u32_at(4) != VERSION
        || u64_at(28) != generation
        || source >= sequences.len()
        || sequence <= sequences[source]
    {
        return Err(invalid());
    }
    let width = u32_at(16);
    let height = u32_at(20);
    let stride = u32_at(24);
    let length = usize::try_from(u64_at(52)).map_err(|_| invalid())?;
    let (expected_width, expected_height, source_id) = if input.video_sources.is_empty() {
        (
            input.canvas_width,
            input.canvas_height,
            "composed".to_string(),
        )
    } else {
        let source = &input.video_sources[source];
        (source.width, source.height, source.source_id.clone())
    };
    if width != expected_width
        || height != expected_height
        || stride > width.checked_mul(4).ok_or_else(invalid)?
        || length
            > (width as usize)
                .checked_mul(height as usize)
                .and_then(|n| n.checked_mul(4))
                .ok_or_else(invalid)?
    {
        return Err(invalid());
    }
    let format = match u32_at(12) {
        0 => CapturedFrameFormat::Yuv444p,
        1 => CapturedFrameFormat::Yuv420p,
        2 => CapturedFrameFormat::Rgbx,
        3 => CapturedFrameFormat::Rgba,
        4 => CapturedFrameFormat::Xrgb,
        5 => CapturedFrameFormat::Argb,
        6 => CapturedFrameFormat::Bgrx,
        7 => CapturedFrameFormat::Bgra,
        _ => return Err(invalid()),
    };
    if stride < width
        || u64_at(44) < input.started_at_ms
        || u64_at(44).saturating_sub(input.started_at_ms) > u64::MAX / 1_000_000
    {
        return Err(invalid());
    }
    let mut data = vec![0; length];
    stream.read_exact(&mut data).map_err(|_| invalid())?;
    let frame = CapturedFrame::new(source_id, u64_at(44), width, height, stride, format, data)
        .map_err(|_| invalid())?;
    sequences[source] = sequence;
    Ok(frame)
}
