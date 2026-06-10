use std::{
    collections::{hash_map::DefaultHasher, BTreeMap},
    fs,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    process::Command,
};

use serde::{Deserialize, Serialize};

use super::{
    library_segments, now_ms, path_to_string, playable_segments, system_time_ms,
    LibraryRecordingSegment, LibraryTrackKind, LibraryWaveformSegment, LibraryWaveformTile,
    LibraryWaveformTileRequest, LibraryWaveformTileTrack, LibraryWaveformTiles,
    LibraryWaveformTrack, LibraryWaveforms,
};
use crate::{ffmpeg, storage::RecordingRow, storage::RecordingSegmentRow};

const WAVEFORM_SAMPLE_INTERVAL_MS: usize = 50;
const WAVEFORM_TILE_DURATION_MS: i64 = 15_000;
const WAVEFORM_TILE_CACHE_VERSION: &str = "v2";
const WAVEFORM_TILE_RESOLUTIONS_MS: &[i64] =
    &[5, 10, 25, 50, 100, 250, 500, 1_000, 2_500, 5_000, 10_000];

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct CachedWaveformTile {
    file_fingerprint: WaveformFileFingerprint,
    scale_amplitude: f64,
    tile: LibraryWaveformTile,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct WaveformTrackScaleCache {
    file_fingerprints: Vec<WaveformFileFingerprint>,
    scale_amplitude: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, Hash, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct WaveformFileFingerprint {
    modified_ms: i64,
    path: String,
    size_bytes: u64,
}

pub(super) fn waveforms_for_recording(
    recording: &RecordingRow,
    segments: &[RecordingSegmentRow],
) -> LibraryWaveforms {
    let mut error = None;
    let ffmpeg_path = ffmpeg::binary_path();
    let mut tracks = BTreeMap::<(LibraryTrackKind, String), LibraryWaveformTrack>::new();

    for segment in playable_segments(segments) {
        if !Path::new(&segment.file_path).is_file() {
            continue;
        }
        for (kind, stream_index) in [
            (
                LibraryTrackKind::DesktopAudio,
                segment.desktop_audio_stream_index,
            ),
            (
                LibraryTrackKind::Microphone,
                segment.microphone_audio_stream_index,
            ),
        ] {
            let Some(stream_index) = stream_index else {
                continue;
            };
            let waveform = match waveform_from_ffmpeg(&ffmpeg_path, segment, stream_index) {
                Ok(waveform) => Some(waveform),
                Err(waveform_error) => {
                    error = Some(waveform_error);
                    None
                }
            };
            let Some(waveform) = waveform else {
                continue;
            };
            let track_id = match kind {
                LibraryTrackKind::DesktopAudio => "desktop-audio",
                LibraryTrackKind::Microphone => "microphone",
                LibraryTrackKind::Video => continue,
            }
            .to_string();
            tracks
                .entry((kind.clone(), track_id.clone()))
                .or_insert_with(|| LibraryWaveformTrack {
                    kind,
                    segments: Vec::new(),
                    track_id,
                })
                .segments
                .push(waveform);
        }
    }

    LibraryWaveforms {
        error,
        recording_id: recording.id.clone(),
        tracks: tracks.into_values().collect(),
    }
}

fn waveform_from_ffmpeg(
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

pub(super) fn waveform_tiles_for_recording(
    recording: &RecordingRow,
    segments: &[RecordingSegmentRow],
    input: &LibraryWaveformTileRequest,
) -> LibraryWaveformTiles {
    let mut error = None;
    let ffmpeg_path = ffmpeg::binary_path();
    let bin_ms = waveform_tile_bin_ms(input);
    let request_start_ms = input.start_timeline_ms.max(0);
    let request_end_ms = input.end_timeline_ms.max(request_start_ms + 1);
    let library_segments_by_id = library_segments(segments)
        .into_iter()
        .map(|segment| (segment.id.clone(), segment))
        .collect::<BTreeMap<_, _>>();
    let playable_segments = playable_segments(segments)
        .into_iter()
        .filter(|segment| Path::new(&segment.file_path).is_file())
        .filter_map(|segment| {
            library_segments_by_id
                .get(&segment.id)
                .map(|library_segment| (segment, library_segment.clone()))
        })
        .collect::<Vec<_>>();
    let track_inputs = [
        (LibraryTrackKind::DesktopAudio, "desktop-audio"),
        (LibraryTrackKind::Microphone, "microphone"),
    ];
    let mut tracks = Vec::new();

    for (kind, track_id) in track_inputs {
        let track_segments = playable_segments
            .iter()
            .filter_map(|(segment, library_segment)| {
                let stream_index = match kind {
                    LibraryTrackKind::DesktopAudio => segment.desktop_audio_stream_index,
                    LibraryTrackKind::Microphone => segment.microphone_audio_stream_index,
                    LibraryTrackKind::Video => None,
                }?;
                Some((*segment, library_segment.clone(), stream_index))
            })
            .collect::<Vec<_>>();
        if track_segments.is_empty() {
            continue;
        }

        let scale_amplitude = match waveform_track_scale_amplitude(
            &ffmpeg_path,
            recording,
            track_id,
            &track_segments,
        ) {
            Ok(scale) => scale,
            Err(scale_error) => {
                error = Some(scale_error);
                1.0
            }
        };
        let mut tiles = Vec::new();
        for (segment, library_segment, stream_index) in track_segments {
            let requested_tiles = waveform_tile_ranges_for_segment(
                &library_segment,
                request_start_ms,
                request_end_ms,
            );
            for tile in requested_tiles {
                match waveform_tile_for_segment(WaveformTileContext {
                    bin_ms,
                    ffmpeg_path: &ffmpeg_path,
                    recording,
                    segment,
                    stream_index,
                    segment_playback_offset_ms: library_segment.playback_offset_ms,
                    scale_amplitude,
                    tile,
                }) {
                    Ok(tile) => tiles.push(tile),
                    Err(tile_error) => {
                        error = Some(tile_error);
                    }
                }
            }
        }
        tiles.sort_by_key(|tile| tile.timeline_start_ms);
        tracks.push(LibraryWaveformTileTrack {
            kind,
            scale_amplitude,
            tiles,
            track_id: track_id.to_string(),
        });
    }

    LibraryWaveformTiles {
        bin_ms,
        error,
        recording_id: recording.id.clone(),
        tracks,
    }
}

pub(super) fn waveform_tile_bin_ms(input: &LibraryWaveformTileRequest) -> i64 {
    let visible_ms = input
        .end_timeline_ms
        .saturating_sub(input.start_timeline_ms)
        .max(1) as f64;
    let visible_ms_per_pixel = (visible_ms / input.viewport_width_px.max(1.0)).ceil() as i64;

    WAVEFORM_TILE_RESOLUTIONS_MS
        .iter()
        .rev()
        .copied()
        .find(|resolution| *resolution <= visible_ms_per_pixel)
        .unwrap_or(WAVEFORM_TILE_RESOLUTIONS_MS[0])
}

pub(super) fn waveform_tile_ranges_for_segment(
    segment: &LibraryRecordingSegment,
    request_start_ms: i64,
    request_end_ms: i64,
) -> Vec<(i64, i64)> {
    let segment_duration_ms = segment.duration_ms.unwrap_or(0).max(1);
    let segment_timeline_start_ms = segment.playback_offset_ms;
    let start_offset_ms = request_start_ms
        .saturating_sub(segment_timeline_start_ms)
        .clamp(0, segment_duration_ms);
    let end_offset_ms = request_end_ms
        .saturating_sub(segment_timeline_start_ms)
        .clamp(0, segment_duration_ms);
    if end_offset_ms <= start_offset_ms {
        return Vec::new();
    }

    let mut ranges = Vec::new();
    let mut tile_start_ms =
        (start_offset_ms / WAVEFORM_TILE_DURATION_MS) * WAVEFORM_TILE_DURATION_MS;
    while tile_start_ms < end_offset_ms {
        let duration_ms = WAVEFORM_TILE_DURATION_MS.min(segment_duration_ms - tile_start_ms);
        ranges.push((tile_start_ms, duration_ms));
        tile_start_ms += WAVEFORM_TILE_DURATION_MS;
    }

    ranges
}

fn waveform_track_scale_amplitude(
    ffmpeg_path: &str,
    recording: &RecordingRow,
    track_id: &str,
    track_segments: &[(&RecordingSegmentRow, LibraryRecordingSegment, i64)],
) -> Result<f64, String> {
    let fingerprints = track_segments
        .iter()
        .map(|(segment, _library_segment, _stream_index)| {
            waveform_file_fingerprint(&segment.file_path)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let cache_path = waveform_track_scale_cache_path(recording, track_id, &fingerprints);
    if let Ok(cache) = read_waveform_track_scale_cache(&cache_path) {
        if cache.file_fingerprints == fingerprints && cache.scale_amplitude > 0.0 {
            return Ok(cache.scale_amplitude);
        }
    }

    let mut scale_amplitude = 0.0;
    for (segment, _library_segment, stream_index) in track_segments {
        scale_amplitude = f64::max(
            scale_amplitude,
            stream_max_amplitude(ffmpeg_path, segment, *stream_index)?,
        );
    }
    let scale_amplitude = scale_amplitude.max(f64::EPSILON);
    write_waveform_track_scale_cache(
        &cache_path,
        &WaveformTrackScaleCache {
            file_fingerprints: fingerprints,
            scale_amplitude,
        },
    )?;

    Ok(scale_amplitude)
}

fn stream_max_amplitude(
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

pub(super) fn parse_volumedetect_max_amplitude(output: &str) -> Option<f64> {
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

struct WaveformTileContext<'a> {
    bin_ms: i64,
    ffmpeg_path: &'a str,
    recording: &'a RecordingRow,
    segment: &'a RecordingSegmentRow,
    segment_playback_offset_ms: i64,
    scale_amplitude: f64,
    stream_index: i64,
    tile: (i64, i64),
}

fn waveform_tile_for_segment(
    context: WaveformTileContext<'_>,
) -> Result<LibraryWaveformTile, String> {
    let fingerprint = waveform_file_fingerprint(&context.segment.file_path)?;
    let cache_path = waveform_tile_cache_path(
        context.recording,
        context.segment,
        context.stream_index,
        context.bin_ms,
        context.tile.0,
        &fingerprint,
    );
    if let Ok(cached) = read_cached_waveform_tile(&cache_path) {
        if cached.file_fingerprint == fingerprint
            && cached.scale_amplitude == context.scale_amplitude
        {
            return Ok(cached.tile);
        }
    }

    let tile = generate_waveform_tile(GenerateWaveformTileInput {
        bin_ms: context.bin_ms,
        duration_ms: context.tile.1,
        ffmpeg_path: context.ffmpeg_path,
        scale_amplitude: context.scale_amplitude,
        segment: context.segment,
        segment_playback_offset_ms: context.segment_playback_offset_ms,
        stream_index: context.stream_index,
        tile_start_ms: context.tile.0,
    })?;
    write_cached_waveform_tile(
        &cache_path,
        &CachedWaveformTile {
            file_fingerprint: fingerprint,
            scale_amplitude: context.scale_amplitude,
            tile: tile.clone(),
        },
    )?;

    Ok(tile)
}

struct GenerateWaveformTileInput<'a> {
    bin_ms: i64,
    duration_ms: i64,
    ffmpeg_path: &'a str,
    scale_amplitude: f64,
    segment: &'a RecordingSegmentRow,
    segment_playback_offset_ms: i64,
    stream_index: i64,
    tile_start_ms: i64,
}

fn generate_waveform_tile(
    input: GenerateWaveformTileInput<'_>,
) -> Result<LibraryWaveformTile, String> {
    let GenerateWaveformTileInput {
        bin_ms,
        duration_ms,
        ffmpeg_path,
        scale_amplitude,
        segment,
        segment_playback_offset_ms,
        stream_index,
        tile_start_ms,
    } = input;
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

    let source_samples = output
        .stdout
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]) as f64)
        .collect::<Vec<_>>();
    let sample_count = waveform_tile_sample_count(duration_ms, bin_ms, source_samples.is_empty());
    let (peak_samples, rms_samples) = waveform_peak_rms_bins(&source_samples, sample_count);
    let scale = scale_amplitude.max(f64::EPSILON);
    let peak_samples = peak_samples
        .into_iter()
        .map(|sample| (sample / scale).clamp(0.0, 1.0))
        .collect();
    let rms_samples = rms_samples
        .into_iter()
        .map(|sample| (sample / scale).clamp(0.0, 1.0))
        .collect();

    Ok(LibraryWaveformTile {
        bin_ms,
        duration_ms,
        generated_at_ms: now_ms(),
        peak_samples,
        rms_samples,
        segment_id: segment.id.clone(),
        stream_index,
        timeline_start_ms: segment_playback_offset_ms + tile_start_ms,
    })
}

pub(super) fn waveform_peak_rms_bins(
    source_samples: &[f64],
    sample_count: usize,
) -> (Vec<f64>, Vec<f64>) {
    if source_samples.is_empty() {
        return (vec![0.0; sample_count], vec![0.0; sample_count]);
    }

    let mut peak_samples = Vec::with_capacity(sample_count);
    let mut rms_samples = Vec::with_capacity(sample_count);
    for index in 0..sample_count {
        let start = index * source_samples.len() / sample_count;
        let end = ((index + 1) * source_samples.len() / sample_count)
            .max(start + 1)
            .min(source_samples.len());
        let mut peak = 0.0_f64;
        let mut sum_squares = 0.0_f64;
        for sample in &source_samples[start..end] {
            let amplitude = sample.abs().min(1.0);
            peak = peak.max(amplitude);
            sum_squares += amplitude * amplitude;
        }
        peak_samples.push(peak);
        rms_samples.push((sum_squares / (end - start) as f64).sqrt());
    }

    (peak_samples, rms_samples)
}

pub(super) fn waveform_tile_sample_count(
    duration_ms: i64,
    bin_ms: i64,
    source_is_empty: bool,
) -> usize {
    let sample_count = ((duration_ms.max(1) + bin_ms.max(1) - 1) / bin_ms.max(1)) as usize;
    if source_is_empty {
        sample_count
    } else {
        sample_count.max(2)
    }
}

fn waveform_tile_cache_path(
    recording: &RecordingRow,
    segment: &RecordingSegmentRow,
    stream_index: i64,
    bin_ms: i64,
    tile_start_ms: i64,
    fingerprint: &WaveformFileFingerprint,
) -> PathBuf {
    Path::new(&recording.storage_dir)
        .join("waveforms")
        .join(WAVEFORM_TILE_CACHE_VERSION)
        .join(format!("segment-{}", safe_cache_component(&segment.id)))
        .join(format!("stream-{stream_index}"))
        .join(format!("bin-{bin_ms}"))
        .join(format!(
            "tile-{tile_start_ms}-{}.json",
            hash_cache_key(fingerprint)
        ))
}

fn waveform_track_scale_cache_path(
    recording: &RecordingRow,
    track_id: &str,
    fingerprints: &[WaveformFileFingerprint],
) -> PathBuf {
    Path::new(&recording.storage_dir)
        .join("waveforms")
        .join(WAVEFORM_TILE_CACHE_VERSION)
        .join(format!("scale-{}", safe_cache_component(track_id)))
        .join(format!("{}.json", hash_cache_key(&fingerprints)))
}

fn read_cached_waveform_tile(path: &Path) -> Result<CachedWaveformTile, String> {
    let content = fs::read_to_string(path).map_err(|error| error.to_string())?;
    serde_json::from_str(&content).map_err(|error| error.to_string())
}

fn write_cached_waveform_tile(path: &Path, tile: &CachedWaveformTile) -> Result<(), String> {
    write_json_cache(path, tile)
}

fn read_waveform_track_scale_cache(path: &Path) -> Result<WaveformTrackScaleCache, String> {
    let content = fs::read_to_string(path).map_err(|error| error.to_string())?;
    serde_json::from_str(&content).map_err(|error| error.to_string())
}

fn write_waveform_track_scale_cache(
    path: &Path,
    cache: &WaveformTrackScaleCache,
) -> Result<(), String> {
    write_json_cache(path, cache)
}

fn write_json_cache<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "Unable to create waveform cache directory {}: {error}",
                path_to_string(parent)
            )
        })?;
    }
    let json = serde_json::to_vec(value).map_err(|error| error.to_string())?;
    fs::write(path, json).map_err(|error| {
        format!(
            "Unable to write waveform cache file {}: {error}",
            path_to_string(path)
        )
    })
}

fn waveform_file_fingerprint(path: &str) -> Result<WaveformFileFingerprint, String> {
    let metadata = fs::metadata(path)
        .map_err(|error| format!("Unable to inspect waveform source file {path}: {error}"))?;
    let modified_ms = metadata
        .modified()
        .ok()
        .and_then(system_time_ms)
        .unwrap_or_default();

    Ok(WaveformFileFingerprint {
        modified_ms,
        path: path.to_string(),
        size_bytes: metadata.len(),
    })
}

fn safe_cache_component(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                character
            } else {
                '_'
            }
        })
        .collect()
}

fn hash_cache_key<T: Hash>(value: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

pub(super) fn waveform_bins(source_samples: &[f64], sample_count: usize) -> Vec<f64> {
    if source_samples.is_empty() {
        return vec![0.0; sample_count];
    }

    (0..sample_count)
        .map(|index| {
            let start = index * source_samples.len() / sample_count;
            let end = ((index + 1) * source_samples.len() / sample_count)
                .max(start + 1)
                .min(source_samples.len());
            source_samples[start..end]
                .iter()
                .copied()
                .fold(0.0, f64::max)
        })
        .collect()
}
