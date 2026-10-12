use super::runs::{concat_manifest, groups};
use crate::storage::{RecordingSegmentRow, SegmentVideoStream};
use std::collections::BTreeMap;

pub(crate) fn fragment(index: i64, path: &str) -> RecordingSegmentRow {
    RecordingSegmentRow {
        capture_run_id: Some("synthetic-run".into()),
        timeline_base_ms: Some(index * 30_000),
        timeline_basis_version: Some(1),
        desktop_audio_stream_index: Some(1),
        duration_ms: Some(30_000),
        ended_at_ms: Some((index + 1) * 30_000),
        error_code: None,
        ffmpeg_exit_code: None,
        file_path: path.into(),
        id: format!("fragment-{index}"),
        microphone_audio_stream_index: Some(2),
        probe_json: None,
        recording_id: "synthetic-recording".into(),
        segment_index: index,
        started_at_ms: index * 30_000,
        state: "finalized".into(),
        video_stream_index: Some(0),
    }
}
#[test]
fn runs_preserve_pauses_missing_media_and_source_changes() {
    let path =
        std::env::temp_dir().join(format!("daydream-synthetic-run-{}", uuid::Uuid::now_v7()));
    std::fs::write(&path, b"synthetic").unwrap();
    let path = path.to_str().unwrap();
    let mut fragments = (0..3)
        .map(|index| fragment(index, path))
        .collect::<Vec<_>>();
    let stream = SegmentVideoStream {
        track_id: "screen-synthetic".into(),
        source_id: Some("source-synthetic".into()),
        label: "Synthetic screen".into(),
        stream_index: 0,
        width: Some(32),
        height: Some(18),
        fps: Some(0.2),
    };
    let mut sources = fragments
        .iter()
        .map(|row| (row.id.clone(), vec![stream.clone()]))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(
        groups(&fragments, &sources)
            .iter()
            .map(Vec::len)
            .collect::<Vec<_>>(),
        vec![3]
    );
    // Durable run identity owns continuity even when wall-clock metadata drifts.
    fragments[1].started_at_ms += 100;
    assert_eq!(groups(&fragments, &sources).len(), 1);
    fragments[1].started_at_ms -= 100;
    fragments[2].capture_run_id = Some("resumed-run".into());
    assert_eq!(
        groups(&fragments, &sources)
            .iter()
            .map(Vec::len)
            .collect::<Vec<_>>(),
        vec![2, 1]
    );
    fragments[2].capture_run_id = Some("synthetic-run".into());
    sources.get_mut(&fragments[1].id).unwrap()[0].source_id = Some("other-source".into());
    assert_eq!(groups(&fragments, &sources).len(), 3);
    sources.get_mut(&fragments[1].id).unwrap()[0] = stream;
    fragments[1].state = "failed".into();
    assert_eq!(groups(&fragments, &sources).len(), 2);
    fragments[1].state = "finalized".into();
    for fragment in &mut fragments {
        fragment.capture_run_id = None;
    }
    assert_eq!(groups(&fragments, &sources).len(), 1);
    fragments[1].started_at_ms += 51;
    assert_eq!(groups(&fragments, &sources).len(), 2);
    fragments[1].started_at_ms -= 51;
    fragments[1].timeline_base_ms = None;
    assert_eq!(groups(&fragments, &sources).len(), 3);
    std::fs::remove_file(path).unwrap();
}
#[test]
fn manifests_use_authoritative_durations_and_reject_unavailable_paths() {
    let path = std::env::temp_dir().join(format!(
        "daydream-synthetic-'quoted-{}",
        uuid::Uuid::now_v7()
    ));
    std::fs::write(&path, b"synthetic").unwrap();
    let manifest = concat_manifest(&[(path.to_str().unwrap().into(), 17_000)]).unwrap();
    assert!(manifest.contains("'\\''quoted"));
    assert!(manifest.contains("duration 17.000"));
    assert!(concat_manifest(&[(path.to_str().unwrap().into(), 0)]).is_err());
    std::fs::remove_file(&path).unwrap();
    assert!(concat_manifest(&[(path.to_str().unwrap().into(), 17_000)]).is_err());
}

#[test]
fn sparse_run_keeps_both_audio_tracks_continuous_past_rollover_and_holds_video_tail() {
    for codec in ["pcm_s16le", "libopus"] {
        assert_sparse_run_continuity(codec);
    }
}

fn assert_sparse_run_continuity(codec: &str) {
    use std::process::Command;
    let directory = std::env::temp_dir().join(format!(
        "daydream-synthetic-concat-{}",
        uuid::Uuid::now_v7()
    ));
    std::fs::create_dir(&directory).unwrap();
    let mut fragments = vec![];
    for (index, color) in ["red", "blue"].iter().enumerate() {
        let path = directory.join(format!("{index}.mkv"));
        let output = Command::new(crate::ffmpeg::binary_path())
            .args([
                "-hide_banner",
                "-v",
                "error",
                "-f",
                "lavfi",
                "-i",
                &format!("color={color}:size=32x18:rate=1/5:duration=25"),
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=440:sample_rate=48000:duration=30",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=880:sample_rate=48000:duration=30",
                "-map",
                "0:v",
                "-map",
                "1:a",
                "-map",
                "2:a",
                "-c:v",
                "libx264",
                "-threads",
                "1",
                "-c:a",
                codec,
                "-f",
                "matroska",
            ])
            .arg(&path)
            .output()
            .unwrap();
        assert!(output.status.success(), "synthetic mux failed");
        fragments.push((path.to_string_lossy().into_owned(), 30_000));
    }
    let resources = super::resources::PlaybackResources::default();
    let lease = resources.begin(None, Some("synthetic".into())).unwrap();
    let segment = super::PlaybackSegment {
        microphone_fragments: vec![],
        fragments,
        duration_ms: 60_000,
        playback_fps: 5.,
        file_path: String::new(),
        video_stream_index: Some(0),
        desktop_audio_stream_index: Some(1),
        microphone_audio_stream_index: Some(2),
        video_streams: vec![],
    };
    let input = super::http::playback_input_args(&segment, 0, &lease).unwrap();
    for stream in [1, 2] {
        let audio = Command::new(crate::ffmpeg::binary_path())
            .args(["-hide_banner", "-v", "error"])
            .args(&input)
            .args([
                "-map",
                &format!("0:{stream}"),
                "-vn",
                "-af",
                &crate::ffmpeg::audio_timeline_filter(48_000, segment.duration_ms),
                "-t",
                "60",
                "-f",
                "s16le",
                "pipe:1",
            ])
            .output()
            .unwrap();
        assert!(audio.status.success(), "synthetic run audio failed");
        assert_eq!(audio.stdout.len(), 60 * 48000 * 2, "run sample continuity");
        let sample = |index: usize| {
            i16::from_le_bytes([audio.stdout[index * 2], audio.stdout[index * 2 + 1]]) as i32
        };
        let boundary = 30 * 48000;
        assert!(
            (sample(boundary) - sample(boundary - 1)).abs() < 1000,
            "rollover introduced an audio discontinuity"
        );
        assert!(
            (boundary - 48..boundary + 48).any(|index| sample(index) != 0),
            "rollover introduced silence"
        );
    }
    let mut video = Command::new(crate::ffmpeg::binary_path());
    video
        .args(["-hide_banner", "-v", "error"])
        .args(&input)
        .args(["-map", "0:0", "-an"]);
    super::encoders::append_video_encoder_args_with_tail(
        &mut video,
        &super::encoders::VideoEncoder::Libx264,
        5.,
        60_000,
    );
    let output = video
        .args(["-t", "60", "-f", "framemd5", "pipe:1"])
        .output()
        .unwrap();
    assert!(output.status.success(), "synthetic run video failed");
    let frames = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .filter(|line| !line.starts_with('#') && !line.is_empty())
        .count();
    assert_eq!(frames, 300, "final frame did not cover the audio tail");
    drop(lease);
    std::fs::remove_dir_all(directory).unwrap();
}
