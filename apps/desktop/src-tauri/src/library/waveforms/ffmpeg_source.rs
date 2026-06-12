use std::process::Command;

use super::super::{now_ms, LibraryWaveformSegment};
use super::math::waveform_bins;
use crate::storage::RecordingSegmentRow;

const WAVEFORM_SAMPLE_INTERVAL_MS: usize = 50;

pub(super) fn waveform_from_ffmpeg(
    ffmpeg_path: &str,
    segment: &RecordingSegmentRow,
    stream_index: i64,
) -> Result<LibraryWaveformSegment, String> {
    let output = Command::new(ffmpeg_path)
        .args([
            "-hide_banner",
            "-v",
            "error",
            "-i",
            &segment.file_path,
            "-map",
            &format!("0:{stream_index}"),
            "-ac",
            "1",
            "-ar",
            "100",
            "-f",
            "f32le",
            "pipe:1",
        ])
        .output()
        .map_err(|error| {
            format!(
                "Unable to run FFmpeg at {ffmpeg_path}. Set DAYDREAM_FFMPEG_PATH or install ffmpeg: {error}"
            )
        })?;

    if !output.status.success() {
        return Err(format!(
            "FFmpeg failed while generating waveform: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    let source_samples = output
        .stdout
        .chunks_exact(4)
        .map(|chunk| {
            f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]])
                .abs()
                .min(1.0) as f64
        })
        .collect::<Vec<_>>();
    let duration_ms = segment.duration_ms.unwrap_or(0).max(1) as usize;
    let sample_count = (duration_ms / WAVEFORM_SAMPLE_INTERVAL_MS).clamp(96, 2_400);
    let samples = waveform_bins(&source_samples, sample_count);
    let max_amplitude = samples.iter().copied().fold(0.0, f64::max);
    let normalized_samples = if max_amplitude > f64::EPSILON {
        samples
            .iter()
            .map(|sample| (sample / max_amplitude).min(1.0))
            .collect()
    } else {
        samples
    };

    Ok(LibraryWaveformSegment {
        generated_at_ms: now_ms(),
        max_amplitude,
        samples: normalized_samples,
        segment_id: segment.id.clone(),
        stream_index,
    })
}

pub(super) fn stream_max_amplitude(
    ffmpeg_path: &str,
    segment: &RecordingSegmentRow,
    stream_index: i64,
) -> Result<f64, String> {
    let output = Command::new(ffmpeg_path)
        .args([
            "-hide_banner",
            "-v",
            "info",
            "-i",
            &segment.file_path,
            "-map",
            &format!("0:{stream_index}"),
            "-af",
            "volumedetect",
            "-f",
            "null",
            "-",
        ])
        .output()
        .map_err(|error| {
            format!(
                "Unable to run FFmpeg at {ffmpeg_path}. Set DAYDREAM_FFMPEG_PATH or install ffmpeg: {error}"
            )
        })?;

    if !output.status.success() {
        return Err(format!(
            "FFmpeg failed while measuring waveform scale: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    parse_volumedetect_max_amplitude(&String::from_utf8_lossy(&output.stderr))
        .ok_or_else(|| "FFmpeg did not report waveform max volume.".to_string())
}

pub(super) fn waveform_tile_samples_from_ffmpeg(
    ffmpeg_path: &str,
    segment: &RecordingSegmentRow,
    stream_index: i64,
    tile_start_ms: i64,
    duration_ms: i64,
) -> Result<Vec<f64>, String> {
    let start_seconds = format!("{:.3}", tile_start_ms as f64 / 1000.0);
    let duration_seconds = format!("{:.3}", duration_ms as f64 / 1000.0);
    let output = Command::new(ffmpeg_path)
        .args([
            "-hide_banner",
            "-v",
            "error",
            "-ss",
            &start_seconds,
            "-i",
            &segment.file_path,
            "-t",
            &duration_seconds,
            "-map",
            &format!("0:{stream_index}"),
            "-ac",
            "1",
            "-ar",
            "48000",
            "-f",
            "f32le",
            "pipe:1",
        ])
        .output()
        .map_err(|error| {
            format!(
                "Unable to run FFmpeg at {ffmpeg_path}. Set DAYDREAM_FFMPEG_PATH or install ffmpeg: {error}"
            )
        })?;

    if !output.status.success() {
        return Err(format!(
            "FFmpeg failed while generating waveform tile: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    Ok(output
        .stdout
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]) as f64)
        .collect())
}

pub(crate) fn parse_volumedetect_max_amplitude(output: &str) -> Option<f64> {
    let value = output.lines().find_map(|line| {
        line.split_once("max_volume:")
            .map(|(_prefix, value)| value.trim())
    })?;
    if value.starts_with("-inf") {
        return Some(0.0);
    }
    let db = value.strip_suffix(" dB")?.parse::<f64>().ok()?;

    Some(10_f64.powf(db / 20.0).clamp(0.0, 1.0))
}
