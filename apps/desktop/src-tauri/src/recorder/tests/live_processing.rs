use super::super::{
    capture_runtime::{bounded_frame_channel, CapturedFrame, CapturedFrameFormat},
    manager::RecordingSegmentRuntime,
    media_writer,
};
use super::*;

#[test]
fn finalized_fragments_keep_one_capture_session_and_stop_preserves_the_final_fragment() {
    let storage = test_storage();
    let root = test_app_data_dir();
    let manager = RecorderManager::new(root.clone());
    let scene = saved_test_scene(&storage);
    manager.begin_selecting(&scene, 1000).unwrap();
    let mut pending = manager
        .prepare_recording_start(&storage, start_input(&scene), 1000)
        .unwrap();
    pending.segment_input.canvas_width = 64;
    pending.segment_input.canvas_height = 32;
    pending.segment_input.fps = 1.0;
    manager
        .create_pending_recording_rows(&storage, &pending)
        .unwrap();
    let id = pending.recording_id.clone();
    let (sender, receiver) = bounded_frame_channel(3);
    let writer =
        media_writer::start_segment_from_frames(&pending.segment_input, receiver, None).unwrap();
    manager
        .activate_pending_recording(
            pending,
            RecordingSegmentRuntime::new(writer, None, None),
            1000,
        )
        .unwrap();
    for index in 0..71u64 {
        let pixels = vec![128; 64 * 32 * 3 / 2];
        sender
            .send(
                CapturedFrame::new(
                    "synthetic-source",
                    1000 + index * 1000,
                    64,
                    32,
                    64,
                    CapturedFrameFormat::Yuv420p,
                    pixels,
                )
                .unwrap(),
            )
            .unwrap();
        // The real capture channel deliberately replaces stale frames. Let the
        // writer consume this frame before producing the next synthetic second.
        thread::sleep(Duration::from_millis(10));
        manager.persist_capture_fragments(&storage, &id).unwrap();
    }
    // A single capture/writer accepts seventy seconds of source PTS in this
    // bounded synthetic soak. No portal, microphone or private media is used.
    stop_test_recording(&manager, &storage, &id, 71_000).unwrap();
    drop(sender);
    let segments = storage
        .with_connection(|connection| storage::list_recording_segments(connection, &id))
        .unwrap();
    assert!(
        segments.len() >= 3,
        "The source must rotate without stopping its capture session."
    );
    assert!(
        segments
            .iter()
            .all(|segment| segment.state == "finalized"
                && PathBuf::from(&segment.file_path).is_file())
    );
    assert_eq!(
        segments
            .iter()
            .map(|segment| segment.duration_ms.unwrap())
            .sum::<i64>(),
        70_000
    );
    assert!(segments
        .windows(2)
        .all(|pair| pair[0].ended_at_ms == Some(pair[1].started_at_ms)));
    assert_eq!(manager.status().unwrap().state, RecorderState::Idle);
    fs::remove_dir_all(root).unwrap();
}
