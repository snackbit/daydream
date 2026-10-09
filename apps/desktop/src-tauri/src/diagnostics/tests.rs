use crate::storage::{RecordingSegmentRow, TranscriptSegmentRow};

use super::ai_processing::transcript_track_diagnostics;
use super::latest_recording::latest_segment_diagnostics;

#[test]
fn parses_latest_segment_probe_metadata() {
    let segment = RecordingSegmentRow {
        capture_run_id: None,
        timeline_base_ms: None,
        timeline_basis_version: None,
        desktop_audio_stream_index: Some(1),
        duration_ms: Some(1200),
        ended_at_ms: Some(2200),
        error_code: None,
        ffmpeg_exit_code: Some(0),
        file_path: "/tmp/daydream/segment-0.mkv".to_string(),
        id: "segment-1".to_string(),
        microphone_audio_stream_index: Some(2),
        probe_json: Some(
            serde_json::json!({
                "available": true,
                "durationMs": 1200,
                "streams": [
                    {
                        "index": 0,
                        "kind": "video",
                        "codecName": "h264"
                    }
                ]
            })
            .to_string(),
        ),
        recording_id: "recording-1".to_string(),
        segment_index: 0,
        started_at_ms: 1000,
        state: "finalized".to_string(),
        video_stream_index: Some(0),
    };

    let diagnostics = latest_segment_diagnostics(&segment);

    assert_eq!(
        diagnostics
            .probe
            .as_ref()
            .and_then(|probe| probe.get("available"))
            .and_then(|available| available.as_bool()),
        Some(true)
    );
    assert_eq!(
        diagnostics
            .probe
            .as_ref()
            .and_then(|probe| probe.get("durationMs"))
            .and_then(|duration| duration.as_u64()),
        Some(1200)
    );
    assert_eq!(diagnostics.writer_exit_code, Some(0));
}

#[test]
fn transcript_track_diagnostics_omit_private_text() {
    let diagnostics = transcript_track_diagnostics(vec![TranscriptSegmentRow {
        chunk_id: "chunk-1".to_string(),
        confidence: Some(0.9),
        created_at_ms: 1,
        end_offset_ms: 900,
        id: "transcript-1".to_string(),
        job_id: "job-1".to_string(),
        model: Some("large-v3-turbo".to_string()),
        provider: "local-whisper-cpp".to_string(),
        recording_id: "recording-1".to_string(),
        segment_id: "segment-1".to_string(),
        source_role: "microphone".to_string(),
        start_offset_ms: 100,
        text: "private transcript text".to_string(),
        track_id: "track-1".to_string(),
    }]);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].segment_count, 1);
    assert_eq!(diagnostics[0].text_char_count, 23);
    assert_eq!(diagnostics[0].first_start_offset_ms, Some(100));
    assert_eq!(diagnostics[0].last_end_offset_ms, Some(900));
}
