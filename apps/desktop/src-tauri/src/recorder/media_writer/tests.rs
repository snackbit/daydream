use std::{fs, sync::mpsc, thread, time::Duration};

use gstreamer as gst;
use uuid::Uuid;

use crate::recorder::capture_runtime::{bounded_frame_channel, CapturedFrame, CapturedFrameFormat};
use crate::recorder::types::RecorderMediaWriterCapabilities;

use super::audio::audio_buffer_is_after_cutoff;
use super::frame_pump::{frame_is_after_stop, live_frame_pts};
use super::pipeline::{audio_source_names, build_pipeline_description};
use super::session::start_segment_with_frame_pump;
use super::video_frame::{
    black_yuv420p_frame, frame_is_tightly_packed_for_gstreamer, gst_video_format,
    prepare_video_frame_for_gstreamer, video_caps, PreparedVideoFrame,
};
use super::{
    duration_to_clock_time, gst_fraction, key_int_max, probe_segment_best_effort,
    start_segment_from_frames, start_synthetic_segment, stream_indexes,
    validate_segment_capabilities, MediaSegmentInput,
};

#[test]
fn gstreamer_fraction_formats_low_fps_values() {
    assert_eq!(gst_fraction(1.0), "1/1");
    assert_eq!(gst_fraction(0.5), "1/2");
    assert_eq!(gst_fraction(0.2), "1/5");
    assert_eq!(gst_fraction(0.1), "1/10");
    assert_eq!(gst_fraction(120.0), "60/1");
}

#[test]
fn video_caps_declare_configured_low_fps_framerate() {
    gst::init().expect("initialize gstreamer");
    let frame = PreparedVideoFrame {
        data: black_yuv420p_frame(1280, 720).expect("black yuv frame"),
        format: CapturedFrameFormat::Yuv420p,
        height: 720,
        width: 1280,
    };
    let caps = video_caps(&frame, 0.2).expect("video caps");
    let framerate = caps
        .structure(0)
        .and_then(|structure| structure.get::<gst::Fraction>("framerate").ok())
        .expect("video caps should include framerate");

    assert_eq!(framerate, gst::Fraction::new(1, 5));
}

#[test]
fn keyframe_interval_uses_frame_count_for_low_fps() {
    assert_eq!(key_int_max(1.0, 20), 20);
    assert_eq!(key_int_max(0.2, 20), 4);
    assert_eq!(key_int_max(0.1, 20), 2);
}

#[test]
fn live_frame_pts_preserves_wall_clock_time_instead_of_frame_count() {
    let frame_interval = Duration::from_millis(200);
    let started_at_ms = 1_000;
    let first = live_frame_pts(1_000, started_at_ms, frame_interval, None);
    let second = live_frame_pts(2_000, started_at_ms, frame_interval, Some(first));
    let third = live_frame_pts(3_000, started_at_ms, frame_interval, Some(second));

    assert_eq!(first, Duration::ZERO);
    assert_eq!(second, Duration::from_secs(1));
    assert_eq!(third, Duration::from_secs(2));
}

#[test]
fn live_frame_pts_clamps_duplicate_or_backwards_timestamps_forward() {
    let frame_interval = Duration::from_millis(200);
    let started_at_ms = 1_000;
    let first = live_frame_pts(1_000, started_at_ms, frame_interval, None);
    let duplicate = live_frame_pts(1_000, started_at_ms, frame_interval, Some(first));
    let backwards = live_frame_pts(900, started_at_ms, frame_interval, Some(duplicate));

    assert_eq!(first, Duration::ZERO);
    assert_eq!(duplicate, Duration::from_millis(200));
    assert_eq!(backwards, Duration::from_millis(400));
}

#[test]
fn frame_stop_cutoff_allows_stop_timestamp_and_drops_later_frames() {
    assert!(!frame_is_after_stop(1_000, 1_000));
    assert!(!frame_is_after_stop(999, 1_000));
    assert!(frame_is_after_stop(1_001, 1_000));
}

#[test]
fn audio_stop_cutoff_allows_stop_pts_and_drops_later_buffers() {
    gst::init().expect("initialize gstreamer");
    let cutoff = duration_to_clock_time(Duration::from_millis(1_000));
    let mut at_cutoff = gst::Buffer::new();
    at_cutoff
        .get_mut()
        .expect("writable buffer")
        .set_pts(cutoff);
    let mut after_cutoff = gst::Buffer::new();
    after_cutoff
        .get_mut()
        .expect("writable buffer")
        .set_pts(duration_to_clock_time(Duration::from_millis(1_001)));
    let no_pts = gst::Buffer::new();

    assert!(!audio_buffer_is_after_cutoff(at_cutoff.as_ref(), cutoff));
    assert!(audio_buffer_is_after_cutoff(after_cutoff.as_ref(), cutoff));
    assert!(!audio_buffer_is_after_cutoff(no_pts.as_ref(), cutoff));
}

#[test]
fn stream_indexes_keep_video_first_and_audio_tracks_separate() {
    let input = segment_input(Some("desktop"), Some("mic"));
    let indexes = stream_indexes(&input);

    assert_eq!(indexes.video_stream_index, 0);
    assert_eq!(indexes.desktop_audio_stream_index, Some(1));
    assert_eq!(indexes.microphone_audio_stream_index, Some(2));
}

#[test]
fn audio_source_names_match_configured_audio_branches() {
    assert!(audio_source_names(&segment_input(None, None)).is_empty());
    assert_eq!(
        audio_source_names(&segment_input(Some("desktop"), None)),
        vec!["desktop_audio_src"]
    );
    assert_eq!(
        audio_source_names(&segment_input(None, Some("mic"))),
        vec!["microphone_audio_src"]
    );
    assert_eq!(
        audio_source_names(&segment_input(Some("desktop"), Some("mic"))),
        vec!["desktop_audio_src", "microphone_audio_src"]
    );
}

#[test]
fn pipeline_uses_low_latency_video_encoder_and_explicit_queues() {
    let mut input = segment_input(Some("desktop"), Some("mic"));
    input.video_bitrate_kbps = 2_500;
    let pipeline = build_pipeline_description(&input);

    assert!(pipeline.contains("tune=zerolatency"));
    assert!(pipeline.contains("x264enc bitrate=2500"));
    assert!(pipeline.contains("rc-lookahead=0"));
    assert!(pipeline.contains("bframes=0"));
    assert!(pipeline.contains("sync-lookahead=0"));
    assert!(pipeline.contains("! videoconvert ! video/x-raw,format=I420 ! x264enc"));
    assert!(!pipeline.contains("caps=video/x-raw,format=I420"));
    assert_eq!(
        pipeline
            .matches("queue max-size-time=0 max-size-bytes=0 max-size-buffers=0")
            .count(),
        6
    );
    assert!(pipeline.contains("pulsesrc name=desktop_audio_src"));
    assert!(pipeline.contains("queue name=desktop_audio_cutoff_queue"));
    assert!(pipeline.contains("pulsesrc name=microphone_audio_src"));
    assert!(pipeline.contains("queue name=microphone_audio_cutoff_queue"));
}

#[test]
fn gst_video_formats_match_raw_video_caps_names() {
    assert_eq!(gst_video_format(CapturedFrameFormat::Yuv420p), "I420");
    assert_eq!(gst_video_format(CapturedFrameFormat::Yuv444p), "Y444");
    assert_eq!(gst_video_format(CapturedFrameFormat::Rgba), "RGBA");
    assert_eq!(gst_video_format(CapturedFrameFormat::Bgra), "BGRA");
}

#[test]
fn writer_frame_prep_keeps_tightly_packed_rgba_without_colorspace_conversion() {
    let width = 5_120;
    let height = 1_440;
    let data = vec![42; width * height * 4];
    let frame = CapturedFrame::new(
        "monitor",
        1,
        width as u32,
        height as u32,
        (width * 4) as u32,
        CapturedFrameFormat::Rgba,
        data.clone(),
    )
    .expect("valid rgba frame");

    assert!(
        frame_is_tightly_packed_for_gstreamer(&frame).expect("tight frame check"),
        "full-canvas RGBA frames should be handed to GStreamer without Rust conversion"
    );
    let started = std::time::Instant::now();
    let prepared = prepare_video_frame_for_gstreamer(frame).expect("prepare frame");

    assert_eq!(prepared.format, CapturedFrameFormat::Rgba);
    assert_eq!(prepared.width, width as u32);
    assert_eq!(prepared.height, height as u32);
    assert_eq!(prepared.data.len(), data.len());
    assert!(
        started.elapsed() < Duration::from_millis(50),
        "tight RGBA prep should be a move, not a per-pixel conversion"
    );
}

#[test]
fn writer_frame_prep_repacks_stride_without_changing_format() {
    let frame = CapturedFrame::new(
        "monitor",
        1,
        2,
        2,
        12,
        CapturedFrameFormat::Rgba,
        vec![
            1, 2, 3, 4, 5, 6, 7, 8, 99, 99, 99, 99, 9, 10, 11, 12, 13, 14, 15, 16, 88, 88, 88, 88,
        ],
    )
    .expect("valid padded rgba frame");

    assert!(!frame_is_tightly_packed_for_gstreamer(&frame).expect("tight frame check"));
    let prepared = prepare_video_frame_for_gstreamer(frame).expect("prepare padded frame");

    assert_eq!(prepared.format, CapturedFrameFormat::Rgba);
    assert_eq!(
        prepared.data,
        vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]
    );
}

#[test]
fn video_only_segment_does_not_require_audio_plugins() {
    let capabilities = video_only_capabilities();
    let input = segment_input(None, None);

    validate_segment_capabilities(&capabilities, &input)
        .expect("video-only segment should not require optional audio elements");
}

#[test]
fn audio_segment_requires_audio_plugins() {
    let capabilities = video_only_capabilities();
    let input = segment_input(Some("desktop"), None);

    let error = validate_segment_capabilities(&capabilities, &input)
        .expect_err("audio segment should require audio elements");

    assert_eq!(error.code, "recorder_media_writer_unavailable");
    assert!(error.message.contains("pulsesrc"));
    assert!(error.message.contains("opusenc"));
    assert!(error.message.contains("gstreamer1.0-pulseaudio"));
    assert!(error.message.contains("gstreamer1-plugins-good"));
}

#[test]
fn synthetic_segment_stop_finalizes_non_empty_mkv() {
    let output_path =
        std::env::temp_dir().join(format!("daydream-media-writer-{}.mkv", Uuid::now_v7()));
    let mut input = segment_input(None, None);
    input.output_path = output_path.clone();

    let segment = start_synthetic_segment(&input).expect("start synthetic media segment");
    thread::sleep(Duration::from_millis(150));
    segment
        .stop(input.started_at_ms.saturating_add(150))
        .expect("stop synthetic media segment");

    let metadata = fs::metadata(&output_path).expect("read finalized media segment metadata");
    assert!(metadata.len() > 0);

    let probe = probe_segment_best_effort(&output_path);
    assert!(probe.available, "{probe:?}");
    assert_eq!(
        probe
            .video
            .as_ref()
            .and_then(|video| video.codec_name.as_deref()),
        Some("video/x-h264")
    );

    let _ = fs::remove_file(output_path);
}

#[test]
fn live_frame_segment_stop_drains_queued_frame_before_eos() {
    let output_path =
        std::env::temp_dir().join(format!("daydream-live-media-writer-{}.mkv", Uuid::now_v7()));
    let mut input = segment_input(None, None);
    input.output_path = output_path.clone();
    let (sender, receiver) = bounded_frame_channel(3);
    let frame = CapturedFrame::new(
        "portal-monitor-a",
        1,
        input.canvas_width,
        input.canvas_height,
        input.canvas_width,
        CapturedFrameFormat::Yuv420p,
        black_yuv420p_frame(input.canvas_width, input.canvas_height).expect("black yuv frame"),
    )
    .expect("captured frame");
    let segment =
        start_segment_from_frames(&input, receiver, None).expect("start live media segment");

    sender.send(frame).expect("send captured frame");
    drop(sender);
    segment
        .stop(input.started_at_ms.saturating_add(1))
        .expect("stop live media segment");

    let metadata = fs::metadata(&output_path).expect("read finalized media segment metadata");
    assert!(metadata.len() > 0);

    let probe = probe_segment_best_effort(&output_path);
    assert!(probe.available, "{probe:?}");
    assert_eq!(
        probe
            .video
            .as_ref()
            .and_then(|video| video.codec_name.as_deref()),
        Some("video/x-h264")
    );

    let _ = fs::remove_file(output_path);
}

#[test]
fn segment_stop_returns_error_when_frame_pump_does_not_exit() {
    let output_path = std::env::temp_dir().join(format!(
        "daydream-stuck-media-writer-{}.mkv",
        Uuid::now_v7()
    ));
    let mut input = segment_input(None, None);
    input.output_path = output_path.clone();
    let (release_sender, release_receiver) = mpsc::channel();
    let segment = start_segment_with_frame_pump(&input, false, None, move |_appsrc, _stop| {
        thread::spawn(move || {
            let _ = release_receiver.recv();
            Ok(())
        })
    })
    .expect("start media segment");

    let error = segment
        .stop(input.started_at_ms)
        .expect_err("stuck frame pump should fail stop");
    let _ = release_sender.send(());

    assert_eq!(error.code, "recorder_media_writer_frame_write_failed");
    assert!(error.message.contains("Timed out"));

    let _ = fs::remove_file(output_path);
}

fn segment_input(desktop: Option<&str>, mic: Option<&str>) -> MediaSegmentInput {
    MediaSegmentInput {
        benchmarks_dir: std::env::temp_dir(),
        canvas_height: 720,
        canvas_width: 1280,
        desktop_audio_source_id: desktop.map(ToString::to_string),
        fps: 1.0,
        microphone_audio_source_id: mic.map(ToString::to_string),
        output_path: "recording.mkv".into(),
        recording_id: "recording-1".to_string(),
        segment_id: "segment-1".to_string(),
        segment_index: 1,
        started_at_ms: 0,
        video_bitrate_kbps: 1_500,
    }
}

fn video_only_capabilities() -> RecorderMediaWriterCapabilities {
    RecorderMediaWriterCapabilities {
        appsrc: true,
        audio_convert: false,
        audio_resample: false,
        file_sink: true,
        h264_parse: true,
        matroska_muxer: true,
        opus_encoder: false,
        pulse_source: false,
        queue: true,
        video_convert: true,
        x264_encoder: true,
    }
}
