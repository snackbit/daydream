use serde::Deserialize;

use super::*;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SegmentProbe {
    audio: Vec<SegmentAudioProbe>,
    video: Option<SegmentVideoProbe>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SegmentAudioProbe {
    codec_name: Option<String>,
    index: i64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SegmentVideoProbe {
    codec_name: Option<String>,
    index: i64,
}

pub(super) fn detail_from_rows(
    recording: &RecordingRow,
    segments: &[RecordingSegmentRow],
    redactions: &[RedactionRow],
) -> LibraryRecordingDetail {
    let summary = summary_from_rows(recording, segments);
    let library_segments = library_segments(segments);
    let tracks = tracks_from_segments(segments);
    let library_redactions = library_redactions(redactions, &tracks);

    LibraryRecordingDetail {
        audio_cleanup: None,
        availability: summary.availability,
        duration_ms: summary.duration_ms,
        ended_at_ms: summary.ended_at_ms,
        id: summary.id,
        missing_reasons: summary.missing_reasons,
        playable_segment_count: summary.playable_segment_count,
        redactions: library_redactions,
        segments: library_segments,
        size_bytes: summary.size_bytes,
        started_at_ms: summary.started_at_ms,
        status: summary.status,
        storage_dir: summary.storage_dir,
        thumbnail_path: summary.thumbnail_path,
        title: summary.title,
        tracks,
    }
}

pub(super) fn detail_for_recording(
    storage: &AppStorage,
    recording_id: &str,
) -> Result<LibraryRecordingDetail, String> {
    let recording = storage
        .with_connection(|connection| storage::get_recording(connection, recording_id))
        .map_err(|error| error.to_string())?;
    let segments = storage
        .with_connection(|connection| storage::list_recording_segments(connection, recording_id))
        .map_err(|error| error.to_string())?;
    let redactions = storage
        .with_connection(|connection| storage::list_redactions(connection, recording_id))
        .map_err(|error| error.to_string())?;

    let mut detail = detail_from_rows(&recording, &segments, &redactions);
    let mappings = storage
        .with_connection(|connection| storage::recording_video_streams(connection, recording_id))
        .map_err(|_| "Video source metadata could not be read.".to_string())?;
    for segment in &mut detail.segments {
        if let Some(streams) = mappings.get(&segment.id) {
            segment.video_streams = streams.clone();
        }
    }
    detail.tracks = source_tracks(&segments, &mappings);
    detail.redactions = library_redactions(&redactions, &detail.tracks);
    detail.audio_cleanup = Some(audio_cleanup::read_state(storage, recording_id)?);
    Ok(detail)
}

pub(super) fn tracks_for_recording(
    connection: &rusqlite::Connection,
    recording_id: &str,
    segments: &[RecordingSegmentRow],
) -> rusqlite::Result<Vec<LibraryRecordingTrack>> {
    let mappings = storage::recording_video_streams(connection, recording_id)?;
    Ok(source_tracks(segments, &mappings))
}

fn source_tracks(
    segments: &[RecordingSegmentRow],
    mappings: &BTreeMap<String, Vec<storage::SegmentVideoStream>>,
) -> Vec<LibraryRecordingTrack> {
    let mut tracks = tracks_from_segments(segments);
    let mut sources = BTreeMap::new();
    for stream in segments
        .iter()
        .filter_map(|segment| mappings.get(&segment.id))
        .flatten()
    {
        sources
            .entry(stream.track_id.clone())
            .or_insert_with(|| LibraryRecordingTrack {
                codec: Some("h264".into()),
                description: if stream.source_id.is_some() {
                    "Independent screen source".into()
                } else {
                    "Composited screen recording".into()
                },
                id: stream.track_id.clone(),
                kind: LibraryTrackKind::Video,
                label: stream.label.clone(),
                stream_index: Some(stream.stream_index),
            });
    }
    if !sources.is_empty() {
        let previous = std::mem::take(&mut tracks);
        tracks = sources.into_values().collect();
        tracks.sort_by_key(|track| track.stream_index);
        tracks.extend(
            previous
                .into_iter()
                .filter(|track| track.kind != LibraryTrackKind::Video),
        );
    }
    tracks
}

pub(super) fn summary_from_rows(
    recording: &RecordingRow,
    segments: &[RecordingSegmentRow],
) -> LibraryRecordingSummary {
    let storage_dir_exists = Path::new(&recording.storage_dir).is_dir();
    let playable_segments = playable_segments(segments);
    let playable_segment_count = playable_segments.len();
    let mut missing_reasons = Vec::new();

    if storage::is_active_recording_status(&recording.status) {
        missing_reasons.push("Recording is still in progress.".to_string());
    } else if !storage_dir_exists {
        missing_reasons.push("Recording directory is missing.".to_string());
    }
    for segment in playable_segments.iter().filter(|segment| {
        !storage::is_active_recording_status(&recording.status)
            && !Path::new(&segment.file_path).is_file()
    }) {
        missing_reasons.push(format!(
            "Segment {} video file is missing.",
            segment.segment_index
        ));
    }

    let availability = if storage::is_active_recording_status(&recording.status) {
        LibraryRecordingAvailability::Locked
    } else if !missing_reasons.is_empty() {
        LibraryRecordingAvailability::MissingFiles
    } else if playable_segment_count == 0 {
        LibraryRecordingAvailability::Incomplete
    } else {
        LibraryRecordingAvailability::Available
    };

    LibraryRecordingSummary {
        availability,
        duration_ms: recording_duration_ms(recording, segments),
        ended_at_ms: recording.ended_at_ms,
        id: recording.id.clone(),
        missing_reasons,
        playable_segment_count,
        segment_count: segments.len(),
        size_bytes: recording_size_bytes(recording, segments),
        started_at_ms: recording.started_at_ms,
        status: recording.status.clone(),
        storage_dir: recording.storage_dir.clone(),
        thumbnail_path: None,
        title: recording.title.clone(),
    }
}

fn library_redactions(
    redactions: &[RedactionRow],
    tracks: &[LibraryRecordingTrack],
) -> Vec<LibraryRecordingRedaction> {
    let media_track_ids = tracks
        .iter()
        .map(|track| track.id.clone())
        .collect::<Vec<_>>();

    redactions
        .iter()
        .map(|redaction| {
            let track_ids = crate::redaction_targets::expanded_redaction_track_ids(
                &redaction.track_ids_json,
                &media_track_ids,
            );

            LibraryRecordingRedaction {
                created_at_ms: redaction.created_at_ms,
                end_offset_ms: redaction.end_offset_ms,
                id: redaction.id.clone(),
                reason: redaction.reason.clone(),
                recording_id: redaction.recording_id.clone(),
                start_offset_ms: redaction.start_offset_ms,
                track_ids,
            }
        })
        .collect()
}

pub(super) fn library_segments(segments: &[RecordingSegmentRow]) -> Vec<LibraryRecordingSegment> {
    let bases = storage::segment_playback_bases(segments);
    let mut output = Vec::with_capacity(segments.len());

    for (index, segment) in segments.iter().enumerate() {
        let next_started_at_ms = segments.get(index + 1).map(|next| next.started_at_ms);
        let pause_after_ms = segment
            .ended_at_ms
            .zip(next_started_at_ms)
            .map(|(ended_at_ms, next_started_at_ms)| next_started_at_ms.saturating_sub(ended_at_ms))
            .unwrap_or(0);

        output.push(LibraryRecordingSegment {
            capture_run_id: segment.capture_run_id.clone(),
            video_streams: Vec::new(),
            desktop_audio_stream_index: segment.desktop_audio_stream_index,
            duration_ms: segment.duration_ms,
            ended_at_ms: segment.ended_at_ms,
            file_exists: Path::new(&segment.file_path).is_file(),
            file_path: segment.file_path.clone(),
            id: segment.id.clone(),
            microphone_audio_stream_index: segment.microphone_audio_stream_index,
            pause_after_ms,
            playback_offset_ms: bases[index],
            segment_index: segment.segment_index,
            started_at_ms: segment.started_at_ms,
            state: segment.state.clone(),
            video_stream_index: segment.video_stream_index,
        });
    }

    output
}

pub(super) fn tracks_from_segments(segments: &[RecordingSegmentRow]) -> Vec<LibraryRecordingTrack> {
    let mut tracks: BTreeMap<(LibraryTrackKind, i64), LibraryRecordingTrack> = BTreeMap::new();

    for segment in playable_segments(segments) {
        let probe = segment_probe(segment);
        if let Some(index) = segment.video_stream_index {
            tracks
                .entry((LibraryTrackKind::Video, index))
                .or_insert_with(|| LibraryRecordingTrack {
                    codec: probe
                        .as_ref()
                        .and_then(|probe| probe.video.as_ref())
                        .and_then(|video| (video.index == index).then(|| video.codec_name.clone()))
                        .flatten(),
                    description: "Composited screen recording".to_string(),
                    id: format!("video-{index}"),
                    kind: LibraryTrackKind::Video,
                    label: "Video".to_string(),
                    stream_index: Some(index),
                });
        }
        if let Some(index) = segment.desktop_audio_stream_index {
            tracks
                .entry((LibraryTrackKind::DesktopAudio, index))
                .or_insert_with(|| {
                    audio_track(
                        "desktop-audio",
                        LibraryTrackKind::DesktopAudio,
                        "Desktop audio",
                        index,
                        probe.as_ref(),
                    )
                });
        }
        if let Some(index) = segment.microphone_audio_stream_index {
            tracks
                .entry((LibraryTrackKind::Microphone, index))
                .or_insert_with(|| {
                    audio_track(
                        "microphone",
                        LibraryTrackKind::Microphone,
                        "Microphone",
                        index,
                        probe.as_ref(),
                    )
                });
        }
    }

    tracks.into_values().collect()
}

fn audio_track(
    id_prefix: &str,
    kind: LibraryTrackKind,
    label: &str,
    index: i64,
    probe: Option<&SegmentProbe>,
) -> LibraryRecordingTrack {
    LibraryRecordingTrack {
        codec: probe
            .and_then(|probe| probe.audio.iter().find(|audio| audio.index == index))
            .and_then(|audio| audio.codec_name.clone()),
        description: format!("{label} stream {index}"),
        id: format!("{id_prefix}-{index}"),
        kind,
        label: label.to_string(),
        stream_index: Some(index),
    }
}

pub(super) fn playable_segments(segments: &[RecordingSegmentRow]) -> Vec<&RecordingSegmentRow> {
    segments
        .iter()
        .filter(|segment| segment.state == "finalized")
        .collect()
}

fn recording_duration_ms(
    _recording: &RecordingRow,
    segments: &[RecordingSegmentRow],
) -> Option<i64> {
    if segments.is_empty() || segments.iter().any(|segment| segment.duration_ms.is_none()) {
        return None;
    }
    let total = playable_segments(segments)
        .iter()
        .filter_map(|segment| segment.duration_ms)
        .sum::<i64>();

    Some(total)
}

pub(super) fn timeline_duration_ms(segments: &[RecordingSegmentRow]) -> i64 {
    library_segments(segments)
        .into_iter()
        .map(|segment| segment.playback_offset_ms + segment.duration_ms.unwrap_or_default())
        .max()
        .unwrap_or_default()
        .max(0)
}

fn segment_probe(segment: &RecordingSegmentRow) -> Option<SegmentProbe> {
    segment
        .probe_json
        .as_ref()
        .and_then(|probe_json| serde_json::from_str(probe_json).ok())
}
