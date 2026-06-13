use super::*;
use crate::{redaction_targets, timeline_lanes::ACTIVITY_TRACK_IDS};

#[derive(Clone, Debug, Eq, PartialEq)]
struct RedactionInterval {
    end_offset_ms: i64,
    start_offset_ms: i64,
}

pub(super) fn build_normalized_redaction_inputs(
    recording_id: &str,
    ranges: &[CreateLibraryRedactionRangeInput],
    existing_redactions: &[RedactionRow],
    tracks: &[LibraryRecordingTrack],
    duration_ms: i64,
) -> Result<Vec<storage::CreateRedactionInput>, String> {
    if ranges.is_empty() {
        return Err("At least one redaction range is required.".to_string());
    }
    let valid_track_ids = tracks
        .iter()
        .map(|track| track.id.as_str())
        .chain(ACTIVITY_TRACK_IDS)
        .collect::<BTreeSet<_>>();
    if valid_track_ids.is_empty() {
        return Err("Recording has no playable tracks to redact.".to_string());
    }
    let timestamp_ms = now_ms();
    let media_track_ids = tracks
        .iter()
        .map(|track| track.id.clone())
        .collect::<Vec<_>>();
    let mut intervals_by_track: BTreeMap<String, Vec<RedactionInterval>> = BTreeMap::new();

    for redaction in existing_redactions {
        if redaction.end_offset_ms <= redaction.start_offset_ms {
            continue;
        }
        let track_ids = redaction_targets::expanded_redaction_track_ids(
            &redaction.track_ids_json,
            &media_track_ids,
        );
        for track_id in track_ids
            .into_iter()
            .filter(|track_id| valid_track_ids.contains(track_id.as_str()))
        {
            intervals_by_track
                .entry(track_id)
                .or_default()
                .push(RedactionInterval {
                    end_offset_ms: redaction.end_offset_ms,
                    start_offset_ms: redaction.start_offset_ms,
                });
        }
    }

    for range in ranges {
        if range.end_offset_ms <= range.start_offset_ms {
            return Err("Redaction range must have positive duration.".to_string());
        }
        if range.start_offset_ms < 0 || range.end_offset_ms > duration_ms {
            return Err(format!(
                "Redaction range must be within recording duration {} ms.",
                duration_ms.max(0)
            ));
        }
        let track_ids = redaction_targets::normalized_track_ids(&range.track_ids);
        if track_ids.is_empty() {
            return Err("Redaction requires at least one target track.".to_string());
        }
        if let Some(track_id) = track_ids
            .iter()
            .find(|track_id| !valid_track_ids.contains(track_id.as_str()))
        {
            return Err(format!("Redaction target track {track_id} does not exist."));
        }

        for track_id in track_ids {
            intervals_by_track
                .entry(track_id)
                .or_default()
                .push(RedactionInterval {
                    end_offset_ms: range.end_offset_ms,
                    start_offset_ms: range.start_offset_ms,
                });
        }
    }

    let mut track_ids_by_interval: BTreeMap<(i64, i64), Vec<String>> = BTreeMap::new();
    for (track_id, intervals) in intervals_by_track {
        for interval in merge_redaction_intervals(intervals) {
            track_ids_by_interval
                .entry((interval.start_offset_ms, interval.end_offset_ms))
                .or_default()
                .push(track_id.clone());
        }
    }

    track_ids_by_interval
        .into_iter()
        .map(|((start_offset_ms, end_offset_ms), track_ids)| {
            Ok(storage::CreateRedactionInput {
                created_at_ms: timestamp_ms,
                end_offset_ms,
                id: Uuid::now_v7().to_string(),
                reason: None,
                recording_id: recording_id.to_string(),
                start_offset_ms,
                track_ids_json: serde_json::to_string(&track_ids)
                    .map_err(|error| error.to_string())?,
            })
        })
        .collect()
}

pub(super) fn build_unredacted_redaction_inputs(
    recording_id: &str,
    ranges: &[CreateLibraryRedactionRangeInput],
    existing_redactions: &[RedactionRow],
    tracks: &[LibraryRecordingTrack],
    duration_ms: i64,
) -> Result<Vec<storage::CreateRedactionInput>, String> {
    if ranges.is_empty() {
        return Err("At least one unredaction range is required.".to_string());
    }
    let valid_track_ids = tracks
        .iter()
        .map(|track| track.id.as_str())
        .chain(ACTIVITY_TRACK_IDS)
        .collect::<BTreeSet<_>>();
    if valid_track_ids.is_empty() {
        return Err("Recording has no playable tracks to unredact.".to_string());
    }
    let media_track_ids = tracks
        .iter()
        .map(|track| track.id.clone())
        .collect::<Vec<_>>();
    let mut intervals_by_track: BTreeMap<String, Vec<RedactionInterval>> = BTreeMap::new();

    for redaction in existing_redactions {
        if redaction.end_offset_ms <= redaction.start_offset_ms {
            continue;
        }
        let track_ids = redaction_targets::expanded_redaction_track_ids(
            &redaction.track_ids_json,
            &media_track_ids,
        );
        for track_id in track_ids
            .into_iter()
            .filter(|track_id| valid_track_ids.contains(track_id.as_str()))
        {
            intervals_by_track
                .entry(track_id)
                .or_default()
                .push(RedactionInterval {
                    end_offset_ms: redaction.end_offset_ms,
                    start_offset_ms: redaction.start_offset_ms,
                });
        }
    }

    for range in ranges {
        if range.end_offset_ms <= range.start_offset_ms {
            return Err("Unredaction range must have positive duration.".to_string());
        }
        if range.start_offset_ms < 0 || range.end_offset_ms > duration_ms {
            return Err(format!(
                "Unredaction range must be within recording duration {} ms.",
                duration_ms.max(0)
            ));
        }
        let track_ids = redaction_targets::normalized_track_ids(&range.track_ids);
        if track_ids.is_empty() {
            return Err("Unredaction requires at least one target track.".to_string());
        }
        if let Some(track_id) = track_ids
            .iter()
            .find(|track_id| !valid_track_ids.contains(track_id.as_str()))
        {
            return Err(format!(
                "Unredaction target track {track_id} does not exist."
            ));
        }

        let removal = RedactionInterval {
            end_offset_ms: range.end_offset_ms,
            start_offset_ms: range.start_offset_ms,
        };
        for track_id in track_ids {
            let remaining = subtract_redaction_interval(
                intervals_by_track.remove(&track_id).unwrap_or_default(),
                &removal,
            );
            if !remaining.is_empty() {
                intervals_by_track.insert(track_id, remaining);
            }
        }
    }

    redaction_inputs_from_intervals(recording_id, intervals_by_track)
}

fn merge_redaction_intervals(mut intervals: Vec<RedactionInterval>) -> Vec<RedactionInterval> {
    intervals.sort_by(|left, right| {
        left.start_offset_ms
            .cmp(&right.start_offset_ms)
            .then(left.end_offset_ms.cmp(&right.end_offset_ms))
    });
    let mut merged: Vec<RedactionInterval> = Vec::new();

    for interval in intervals {
        let Some(current) = merged.last_mut() else {
            merged.push(interval);
            continue;
        };
        if interval.start_offset_ms <= current.end_offset_ms {
            current.end_offset_ms = current.end_offset_ms.max(interval.end_offset_ms);
            continue;
        }

        merged.push(interval);
    }

    merged
}

fn subtract_redaction_interval(
    intervals: Vec<RedactionInterval>,
    removal: &RedactionInterval,
) -> Vec<RedactionInterval> {
    let mut remaining = Vec::new();

    for interval in intervals {
        if removal.end_offset_ms <= interval.start_offset_ms
            || removal.start_offset_ms >= interval.end_offset_ms
        {
            remaining.push(interval);
            continue;
        }

        if removal.start_offset_ms > interval.start_offset_ms {
            remaining.push(RedactionInterval {
                end_offset_ms: removal.start_offset_ms.min(interval.end_offset_ms),
                start_offset_ms: interval.start_offset_ms,
            });
        }
        if removal.end_offset_ms < interval.end_offset_ms {
            remaining.push(RedactionInterval {
                end_offset_ms: interval.end_offset_ms,
                start_offset_ms: removal.end_offset_ms.max(interval.start_offset_ms),
            });
        }
    }

    merge_redaction_intervals(remaining)
}

fn redaction_inputs_from_intervals(
    recording_id: &str,
    intervals_by_track: BTreeMap<String, Vec<RedactionInterval>>,
) -> Result<Vec<storage::CreateRedactionInput>, String> {
    let timestamp_ms = now_ms();
    let mut track_ids_by_interval: BTreeMap<(i64, i64), Vec<String>> = BTreeMap::new();
    for (track_id, intervals) in intervals_by_track {
        for interval in merge_redaction_intervals(intervals) {
            track_ids_by_interval
                .entry((interval.start_offset_ms, interval.end_offset_ms))
                .or_default()
                .push(track_id.clone());
        }
    }

    track_ids_by_interval
        .into_iter()
        .map(|((start_offset_ms, end_offset_ms), track_ids)| {
            Ok(storage::CreateRedactionInput {
                created_at_ms: timestamp_ms,
                end_offset_ms,
                id: Uuid::now_v7().to_string(),
                reason: None,
                recording_id: recording_id.to_string(),
                start_offset_ms,
                track_ids_json: serde_json::to_string(&track_ids)
                    .map_err(|error| error.to_string())?,
            })
        })
        .collect()
}

pub(super) fn ensure_recording_allows_redaction(recording: &RecordingRow) -> Result<(), String> {
    if storage::is_active_recording_status(&recording.status) {
        return Err(
            "Recording is still controlled by the recorder and cannot be redacted yet.".to_string(),
        );
    }

    Ok(())
}

#[cfg(test)]
pub(super) fn parse_redaction_track_ids(track_ids_json: &str) -> Option<Vec<String>> {
    redaction_targets::parse_redaction_track_ids(track_ids_json)
}
