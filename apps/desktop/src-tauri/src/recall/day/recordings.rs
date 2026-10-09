use crate::{
    recall::types::RecallDayRecording,
    storage::{self, RecordingRow, RecordingSegmentRow},
};
use rusqlite::Connection;
use std::path::Path;

pub(super) fn for_day(
    connection: &Connection,
    recording: &RecordingRow,
    segments: &[RecordingSegmentRow],
    start: i64,
    end: i64,
    protected: bool,
    now: i64,
) -> rusqlite::Result<Option<RecallDayRecording>> {
    let mut ranges = Vec::new();
    let mut first = None;
    let mut offset = None;
    let mut last = None;
    let mut unknown = false;
    let mut playable = 0;
    let mut count = 0;
    for segment in segments {
        let stop = segment
            .duration_ms
            .filter(|duration| *duration >= 0)
            .and_then(|duration| segment.started_at_ms.checked_add(duration))
            .or(segment.ended_at_ms)
            .or_else(|| (segment.state == "recording").then_some(now));
        if segment.started_at_ms >= end || stop.is_some_and(|stop| stop <= start) {
            continue;
        }
        // An unfinished recovery has no trusted duration, but remains discoverable.
        if stop.is_none() && segment.started_at_ms < start {
            continue;
        }
        count += 1;
        let from = segment.started_at_ms.max(start);
        if first.is_none_or(|value| from < value) {
            first = Some(from);
            offset = segment
                .timeline_base_ms
                .and_then(|base| base.checked_add(from - segment.started_at_ms));
        }
        if let Some(to) = stop {
            let to = to.min(end).max(from);
            last = Some(last.unwrap_or(to).max(to));
            if segment.duration_ms.is_some() || segment.state == "recording" {
                ranges.push((from, to));
            } else {
                unknown = true;
            }
        } else {
            unknown = true;
        }
        if segment.state == "finalized" && Path::new(&segment.file_path).is_file() {
            playable += 1;
        }
    }
    let Some(first) = first else {
        return Ok(None);
    };
    let processing = storage::get_latest_processing_job_for_recording(
        connection,
        &recording.id,
        "ai-processing",
    )?;
    Ok(Some(RecallDayRecording {
        recording_id: recording.id.clone(),
        title: recording.title.clone(),
        start_ms: first,
        end_ms: if unknown { None } else { last },
        start_offset_ms: offset,
        duration_ms: if unknown {
            None
        } else {
            Some(super::union_duration(&ranges))
        },
        availability: if protected {
            "hidden"
        } else if playable == count {
            "available"
        } else if playable > 0 {
            "partial"
        } else {
            "unavailable"
        }
        .into(),
        processing_status: processing.map(|job| job.status),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn day_recordings_exclude_pause_gaps_and_keep_unknown_timing() {
        let connection = Connection::open_in_memory().unwrap();
        connection.execute_batch("CREATE TABLE processing_jobs(id TEXT,recording_id TEXT,kind TEXT,status TEXT,created_at_ms INTEGER,updated_at_ms INTEGER,error TEXT); INSERT INTO processing_jobs VALUES('job','synthetic-recording','ai-processing','failed',0,0,'synthetic');").unwrap();
        let recording = RecordingRow {
            created_at_ms: 0,
            default_fps: 0.2,
            id: "synthetic-recording".into(),
            active_scene_id: None,
            ended_at_ms: Some(50_000),
            output_root: None,
            quality_label: "synthetic".into(),
            started_at_ms: Some(0),
            status: "completed".into(),
            storage_dir: "/tmp/synthetic".into(),
            title: None,
        };
        let mut a = RecordingSegmentRow {
            capture_run_id: Some("synthetic".into()),
            timeline_base_ms: Some(0),
            timeline_basis_version: Some(1),
            desktop_audio_stream_index: None,
            duration_ms: Some(8000),
            ended_at_ms: Some(9000),
            error_code: None,
            ffmpeg_exit_code: None,
            file_path: "/tmp/absent-synthetic-media".into(),
            id: "first".into(),
            microphone_audio_stream_index: None,
            probe_json: None,
            recording_id: recording.id.clone(),
            segment_index: 0,
            started_at_ms: 0,
            state: "finalized".into(),
            video_stream_index: Some(0),
        };
        a.duration_ms = Some(8000);
        a.ended_at_ms = Some(9000);
        let mut b = a.clone();
        b.id = "second".into();
        b.started_at_ms = 10_000;
        b.duration_ms = Some(9000);
        b.ended_at_ms = Some(20_000);
        b.timeline_base_ms = Some(10_000);
        let row = for_day(
            &connection,
            &recording,
            &[a.clone(), b.clone()],
            0,
            100_000,
            false,
            100_000,
        )
        .unwrap()
        .unwrap();
        assert_eq!(row.duration_ms, Some(17_000));
        assert_eq!(row.processing_status.as_deref(), Some("failed"));
        assert_eq!(row.availability, "unavailable");
        b.duration_ms = None;
        assert_eq!(
            for_day(
                &connection,
                &recording,
                &[a, b.clone()],
                0,
                100_000,
                false,
                100_000
            )
            .unwrap()
            .unwrap()
            .duration_ms,
            None
        );
        b.ended_at_ms = None;
        assert!(for_day(
            &connection,
            &recording,
            &[b.clone()],
            0,
            100_000,
            true,
            100_000
        )
        .unwrap()
        .is_some());
        b.duration_ms = Some(9000);
        b.ended_at_ms = Some(19_000);
        let overlap = for_day(
            &connection,
            &recording,
            &[b],
            15_000,
            100_000,
            false,
            100_000,
        )
        .unwrap()
        .unwrap();
        assert_eq!(overlap.duration_ms, Some(4000));
        assert_eq!(overlap.start_offset_ms, Some(15_000));
    }
}
