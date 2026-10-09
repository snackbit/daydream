use super::super::*;
use crate::timeline_lanes::ACTIVITY_TRACK_IDS;

#[derive(Clone, Debug)]
pub(in crate::ai) struct AudioChunkPlan {
    pub(in crate::ai) end_offset_ms: i64,
    pub(in crate::ai) segment_id: String,
    pub(in crate::ai) source_role: String,
    pub(in crate::ai) start_offset_ms: i64,
}

#[derive(Clone, Debug)]
pub(in crate::ai) struct ActivityChunkPlan {
    pub(in crate::ai) end_offset_ms: i64,
    pub(in crate::ai) segment_id: String,
    pub(in crate::ai) start_offset_ms: i64,
}

#[derive(Clone, Debug)]
pub(in crate::ai) struct VisualChunkPlan {
    pub(in crate::ai) track_id: Option<String>,
    pub(in crate::ai) end_offset_ms: i64,
    pub(in crate::ai) segment_id: String,
    pub(in crate::ai) start_offset_ms: i64,
}

pub(in crate::ai) fn audio_chunk_plans(
    segments: &[RecordingSegmentRow],
    redactions: &[RedactionRow],
) -> Vec<AudioChunkPlan> {
    let bases = storage::segment_playback_bases(segments);
    let mut plans = Vec::new();
    for (index, segment) in segments.iter().enumerate() {
        let playback_offset_ms = bases[index];
        let duration_ms = segment.duration_ms.unwrap_or_else(|| {
            segment
                .ended_at_ms
                .map(|ended_at_ms| ended_at_ms.saturating_sub(segment.started_at_ms))
                .unwrap_or(0)
        });
        if segment.state == "finalized" && Path::new(&segment.file_path).is_file() {
            if segment.desktop_audio_stream_index.is_some() {
                append_audio_slice_plans(
                    &mut plans,
                    segment,
                    "desktop-audio",
                    playback_offset_ms,
                    duration_ms.max(0),
                    redactions,
                );
            }
            if segment.microphone_audio_stream_index.is_some() {
                append_audio_slice_plans(
                    &mut plans,
                    segment,
                    "microphone",
                    playback_offset_ms,
                    duration_ms.max(0),
                    redactions,
                );
            }
        }
    }

    plans
}

pub(in crate::ai) fn activity_chunk_plans(
    segments: &[RecordingSegmentRow],
    app_settings: &settings::AppSettings,
    redactions: &[RedactionRow],
) -> Vec<ActivityChunkPlan> {
    if !app_settings.ai.activity.frame_deltas.enabled
        && !app_settings.ai.activity.keyboard.enabled
        && !app_settings.ai.activity.mouse.enabled
    {
        return Vec::new();
    }

    let bases = storage::segment_playback_bases(segments);
    let mut plans = Vec::new();
    for (index, segment) in segments.iter().enumerate() {
        let playback_offset_ms = bases[index];
        let duration_ms = segment.duration_ms.unwrap_or_else(|| {
            segment
                .ended_at_ms
                .map(|ended_at_ms| ended_at_ms.saturating_sub(segment.started_at_ms))
                .unwrap_or(0)
        });
        if segment.state == "finalized" && Path::new(&segment.file_path).is_file() {
            let start_offset_ms = playback_offset_ms;
            let end_offset_ms = playback_offset_ms + duration_ms.max(0);
            if !redactions_overlap_any_activity(redactions, start_offset_ms, end_offset_ms) {
                let mut start = start_offset_ms;
                while start < end_offset_ms {
                    let end = (start + 30_000).min(end_offset_ms);
                    plans.push(ActivityChunkPlan {
                        end_offset_ms: end,
                        segment_id: segment.id.clone(),
                        start_offset_ms: start,
                    });
                    start = end;
                }
            }
        }
    }

    plans
}

pub(in crate::ai) fn visual_chunk_plans(
    segments: &[RecordingSegmentRow],
    app_settings: &settings::AppSettings,
    redactions: &[RedactionRow],
    source_kind: &str,
) -> Vec<VisualChunkPlan> {
    let enabled = match source_kind {
        "ocr" => app_settings.ai.ocr.enabled,
        "vlm" => app_settings.ai.vision.observations_enabled,
        _ => false,
    };
    if !enabled {
        return Vec::new();
    }

    let bases = storage::segment_playback_bases(segments);
    let mut plans = Vec::new();
    for (index, segment) in segments.iter().enumerate() {
        let playback_offset_ms = bases[index];
        let duration_ms = segment.duration_ms.unwrap_or_else(|| {
            segment
                .ended_at_ms
                .map(|ended_at_ms| ended_at_ms.saturating_sub(segment.started_at_ms))
                .unwrap_or(0)
        });
        if segment.state == "finalized"
            && segment.video_stream_index.is_some()
            && Path::new(&segment.file_path).is_file()
        {
            let start_offset_ms = playback_offset_ms;
            let end_offset_ms = playback_offset_ms + duration_ms.max(0);
            let track_id = segment
                .video_stream_index
                .map(|index| format!("video-{index}"));
            for (start_offset_ms, end_offset_ms) in visible_ranges(
                start_offset_ms,
                end_offset_ms,
                redactions,
                track_id.as_deref(),
            ) {
                let mut start = start_offset_ms;
                while start < end_offset_ms {
                    let end = (start + 30_000).min(end_offset_ms);
                    plans.push(VisualChunkPlan {
                        track_id: None,
                        end_offset_ms: end,
                        segment_id: segment.id.clone(),
                        start_offset_ms: start,
                    });
                    start = end;
                }
            }
        }
    }

    plans
}

pub(in crate::ai) fn append_audio_slice_plans(
    plans: &mut Vec<AudioChunkPlan>,
    segment: &RecordingSegmentRow,
    source_role: &str,
    playback_offset_ms: i64,
    duration_ms: i64,
    redactions: &[RedactionRow],
) {
    let mut slice_start_ms = 0;
    while slice_start_ms < duration_ms {
        let slice_end_ms = (slice_start_ms + MAX_TRANSCRIPTION_CHUNK_DURATION_MS).min(duration_ms);
        let start_offset_ms = playback_offset_ms + slice_start_ms;
        let end_offset_ms = playback_offset_ms + slice_end_ms;
        let track_id = audio_library_track_id(segment, source_role);
        for (start_offset_ms, end_offset_ms) in visible_ranges(
            start_offset_ms,
            end_offset_ms,
            redactions,
            track_id.as_deref(),
        ) {
            plans.push(AudioChunkPlan {
                end_offset_ms,
                segment_id: segment.id.clone(),
                source_role: source_role.to_string(),
                start_offset_ms,
            });
        }
        slice_start_ms = slice_end_ms;
    }
}

fn audio_library_track_id(segment: &RecordingSegmentRow, source_role: &str) -> Option<String> {
    match source_role {
        "desktop-audio" => segment
            .desktop_audio_stream_index
            .map(|index| format!("desktop-audio-{index}")),
        "microphone" => segment
            .microphone_audio_stream_index
            .map(|index| format!("microphone-{index}")),
        _ => None,
    }
}

fn redactions_overlap_any_activity(
    redactions: &[RedactionRow],
    start_offset_ms: i64,
    end_offset_ms: i64,
) -> bool {
    redactions.iter().any(|redaction| {
        redaction_overlaps(redaction, start_offset_ms, end_offset_ms)
            && (redaction_track_ids(redaction).is_empty()
                || ACTIVITY_TRACK_IDS
                    .iter()
                    .any(|track_id| redaction_targets_track(redaction, track_id)))
    })
}

pub(in crate::ai) fn visible_ranges(
    start: i64,
    end: i64,
    redactions: &[RedactionRow],
    track: Option<&str>,
) -> Vec<(i64, i64)> {
    let mut cuts = redactions
        .iter()
        .filter(|redaction| {
            redaction_overlaps(redaction, start, end)
                && track.is_none_or(|track| redaction_targets_track(redaction, track))
        })
        .map(|redaction| {
            (
                redaction.start_offset_ms.max(start),
                redaction.end_offset_ms.min(end),
            )
        })
        .collect::<Vec<_>>();
    cuts.sort_unstable();
    let mut cursor = start;
    let mut ranges = Vec::new();
    for (from, to) in cuts {
        if from > cursor {
            ranges.push((cursor, from));
        }
        cursor = cursor.max(to);
    }
    if cursor < end {
        ranges.push((cursor, end));
    }
    ranges
}

fn redaction_overlaps(redaction: &RedactionRow, start_offset_ms: i64, end_offset_ms: i64) -> bool {
    start_offset_ms < redaction.end_offset_ms && end_offset_ms > redaction.start_offset_ms
}

fn redaction_targets_track(redaction: &RedactionRow, track_id: &str) -> bool {
    let track_ids = redaction_track_ids(redaction);
    track_ids.is_empty() || track_ids.iter().any(|candidate| candidate == track_id)
}

fn redaction_track_ids(redaction: &RedactionRow) -> Vec<String> {
    serde_json::from_str::<Vec<String>>(&redaction.track_ids_json)
        .unwrap_or_default()
        .into_iter()
        .map(|track_id| track_id.trim().to_string())
        .filter(|track_id| !track_id.is_empty())
        .collect()
}

pub(in crate::ai) fn playback_offset_for_segment(
    segments: &[RecordingSegmentRow],
    segment_id: &str,
) -> Option<i64> {
    let index = segments
        .iter()
        .position(|segment| segment.id == segment_id)?;
    storage::segment_playback_bases(segments)
        .get(index)
        .copied()
}

pub(in crate::ai) fn ensure_audio_track(
    connection: &rusqlite::Connection,
    recording_id: &str,
    kind: &str,
    label: &str,
    timestamp_ms: i64,
) -> rusqlite::Result<RecordingTrackRow> {
    if let Some(track) = storage::get_recording_track_by_kind(connection, recording_id, kind)? {
        return Ok(track);
    }
    storage::upsert_recording_track(
        connection,
        &UpsertRecordingTrackInput {
            codec: None,
            created_at_ms: timestamp_ms,
            id: Uuid::now_v7().to_string(),
            kind: kind.to_string(),
            label: label.to_string(),
            path: None,
            recording_id: recording_id.to_string(),
            source_id: None,
        },
    )
}

#[cfg(test)]
mod privacy_slice_tests {
    use super::*;
    #[test]
    fn a_small_cut_does_not_discard_the_rest_of_a_processing_window() {
        let cuts = vec![RedactionRow {
            id: "synthetic".into(),
            recording_id: "synthetic".into(),
            created_at_ms: 0,
            reason: None,
            start_offset_ms: 1000,
            end_offset_ms: 2000,
            track_ids_json: "[\"video-0\"]".into(),
        }];
        assert_eq!(
            visible_ranges(0, 30_000, &cuts, Some("video-0")),
            vec![(0, 1000), (2000, 30_000)]
        );
        assert_eq!(
            visible_ranges(0, 30_000, &cuts, Some("microphone-2")),
            vec![(0, 30_000)]
        );
    }
}
