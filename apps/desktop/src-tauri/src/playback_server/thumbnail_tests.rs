use super::{ffmpeg_args::thumbnail_ffmpeg_args, PlaybackSegment};
use std::{path::Path, process::Command};

fn jpeg(segment: &PlaybackSegment, stream: i64, offset: i64) -> Vec<u8> {
    let output = Command::new(crate::ffmpeg::binary_path())
        .args(thumbnail_ffmpeg_args(segment, stream, offset))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "synthetic thumbnail extraction failed"
    );
    assert!(
        output.stdout.starts_with(&[0xff, 0xd8]),
        "No JPEG at {offset} ms on stream {stream}"
    );
    output.stdout
}

fn sparse_media(path: &Path) {
    let output = Command::new(crate::ffmpeg::binary_path())
        .args([
            "-hide_banner",
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            "color=red:size=32x18:rate=1/5:duration=10",
            "-f",
            "lavfi",
            "-i",
            "color=green:size=32x18:rate=1/5:duration=10",
            "-f",
            "lavfi",
            "-i",
            "anullsrc=r=48000:cl=mono:d=12",
            "-filter_complex",
            "[0:v]drawbox=color=blue:t=fill:enable='gte(t,5)'[changing]",
            "-map",
            "[changing]",
            "-map",
            "1:v",
            "-map",
            "2:a",
            "-c:v",
            "libx264",
            "-threads",
            "1",
            "-filter_complex_threads",
            "1",
            "-preset",
            "ultrafast",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "pcm_s16le",
            "-y",
        ])
        .arg(path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "synthetic sparse MKV creation failed"
    );
}

#[test]
fn sparse_video_thumbnails_hold_the_current_frame_through_audio_tail_without_looking_ahead() {
    let directory = std::env::temp_dir().join(format!(
        "daydream-sparse-thumbnails-{}",
        uuid::Uuid::now_v7()
    ));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("synthetic.mkv");
    sparse_media(&path);
    let segment = PlaybackSegment {
        fragments: vec![],
        duration_ms: 10_000,
        video_streams: vec![],
        desktop_audio_stream_index: Some(2),
        file_path: path.to_string_lossy().into(),
        microphone_audio_stream_index: None,
        playback_fps: 1.0,
        video_stream_index: Some(0),
    };
    let first = jpeg(&segment, 0, 0);
    assert_eq!(
        jpeg(&segment, 0, 3000),
        first,
        "The frame at five seconds must not appear early"
    );
    assert_eq!(jpeg(&segment, 0, 4999), first);
    let second = jpeg(&segment, 0, 5000);
    assert_ne!(second, first);
    for offset in [6000, 9000, 11000] {
        assert_eq!(jpeg(&segment, 0, offset), second);
    }
    let other = jpeg(&segment, 1, 0);
    assert_ne!(other, first);
    assert_ne!(other, second);
    assert_eq!(
        jpeg(&segment, 1, 11000),
        other,
        "Each MKV video stream retains its own last frame"
    );
    std::fs::remove_dir_all(directory).unwrap();
}
