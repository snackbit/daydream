use super::{parent, protocol, recovery};
use crate::recorder::{
    capture_runtime::{
        bounded_frame_channel, latest_frame_channel, CapturedFrame, CapturedFrameFormat,
    },
    media_writer::MediaSegmentInput,
};
use std::{
    io::Cursor,
    sync::Mutex,
    time::{Duration, Instant},
};
pub(crate) static PROCESS_TEST: Mutex<()> = Mutex::new(());

mod transport_budget;

#[test]
fn worker_entry() {
    if std::env::var_os("DAYDREAM_TEST_MEDIA_WORKER").is_some() {
        std::process::exit(super::run_worker_process(198, 199));
    }
}
fn input(width: u32, height: u32, fps: f64) -> MediaSegmentInput {
    let dir = std::env::temp_dir().join(format!("daydream-worker-test-{}", uuid::Uuid::now_v7()));
    std::fs::create_dir_all(&dir).unwrap();
    MediaSegmentInput {
        video_sources: vec![],
        benchmarks_dir: dir.join("benchmarks"),
        canvas_width: width,
        canvas_height: height,
        desktop_audio_source_id: Some("__synthetic_audio__".into()),
        microphone_audio_source_id: Some("__synthetic_audio__".into()),
        fps,
        output_path: dir.join("0001.mkv"),
        recording_id: "synthetic".into(),
        segment_id: "synthetic-segment".into(),
        segment_index: 1,
        started_at_ms: 1000,
        video_bitrate_kbps: 1000,
    }
}
fn frame(width: u32, height: u32, timestamp_ms: u64) -> CapturedFrame {
    CapturedFrame::new(
        "synthetic",
        timestamp_ms,
        width,
        height,
        width * 4,
        CapturedFrameFormat::Bgrx,
        vec![0; width as usize * height as usize * 4],
    )
    .unwrap()
}

#[test]
fn transport_rejects_stale_generation_source_sequences_and_truncated_pixels() {
    let input = input(16, 16, 5.0);
    let mut bytes = vec![];
    protocol::write_frame(&mut bytes, 7, 0, 1, frame(16, 16, 1000)).unwrap();
    assert!(protocol::read_frame(&mut Cursor::new(&bytes), 8, &input, &mut [0]).is_err());
    assert!(protocol::read_frame(&mut Cursor::new(&bytes), 7, &input, &mut [1]).is_err());
    let mut invalid = bytes.clone();
    invalid[8..12].copy_from_slice(&1u32.to_le_bytes());
    assert!(protocol::read_frame(&mut Cursor::new(&invalid), 7, &input, &mut [0]).is_err());
    bytes.pop();
    let error = protocol::read_frame(&mut Cursor::new(&bytes), 7, &input, &mut [0]).unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    std::fs::remove_dir_all(input.output_path.parent().unwrap()).unwrap();
}
#[test]
fn latest_pending_frame_replaces_unsent_source_without_cloning_pixels() {
    let (sender, receiver) = latest_frame_channel(2);
    sender.try_send(frame(16, 16, 1000)).unwrap();
    let newest = frame(16, 16, 1001);
    let allocation = newest.data.as_ptr();
    sender.try_send(newest).unwrap();
    let received = receiver.recv_timeout(Duration::ZERO).unwrap();
    assert_eq!(received.timestamp_ms, 1001);
    assert_eq!(received.data.as_ptr(), allocation);
}

#[test]
pub(super) fn same_executable_worker_finalizes_video_and_two_real_gstreamer_audio_branches() {
    let _serial = PROCESS_TEST.lock().unwrap();
    let input = input(320, 180, 5.0);
    let (sender, frames) = bounded_frame_channel(1);
    let writer = parent::start_process_segment(&input, frames).unwrap();
    let app_root = input.output_path.parent().unwrap();
    let storage = crate::storage::test_storage(app_root.into());
    let original = uuid::Uuid::now_v7().to_string();
    let output = uuid::Uuid::now_v7().to_string();
    let root = app_root.join("recordings");
    let copied = root.join(&output);
    std::fs::create_dir_all(&copied).unwrap();
    std::fs::write(copied.join("private.mkv"), b"synthetic recovery copy").unwrap();
    storage
        .with_connection(|connection| {
            crate::storage::create_recording(
                connection,
                &crate::storage::CreateRecordingInput {
                    id: original.clone(),
                    active_scene_id: None,
                    created_at_ms: 1,
                    default_fps: 1.0,
                    notes: None,
                    output_root: None,
                    quality_label: "balanced-ocr".into(),
                    storage_dir: root.join(&original).to_string_lossy().into(),
                },
            )?;
            crate::storage::recovery::start_operation(
                connection,
                &uuid::Uuid::now_v7().to_string(),
                &original,
                &output,
                1,
            )
        })
        .unwrap();
    // Cleanup must retain ownership until every media writer has exited.
    let cleanup =
        crate::recording_deletion::remove_recovery_outputs(&storage, &root, Some(&original));
    let retained_file = copied.join("private.mkv").exists();
    let retained_owner = storage
        .with_connection(|connection| {
            crate::storage::recovery::pending_outputs(connection, Some(&original))
        })
        .unwrap();
    for index in 0..5 {
        sender.send(frame(320, 180, 1000 + index * 200)).unwrap();
        std::thread::sleep(Duration::from_millis(200));
    }
    drop(sender);
    let started = Instant::now();
    let events = writer.stop(2000).unwrap();
    assert!(started.elapsed() < Duration::from_secs(20));
    assert!(events.iter().any(|event| !event.opened));
    assert!(parent::is_quiescent());
    assert!(cleanup.is_err());
    assert!(retained_file);
    assert_eq!(retained_owner.len(), 1);
    crate::recording_deletion::remove_recovery_outputs(&storage, &root, Some(&original)).unwrap();
    assert!(!copied.exists());
    let probe = crate::recorder::media_writer::probe_segment_best_effort(&input.output_path);
    assert!(probe.available);
    assert_eq!(probe.audio.len(), 2);
    assert_eq!(probe.videos.len(), 1);
    let recovered = recovery::recover_media(
        &input.output_path,
        &input.output_path.with_file_name("recovered.mkv"),
    )
    .unwrap();
    assert_eq!(recovered.streams.len(), 3);
    assert!(recovered
        .streams
        .iter()
        .all(|stream| stream.original_index == stream.output_index));
    assert!(recovered.end_pts_ms > recovered.start_pts_ms);
    std::fs::remove_dir_all(input.output_path.parent().unwrap()).unwrap();
}

#[test]
fn wide_monitor_low_fps_fourteen_second_recording_finishes_without_stream_lock_deadlock() {
    let _serial = PROCESS_TEST.lock().unwrap();
    let input = input(5120, 1440, 0.2);
    let (sender, frames) = latest_frame_channel(1);
    let writer = parent::start_process_segment(&input, frames).unwrap();
    sender.send(frame(5120, 1440, 1000)).unwrap();
    std::thread::sleep(Duration::from_secs(5));
    sender.send(frame(5120, 1440, 6000)).unwrap();
    std::thread::sleep(Duration::from_secs(5));
    sender.send(frame(5120, 1440, 11000)).unwrap();
    std::thread::sleep(Duration::from_secs(4));
    drop(sender);
    let started = Instant::now();
    let events = writer.stop(15000).unwrap();
    assert!(started.elapsed() < Duration::from_secs(20));
    assert!(events.iter().any(|event| !event.opened));
    assert!(parent::is_quiescent());
    let probe = crate::recorder::media_writer::probe_segment_best_effort(&input.output_path);
    assert_eq!(probe.audio.len(), 2);
    assert_eq!(probe.video.unwrap().width, Some(5120));
    std::fs::remove_dir_all(input.output_path.parent().unwrap()).unwrap();
}

#[test]
fn stalled_worker_is_killed_reaped_and_reports_failure_before_outward_deadline() {
    let _serial = PROCESS_TEST.lock().unwrap();
    let input = input(320, 180, 5.0);
    let (sender, frames) = bounded_frame_channel(1);
    let mut writer = parent::start_process_segment(&input, frames).unwrap();
    sender.send(frame(320, 180, 1000)).unwrap();
    std::thread::sleep(Duration::from_millis(100));
    writer.stall_for_test();
    drop(sender);
    let began = Instant::now();
    let error = writer
        .stop_until(1100, began + Duration::from_millis(5500))
        .unwrap_err();
    assert_eq!(error.code, "recorder_media_worker_stop_timeout");
    assert!(began.elapsed() < Duration::from_millis(5500));
    assert!(parent::is_quiescent());
    assert!(input.output_path.exists());
    std::fs::remove_dir_all(input.output_path.parent().unwrap()).unwrap();
}

#[test]
fn unexpected_transport_termination_reports_failure_before_stop() {
    let _serial = PROCESS_TEST.lock().unwrap();
    let input = input(320, 180, 5.0);
    let (sender, frames) = bounded_frame_channel(1);
    let mut writer = parent::start_process_segment(&input, frames).unwrap();
    sender.send(frame(320, 180, 1000)).unwrap();
    drop(sender);
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        if let Some(error) = writer.poll_error() {
            assert_eq!(error.code, "recorder_media_transport_failed");
            break;
        }
        assert!(
            Instant::now() < deadline,
            "Transport termination remained invisible during capture."
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    writer.abort();
    assert!(parent::is_quiescent());
    std::fs::remove_dir_all(input.output_path.parent().unwrap()).unwrap();
}

#[test]
fn wide_monitor_max_fps_transport_benchmark_keeps_capture_handoff_bounded() {
    let _serial = PROCESS_TEST.lock().unwrap();
    let input = input(5120, 1440, 5.0);
    let (sender, frames) = latest_frame_channel(1);
    let writer = parent::start_process_segment(&input, frames).unwrap();
    let began = Instant::now();
    let mut longest = Duration::ZERO;
    for index in 0..10 {
        let next = frame(5120, 1440, 1000 + index * 200);
        let handoff = Instant::now();
        sender.try_send(next).unwrap();
        longest = longest.max(handoff.elapsed());
        let scheduled = began + Duration::from_millis((index + 1) * 200);
        std::thread::sleep(scheduled.saturating_duration_since(Instant::now()));
    }
    drop(sender);
    let stop = Instant::now();
    writer.stop(3000).unwrap();
    assert!(stop.elapsed() < Duration::from_secs(20));
    assert!(
        longest < Duration::from_millis(100),
        "Capture callback waited for transport or encoding."
    );
    assert!(parent::is_quiescent());
    eprintln!(
        "synthetic 5120x1440 benchmark: 10 owned 28 MiB frames at 5 fps, maximum handoff={} us, stop={} ms",
        longest.as_micros(),
        stop.elapsed().as_millis()
    );
    std::fs::remove_dir_all(input.output_path.parent().unwrap()).unwrap();
}

#[test]
fn legacy_audio_eos_probe_reports_observed_wide_low_fps_graph_result() {
    let _serial = PROCESS_TEST.lock().unwrap();
    let input = input(5120, 1440, 0.2);
    let (sender, frames) = latest_frame_channel(1);
    let mut writer = parent::start_process_segment(&input, frames).unwrap();
    sender.send(frame(5120, 1440, 1000)).unwrap();
    std::thread::sleep(Duration::from_secs(5));
    sender.send(frame(5120, 1440, 6000)).unwrap();
    std::thread::sleep(Duration::from_secs(5));
    sender.send(frame(5120, 1440, 11000)).unwrap();
    std::thread::sleep(Duration::from_secs(4));
    drop(sender);
    let began = Instant::now();
    let result = writer.legacy_eos_for_test(15000);
    writer.abort();
    assert!(parent::is_quiescent());
    match result{Ok(())=>eprintln!("synthetic audiotestsrc legacy EOS completed; original Pulse timeout was not reproduced by this unmodified graph"),Err(failure)=>{assert_eq!(failure.code,"recorder_media_writer_finalize_failed");assert!(failure.message.contains("Timed out sending EOS"));eprintln!("synthetic audiotestsrc legacy EOS observed its source timeout");}}
    assert!(began.elapsed() < Duration::from_secs(5));
    eprintln!(
        "synthetic legacy desktop EOS probe finished after {} ms; child killed and reaped",
        began.elapsed().as_millis()
    );
    std::fs::remove_dir_all(input.output_path.parent().unwrap()).unwrap();
}

#[test]
fn audio_stream_lock_reproduces_eos_then_null_deadlock_and_parent_stop_stays_bounded() {
    let _serial = PROCESS_TEST.lock().unwrap();
    let input = input(320, 180, 5.0);
    let (sender, frames) = bounded_frame_channel(1);
    let mut writer = parent::start_process_segment(&input, frames).unwrap();
    sender.send(frame(320, 180, 1000)).unwrap();
    writer.hold_audio_pad_for_test();
    let eos = Instant::now();
    let error = writer.legacy_eos_for_test(1200).unwrap_err();
    assert_eq!(error.code, "recorder_media_writer_finalize_failed");
    assert!(error.message.contains("Timed out sending EOS"));
    assert!(
        writer.null_is_blocked_for_test(),
        "GStreamer Null returned while the EOS sender held its element state lock."
    );
    drop(sender);
    let stop = Instant::now();
    let error = writer
        .stop_until(1200, stop + Duration::from_millis(5500))
        .unwrap_err();
    assert_eq!(error.code, "recorder_media_worker_stop_timeout");
    assert!(stop.elapsed() < Duration::from_millis(5500));
    assert!(parent::is_quiescent());
    eprintln!("synthetic GStreamer source STREAM_LOCK held: source EOS timed out and Null remained blocked; total fault cleanup {} ms",eos.elapsed().as_millis());
    std::fs::remove_dir_all(input.output_path.parent().unwrap()).unwrap();
}

#[test]
fn media_worker_survives_short_lived_startup_task_thread_exit() {
    let _serial = PROCESS_TEST.lock().unwrap();
    let input = input(320, 180, 5.0);
    let start_input = input.clone();
    let (sender, frames) = bounded_frame_channel(1);
    let task =
        std::thread::spawn(move || parent::start_process_segment(&start_input, frames).unwrap());
    let mut writer = task.join().unwrap();
    std::thread::sleep(Duration::from_millis(100));
    assert!(
        writer.poll_error().is_none(),
        "Creating startup task thread exiting terminated the media worker."
    );
    sender.send(frame(320, 180, 1000)).unwrap();
    std::thread::sleep(Duration::from_millis(200));
    drop(sender);
    writer.stop(1200).unwrap();
    assert!(parent::is_quiescent());
    std::fs::remove_dir_all(input.output_path.parent().unwrap()).unwrap();
}
