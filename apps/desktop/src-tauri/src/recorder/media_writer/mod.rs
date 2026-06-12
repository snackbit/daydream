use std::time::Duration;

use gstreamer as gst;

mod audio;
mod capabilities;
mod frame_pump;
mod pipeline;
mod probe;
mod session;
mod types;
mod video_frame;
pub(crate) use capabilities::{collect_media_writer_health, validate_segment_capabilities};
pub(crate) use pipeline::stream_indexes;
pub(crate) use probe::{probe_json_string, probe_segment_best_effort};
#[cfg(test)]
pub(crate) use session::start_synthetic_segment;
pub(crate) use session::{start_segment_from_frames, MediaWriterSegment};
pub(crate) use types::*;

fn gst_framerate_fraction(fps: f64) -> gst::Fraction {
    if !fps.is_finite() || fps <= 0.0 {
        return gst::Fraction::new(1, 1);
    }
    if fps.fract() == 0.0 {
        return gst::Fraction::new(fps.round().clamp(1.0, 60.0) as i32, 1);
    }
    if (fps - 0.5).abs() < f64::EPSILON {
        return gst::Fraction::new(1, 2);
    }
    if (fps - 0.2).abs() < f64::EPSILON {
        return gst::Fraction::new(1, 5);
    }
    if (fps - 0.1).abs() < f64::EPSILON {
        return gst::Fraction::new(1, 10);
    }

    gst::Fraction::new((fps * 1000.0).round().clamp(1.0, 60_000.0) as i32, 1000)
}

#[cfg(test)]
fn gst_fraction(fps: f64) -> String {
    let fraction = gst_framerate_fraction(fps);
    format!("{}/{}", fraction.numer(), fraction.denom())
}

fn key_int_max(fps: f64, seconds: u32) -> u32 {
    (fps * seconds as f64).round().max(1.0) as u32
}

fn duration_to_clock_time(duration: Duration) -> gst::ClockTime {
    gst::ClockTime::from_nseconds(duration.as_nanos().try_into().unwrap_or(u64::MAX))
}

fn launch_value(value: impl AsRef<std::ffi::OsStr>) -> String {
    let value = value.as_ref().to_string_lossy();
    if !value.chars().any(char::is_whitespace) {
        return value.to_string();
    }

    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

#[cfg(test)]
mod tests;
