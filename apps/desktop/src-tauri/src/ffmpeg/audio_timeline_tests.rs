use crate::storage::RecordingSegmentRow;
use std::process::Command;

pub(crate) fn gap_fixture() -> RecordingSegmentRow {
    let path = std::env::temp_dir().join(format!(
        "daydream-synthetic-audio-gap-{}.mkv",
        uuid::Uuid::now_v7()
    ));
    let output = Command::new(super::binary_path()).args([
        "-hide_banner", "-v", "error", "-f", "lavfi", "-i", "color=size=32x18:rate=1/5:duration=12",
        "-f", "lavfi", "-i", "sine=frequency=440:sample_rate=48000:duration=12",
        "-f", "lavfi", "-i", "sine=frequency=880:sample_rate=48000:duration=12",
        "-filter_complex", "[1:a]aselect=not(between(t\\,4.4\\,5.2))[desktop];[2:a]aselect=not(between(t\\,4.4\\,5.2))[mic]",
        "-map", "0:v", "-map", "[desktop]", "-map", "[mic]", "-c:v", "libx264", "-threads", "1", "-c:a", "libopus", "-f", "matroska",
    ]).arg(&path).output().unwrap();
    assert!(output.status.success(), "synthetic audio gap mux failed");
    RecordingSegmentRow {
        capture_run_id: None,
        timeline_base_ms: Some(0),
        timeline_basis_version: Some(1),
        desktop_audio_stream_index: Some(1),
        microphone_audio_stream_index: Some(2),
        video_stream_index: Some(0),
        duration_ms: Some(12_000),
        started_at_ms: 0,
        ended_at_ms: Some(12_000),
        error_code: None,
        ffmpeg_exit_code: None,
        file_path: path.to_string_lossy().into_owned(),
        id: "synthetic-segment".into(),
        recording_id: "synthetic-recording".into(),
        segment_index: 1,
        probe_json: None,
        state: "finalized".into(),
    }
}
