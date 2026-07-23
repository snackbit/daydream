use super::super::*;
use crate::ai::types::{
    ActivityCaptureState, ActivityWindowSnapshot, ActivityWindowSnapshotWindow,
};
use crate::redaction_targets::expanded_redaction_track_ids;
use crate::timeline_lanes::{
    media_track_ids_from_segments, ACTIVITY_FOCUS_TRACK_ID, ACTIVITY_KEYBOARD_TRACK_ID,
    ACTIVITY_MOUSE_TRACK_ID,
};

pub(in crate::ai) fn activity_for_recording(
    storage: &AppStorage,
    recording_id: &str,
) -> Result<RecordingActivity, AiError> {
    let (intervals, focus_events, window_events, capture_state_events, segments, redactions) =
        storage
            .with_connection(|connection| {
                Ok((
                    storage::list_activity_intervals(connection, recording_id)?,
                    storage::list_capture_events(
                        connection,
                        recording_id,
                        &["focused-window"],
                        0,
                        i64::MAX,
                    )?,
                    storage::list_capture_events(
                        connection,
                        recording_id,
                        &["open-windows"],
                        0,
                        i64::MAX,
                    )?,
                    storage::list_capture_events(
                        connection,
                        recording_id,
                        &["capture-state"],
                        0,
                        i64::MAX,
                    )?,
                    storage::list_recording_segments(connection, recording_id)?,
                    storage::list_redactions(connection, recording_id)?,
                ))
            })
            .map_err(storage_error)?;
    let media_track_ids = media_track_ids_from_segments(&segments);
    Ok(RecordingActivity {
        capture_states: visible_capture_states(capture_state_events, &redactions, &media_track_ids),
        events: visible_activity_events(focus_events, &redactions, &media_track_ids),
        intervals: intervals
            .into_iter()
            .flat_map(|row| visible_activity_intervals(row, &redactions, &media_track_ids))
            .collect(),
        recording_id: recording_id.to_string(),
        window_snapshots: visible_window_snapshots(window_events, &redactions, &media_track_ids),
    })
}

fn visible_activity_intervals(
    row: ActivityIntervalRow,
    redactions: &[RedactionRow],
    media_track_ids: &[String],
) -> Vec<ActivityInterval> {
    let Some(track_id) = activity_track_id_for_source(&row.source_kind) else {
        let start_offset_ms = row.start_offset_ms;
        let end_offset_ms = row.end_offset_ms;
        return vec![activity_interval_from_row(
            row,
            start_offset_ms,
            end_offset_ms,
        )];
    };
    subtract_activity_redactions(
        row.start_offset_ms,
        row.end_offset_ms,
        track_id,
        redactions,
        media_track_ids,
    )
    .into_iter()
    .map(|(start_offset_ms, end_offset_ms)| {
        activity_interval_from_row(row.clone(), start_offset_ms, end_offset_ms)
    })
    .collect()
}

fn activity_interval_from_row(
    row: ActivityIntervalRow,
    start_offset_ms: i64,
    end_offset_ms: i64,
) -> ActivityInterval {
    ActivityInterval {
        end_offset_ms,
        id: row.id,
        job_id: row.job_id,
        recording_id: row.recording_id,
        score: row.score,
        source_kind: row.source_kind,
        start_offset_ms,
        state: row.state,
    }
}

fn visible_capture_states(
    events: Vec<CaptureEventRow>,
    redactions: &[RedactionRow],
    media_track_ids: &[String],
) -> Vec<ActivityCaptureState> {
    events
        .into_iter()
        .filter(capture_state_is_input_activity)
        .filter(|event| {
            let mouse_visible = !redactions.iter().any(|redaction| {
                redaction_hides_activity_point(
                    redaction,
                    ACTIVITY_MOUSE_TRACK_ID,
                    event.offset_ms,
                    media_track_ids,
                )
            });
            let keyboard_visible = !redactions.iter().any(|redaction| {
                redaction_hides_activity_point(
                    redaction,
                    ACTIVITY_KEYBOARD_TRACK_ID,
                    event.offset_ms,
                    media_track_ids,
                )
            });

            mouse_visible || keyboard_visible
        })
        .map(activity_capture_state_from_capture_event)
        .collect()
}

fn capture_state_is_input_activity(event: &CaptureEventRow) -> bool {
    serde_json::from_str::<serde_json::Value>(&event.payload_json)
        .ok()
        .and_then(|payload| string_field(&payload, &["source"]))
        .is_none_or(|source| source == "evdev")
}

fn activity_capture_state_from_capture_event(row: CaptureEventRow) -> ActivityCaptureState {
    let payload = serde_json::from_str::<serde_json::Value>(&row.payload_json).ok();

    ActivityCaptureState {
        id: row.id,
        offset_ms: row.offset_ms,
        reason: payload
            .as_ref()
            .and_then(|payload| string_field(payload, &["reason"])),
        recording_id: row.recording_id,
        source: payload
            .as_ref()
            .and_then(|payload| string_field(payload, &["source"])),
        status: payload
            .as_ref()
            .and_then(|payload| string_field(payload, &["status"])),
    }
}

fn visible_activity_events(
    events: Vec<CaptureEventRow>,
    redactions: &[RedactionRow],
    media_track_ids: &[String],
) -> Vec<ActivityEvent> {
    events
        .into_iter()
        .filter(|event| {
            !redactions.iter().any(|redaction| {
                redaction_hides_activity_point(
                    redaction,
                    ACTIVITY_FOCUS_TRACK_ID,
                    event.offset_ms,
                    media_track_ids,
                )
            })
        })
        .map(activity_event_from_capture_event)
        .collect()
}

fn visible_window_snapshots(
    events: Vec<CaptureEventRow>,
    redactions: &[RedactionRow],
    media_track_ids: &[String],
) -> Vec<ActivityWindowSnapshot> {
    events
        .into_iter()
        .filter(|event| {
            !redactions.iter().any(|redaction| {
                redaction_hides_activity_point(
                    redaction,
                    ACTIVITY_FOCUS_TRACK_ID,
                    event.offset_ms,
                    media_track_ids,
                )
            })
        })
        .map(window_snapshot_from_capture_event)
        .collect()
}

fn window_snapshot_from_capture_event(row: CaptureEventRow) -> ActivityWindowSnapshot {
    let payload = serde_json::from_str::<serde_json::Value>(&row.payload_json).ok();
    let windows = payload
        .as_ref()
        .and_then(|payload| payload.get("windows"))
        .and_then(|windows| windows.as_array())
        .map(|windows| {
            windows
                .iter()
                .filter_map(activity_window_from_value)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    ActivityWindowSnapshot {
        focused_window_id: payload
            .as_ref()
            .and_then(|payload| string_field(payload, &["focusedWindowId"])),
        id: row.id,
        occurred_at_ms: row.occurred_at_ms,
        offset_ms: row.offset_ms,
        provider: payload
            .as_ref()
            .and_then(|payload| string_field(payload, &["provider"])),
        recording_id: row.recording_id,
        windows,
    }
}

fn activity_window_from_value(value: &serde_json::Value) -> Option<ActivityWindowSnapshotWindow> {
    Some(ActivityWindowSnapshotWindow {
        app_identity: string_field(value, &["appIdentity"]),
        id: string_field(value, &["id"])?,
        process_id: value
            .get("processId")
            .and_then(|value| value.as_u64())
            .and_then(|value| u32::try_from(value).ok()),
        process_name: string_field(value, &["processName"]),
        title: string_field(value, &["title"]),
    })
}

fn activity_event_from_capture_event(row: CaptureEventRow) -> ActivityEvent {
    let payload = serde_json::from_str::<serde_json::Value>(&row.payload_json).ok();

    ActivityEvent {
        application_name: payload.as_ref().and_then(|payload| {
            string_field(payload, &["applicationName", "appName", "processName"])
        }),
        id: row.id,
        kind: "focused-window".to_string(),
        occurred_at_ms: row.occurred_at_ms,
        offset_ms: row.offset_ms,
        recording_id: row.recording_id,
        title: payload
            .as_ref()
            .and_then(|payload| string_field(payload, &["title", "windowTitle"])),
    }
}

fn string_field(payload: &serde_json::Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| payload.get(key)?.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn activity_track_id_for_source(source_kind: &str) -> Option<&'static str> {
    match source_kind {
        "keyboard" => Some(ACTIVITY_KEYBOARD_TRACK_ID),
        "mouse" => Some(ACTIVITY_MOUSE_TRACK_ID),
        _ => None,
    }
}

fn subtract_activity_redactions(
    start_offset_ms: i64,
    end_offset_ms: i64,
    track_id: &str,
    redactions: &[RedactionRow],
    media_track_ids: &[String],
) -> Vec<(i64, i64)> {
    let mut visible = vec![(start_offset_ms, end_offset_ms)];
    for redaction in redactions {
        if !redaction_hides_activity_range(
            redaction,
            track_id,
            start_offset_ms,
            end_offset_ms,
            media_track_ids,
        ) {
            continue;
        }
        visible = visible
            .into_iter()
            .flat_map(|(start, end)| subtract_range(start, end, redaction))
            .collect();
    }

    visible
        .into_iter()
        .filter(|(start, end)| end > start)
        .collect()
}

fn subtract_range(start: i64, end: i64, redaction: &RedactionRow) -> Vec<(i64, i64)> {
    if end <= redaction.start_offset_ms || start >= redaction.end_offset_ms {
        return vec![(start, end)];
    }

    let mut ranges = Vec::new();
    if start < redaction.start_offset_ms {
        ranges.push((start, redaction.start_offset_ms.min(end)));
    }
    if end > redaction.end_offset_ms {
        ranges.push((redaction.end_offset_ms.max(start), end));
    }

    ranges
}

fn redaction_hides_activity_range(
    redaction: &RedactionRow,
    track_id: &str,
    start_offset_ms: i64,
    end_offset_ms: i64,
    media_track_ids: &[String],
) -> bool {
    start_offset_ms < redaction.end_offset_ms
        && end_offset_ms > redaction.start_offset_ms
        && redaction_targets_track(redaction, track_id, media_track_ids)
}

fn redaction_hides_activity_point(
    redaction: &RedactionRow,
    track_id: &str,
    offset_ms: i64,
    media_track_ids: &[String],
) -> bool {
    offset_ms >= redaction.start_offset_ms
        && offset_ms < redaction.end_offset_ms
        && redaction_targets_track(redaction, track_id, media_track_ids)
}

fn redaction_targets_track(
    redaction: &RedactionRow,
    track_id: &str,
    media_track_ids: &[String],
) -> bool {
    expanded_redaction_track_ids(&redaction.track_ids_json, media_track_ids)
        .iter()
        .any(|candidate| candidate == track_id)
}
