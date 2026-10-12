use super::{run_tests::fragment, PlaybackServer};
use crate::{ffmpeg, storage::RecordingRow};
use std::{collections::BTreeMap, process::Command};

#[test]
fn day_slice_streams_never_include_the_next_day_video_or_audio_tail() {
    let directory = std::env::temp_dir().join(format!("daydream-slice-{}", uuid::Uuid::now_v7()));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("synthetic.mkv");
    let generated = Command::new(ffmpeg::binary_path())
        .args([
            "-hide_banner",
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            "color=c=red:s=8x8:r=10:d=1[r];color=c=blue:s=8x8:r=10:d=1[b];[r][b]concat=n=2:v=1:a=0",
            "-f",
            "lavfi",
            "-i",
            "color=c=lime:s=8x8:r=10:d=1[g];color=c=cyan:s=8x8:r=10:d=1[c];[g][c]concat=n=2:v=1:a=0",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:sample_rate=48000:duration=2",
            "-map",
            "0:v",
            "-map",
            "1:v",
            "-map",
            "2:a",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            "-threads",
            "1",
            "-c:a",
            "pcm_s16le",
        ])
        .arg(&path)
        .status()
        .unwrap();
    assert!(generated.success());
    let recording = RecordingRow {
        id: uuid::Uuid::now_v7().to_string(),
        active_scene_id: None,
        created_at_ms: 0,
        started_at_ms: Some(0),
        ended_at_ms: Some(2000),
        default_fps: 10.0,
        output_root: None,
        quality_label: "synthetic".into(),
        status: "completed".into(),
        storage_dir: directory.to_string_lossy().into_owned(),
        title: None,
    };
    let mut segment = fragment(0, path.to_str().unwrap());
    segment.recording_id = recording.id.clone();
    segment.microphone_audio_stream_index = None;
    segment.desktop_audio_stream_index = Some(2);
    segment.duration_ms = Some(2000);
    let server = PlaybackServer::start().unwrap();
    let sources = BTreeMap::from([(
        segment.id.clone(),
        (0..2)
            .map(|index| crate::storage::SegmentVideoStream {
                track_id: format!("screen-{index}"),
                source_id: Some(format!("monitor-{index}")),
                label: format!("Monitor {}", index + 1),
                stream_index: index,
                width: Some(8),
                height: Some(8),
                fps: Some(10.0),
            })
            .collect(),
    )]);
    let assets = server
        .create_source_session(&recording, &[segment], &sources)
        .unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let client = reqwest::Client::new();
        assert_eq!(assets.segments[0].videos.len(), 2);
        for (name, url, video, channel) in [
            ("monitor-1", &assets.segments[0].videos[0].url, true, 0),
            ("monitor-2", &assets.segments[0].videos[1].url, true, 1),
            ("audio", &assets.segments[0].audio[0].url, false, 0),
        ] {
            let response = client
                .get(format!("{url}?startMs=250&durationMs=500"))
                .send()
                .await
                .unwrap();
            assert!(response.status().is_success());
            let bytes = response.bytes().await.unwrap();
            let media = directory.join(format!("{name}.mp4"));
            std::fs::write(&media, bytes).unwrap();
            let mut decode = Command::new(ffmpeg::binary_path());
            decode
                .args(["-hide_banner", "-v", "error", "-i"])
                .arg(&media);
            if video {
                decode.args(["-an", "-f", "rawvideo", "-pix_fmt", "rgb24", "pipe:1"]);
            } else {
                decode.args(["-vn", "-ar", "48000", "-ac", "1", "-f", "s16le", "pipe:1"]);
            }
            let output = decode.output().unwrap();
            assert!(output.status.success());
            if video {
                assert!(!output.stdout.is_empty());
                assert!(output.stdout.len() <= 6 * 8 * 8 * 3);
                assert!(
                    output
                        .stdout
                        .chunks_exact(3)
                        .all(|pixel| pixel[channel] > 200
                            && pixel[1 - channel] < 40
                            && pixel[2] < 40),
                    "each monitor must retain its own colour and exclude the next-day tail"
                );
            } else {
                assert!(
                    output.stdout.len() > 38400 && output.stdout.len() < 57600,
                    "audio must contain only the half-second slice, including codec padding"
                );
            }
        }
    });
    server.close_session(&assets.session_id).unwrap();
    std::fs::remove_dir_all(directory).unwrap();
}
