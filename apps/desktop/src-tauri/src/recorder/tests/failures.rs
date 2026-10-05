use super::*;

#[test]
fn selecting_failure_cannot_be_hidden_by_resume_preparation_recovery() {
    let storage = test_storage();
    let app_data_dir = test_app_data_dir();
    let manager = RecorderManager::new(app_data_dir.clone());
    let scene_id = saved_test_scene(&storage);
    let id = start_test_recording(&manager, &storage, start_input(&scene_id), 100)
        .unwrap()
        .active_recording_id
        .unwrap();
    pause_test_recording(&manager, &storage, &id, 200).unwrap();
    let active = manager.begin_resume_selecting(&id, 300).unwrap();
    let error = super::super::errors::dependency_error(
        "recorder_window_protection_lost",
        "Window metadata unavailable.",
    );
    manager.mark_recording_failure(&id, &error, 301).unwrap();
    let recovered = manager
        .restore_paused_after_selecting(&storage, active, 302)
        .unwrap();
    assert_eq!(recovered.state, RecorderState::Error);
    assert_eq!(recovered.last_error.unwrap().code, error.code);
    assert_eq!(
        storage
            .with_connection(|connection| storage::get_recording(connection, &id))
            .unwrap()
            .status,
        "failed"
    );
    let _ = fs::remove_dir_all(app_data_dir);
}

#[test]
fn failed_segment_finalization_persists_terminal_failure_for_pause_and_stop() {
    for pause in [true, false] {
        let storage = test_storage();
        let app_data_dir = test_app_data_dir();
        let manager = RecorderManager::new(app_data_dir.clone());
        let scene_id = saved_test_scene(&storage);
        let id = start_test_recording(&manager, &storage, start_input(&scene_id), 100)
            .unwrap()
            .active_recording_id
            .unwrap();
        storage.with_connection(|connection| connection.execute_batch("CREATE TRIGGER deny_finalization BEFORE UPDATE ON recording_segments WHEN NEW.state = 'finalized' BEGIN SELECT RAISE(FAIL, 'simulated write failure'); END;")).unwrap();
        let result = if pause {
            manager.begin_pause_recording(&id, 200).unwrap();
            manager.complete_pause_recording(&storage, &id, 201)
        } else {
            manager.begin_stop_recording(&id, 200).unwrap();
            manager.complete_stop_recording(&storage, &id, 201)
        };
        assert_eq!(result.unwrap_err().code, "recorder_storage_error");
        let status = manager.status().unwrap();
        assert_eq!(status.state, RecorderState::Error);
        assert!(status.active_recording_id.is_none());
        assert_eq!(
            storage
                .with_connection(|connection| storage::get_recording(connection, &id))
                .unwrap()
                .status,
            "failed"
        );
        assert_eq!(
            storage
                .with_connection(|connection| storage::list_recording_segments(connection, &id))
                .unwrap()[0]
                .state,
            "failed"
        );
        let _ = fs::remove_dir_all(app_data_dir);
    }
}

#[test]
fn failure_while_pause_finalizes_cannot_be_overwritten_by_paused_success() {
    let storage = test_storage();
    let app_data_dir = test_app_data_dir();
    let manager = RecorderManager::new(app_data_dir.clone());
    let scene_id = saved_test_scene(&storage);
    let id = start_test_recording(&manager, &storage, start_input(&scene_id), 100)
        .unwrap()
        .active_recording_id
        .unwrap();
    manager.begin_pause_recording(&id, 200).unwrap();
    let error = super::super::errors::dependency_error(
        "recorder_window_protection_lost",
        "Window protection lost.",
    );
    manager
        .report_active_failure(&storage, &id, error, 201)
        .unwrap();
    assert_eq!(
        manager
            .complete_pause_recording(&storage, &id, 202)
            .unwrap()
            .state,
        RecorderState::Error
    );
    assert_eq!(
        storage
            .with_connection(|connection| storage::get_recording(connection, &id))
            .unwrap()
            .status,
        "failed"
    );
    let _ = fs::remove_dir_all(app_data_dir);
}

#[test]
fn stop_materialization_failure_never_marks_recording_completed() {
    let storage = test_storage();
    let app_data_dir = test_app_data_dir();
    let manager = RecorderManager::new(app_data_dir.clone());
    let scene_id = saved_test_scene(&storage);
    let id = start_test_recording(&manager, &storage, start_input(&scene_id), 100)
        .unwrap()
        .active_recording_id
        .unwrap();
    storage.with_connection(|connection| {
        storage::create_recording_protection_span(connection, &storage::CreateRecordingProtectionSpanInput { action: "pause".into(), id: Uuid::now_v7().to_string(), opened_at_ms: 150, recording_id: id.clone(), rule_id: "private".into(), rule_label: "Private".into(), start_offset_ms: 50 })?;
        connection.execute_batch("CREATE TRIGGER deny_redaction BEFORE INSERT ON redactions BEGIN SELECT RAISE(FAIL, 'simulated storage failure'); END;")
    }).unwrap();
    manager.begin_stop_recording(&id, 200).unwrap();
    let stopped = manager.complete_stop_recording(&storage, &id, 201).unwrap();
    assert_eq!(stopped.state, RecorderState::Error);
    assert_eq!(
        stopped.last_error.unwrap().code,
        "recorder_window_protection_storage_failed"
    );
    assert_eq!(
        storage
            .with_connection(|connection| storage::get_recording(connection, &id))
            .unwrap()
            .status,
        "failed"
    );
    assert_eq!(
        storage
            .with_connection(|connection| storage::list_open_recording_protection_spans(
                connection, &id
            ))
            .unwrap()
            .len(),
        1
    );
    let _ = fs::remove_dir_all(app_data_dir);
}

#[cfg(target_os = "linux")]
#[test]
fn capture_stops_before_window_monitor_is_joined_and_completed_metadata_is_published() {
    use super::super::{
        capture_runtime::{
            bounded_frame_channel, CaptureSession, CapturedFrame, CapturedFrameFormat,
        },
        manager::RecordingSegmentRuntime,
        media_writer,
        types::RecorderError,
        window_protection::WindowProtectionMonitor,
    };
    use std::sync::Mutex;
    #[derive(Debug)]
    struct OrderedCapture(Arc<Mutex<Vec<&'static str>>>);
    impl CaptureSession for OrderedCapture {
        fn stop(self: Box<Self>, _: u64) -> Result<(), RecorderError> {
            self.0.lock().unwrap().push("capture");
            Ok(())
        }
    }
    let storage = test_storage();
    let app_data_dir = test_app_data_dir();
    let manager = RecorderManager::new(app_data_dir.clone());
    let scene_id = saved_test_scene(&storage);
    manager.begin_selecting(&scene_id, 100).unwrap();
    let mut pending = manager
        .prepare_recording_start(&storage, start_input(&scene_id), 100)
        .unwrap();
    pending.segment_input.canvas_width = 2;
    pending.segment_input.canvas_height = 2;
    manager
        .create_pending_recording_rows(&storage, &pending)
        .unwrap();
    let id = pending.recording_id.clone();
    let order = Arc::new(Mutex::new(Vec::new()));
    let (sender, frames) = bounded_frame_channel(1);
    sender
        .send(
            CapturedFrame::new(
                "composed",
                100,
                2,
                2,
                8,
                CapturedFrameFormat::Rgba,
                vec![0; 16],
            )
            .unwrap(),
        )
        .unwrap();
    let media =
        media_writer::start_segment_from_frames(&pending.segment_input, frames, None).unwrap();
    manager
        .activate_pending_recording(
            pending,
            RecordingSegmentRuntime::new(
                media,
                Some(Box::new(OrderedCapture(order.clone()))),
                None,
            ),
            101,
        )
        .unwrap();
    let monitor_order = order.clone();
    manager
        .attach_protection_monitor(
            &id,
            WindowProtectionMonitor::for_test(move || {
                monitor_order.lock().unwrap().push("monitor")
            }),
        )
        .unwrap();
    manager.begin_stop_recording(&id, 200).unwrap();
    let stopped = manager.complete_stop_recording(&storage, &id, 201).unwrap();
    assert_eq!(*order.lock().unwrap(), ["capture", "monitor"]);
    assert_eq!(stopped.state, RecorderState::Idle);
    assert_eq!(
        storage
            .with_connection(|connection| storage::get_recording(connection, &id))
            .unwrap()
            .status,
        "completed"
    );
    drop(sender);
    let _ = fs::remove_dir_all(app_data_dir);
}

#[cfg(target_os = "linux")]
#[test]
fn active_capture_failure_stops_media_and_persists_visible_error_without_user_stop() {
    use super::super::{
        capture_runtime::CaptureSession, manager::RecordingSegmentRuntime, media_writer,
        types::RecorderError,
    };
    use std::sync::atomic::AtomicBool;
    #[derive(Debug)]
    struct FailedCapture(Arc<AtomicBool>);
    impl CaptureSession for FailedCapture {
        fn poll_error(&self) -> Option<RecorderError> {
            Some(super::super::errors::capture_adapter_failed_error(
                "Capture failed.",
            ))
        }
        fn stop(self: Box<Self>, _: u64) -> Result<(), RecorderError> {
            self.0.store(true, Ordering::SeqCst);
            Ok(())
        }
    }
    let storage = test_storage();
    let app_data_dir = test_app_data_dir();
    let manager = RecorderManager::new(app_data_dir.clone());
    let scene_id = saved_test_scene(&storage);
    manager.begin_selecting(&scene_id, 100).unwrap();
    let mut pending = manager
        .prepare_recording_start(&storage, start_input(&scene_id), 100)
        .unwrap();
    pending.segment_input.canvas_width = 2;
    pending.segment_input.canvas_height = 2;
    manager
        .create_pending_recording_rows(&storage, &pending)
        .unwrap();
    let id = pending.recording_id.clone();
    let stopped = Arc::new(AtomicBool::new(false));
    let media = media_writer::start_synthetic_segment(&pending.segment_input).unwrap();
    manager
        .activate_pending_recording(
            pending,
            RecordingSegmentRuntime::new(
                media,
                Some(Box::new(FailedCapture(stopped.clone()))),
                None,
            ),
            101,
        )
        .unwrap();
    let error = manager
        .active_runtime_error(&id)
        .unwrap()
        .expect("observe native failure");
    let status = manager
        .report_active_failure(&storage, &id, error, 102)
        .unwrap();
    assert_eq!(status.state, RecorderState::Error);
    assert_eq!(
        status.last_error.unwrap().code,
        "recorder_capture_adapter_failed"
    );
    assert!(stopped.load(Ordering::SeqCst));
    assert!(status.active_recording_id.is_none());
    assert_eq!(
        storage
            .with_connection(|connection| storage::get_recording(connection, &id))
            .unwrap()
            .status,
        "failed"
    );
    assert_eq!(
        storage
            .with_connection(|connection| storage::list_recording_segments(connection, &id))
            .unwrap()[0]
            .state,
        "failed"
    );
    let _ = fs::remove_dir_all(app_data_dir);
}
