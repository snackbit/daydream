use super::types::{LatestRecordingDiagnostics, LatestSegmentDiagnostics};
use crate::recorder::RecorderManager;
use crate::storage::{
    get_latest_recording_id, list_recording_segments, AppStorage, RecordingSegmentRow,
};

pub(super) fn collect_latest_recording_diagnostics(
    storage: &AppStorage,
    manager: &RecorderManager,
) -> LatestRecordingDiagnostics {
    let status = manager.status().ok();
    let active_recording_id = status
        .as_ref()
        .and_then(|status| status.active_recording_id.clone());
    let active_segment_id = status
        .as_ref()
        .and_then(|status| status.active_segment_id.clone());
    let active_capture_timing = manager.active_capture_timing().ok().flatten();
    let preview_status = manager.preview_status().ok();

    let recording_id = match active_recording_id.clone() {
        Some(recording_id) => Some(recording_id),
        None => match storage.with_connection(get_latest_recording_id) {
            Ok(recording_id) => recording_id,
            Err(error) => {
                return LatestRecordingDiagnostics {
                    active_capture_timing,
                    active_recording_id,
                    active_segment_id,
                    error: Some(error.to_string()),
                    latest_segment: None,
                    preview_status,
                    recording_id: None,
                    segment_count: 0,
                };
            }
        },
    };

    let Some(recording_id) = recording_id else {
        return LatestRecordingDiagnostics {
            active_capture_timing,
            active_recording_id,
            active_segment_id,
            error: None,
            latest_segment: None,
            preview_status,
            recording_id: None,
            segment_count: 0,
        };
    };

    let segments = match storage
        .with_connection(|connection| list_recording_segments(connection, &recording_id))
    {
        Ok(segments) => segments,
        Err(error) => {
            return LatestRecordingDiagnostics {
                active_capture_timing,
                active_recording_id,
                active_segment_id,
                error: Some(error.to_string()),
                latest_segment: None,
                preview_status,
                recording_id: Some(recording_id),
                segment_count: 0,
            };
        }
    };

    LatestRecordingDiagnostics {
        active_capture_timing,
        active_recording_id,
        active_segment_id,
        error: None,
        latest_segment: segments.last().map(latest_segment_diagnostics),
        preview_status,
        recording_id: Some(recording_id),
        segment_count: segments.len(),
    }
}

pub(super) fn latest_segment_diagnostics(
    segment: &RecordingSegmentRow,
) -> LatestSegmentDiagnostics {
    LatestSegmentDiagnostics {
        desktop_audio_stream_index: segment.desktop_audio_stream_index,
        duration_ms: segment.duration_ms,
        ended_at_ms: segment.ended_at_ms,
        error_code: segment.error_code.clone(),
        writer_exit_code: segment.ffmpeg_exit_code,
        file_path: segment.file_path.clone(),
        id: segment.id.clone(),
        microphone_audio_stream_index: segment.microphone_audio_stream_index,
        probe: segment
            .probe_json
            .as_deref()
            .and_then(|probe_json| serde_json::from_str(probe_json).ok()),
        recording_id: segment.recording_id.clone(),
        segment_index: segment.segment_index,
        started_at_ms: segment.started_at_ms,
        state: segment.state.clone(),
        video_stream_index: segment.video_stream_index,
    }
}
