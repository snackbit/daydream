use std::{collections::BTreeMap, path::Path};

use super::{playable_segments, LibraryTrackKind, LibraryWaveformTrack, LibraryWaveforms};
use crate::{ffmpeg, storage::RecordingRow, storage::RecordingSegmentRow};

mod admission_requests;
mod cache;
mod ffmpeg_source;
pub(crate) mod jobs;
mod math;
mod prepared;
mod pyramid;
#[cfg(test)]
mod tests;
mod tiles;
pub(crate) mod viewport;

use ffmpeg_source::waveform_from_ffmpeg;

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
