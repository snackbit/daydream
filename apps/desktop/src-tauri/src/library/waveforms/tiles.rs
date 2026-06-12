use super::super::{
    now_ms, LibraryRecordingSegment, LibraryWaveformTile, LibraryWaveformTileRequest,
};
use super::{
    cache::{
        read_cached_waveform_tile, waveform_file_fingerprint, waveform_tile_cache_path,
        write_cached_waveform_tile, CachedWaveformTile,
    },
    ffmpeg_source::waveform_tile_samples_from_ffmpeg,
    math::{waveform_peak_rms_bins, waveform_tile_sample_count},
};
use crate::storage::{RecordingRow, RecordingSegmentRow};

const WAVEFORM_TILE_DURATION_MS: i64 = 15_000;
const WAVEFORM_TILE_RESOLUTIONS_MS: &[i64] =
    &[5, 10, 25, 50, 100, 250, 500, 1_000, 2_500, 5_000, 10_000];

pub(crate) fn waveform_tile_bin_ms(input: &LibraryWaveformTileRequest) -> i64 {
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

pub(crate) fn waveform_tile_ranges_for_segment(
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

pub(super) struct WaveformTileContext<'a> {
    pub(super) bin_ms: i64,
    pub(super) ffmpeg_path: &'a str,
    pub(super) recording: &'a RecordingRow,
    pub(super) segment: &'a RecordingSegmentRow,
    pub(super) segment_playback_offset_ms: i64,
    pub(super) scale_amplitude: f64,
    pub(super) stream_index: i64,
    pub(super) tile: (i64, i64),
}

pub(super) fn waveform_tile_for_segment(
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
    let source_samples = waveform_tile_samples_from_ffmpeg(
        ffmpeg_path,
        segment,
        stream_index,
        tile_start_ms,
        duration_ms,
    )?;
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
