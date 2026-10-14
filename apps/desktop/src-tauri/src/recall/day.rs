//! Calendar facts are computed from capture metadata, independently of models.
use super::types::*;
use crate::storage::{self, AppStorage};
use chrono::Utc;
use rusqlite::Connection;
use std::{collections::BTreeSet, path::Path};

mod calendar;
mod fingerprint;
mod recordings;
pub(crate) use calendar::capture_dates;
pub use calendar::{day_bounds, local_timezone};

pub(crate) type CaptureRows = Vec<(storage::RecordingRow, Vec<storage::RecordingSegmentRow>)>;

pub(crate) fn capture_rows(connection: &Connection) -> rusqlite::Result<CaptureRows> {
    storage::list_recordings(connection)?
        .into_iter()
        .map(|recording| {
            let segments = storage::list_recording_segments(connection, &recording.id)?;
            Ok((recording, segments))
        })
        .collect()
}

pub(crate) struct DaySnapshot {
    pub day: RecallDayCard,
    pub recordings: Vec<RecallDayRecording>,
    pub coverage: Vec<RecallCoverageInterval>,
}

pub fn day_key(input: &RecallDayInput) -> String {
    format!("{}@{}", input.day_id, input.timezone)
}

pub fn list_days(
    storage: &AppStorage,
    input: &ListRecallDaysInput,
) -> Result<Vec<RecallDayCard>, String> {
    calendar::input_timezone(input)?;
    let profile = super::worker::profile(storage).ok();
    let cards = storage
        .with_connection(|connection| {
            storage::refresh_evidence(connection)?;
            let rows = capture_rows(connection)?;
            let dates = capture_dates(&rows, input, Utc::now().timestamp_millis())
                .map_err(|_| rusqlite::Error::InvalidQuery)?;
            dates
                .into_iter()
                .rev()
                .take(input.limit.clamp(1, 100))
                .map(|day_id| {
                    super::generation_service::compose(
                        connection,
                        &RecallDayInput {
                            day_id,
                            timezone: input.timezone.clone(),
                        },
                        profile.as_ref().map(|profile| profile.fingerprint.as_str()),
                    )
                    .map(|day| day.day)
                    .map_err(|_| rusqlite::Error::InvalidQuery)
                })
                .collect::<rusqlite::Result<Vec<_>>>()
        })
        .map_err(|_| "Days could not be read.".to_string())?;
    Ok(cards)
}

pub fn base_day(connection: &Connection, input: &RecallDayInput) -> Result<RecallDay, String> {
    storage::refresh_evidence(connection)
        .map_err(|_| "Day text and transcripts could not be read.")?;
    let (start, end) = day_bounds(&input.day_id, &input.timezone)?;
    let evidence = storage::list_evidence_range(connection, start, end)
        .map_err(|_| "Day text and transcripts could not be read.")?;
    let rows = capture_rows(connection).map_err(|_| "Capture coverage could not be read.")?;
    let snapshot = day_snapshot(
        connection,
        input,
        &rows,
        evidence.len(),
        Some(&storage::evidence_fingerprint(&evidence)),
    )?;
    Ok(RecallDay {
        day: snapshot.day,
        evidence: evidence.into_iter().take(100).collect(),
        privacy_revision: storage::recall_privacy_revision(connection)
            .map_err(|_| "Privacy revision could not be read.")?,
        recordings: snapshot.recordings,
        coverage: snapshot.coverage,
        sessions: vec![],
        suggestions: vec![],
        tasks: vec![],
        feedback: vec![],
    })
}

/// Capture facts and review fingerprints do not require evidence bodies.
pub(crate) fn day_snapshot(
    connection: &Connection,
    input: &RecallDayInput,
    rows: &CaptureRows,
    evidence_count: usize,
    evidence_fingerprint: Option<&str>,
) -> Result<DaySnapshot, String> {
    let (start, end) = day_bounds(&input.day_id, &input.timezone)?;
    let mut coverage = Vec::new();
    let mut recordings = BTreeSet::new();
    let mut recording_summaries = Vec::new();
    let mut thumbnail = None;
    let mut thumbnail_recording_id = None;
    let mut recording_active = false;
    for (recording, segments) in rows {
        if !segments.iter().any(|segment| {
            let stop = calendar::segment_stop(segment, Utc::now().timestamp_millis());
            segment.started_at_ms < end
                && stop.is_none_or(|stop| stop > start)
                && (stop.is_some() || segment.started_at_ms >= start)
        }) {
            continue;
        }
        let redactions = storage::list_redactions(connection, &recording.id)
            .map_err(|_| "Privacy state could not be read.")?;
        let protected = storage::list_recording_protection_spans(connection, &recording.id)
            .map_err(|_| "Privacy state could not be read.")?
            .iter()
            .any(|span| span.status != "materialized");
        if let Some(summary) = recordings::for_day(
            connection,
            recording,
            segments,
            start,
            end,
            protected,
            Utc::now().timestamp_millis(),
        )
        .map_err(|_| "Recordings could not be read.")?
        {
            recordings.insert(recording.id.clone());
            recording_active |= storage::is_active_recording_status(&recording.status);
            recording_summaries.push(summary);
        }
        let thumbnail_segment = segments
            .iter()
            .find(|segment| segment.state == "finalized" && Path::new(&segment.file_path).is_file())
            .map(|segment| segment.id.clone());
        for segment in segments {
            let Some(stop) = segment
                .duration_ms
                .filter(|duration| *duration >= 0)
                .and_then(|duration| segment.started_at_ms.checked_add(duration))
                .or(segment.ended_at_ms)
                .or_else(|| (segment.state == "recording").then(|| Utc::now().timestamp_millis()))
            else {
                continue;
            };
            let from = segment.started_at_ms.max(start);
            let to = stop.min(end);
            if from >= to {
                continue;
            }
            recordings.insert(recording.id.clone());
            recording_active |= storage::is_active_recording_status(&recording.status);
            let available = matches!(segment.state.as_str(), "finalized" | "recording")
                && Path::new(&segment.file_path).is_file();
            let kind = if protected {
                "redacted"
            } else if available {
                "captured"
            } else {
                "missing"
            };
            coverage.push(interval(&recording.id, Some(&segment.id), from, to, kind));
            if available && !protected {
                let ranges = redactions
                    .iter()
                    .filter(|cut| {
                        serde_json::from_str::<Vec<String>>(&cut.track_ids_json)
                            .map_or(true, |tracks| tracks.is_empty())
                    })
                    .filter_map(|cut| {
                        segment.timeline_base_ms.map(|base| {
                            (
                                segment
                                    .started_at_ms
                                    .saturating_add(cut.start_offset_ms - base)
                                    .max(from),
                                segment
                                    .started_at_ms
                                    .saturating_add(cut.end_offset_ms - base)
                                    .min(to),
                            )
                        })
                    })
                    .collect::<Vec<_>>();
                for (a, b) in complement(from, to, &ranges) {
                    coverage.push(interval(
                        &recording.id,
                        Some(&segment.id),
                        a,
                        b,
                        "uninterpreted",
                    ));
                }
            }
            if let Some(base) = segment.timeline_base_ms {
                for cut in &redactions {
                    let a = cut.start_offset_ms.max(base);
                    let b = cut
                        .end_offset_ms
                        .min(base.saturating_add(segment.duration_ms.unwrap_or(0)));
                    if a >= b {
                        continue;
                    }
                    let absolute_a = segment.started_at_ms.saturating_add(a - base).max(start);
                    let absolute_b = segment.started_at_ms.saturating_add(b - base).min(end);
                    if absolute_a < absolute_b {
                        let mut redacted = interval(
                            &recording.id,
                            Some(&segment.id),
                            absolute_a,
                            absolute_b,
                            "redacted",
                        );
                        redacted.source_track_ids =
                            serde_json::from_str(&cut.track_ids_json).unwrap_or_default();
                        coverage.push(redacted);
                    }
                }
                // Reuse only an existing Library-owned image and its exact
                // first-frame eligibility. Generation remains the thumbnail API.
                if (thumbnail.is_none() || thumbnail_recording_id.is_none())
                    && available
                    && !protected
                    && thumbnail_segment.as_deref() == Some(segment.id.as_str())
                    && segment.video_stream_index.is_some()
                    && segment.started_at_ms >= start
                    && segment.started_at_ms < end
                {
                    let track = format!("video-{}", segment.video_stream_index.unwrap_or(0));
                    let video_sources = storage::segment_video_streams(connection, segment)
                        .map_err(|_| "Source metadata could not be read.")?;
                    let source_track = video_sources
                        .iter()
                        .find(|stream| Some(stream.stream_index) == segment.video_stream_index)
                        .map(|stream| stream.track_id.as_str());
                    let hidden = redactions.iter().any(|cut| {
                        cut.start_offset_ms <= base && cut.end_offset_ms > base && {
                            let tracks = serde_json::from_str::<Vec<String>>(&cut.track_ids_json);
                            tracks.is_err()
                                || tracks.as_ref().is_ok_and(|tracks| {
                                    tracks.is_empty()
                                        || tracks.iter().any(|id| {
                                            id == &track || source_track == Some(id.as_str())
                                        })
                                })
                        }
                    });
                    let path = Path::new(&recording.storage_dir)
                        .join("playback")
                        .join(&segment.id)
                        .join("overview.jpg");
                    if !hidden && path.is_file() {
                        thumbnail = Some(path.to_string_lossy().to_string());
                        thumbnail_recording_id = Some(recording.id.clone());
                    } else if !hidden
                        && thumbnail_recording_id.is_none()
                        && matches!(
                            recording.status.as_str(),
                            "completed" | "failed" | "recovered-partial"
                        )
                    {
                        thumbnail_recording_id = Some(recording.id.clone());
                    }
                }
            }
        }
    }
    let capture_ranges = coverage
        .iter()
        .filter(|item| matches!(item.kind.as_str(), "captured" | "missing" | "redacted"))
        .map(|item| (item.start_ms, item.end_ms))
        .collect::<Vec<_>>();
    for (a, b) in complement(start, end, &capture_ranges) {
        coverage.push(interval("", None, a, b, "gap"));
    }
    coverage.sort_by(|a, b| {
        (a.start_ms, &a.recording_id, &a.kind).cmp(&(b.start_ms, &b.recording_id, &b.kind))
    });
    let (fingerprint, reviewed) = match evidence_fingerprint {
        Some(value) => fingerprint::review_state(connection, input, value, &coverage)?,
        None => (String::new(), false),
    };
    let redaction_count = coverage
        .iter()
        .filter(|item| item.kind == "redacted")
        .count();
    let privacy_revision = storage::recall_privacy_revision(connection)
        .map_err(|_| "Privacy revision could not be read.")?;
    Ok(DaySnapshot {
        recordings: recording_summaries,
        day: RecallDayCard {
            privacy_revision,
            day_id: input.day_id.clone(),
            start_ms: start,
            end_ms: end,
            timezone: input.timezone.clone(),
            recording_count: recordings.len(),
            evidence_count,
            summary: None,
            reviewed,
            status: if evidence_count == 0 {
                "awaiting-evidence"
            } else {
                "not-generated"
            }
            .into(),
            input_fingerprint: fingerprint,
            captured_duration_ms: union_duration(&capture_ranges),
            thumbnail_path: thumbnail,
            thumbnail_recording_id,
            redaction_count,
            recording_active,
        },
        coverage,
    })
}

fn interval(
    recording: &str,
    segment: Option<&str>,
    start: i64,
    end: i64,
    kind: &str,
) -> RecallCoverageInterval {
    RecallCoverageInterval {
        source_track_ids: vec![],
        recording_id: recording.into(),
        segment_id: segment.map(str::to_string),
        start_ms: start,
        end_ms: end,
        kind: kind.into(),
    }
}
fn merged(ranges: &[(i64, i64)]) -> Vec<(i64, i64)> {
    let mut sorted = ranges.to_vec();
    sorted.sort_unstable();
    let mut result: Vec<(i64, i64)> = vec![];
    for (a, b) in sorted {
        if a >= b {
            continue;
        }
        if let Some(last) = result.last_mut().filter(|last| last.1 >= a) {
            last.1 = last.1.max(b);
        } else {
            result.push((a, b));
        }
    }
    result
}
pub fn union_duration(ranges: &[(i64, i64)]) -> i64 {
    merged(ranges).iter().map(|(a, b)| b - a).sum()
}
pub fn complement(start: i64, end: i64, ranges: &[(i64, i64)]) -> Vec<(i64, i64)> {
    let mut cursor = start;
    let mut gaps = vec![];
    for (a, b) in merged(ranges) {
        let a = a.max(start);
        let b = b.min(end);
        if a >= b {
            continue;
        }
        if a > cursor {
            gaps.push((cursor, a));
        }
        cursor = cursor.max(b);
    }
    if cursor < end {
        gaps.push((cursor, end));
    }
    gaps
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn overlapping_recordings_count_time_once_and_gaps_stay_real() {
        assert_eq!(union_duration(&[(10, 30), (20, 50), (80, 90)]), 50);
        assert_eq!(
            complement(0, 100, &[(10, 30), (20, 50), (80, 90)]),
            vec![(0, 10), (50, 80), (90, 100)]
        );
    }
}
