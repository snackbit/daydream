use std::{collections::BTreeMap, path::Path};

use super::{
    library_segments, playable_segments, LibraryRecordingSegment, LibraryTrackKind,
    LibraryWaveformTileRequest, LibraryWaveformTileTrack, LibraryWaveformTiles,
    LibraryWaveformTrack, LibraryWaveforms,
};
use crate::{ffmpeg, storage::RecordingRow, storage::RecordingSegmentRow};

mod cache;
mod ffmpeg_source;
mod math;
mod tiles;

use cache::{
    read_waveform_track_scale_cache, waveform_file_fingerprint, waveform_track_scale_cache_path,
    write_waveform_track_scale_cache, WaveformTrackScaleCache,
};
use ffmpeg_source::{stream_max_amplitude, waveform_from_ffmpeg};
use tiles::{waveform_tile_for_segment, WaveformTileContext};

#[cfg(test)]
pub(super) use ffmpeg_source::parse_volumedetect_max_amplitude;
#[cfg(test)]
pub(super) use math::{waveform_bins, waveform_peak_rms_time_bins};
#[cfg(test)]
pub(super) use tiles::{waveform_tile_bin_ms, waveform_tile_ranges_for_segment};

#[cfg(test)]
pub(super) fn waveforms_for_recording(
    recording: &RecordingRow,
    segments: &[RecordingSegmentRow],
) -> LibraryWaveforms {
    waveforms_with_cleanup(recording, segments, None)
}
pub(super) fn waveforms_with_cleanup(
    recording: &RecordingRow,
    segments: &[RecordingSegmentRow],
    cleanup: Option<&crate::library::audio_cleanup::CleanupManifest>,
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
            let source = cleanup_source(segment, &kind, cleanup);
            let source_stream = if source.file_path != segment.file_path {
                0
            } else {
                stream_index
            };
            let waveform = match waveform_from_ffmpeg(&ffmpeg_path, &source, source_stream) {
                Ok(waveform) => Some(waveform),
                Err(waveform_error) => {
                    error = Some(waveform_error);
                    None
                }
            };
            let Some(mut waveform) = waveform else {
                continue;
            };
            waveform.stream_index = stream_index;
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

pub(super) fn waveform_tiles_with_cleanup(
    recording: &RecordingRow,
    segments: &[RecordingSegmentRow],
    input: &LibraryWaveformTileRequest,
    cleanup: Option<&crate::library::audio_cleanup::CleanupManifest>,
) -> LibraryWaveformTiles {
    let mut error = None;
    let ffmpeg_path = ffmpeg::binary_path();
    let bin_ms = tiles::waveform_tile_bin_ms(input);
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
        let owned_sources: Vec<_> = playable_segments
            .iter()
            .map(|(segment, _)| cleanup_source(segment, &kind, cleanup))
            .collect();
        let track_segments = playable_segments
            .iter()
            .zip(&owned_sources)
            .filter_map(|((segment, library_segment), source)| {
                let stream_index = match kind {
                    LibraryTrackKind::DesktopAudio => segment.desktop_audio_stream_index,
                    LibraryTrackKind::Microphone => segment.microphone_audio_stream_index,
                    LibraryTrackKind::Video => None,
                }?;
                Some((
                    source,
                    library_segment.clone(),
                    if source.file_path != segment.file_path {
                        0
                    } else {
                        stream_index
                    },
                ))
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
            let requested_tiles = tiles::waveform_tile_ranges_for_segment(
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
                    Ok(mut tile) => {
                        if let Some(original) =
                            segments.iter().find(|part| part.id == tile.segment_id)
                        {
                            tile.stream_index = match kind {
                                LibraryTrackKind::Microphone => original
                                    .microphone_audio_stream_index
                                    .unwrap_or(tile.stream_index),
                                _ => tile.stream_index,
                            };
                        }
                        tiles.push(tile)
                    }
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

fn cleanup_source(
    segment: &RecordingSegmentRow,
    kind: &LibraryTrackKind,
    cleanup: Option<&crate::library::audio_cleanup::CleanupManifest>,
) -> RecordingSegmentRow {
    let mut source = segment.clone();
    if *kind == LibraryTrackKind::Microphone {
        if let Some(block) = cleanup.and_then(|cleanup| {
            cleanup
                .blocks
                .iter()
                .find(|block| block.segment_id == segment.id)
        }) {
            source.file_path = block.path.clone();
        }
    }
    source
}
