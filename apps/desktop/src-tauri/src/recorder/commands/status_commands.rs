use std::fs;

use tauri::State;

use crate::storage::{self, AppStorage, RecordingSegmentRow};

use super::super::{
    errors::storage_error,
    manager::{now_ms, RecorderManager},
    platform,
    types::{
        ProtectableWindowList, RecorderError, RecorderPerformanceStats, RecorderRecordingStats,
        RecorderStatus, WindowProtectionStatus,
    },
};

pub async fn get_recorder_status(
    manager: State<'_, RecorderManager>,
) -> Result<RecorderStatus, RecorderError> {
    manager.status_with_backend_health(platform::collect_backend_health().await, now_ms())
}

pub fn get_recorder_performance_stats(
    manager: State<'_, RecorderManager>,
) -> Result<RecorderPerformanceStats, RecorderError> {
    manager.performance_stats(now_ms())
}

pub fn get_recorder_recording_stats(
    storage: State<'_, AppStorage>,
    manager: State<'_, RecorderManager>,
) -> Result<Option<RecorderRecordingStats>, RecorderError> {
    collect_recorder_recording_stats(&storage, &manager, now_ms())
}

pub fn list_protectable_windows() -> ProtectableWindowList {
    platform::list_protectable_windows()
}

pub fn get_window_protection_status() -> WindowProtectionStatus {
    platform::window_protection_status()
}

pub(crate) fn collect_recorder_recording_stats(
    storage: &AppStorage,
    manager: &RecorderManager,
    timestamp_ms: u64,
) -> Result<Option<RecorderRecordingStats>, RecorderError> {
    let status = manager.status()?;
    let Some(recording_id) = status.active_recording_id else {
        return Ok(None);
    };

    let recording = storage
        .with_connection(|connection| storage::get_recording(connection, &recording_id))
        .map_err(storage_error)?;
    let segments = storage
        .with_connection(|connection| storage::list_recording_segments(connection, &recording_id))
        .map_err(storage_error)?;
    let active_segment = active_segment_for_stats(&segments, status.active_segment_id.as_deref());
    let active_segment_size_bytes =
        active_segment.and_then(|segment| segment_file_size_bytes(segment).ok());
    let total_size_bytes = segments
        .iter()
        .filter_map(|segment| segment_file_size_bytes(segment).ok())
        .sum();

    Ok(Some(RecorderRecordingStats {
        active_segment_id: status.active_segment_id,
        active_segment_index: active_segment
            .and_then(|segment| u32::try_from(segment.segment_index).ok()),
        active_segment_size_bytes,
        active_segment_started_at_ms: active_segment
            .and_then(|segment| u64::try_from(segment.started_at_ms).ok()),
        recording_id,
        recording_started_at_ms: recording
            .started_at_ms
            .and_then(|started_at_ms| u64::try_from(started_at_ms).ok()),
        segment_count: segments.len(),
        total_size_bytes,
        updated_at_ms: timestamp_ms,
    }))
}

fn active_segment_for_stats<'a>(
    segments: &'a [RecordingSegmentRow],
    active_segment_id: Option<&str>,
) -> Option<&'a RecordingSegmentRow> {
    active_segment_id
        .and_then(|segment_id| segments.iter().find(|segment| segment.id == segment_id))
        .or_else(|| {
            segments
                .iter()
                .rev()
                .find(|segment| segment.state == "recording")
        })
}

fn segment_file_size_bytes(segment: &RecordingSegmentRow) -> std::io::Result<u64> {
    fs::metadata(&segment.file_path).map(|metadata| metadata.len())
}
