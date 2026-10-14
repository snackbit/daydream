//! Day playback keeps recording-local provenance and excludes unavailable media.
use super::*;
use serde::{Deserialize, Serialize};

pub(crate) mod listing;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryDayTimelineInput {
    day_id: String,
    timezone: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryDayTimeline {
    day_id: String,
    start_ms: i64,
    end_ms: i64,
    privacy_revision: i64,
    recordings: Vec<LibraryRecordingDetail>,
    spans: Vec<LibraryDayPlaybackSpan>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LibraryDayPlaybackSpan {
    recording_id: String,
    segment_id: String,
    start_ms: i64,
    duration_ms: i64,
    source_start_ms: i64,
    source_offset_ms: i64,
}

fn slice(segment: &LibraryRecordingSegment, start: i64, end: i64) -> Option<(i64, i64, i64)> {
    if segment.state != "finalized" || !segment.file_exists || segment.video_stream_index.is_none()
    {
        return None;
    }
    let stop = segment
        .started_at_ms
        .checked_add(segment.duration_ms.filter(|d| *d > 0)?)?;
    let from = segment.started_at_ms.max(start);
    let to = stop.min(end);
    (to > from).then_some((from, to - from, from - segment.started_at_ms))
}

pub(crate) fn read_day(
    storage: &AppStorage,
    input: LibraryDayTimelineInput,
) -> Result<LibraryDayTimeline, String> {
    let (start_ms, end_ms) = crate::recall::day::day_bounds(&input.day_id, &input.timezone)?;
    let (rows, revision) = storage
        .with_read_connection(|connection| {
            let snapshot = connection.unchecked_transaction()?;
            let revision = storage::recall_privacy_revision(&snapshot)?;
            let mut rows = Vec::new();
            for recording in storage::list_recordings_in_range(&snapshot, start_ms, end_ms)? {
                if storage::is_active_recording_status(&recording.status)
                    || storage::list_recording_protection_spans(&snapshot, &recording.id)?
                        .iter()
                        .any(|span| span.status != "materialized")
                {
                    continue;
                }
                let segments = storage::list_recording_segments(&snapshot, &recording.id)?;
                let redactions = storage::list_redactions(&snapshot, &recording.id)?;
                let mappings = storage::recording_video_streams(&snapshot, &recording.id)?;
                let cleanup =
                    audio_cleanup::read_state_from_connection(&snapshot, &recording.id, &segments)?;
                rows.push((recording, segments, redactions, mappings, cleanup));
            }
            snapshot.commit()?;
            Ok((rows, revision))
        })
        .map_err(|_| "Day recordings could not be read.")?;
    let mut recordings = Vec::new();
    let mut spans = Vec::new();
    // Availability checks touch only known media paths, outside the snapshot.
    // Recursive disk usage is intentionally deferred to recording details.
    for (recording, segments, redactions, mappings, cleanup) in rows {
        let mut detail = detail_metadata_from_rows(&recording, &segments, &redactions);
        apply_source_metadata(&mut detail, &segments, &redactions, &mappings);
        detail.audio_cleanup = Some(cleanup);
        let before = spans.len();
        for segment in &detail.segments {
            if let Some((from, duration, trim)) = slice(segment, start_ms, end_ms) {
                spans.push(LibraryDayPlaybackSpan {
                    recording_id: recording.id.clone(),
                    segment_id: segment.id.clone(),
                    start_ms: from,
                    duration_ms: duration,
                    source_start_ms: trim,
                    source_offset_ms: segment.playback_offset_ms + trim,
                });
            }
        }
        if spans.len() > before {
            recordings.push(detail);
        }
    }
    let current = storage
        .with_read_connection(storage::recall_privacy_revision)
        .map_err(|_| "Recording privacy state could not be read.")?;
    if current != revision {
        return Err("Recording privacy state changed. Reload the day.".into());
    }
    spans.sort_by(|a, b| {
        a.start_ms
            .cmp(&b.start_ms)
            .then(a.segment_id.cmp(&b.segment_id))
    });
    let mut first_starts = BTreeMap::new();
    for span in &spans {
        first_starts
            .entry(&span.recording_id)
            .or_insert(span.start_ms);
    }
    recordings.sort_by_key(|r| first_starts.get(&r.id).copied());
    Ok(LibraryDayTimeline {
        day_id: input.day_id,
        start_ms,
        end_ms,
        privacy_revision: revision,
        recordings,
        spans,
    })
}

#[tauri::command(rename = "getLibraryDayTimeline")]
pub async fn get_library_day_timeline(
    input: LibraryDayTimelineInput,
    app: tauri::AppHandle,
) -> Result<LibraryDayTimeline, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let _activity = crate::maintenance::runtime::admit(&app, "playback-prepare")?;
        read_day(app.state::<AppStorage>().inner(), input)
    })
    .await
    .map_err(|_| "Day playback worker failed.".to_string())?
}

#[cfg(test)]
mod integration_tests;

#[cfg(test)]
mod tests {
    use super::*;
    fn segment() -> LibraryRecordingSegment {
        LibraryRecordingSegment {
            capture_run_id: None,
            video_streams: vec![],
            desktop_audio_stream_index: None,
            duration_ms: Some(20),
            ended_at_ms: Some(30),
            file_exists: true,
            file_path: String::new(),
            id: "synthetic".into(),
            microphone_audio_stream_index: None,
            pause_after_ms: 0,
            playback_offset_ms: 100,
            segment_index: 0,
            started_at_ms: 10,
            state: "finalized".into(),
            video_stream_index: Some(0),
        }
    }
    #[test]
    fn day_slices_preserve_source_offsets_and_reject_unplayable_media() {
        let mut s = segment();
        assert_eq!(slice(&s, 15, 25), Some((15, 10, 5)));
        assert_eq!(slice(&s, 30, 50), None);
        s.file_exists = false;
        assert_eq!(slice(&s, 0, 50), None);
        s.file_exists = true;
        s.duration_ms = Some(0);
        assert_eq!(slice(&s, 0, 50), None);
        s.duration_ms = Some(20);
        s.state = "failed".into();
        assert_eq!(slice(&s, 0, 50), None);
        s.state = "finalized".into();
        s.video_stream_index = None;
        assert_eq!(slice(&s, 0, 50), None);
    }
}
