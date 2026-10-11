use super::{
    frame, input, latest_frame_channel, parent, protocol, Duration, Instant, PROCESS_TEST,
};
use crate::storage::RecordingVideoSource;

fn sources(dimensions: &[(u32, u32)]) -> Vec<RecordingVideoSource> {
    dimensions
        .iter()
        .enumerate()
        .map(|(index, &(width, height))| RecordingVideoSource {
            track_id: format!("synthetic-track-{index}"),
            source_id: format!("synthetic-monitor-{index}"),
            label: format!("Synthetic monitor {index}"),
            width,
            height,
            config_json: "{}".into(),
        })
        .collect()
}

#[test]
fn native_resolution_dual_monitors_fit_at_max_fps() {
    let mut input = input(16, 16, 5.0);
    // Synthetic fixtures for common native monitor resolutions, without pixels.
    for dimensions in [
        [(3840, 2160), (3840, 2160)],
        [(5120, 1440), (5120, 1440)],
        [(5120, 1440), (3840, 2160)],
    ] {
        input.video_sources = sources(&dimensions);
        assert!(
            protocol::validate_input(&input).is_ok(),
            "Native monitor outputs {dimensions:?} must fit the transport allowance."
        );
    }
    std::fs::remove_dir_all(input.output_path.parent().unwrap()).unwrap();
}

#[test]
fn transport_budget_preserves_dimension_fps_and_aggregate_guards() {
    let mut input = input(5120, 1440, 5.0);
    assert!(protocol::validate_input(&input).is_ok());
    for fps in [0.0, -1.0, 5.01, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        input.fps = fps;
        assert!(protocol::validate_input(&input).is_err(), "FPS {fps}");
    }
    input.fps = 0.2;
    assert!(protocol::validate_input(&input).is_ok());
    input.fps = 5.0;
    for (width, height) in [(0, 1440), (5120, 0), (16000, 1440), (u32::MAX, u32::MAX)] {
        input.canvas_width = width;
        input.canvas_height = height;
        assert!(
            protocol::validate_input(&input).is_err(),
            "{width}x{height}"
        );
    }
    // 512 MiB less the 2 MiB socket reserve admits 19,099,062 pixels at
    // seven four-byte frames per pixel; the next pixel exceeds that allowance.
    input.canvas_width = 19_099_062;
    input.canvas_height = 1;
    assert!(protocol::validate_input(&input).is_ok());
    input.canvas_width += 1;
    assert!(protocol::validate_input(&input).is_err());

    // Independent outputs use their own dimensions rather than the canvas.
    input.canvas_width = 16;
    input.canvas_height = 16;
    input.video_sources = sources(&[(3840, 2160); 3]);
    assert!(protocol::validate_input(&input).is_err());
    for dimensions in [
        [(3840, 2160), (0, 2160)],
        [(3840, 2160), (3840, 0)],
        [(3840, 2160), (u32::MAX, u32::MAX)],
    ] {
        input.video_sources = sources(&dimensions);
        assert!(protocol::validate_input(&input).is_err(), "{dimensions:?}");
    }
    std::fs::remove_dir_all(input.output_path.parent().unwrap()).unwrap();
}

#[test]
fn dual_native_monitor_worker_finalizes_both_video_and_audio_streams() {
    let _serial = PROCESS_TEST.lock().unwrap();
    let mut input = input(16, 16, 5.0);
    input.video_sources = sources(&[(3840, 2160); 3]);
    let (sender, frames) = latest_frame_channel(3);
    let error = parent::start_process_segment(&input, frames).unwrap_err();
    assert_eq!(error.code, "recorder_media_transport_budget_exceeded");
    assert!(!input.output_path.exists());
    assert!(parent::is_quiescent());
    drop(sender);

    input.video_sources = sources(&[(5120, 1440), (3840, 2160)]);
    let (sender, frames) = latest_frame_channel(2);
    // Explicitly use the subprocess path: the default test writer runs locally
    // and bypasses transport admission, serialization and child validation.
    let writer = parent::start_process_segment(&input, frames).unwrap();
    let began = Instant::now();
    for tick in 0..10 {
        for source in &input.video_sources {
            let mut next = frame(
                source.width,
                source.height,
                input.started_at_ms + tick * 200,
            );
            next.source_id = source.source_id.clone();
            sender.send(next).unwrap();
        }
        let scheduled = began + Duration::from_millis((tick + 1) * 200);
        std::thread::sleep(scheduled.saturating_duration_since(Instant::now()));
    }
    drop(sender);
    let stop = Instant::now();
    let events = writer.stop(input.started_at_ms + 2000).unwrap();
    assert!(stop.elapsed() < Duration::from_secs(20));
    assert!(events.iter().any(|event| !event.opened));
    assert!(parent::is_quiescent());

    let probe = crate::recorder::media_writer::probe_segment_best_effort(&input.output_path);
    assert!(probe.available, "{probe:?}");
    assert_eq!(probe.videos.len(), 2);
    assert_eq!(probe.audio.len(), 2);
    for (index, source) in input.video_sources.iter().enumerate() {
        let video = &probe.videos[index];
        assert_eq!(video.index, index as i64);
        assert_eq!(
            (video.width, video.height),
            (Some(source.width), Some(source.height))
        );
        assert_eq!(video.codec_name.as_deref(), Some("video/x-h264"));
    }
    assert_eq!(probe.audio[0].index, 2);
    assert_eq!(probe.audio[1].index, 3);
    assert!(probe.duration_ms.is_some_and(|duration| duration > 0));
    std::fs::remove_dir_all(input.output_path.parent().unwrap()).unwrap();
}
