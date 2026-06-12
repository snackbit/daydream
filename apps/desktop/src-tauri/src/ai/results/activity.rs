use super::super::*;

pub(in crate::ai) fn activity_for_recording(
    storage: &AppStorage,
    recording_id: &str,
) -> Result<RecordingActivity, AiError> {
    let intervals = storage
        .with_connection(|connection| storage::list_activity_intervals(connection, recording_id))
        .map_err(storage_error)?;
    Ok(RecordingActivity {
        intervals: intervals
            .into_iter()
            .map(activity_interval_from_row)
            .collect(),
        recording_id: recording_id.to_string(),
    })
}

fn activity_interval_from_row(row: ActivityIntervalRow) -> ActivityInterval {
    ActivityInterval {
        end_offset_ms: row.end_offset_ms,
        id: row.id,
        job_id: row.job_id,
        recording_id: row.recording_id,
        score: row.score,
        source_kind: row.source_kind,
        start_offset_ms: row.start_offset_ms,
        state: row.state,
    }
}
