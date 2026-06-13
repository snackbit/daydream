use super::super::*;

pub(in crate::ai) async fn process_activity_chunk_inner(
    app: &AppHandle,
    chunk: &ProcessingChunkRow,
    cancel: Arc<AtomicBool>,
) -> Result<(), AiError> {
    let storage = app.state::<AppStorage>();
    let app_settings = settings::load_app_settings(&storage).map_err(settings_error)?;
    let chunk_start = chunk.start_offset_ms.ok_or_else(|| {
        ai_error(
            "ai_processing_chunk_offset_missing",
            "Processing chunk start offset is missing.",
        )
    })?;
    let chunk_end = chunk.end_offset_ms.ok_or_else(|| {
        ai_error(
            "ai_processing_chunk_offset_missing",
            "Processing chunk end offset is missing.",
        )
    })?;
    if chunk_end <= chunk_start {
        return Err(ai_error(
            "ai_processing_chunk_duration_invalid",
            "Processing chunk duration is invalid.",
        ));
    }

    check_processing_cancel(&cancel)?;
    let (segment, segment_playback_offset_ms, events) = storage
        .with_connection(|connection| {
            let segment_id = chunk
                .segment_id
                .as_deref()
                .ok_or(rusqlite::Error::InvalidQuery)?;
            let segments = storage::list_recording_segments(connection, &chunk.recording_id)?;
            let segment_playback_offset_ms = playback_offset_for_segment(&segments, segment_id)
                .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
            let segment = segments
                .into_iter()
                .find(|candidate| candidate.id == segment_id)
                .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
            let events = storage::list_capture_events(
                connection,
                &chunk.recording_id,
                &["keyboard-activity", "mouse-activity", "capture-state"],
                chunk_start,
                chunk_end,
            )?;
            Ok((segment, segment_playback_offset_ms, events))
        })
        .map_err(storage_error)?;

    let mut intervals = Vec::new();
    if app_settings.ai.activity.keyboard.enabled {
        intervals.extend(activity_intervals_from_events(
            "keyboard",
            "keyboard-activity",
            &events,
            chunk_start,
            chunk_end,
            app_settings.ai.activity.keyboard.window_ms.into(),
        ));
    }
    if app_settings.ai.activity.mouse.enabled {
        intervals.extend(activity_intervals_from_events(
            "mouse",
            "mouse-activity",
            &events,
            chunk_start,
            chunk_end,
            app_settings.ai.activity.mouse.window_ms.into(),
        ));
    }
    check_processing_cancel(&cancel)?;
    if app_settings.ai.activity.frame_deltas.enabled {
        intervals.extend(frame_delta_intervals(
            &segment,
            segment_playback_offset_ms,
            chunk_start,
            chunk_end,
            &cancel,
        )?);
    }
    if !intervals.is_empty() {
        intervals.extend(merge_activity_intervals(&intervals, chunk_start, chunk_end));
    }

    storage
        .with_connection(|connection| {
            storage::delete_activity_output_for_chunks(
                connection,
                std::slice::from_ref(&chunk.id),
            )?;
            for interval in intervals {
                storage::create_activity_interval(
                    connection,
                    &CreateActivityIntervalInput {
                        chunk_id: Some(chunk.id.clone()),
                        end_offset_ms: interval.end_offset_ms,
                        id: Uuid::now_v7().to_string(),
                        job_id: chunk.job_id.clone(),
                        recording_id: chunk.recording_id.clone(),
                        regions_json: interval.regions_json,
                        score: interval.score,
                        source_kind: interval.source_kind,
                        start_offset_ms: interval.start_offset_ms,
                        state: interval.state,
                    },
                )?;
            }
            Ok(())
        })
        .map_err(storage_error)?;

    Ok(())
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::ai) struct PlannedActivityInterval {
    pub(in crate::ai) end_offset_ms: i64,
    pub(in crate::ai) regions_json: Option<String>,
    pub(in crate::ai) score: Option<f64>,
    pub(in crate::ai) source_kind: String,
    pub(in crate::ai) start_offset_ms: i64,
    pub(in crate::ai) state: String,
}

pub(in crate::ai) fn activity_intervals_from_events(
    source_kind: &str,
    event_kind: &str,
    events: &[CaptureEventRow],
    chunk_start: i64,
    chunk_end: i64,
    window_ms: i64,
) -> Vec<PlannedActivityInterval> {
    if input_activity_unavailable(events) {
        return vec![PlannedActivityInterval {
            end_offset_ms: chunk_end,
            regions_json: None,
            score: None,
            source_kind: source_kind.to_string(),
            start_offset_ms: chunk_start,
            state: "unknown".to_string(),
        }];
    }
    let mut windows = events
        .iter()
        .filter(|event| event.kind == event_kind)
        .map(|event| (event.offset_ms, event.offset_ms.saturating_add(window_ms)))
        .collect::<Vec<_>>();
    windows.sort_unstable();
    intervals_from_active_windows(source_kind, chunk_start, chunk_end, windows)
}

fn intervals_from_active_windows(
    source_kind: &str,
    chunk_start: i64,
    chunk_end: i64,
    windows: Vec<(i64, i64)>,
) -> Vec<PlannedActivityInterval> {
    let mut intervals = Vec::new();
    let mut cursor = chunk_start;
    for (start, end) in merge_windows(windows, chunk_start, chunk_end) {
        if cursor < start {
            intervals.push(planned_interval(
                source_kind,
                "idle",
                cursor,
                start,
                Some(0.0),
                None,
            ));
        }
        intervals.push(planned_interval(
            source_kind,
            "active",
            start,
            end,
            Some(1.0),
            None,
        ));
        cursor = end;
    }
    if cursor < chunk_end {
        intervals.push(planned_interval(
            source_kind,
            "idle",
            cursor,
            chunk_end,
            Some(0.0),
            None,
        ));
    }
    if intervals.is_empty() {
        intervals.push(planned_interval(
            source_kind,
            "idle",
            chunk_start,
            chunk_end,
            Some(0.0),
            None,
        ));
    }
    intervals
}

fn merge_windows(windows: Vec<(i64, i64)>, chunk_start: i64, chunk_end: i64) -> Vec<(i64, i64)> {
    let mut merged = Vec::<(i64, i64)>::new();
    for (start, end) in windows {
        let start = start.max(chunk_start);
        let end = end.min(chunk_end);
        if end <= start {
            continue;
        }
        if let Some(last) = merged.last_mut() {
            if start <= last.1 {
                last.1 = last.1.max(end);
                continue;
            }
        }
        merged.push((start, end));
    }
    merged
}

fn frame_delta_intervals(
    segment: &RecordingSegmentRow,
    segment_playback_offset_ms: i64,
    chunk_start: i64,
    chunk_end: i64,
    cancel: &AtomicBool,
) -> Result<Vec<PlannedActivityInterval>, AiError> {
    let video_stream_index = segment.video_stream_index.ok_or_else(|| {
        ai_error(
            "ai_processing_video_stream_missing",
            "Video stream is missing.",
        )
    })?;
    let slice_start_ms = chunk_start
        .checked_sub(segment_playback_offset_ms)
        .ok_or_else(|| {
            ai_error(
                "ai_processing_chunk_offset_invalid",
                "Processing chunk offset is outside its segment.",
            )
        })?;
    let slice_duration_ms = chunk_end.saturating_sub(chunk_start);
    let frame_bytes = FRAME_DELTA_WIDTH * FRAME_DELTA_HEIGHT;
    let output = Command::new(ffmpeg::binary_path())
        .args([
            "-hide_banner",
            "-v",
            "error",
            "-ss",
            &format_seconds(slice_start_ms),
            "-t",
            &format_seconds(slice_duration_ms),
            "-i",
            &segment.file_path,
            "-map",
            &format!("0:{video_stream_index}"),
            "-vf",
            &format!(
                "fps=1,scale={}:{}:flags=fast_bilinear,format=gray",
                FRAME_DELTA_WIDTH, FRAME_DELTA_HEIGHT
            ),
            "-f",
            "rawvideo",
            "-",
        ])
        .output()
        .map_err(file_error)?;
    check_processing_cancel(cancel)?;
    if !output.status.success() {
        return Err(ai_error(
            "ai_processing_frame_delta_extract_failed",
            "Frame delta extraction failed.",
        ));
    }

    let frames = output
        .stdout
        .chunks_exact(frame_bytes)
        .map(|chunk| chunk.to_vec())
        .collect::<Vec<_>>();
    Ok(frame_intervals_from_frames(
        &frames,
        chunk_start,
        chunk_end,
        frame_bytes,
    ))
}

fn frame_intervals_from_frames(
    frames: &[Vec<u8>],
    chunk_start: i64,
    chunk_end: i64,
    frame_bytes: usize,
) -> Vec<PlannedActivityInterval> {
    if frames.is_empty() {
        return vec![planned_interval(
            "frame-delta",
            "unknown",
            chunk_start,
            chunk_end,
            None,
            None,
        )];
    }

    let mut intervals = Vec::new();
    for (index, frame) in frames.iter().enumerate() {
        let start = chunk_start + (index as i64 * FRAME_DELTA_SAMPLE_INTERVAL_MS);
        let end = (start + FRAME_DELTA_SAMPLE_INTERVAL_MS).min(chunk_end);
        if end <= start {
            continue;
        }
        let score = index
            .checked_sub(1)
            .and_then(|previous| frames.get(previous))
            .map(|previous| frame_delta_score(previous, frame, frame_bytes))
            .unwrap_or(0.0);
        let active = score >= FRAME_DELTA_THRESHOLD;
        intervals.push(planned_interval(
            "frame-delta",
            if active { "active" } else { "idle" },
            start,
            end,
            Some(score),
            if active {
                Some(r#"[{"x":0,"y":0,"width":1,"height":1}]"#.to_string())
            } else {
                None
            },
        ));
    }

    coalesce_activity_intervals(intervals)
}

pub(in crate::ai) fn frame_delta_score(previous: &[u8], current: &[u8], frame_bytes: usize) -> f64 {
    let changed = previous
        .iter()
        .zip(current.iter())
        .take(frame_bytes)
        .filter(|(left, right)| u8::abs_diff(**left, **right) > 12)
        .count();
    changed as f64 / frame_bytes as f64
}

pub(in crate::ai) fn merge_activity_intervals(
    source_intervals: &[PlannedActivityInterval],
    chunk_start: i64,
    chunk_end: i64,
) -> Vec<PlannedActivityInterval> {
    let mut boundaries = vec![chunk_start, chunk_end];
    for interval in source_intervals {
        if interval.source_kind == "merged" {
            continue;
        }
        boundaries.push(interval.start_offset_ms.max(chunk_start).min(chunk_end));
        boundaries.push(interval.end_offset_ms.max(chunk_start).min(chunk_end));
    }
    boundaries.sort_unstable();
    boundaries.dedup();

    let mut merged = Vec::new();
    for pair in boundaries.windows(2) {
        let start = pair[0];
        let end = pair[1];
        if end <= start {
            continue;
        }
        let midpoint = start + ((end - start) / 2);
        let states = source_intervals
            .iter()
            .filter(|interval| interval.source_kind != "merged")
            .filter(|interval| {
                interval.start_offset_ms <= midpoint && interval.end_offset_ms > midpoint
            })
            .map(|interval| interval.state.as_str())
            .collect::<Vec<_>>();
        if states.is_empty() {
            continue;
        }
        let state = if states.contains(&"active") {
            "active"
        } else if states.iter().all(|state| *state == "unknown") {
            "unknown"
        } else {
            "idle"
        };
        merged.push(planned_interval("merged", state, start, end, None, None));
    }

    coalesce_activity_intervals(merged)
}

fn coalesce_activity_intervals(
    intervals: Vec<PlannedActivityInterval>,
) -> Vec<PlannedActivityInterval> {
    let mut coalesced = Vec::<PlannedActivityInterval>::new();
    for interval in intervals {
        if let Some(last) = coalesced.last_mut() {
            if last.source_kind == interval.source_kind
                && last.state == interval.state
                && last.regions_json == interval.regions_json
            {
                last.end_offset_ms = interval.end_offset_ms;
                last.score = match (last.score, interval.score) {
                    (Some(left), Some(right)) => Some(left.max(right)),
                    (left, right) => left.or(right),
                };
                continue;
            }
        }
        coalesced.push(interval);
    }
    coalesced
}

pub(in crate::ai) fn planned_interval(
    source_kind: &str,
    state: &str,
    start_offset_ms: i64,
    end_offset_ms: i64,
    score: Option<f64>,
    regions_json: Option<String>,
) -> PlannedActivityInterval {
    PlannedActivityInterval {
        end_offset_ms,
        regions_json,
        score,
        source_kind: source_kind.to_string(),
        start_offset_ms,
        state: state.to_string(),
    }
}

fn input_activity_unavailable(events: &[CaptureEventRow]) -> bool {
    events
        .iter()
        .filter(|event| event.kind == "capture-state")
        .any(capture_state_marks_input_unavailable)
}

fn capture_state_marks_input_unavailable(event: &CaptureEventRow) -> bool {
    let Ok(payload) = serde_json::from_str::<serde_json::Value>(&event.payload_json) else {
        return event
            .payload_json
            .contains("input-device-access-unavailable");
    };

    let status_unavailable =
        payload.get("status").and_then(|value| value.as_str()) == Some("unavailable");
    let reason = payload.get("reason").and_then(|value| value.as_str());

    status_unavailable || reason.is_some_and(input_activity_unavailable_reason)
}

fn input_activity_unavailable_reason(reason: &str) -> bool {
    matches!(
        reason,
        "dev-input-missing"
            | "input-device-access-unavailable"
            | "no-supported-devices"
            | "permission-denied"
            | "unsupported-platform"
    )
}
