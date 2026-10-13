#[cfg(test)]
use super::super::LibraryRecordingSegment;
use super::super::LibraryWaveformTileRequest;

#[cfg(test)]
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

#[cfg(test)]
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
