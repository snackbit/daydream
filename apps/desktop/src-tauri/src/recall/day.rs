//! Calendar facts are computed from capture metadata, independently of models.
use super::types::*;
use crate::storage::{self, AppStorage};
use chrono::{TimeZone, Utc};
use chrono_tz::Tz;
use rusqlite::{Connection, OptionalExtension};
use std::{collections::BTreeSet, path::Path};

mod calendar;
mod recordings;
pub use calendar::{day_bounds, local_timezone};

pub fn day_key(input: &RecallDayInput) -> String {
    format!("{}@{}", input.day_id, input.timezone)
}

pub fn list_days(
    storage: &AppStorage,
    input: &ListRecallDaysInput,
) -> Result<Vec<RecallDayCard>, String> {
    let tz: Tz = input
        .timezone
        .parse()
        .map_err(|_| "Invalid timezone.".to_string())?;
    if let Some(before) = &input.before_day_id {
        day_bounds(before, &input.timezone)?;
    }
    let profile = super::worker::profile(storage).ok();
    let cards = storage
        .with_connection(|connection| {
            storage::refresh_evidence(connection)?;
            let mut dates = BTreeSet::new();
            for recording in storage::list_recordings(connection)? {
                for segment in storage::list_recording_segments(connection, &recording.id)? {
                    let end = segment
                        .duration_ms
                        .filter(|duration| *duration >= 0)
                        .and_then(|duration| segment.started_at_ms.checked_add(duration))
                        .or(segment.ended_at_ms)
                        .or_else(|| {
                            (segment.state == "recording").then(|| Utc::now().timestamp_millis())
                        })
                        .unwrap_or_else(|| segment.started_at_ms.saturating_add(1))
                        .max(segment.started_at_ms.saturating_add(1));
                    let Some(first) = Utc.timestamp_millis_opt(segment.started_at_ms).single()
                    else {
                        continue;
                    };
                    let Some(last) = Utc.timestamp_millis_opt(end.saturating_sub(1)).single()
                    else {
                        continue;
                    };
                    let mut date = first.with_timezone(&tz).date_naive();
                    let last = last.with_timezone(&tz).date_naive();
                    // Corrupt metadata must not create an unbounded enumeration.
                    if last.signed_duration_since(date).num_days() > 3660 {
                        return Err(rusqlite::Error::InvalidQuery);
                    }
                    while date <= last {
                        let id = date.format("%Y-%m-%d").to_string();
                        if input
                            .before_day_id
                            .as_ref()
                            .is_none_or(|before| &id < before)
                        {
                            dates.insert(id);
                        }
                        date = date.succ_opt().ok_or(rusqlite::Error::InvalidQuery)?;
                    }
                }
            }
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
    let mut coverage = Vec::new();
    let mut recordings = BTreeSet::new();
    let mut recording_summaries = Vec::new();
    let mut thumbnail = None;
    let mut thumbnail_recording_id = None;
    let mut recording_active = false;
    let rows =
        storage::list_recordings(connection).map_err(|_| "Capture coverage could not be read.")?;
    for recording in rows {
        let redactions = storage::list_redactions(connection, &recording.id)
            .map_err(|_| "Privacy state could not be read.")?;
        let protected = storage::list_recording_protection_spans(connection, &recording.id)
            .map_err(|_| "Privacy state could not be read.")?
            .iter()
            .any(|span| span.status != "materialized");
        let segments = storage::list_recording_segments(connection, &recording.id)
            .map_err(|_| "Capture coverage could not be read.")?;
        if let Some(summary) = recordings::for_day(
            connection,
            &recording,
            &segments,
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
                    let video_sources = storage::segment_video_streams(connection, &segment)
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
    let all_ranges = coverage
        .iter()
        .filter(|item| item.kind == "captured" || item.kind == "missing" || item.kind == "redacted")
        .map(|item| (item.start_ms, item.end_ms))
        .collect::<Vec<_>>();
    for (a, b) in complement(start, end, &all_ranges) {
        coverage.push(interval("", None, a, b, "gap"));
    }
    coverage.sort_by(|a, b| {
        (a.start_ms, &a.recording_id, &a.kind).cmp(&(b.start_ms, &b.recording_id, &b.kind))
    });
    let mut input_hasher = sha2::Sha256::new();
    use sha2::Digest;
    input_hasher.update(storage::evidence_fingerprint(&evidence));
    input_hasher.update(input.timezone.as_bytes());
    // Coverage changes and local corrections make a previously reviewed day new.
    input_hasher.update(serde_json::to_vec(&coverage).map_err(|_| "Invalid coverage.")?);
    let key = day_key(input);
    let mut statement=connection.prepare("SELECT session_id,correction,title_correction,category_correction FROM recall_session_corrections WHERE day_key=?1 ORDER BY session_id").map_err(|_|"Day corrections could not be read.")?;
    for row in statement
        .query_map([&key], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        })
        .map_err(|_| "Day corrections could not be read.")?
    {
        let (id, text, title, category) = row.map_err(|_| "Day corrections could not be read.")?;
        for value in [Some(id), text, title, category] {
            let value = value.unwrap_or_default();
            input_hasher.update((value.len() as u64).to_le_bytes());
            input_hasher.update(value.as_bytes());
        }
    }
    let generation: Option<(String, String)> = connection
        .query_row(
            "SELECT output_json,status FROM recall_generations WHERE day_key=?1",
            [&key],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|_| "Interpretation state could not be read.")?;
    if let Some((output, status)) = generation {
        input_hasher.update(output.as_bytes());
        input_hasher.update(status.as_bytes());
    }
    let configuration: Option<String> = connection
        .query_row(
            "SELECT value_json FROM app_settings WHERE key='ai.recall'",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|_| "Configuration could not be read.")?;
    input_hasher.update(configuration.unwrap_or_default().as_bytes());
    let fingerprint = format!("{:x}", input_hasher.finalize());
    let reviewed = connection
        .query_row(
            "SELECT fingerprint FROM recall_day_reviews WHERE day_key=?1",
            [&key],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|_| "Review state could not be read.")?
        .is_some_and(|value| value == fingerprint);
    let redaction_count = coverage
        .iter()
        .filter(|item| item.kind == "redacted")
        .count();
    let privacy_revision = storage::recall_privacy_revision(connection)
        .map_err(|_| "Privacy revision could not be read.")?;
    let evidence_count = evidence.len();
    Ok(RecallDay {
        evidence: evidence.into_iter().take(100).collect(),
        privacy_revision,
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
        sessions: vec![],
        suggestions: vec![],
        tasks: vec![],
        feedback: vec![],
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
