use crate::storage::{self, AppStorage, FailRecordingSegmentInput};

use super::super::{errors::storage_error, types::RecorderError};

pub(super) fn fail_segment_for_error(
    storage: &AppStorage,
    segment_id: &str,
    timestamp_ms: u64,
    error: &RecorderError,
    writer_exit_code: Option<i64>,
) -> Result<(), RecorderError> {
    storage
        .with_connection(|connection| {
            storage::fail_recording_segment(
                connection,
                &FailRecordingSegmentInput {
                    ended_at_ms: timestamp_ms as i64,
                    error_code: error.code.clone(),
                    ffmpeg_exit_code: writer_exit_code,
                    id: segment_id.to_string(),
                    updated_at_ms: timestamp_ms as i64,
                },
            )
        })
        .map(|_segment| ())
        .map_err(storage_error)
}
