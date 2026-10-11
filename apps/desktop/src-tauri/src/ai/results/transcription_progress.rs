use super::super::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SourceTimeRange {
    pub start_offset_ms: i64,
    pub end_offset_ms: i64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecordingTranscriptionProgress {
    pub job_id: String,
    pub track_id: String,
    pub source_role: String,
    pub status: String,
    pub ranges: Vec<SourceTimeRange>,
    pub published_ranges: Vec<SourceTimeRange>,
}

pub(super) fn source_progress(
    connection: &rusqlite::Connection,
    job: Option<&ProcessingJobRow>,
    chunks: &[ProcessingChunkRow],
    tracks: &[RecordingTrackRow],
    segments: &[RecordingSegmentRow],
) -> rusqlite::Result<Vec<RecordingTranscriptionProgress>> {
    let Some(job) = job else {
        return Ok(Vec::new());
    };
    if !storage::speech_strategy(connection, &job.id)? {
        return Ok(Vec::new());
    }
    let mut result = Vec::new();
    for track in tracks
        .iter()
        .filter(|track| matches!(track.kind.as_str(), "microphone" | "desktop-audio"))
    {
        let owned = chunks
            .iter()
            .filter(|chunk| {
                chunk.source_kind == "audio" && chunk.track_id.as_ref() == Some(&track.id)
            })
            .collect::<Vec<_>>();
        let ranges = owned
            .iter()
            .filter_map(|chunk| {
                Some(SourceTimeRange {
                    start_offset_ms: chunk.start_offset_ms?,
                    end_offset_ms: chunk.end_offset_ms?,
                })
            })
            .collect::<Vec<_>>();
        if ranges.is_empty() {
            continue;
        }
        let published_ranges = storage::speech_units(connection, &job.id, &track.id)?
            .into_iter()
            .filter(|unit| unit.status == "published")
            .map(|unit| SourceTimeRange {
                start_offset_ms: unit.start_ms,
                end_offset_ms: unit.end_ms,
            })
            .collect();
        let status = if matches!(
            job.status.as_str(),
            "paused" | "failed" | "cancelled" | "completed-with-errors"
        ) {
            job.status.clone()
        } else if owned
            .iter()
            .any(|chunk| matches!(chunk.status.as_str(), "failed" | "completed-with-errors"))
        {
            "failed".into()
        } else if owned.iter().any(|chunk| chunk.status == "running") {
            "running".into()
        } else if owned.iter().all(|chunk| chunk.status == "succeeded") {
            "succeeded".into()
        } else {
            "queued".into()
        };
        let source = segments
            .iter()
            .find_map(|segment| crate::recording_privacy::source_track_id(segment, &track.kind))
            .unwrap_or_else(|| track.id.clone());
        result.push(RecordingTranscriptionProgress {
            job_id: job.id.clone(),
            track_id: source,
            source_role: track.kind.clone(),
            status,
            ranges,
            published_ranges,
        });
    }
    Ok(result)
}
