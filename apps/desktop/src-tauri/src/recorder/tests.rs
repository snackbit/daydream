use std::{
    fs,
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Barrier,
    },
    thread,
    time::{Duration, Instant},
};

use serde_json::json;
use tauri::Manager as _;
use uuid::Uuid;

use super::capture_runtime::CapturePreviewSession;
use super::commands::{
    collect_recorder_recording_stats, get_recorder_scene, pause_recording, resume_recording,
    save_recorder_scene, select_capture_sources, start_recording, stop_empty_preview_start,
    stop_recording,
};
use super::errors::unavailable_error;
use super::manager::{PreviewStartDecision, RecorderManager};
use super::storage_mapping::save_recorder_scene_for_storage;
use super::types::{
    CanvasRect, CaptureSource, CaptureSourceKind, CropRect, PauseRecordingInput,
    RecorderErrorCategory, RecorderQualityPreset, RecorderScene, RecorderSceneLayoutMode,
    RecorderSceneSource, RecorderState, RecorderStatus, ResumeRecordingInput,
    SaveRecorderSceneInput, StartRecordingInput, StopRecordingInput,
};
use crate::storage;

mod failures;
mod live_processing;
mod protection;
mod setup;

#[test]
fn unavailable_error_uses_locked_gate_a_shape() {
    let error = unavailable_error();

    assert_eq!(error.category, RecorderErrorCategory::Unavailable);
    assert_eq!(error.code, "recorder_backend_not_implemented");
    assert_eq!(error.message, "Recorder backend is not implemented yet.");
    assert_eq!(error.details, None);

    let serialized = serde_json::to_value(error).expect("serialize recorder error");
    assert_eq!(
        serialized,
        json!({
            "category": "unavailable",
            "code": "recorder_backend_not_implemented",
            "message": "Recorder backend is not implemented yet.",
        }),
    );
}

#[test]
fn enum_serialization_uses_kebab_case() {
    assert_eq!(
        serde_json::to_value(RecorderState::Unavailable).expect("serialize recorder state"),
        json!("unavailable"),
    );
    assert_eq!(
        serde_json::to_value(RecorderQualityPreset::BalancedOcr).expect("serialize quality preset"),
        json!("balanced-ocr"),
    );
    assert_eq!(
        serde_json::to_value(RecorderSceneLayoutMode::Auto169)
            .expect("serialize auto scene layout"),
        json!("auto-16-9"),
    );
    assert_eq!(
        serde_json::to_value(RecorderSceneLayoutMode::Vertical)
            .expect("serialize vertical scene layout"),
        json!("vertical"),
    );
}

#[test]
fn struct_serialization_uses_camel_case_fields() {
    let value = serde_json::to_value(CanvasRect {
        height: 1440.0,
        width: 2560.0,
        x: 0.0,
        y: 0.0,
    })
    .expect("serialize canvas rect");

    assert_eq!(
        value,
        json!({
            "height": 1440.0,
            "width": 2560.0,
            "x": 0.0,
            "y": 0.0,
        }),
    );
}

#[test]
fn manager_initial_status_is_idle() {
    let manager = RecorderManager::new(test_app_data_dir());
    let status = manager.status().expect("read status");

    assert_eq!(status.state, RecorderState::Idle);
    assert_eq!(status.active_recording_id, None);
    assert_eq!(status.active_segment_id, None);
    assert!(!status.backend_health.media_writer.available);
}

#[test]
fn recording_stats_are_empty_without_active_recording() {
    let storage = test_storage();
    let manager = RecorderManager::new(test_app_data_dir());

    let stats =
        collect_recorder_recording_stats(&storage, &manager, 10).expect("collect recording stats");

    assert_eq!(stats, None);
}

#[test]
fn recording_stats_include_active_segment_and_total_file_sizes() {
    let storage = test_storage();
    let app_data_dir = test_app_data_dir();
    let manager = RecorderManager::new(app_data_dir.clone());
    let scene_id = saved_test_scene(&storage);
    let started = start_test_recording(&manager, &storage, start_input(&scene_id), 100)
        .expect("start recording");
    let recording_id = started
        .active_recording_id
        .clone()
        .expect("active recording id");

    let first_segments = storage
        .with_connection(|connection| storage::list_recording_segments(connection, &recording_id))
        .expect("list first segment");
    let first_segment = first_segments.first().expect("first segment");
    fs::write(&first_segment.file_path, [1_u8, 2, 3, 4]).expect("write first segment");

    pause_test_recording(&manager, &storage, &recording_id, 200).expect("pause recording");
    resume_test_recording(&manager, &storage, &recording_id, 250).expect("resume recording");

    let resumed_segments = storage
        .with_connection(|connection| storage::list_recording_segments(connection, &recording_id))
        .expect("list resumed segments");
    let active_segment = resumed_segments.last().expect("active segment");
    fs::write(&active_segment.file_path, [1_u8, 2, 3, 4, 5, 6, 7]).expect("write active segment");

    let stats = collect_recorder_recording_stats(&storage, &manager, 300)
        .expect("collect recording stats")
        .expect("active recording stats");

    assert_eq!(stats.recording_id, recording_id);
    assert_eq!(stats.recording_started_at_ms, Some(100));
    assert_eq!(
        stats.active_segment_id.as_deref(),
        Some(active_segment.id.as_str())
    );
    assert_eq!(stats.active_segment_index, Some(2));
    assert_eq!(stats.active_segment_started_at_ms, Some(250));
    assert_eq!(stats.active_segment_size_bytes, Some(7));
    assert_eq!(stats.total_size_bytes, 11);
    assert_eq!(stats.segment_count, 2);
    assert_eq!(stats.updated_at_ms, 300);

    let _ = fs::remove_dir_all(app_data_dir);
}

#[test]
fn recording_stats_ignore_missing_segment_files_for_size_totals() {
    let storage = test_storage();
    let app_data_dir = test_app_data_dir();
    let manager = RecorderManager::new(app_data_dir.clone());
    let scene_id = saved_test_scene(&storage);
    let started = start_test_recording(&manager, &storage, start_input(&scene_id), 100)
        .expect("start recording");
    let recording_id = started
        .active_recording_id
        .clone()
        .expect("active recording id");

    let stats = collect_recorder_recording_stats(&storage, &manager, 150)
        .expect("collect recording stats")
        .expect("active recording stats");

    assert_eq!(stats.recording_id, recording_id);
    assert_eq!(stats.active_segment_size_bytes, None);
    assert_eq!(stats.total_size_bytes, 0);

    let _ = fs::remove_dir_all(app_data_dir);
}

#[test]
fn manager_does_not_launch_duplicate_preview_for_same_pending_scene() {
    let manager = RecorderManager::new(test_app_data_dir());
    let scene_id = "scene-1";
    let first = manager
        .begin_preview_start(scene_id, 10)
        .expect("begin first preview");
    let second = manager
        .begin_preview_start(scene_id, 11)
        .expect("begin duplicate preview");

    let PreviewStartDecision::Pending { generation, .. } = first else {
        panic!("first preview should be pending");
    };
    let PreviewStartDecision::AlreadyCurrent(status) = second else {
        panic!("duplicate preview should use current starting status");
    };

    assert_eq!(status.state, super::types::RecorderPreviewState::Starting);
    assert_eq!(status.scene_id.as_deref(), Some(scene_id));

    let stopped = Arc::new(AtomicUsize::new(0));
    let activated = manager
        .activate_preview(
            scene_id.to_string(),
            generation,
            vec!["monitor-1".to_string()],
            Box::new(TestPreviewSession {
                stopped: stopped.clone(),
            }),
            12,
        )
        .expect("activate current preview");

    assert_eq!(activated.state, super::types::RecorderPreviewState::Live);
    assert_eq!(stopped.load(Ordering::SeqCst), 0);

    manager.stop_preview(13).expect("stop live preview");
    assert_eq!(stopped.load(Ordering::SeqCst), 1);
}

#[test]
fn manager_stops_preview_session_that_finishes_after_cancellation() {
    let manager = RecorderManager::new(test_app_data_dir());
    let scene_id = "scene-1";
    let PreviewStartDecision::Pending { generation, .. } = manager
        .begin_preview_start(scene_id, 10)
        .expect("begin preview")
    else {
        panic!("preview should be pending");
    };

    manager.stop_preview(11).expect("cancel preview");
    let stopped = Arc::new(AtomicUsize::new(0));
    let status = manager
        .activate_preview(
            scene_id.to_string(),
            generation,
            vec!["monitor-1".to_string()],
            Box::new(TestPreviewSession {
                stopped: stopped.clone(),
            }),
            12,
        )
        .expect("stale activation should clean up");

    assert_eq!(status.state, super::types::RecorderPreviewState::Idle);
    assert_eq!(stopped.load(Ordering::SeqCst), 1);
    assert_eq!(
        manager.preview_status().expect("preview status").state,
        super::types::RecorderPreviewState::Idle
    );
}

#[test]
fn manager_marks_cancelled_preview_start_stale_before_platform_start() {
    let manager = RecorderManager::new(test_app_data_dir());
    let scene_id = "scene-1";
    let PreviewStartDecision::Pending { generation, .. } = manager
        .begin_preview_start(scene_id, 10)
        .expect("begin preview")
    else {
        panic!("preview should be pending");
    };

    assert!(manager.preview_start_is_current(scene_id, generation));

    manager.stop_preview(11).expect("cancel preview");

    assert!(!manager.preview_start_is_current(scene_id, generation));
}

#[test]
fn manager_stops_preview_session_that_finishes_after_recording_selection_starts() {
    let manager = RecorderManager::new(test_app_data_dir());
    let scene_id = "scene-1";
    let PreviewStartDecision::Pending { generation, .. } = manager
        .begin_preview_start(scene_id, 10)
        .expect("begin preview")
    else {
        panic!("preview should be pending");
    };

    manager
        .begin_selecting(scene_id, 11)
        .expect("begin recording selection");
    let stopped = Arc::new(AtomicUsize::new(0));
    let status = manager
        .activate_preview(
            scene_id.to_string(),
            generation,
            vec!["monitor-1".to_string()],
            Box::new(TestPreviewSession {
                stopped: stopped.clone(),
            }),
            12,
        )
        .expect("recording lifecycle should reject preview activation");

    assert_eq!(status.state, super::types::RecorderPreviewState::Idle);
    assert_eq!(stopped.load(Ordering::SeqCst), 1);
    assert_eq!(
        manager.preview_status().expect("preview status").state,
        super::types::RecorderPreviewState::Idle
    );
}

#[test]
fn manager_ignores_stale_preview_start_failure_after_cancellation() {
    let manager = RecorderManager::new(test_app_data_dir());
    let scene_id = "scene-1";
    let PreviewStartDecision::Pending { generation, .. } = manager
        .begin_preview_start(scene_id, 10)
        .expect("begin preview")
    else {
        panic!("preview should be pending");
    };

    manager.stop_preview(11).expect("cancel preview");
    let status = manager
        .fail_preview_start(scene_id.to_string(), generation, unavailable_error(), 12)
        .expect("stale failure should be ignored");

    assert_eq!(status.state, super::types::RecorderPreviewState::Idle);
    assert_eq!(status.error, None);
}

#[test]
fn save_scene_persists_through_app_storage() {
    let storage = test_storage();
    let scene_id = Uuid::now_v7().to_string();

    let scene = save_recorder_scene_for_storage(
        &storage,
        SaveRecorderSceneInput {
            scene: test_scene(&scene_id),
        },
        10,
    )
    .expect("save scene");

    assert_eq!(scene.id, scene_id);
    assert_eq!(
        scene.desktop_audio_source_id,
        Some("desktop-monitor".to_string()),
    );
    assert_eq!(scene.microphone_audio_source_id, Some("mic".to_string()));
    assert_eq!(scene.sources.len(), 1);
    assert_eq!(scene.sources[0].source_id, "monitor-1");
}

#[test]
fn get_recorder_scene_returns_null_when_no_scene_exists() {
    let app = tauri::test::mock_app();
    app.manage(test_storage());

    let scene =
        get_recorder_scene(app.state::<storage::AppStorage>()).expect("read latest recorder scene");

    assert_eq!(scene, None);
}

#[test]
fn get_recorder_scene_returns_latest_saved_scene() {
    let app = tauri::test::mock_app();
    let storage = test_storage();
    let older_scene_id = Uuid::now_v7().to_string();
    let newer_scene_id = Uuid::now_v7().to_string();

    save_recorder_scene_for_storage(
        &storage,
        SaveRecorderSceneInput {
            scene: test_scene(&older_scene_id),
        },
        10,
    )
    .expect("save older scene");
    save_recorder_scene_for_storage(
        &storage,
        SaveRecorderSceneInput {
            scene: test_scene(&newer_scene_id),
        },
        20,
    )
    .expect("save newer scene");
    app.manage(storage);

    let scene = get_recorder_scene(app.state::<storage::AppStorage>())
        .expect("read latest recorder scene")
        .expect("latest recorder scene");

    assert_eq!(scene.id, newer_scene_id);
}

#[test]
fn public_configuration_commands_reject_while_recording_is_active() {
    let app = tauri::test::mock_app();
    let app_data_dir = test_app_data_dir();
    let manager = RecorderManager::new(app_data_dir.clone());
    let storage = test_storage();
    let scene_id = saved_test_scene(&storage);
    start_test_recording(&manager, &storage, start_input(&scene_id), 100)
        .expect("start active recording");
    app.manage(storage);
    app.manage(manager);

    let save_error = save_recorder_scene(
        SaveRecorderSceneInput {
            scene: test_scene(&scene_id),
        },
        app.state::<storage::AppStorage>(),
        app.state::<RecorderManager>(),
    )
    .expect_err("save should reject while recording is active");
    let select_error = tauri::async_runtime::block_on(select_capture_sources(
        Some(super::types::SelectCaptureSourcesInput {
            force_chooser: true,
        }),
        app.state::<storage::AppStorage>(),
        app.state::<RecorderManager>(),
    ))
    .expect_err("source selection should reject while recording is active");

    assert_eq!(save_error.code, "recorder_configuration_locked");
    assert_eq!(select_error.code, "recorder_configuration_locked");
    let _ = fs::remove_dir_all(app_data_dir);
}

#[test]
fn manager_start_creates_recording_directory_metadata_and_segment() {
    let app_data_dir = test_app_data_dir();
    let manager = RecorderManager::new(app_data_dir.clone());
    let storage = test_storage();
    let scene_id = saved_test_scene(&storage);

    let status = start_test_recording(&manager, &storage, start_input(&scene_id), 100)
        .expect("start recording");
    let recording_id = status
        .active_recording_id
        .clone()
        .expect("active recording id");
    let segments = storage
        .with_connection(|connection| storage::list_recording_segments(connection, &recording_id))
        .expect("list segments");

    assert_eq!(status.state, RecorderState::Recording);
    assert_eq!(segments.len(), 1);
    assert_eq!(segments[0].segment_index, 1);
    assert!(segments[0].file_path.ends_with("segments/0001.mkv"));
    assert!(PathBuf::from(&segments[0].file_path)
        .parent()
        .expect("segment parent")
        .exists());
    assert!(!PathBuf::from(&segments[0].file_path).exists());

    let _ = fs::remove_dir_all(app_data_dir);
}

#[test]
fn manager_pause_resume_and_stop_persist_segments_in_order() {
    let app_data_dir = test_app_data_dir();
    let manager = RecorderManager::new(app_data_dir.clone());
    let storage = test_storage();
    let scene_id = saved_test_scene(&storage);

    let started = start_test_recording(&manager, &storage, start_input(&scene_id), 10)
        .expect("start recording");
    let recording_id = started
        .active_recording_id
        .clone()
        .expect("active recording id");

    let paused = pause_test_recording(&manager, &storage, &recording_id, 25).expect("pause");
    assert_eq!(paused.state, RecorderState::Paused);
    assert_eq!(paused.active_segment_id, None);

    let resumed = resume_test_recording(&manager, &storage, &recording_id, 30).expect("resume");
    assert_eq!(resumed.state, RecorderState::Recording);
    assert!(resumed.active_segment_id.is_some());

    let stopped = stop_test_recording(&manager, &storage, &recording_id, 42).expect("stop");
    assert_eq!(stopped.state, RecorderState::Idle);
    assert_eq!(stopped.active_recording_id, None);

    let recording = storage
        .with_connection(|connection| storage::get_recording(connection, &recording_id))
        .expect("read recording");
    let segments = storage
        .with_connection(|connection| storage::list_recording_segments(connection, &recording_id))
        .expect("list segments");

    assert_eq!(recording.status, "completed");
    assert_eq!(recording.started_at_ms, Some(10));
    assert_eq!(recording.ended_at_ms, Some(42));
    assert_eq!(segments.len(), 2);
    assert_eq!(segments[0].state, "finalized");
    assert_eq!(segments[0].duration_ms, Some(15));
    assert!(segments[0].file_path.ends_with("0001.mkv"));
    assert_eq!(segments[1].state, "finalized");
    assert_eq!(segments[1].duration_ms, Some(12));
    assert!(segments[1].file_path.ends_with("0002.mkv"));

    let _ = fs::remove_dir_all(app_data_dir);
}

#[test]
fn manager_stop_from_paused_does_not_create_new_segment() {
    let app_data_dir = test_app_data_dir();
    let manager = RecorderManager::new(app_data_dir.clone());
    let storage = test_storage();
    let scene_id = saved_test_scene(&storage);

    let started = start_test_recording(&manager, &storage, start_input(&scene_id), 10)
        .expect("start recording");
    let recording_id = started
        .active_recording_id
        .clone()
        .expect("active recording id");
    pause_test_recording(&manager, &storage, &recording_id, 20).expect("pause");
    stop_test_recording(&manager, &storage, &recording_id, 30).expect("stop");

    let segments = storage
        .with_connection(|connection| storage::list_recording_segments(connection, &recording_id))
        .expect("list segments");

    assert_eq!(segments.len(), 1);
    assert_eq!(segments[0].state, "finalized");

    let _ = fs::remove_dir_all(app_data_dir);
}

#[test]
fn manager_begin_and_complete_pause_exposes_async_pausing_state() {
    let app_data_dir = test_app_data_dir();
    let manager = RecorderManager::new(app_data_dir.clone());
    let storage = test_storage();
    let scene_id = saved_test_scene(&storage);

    let started = start_test_recording(&manager, &storage, start_input(&scene_id), 10)
        .expect("start recording");
    let recording_id = started
        .active_recording_id
        .clone()
        .expect("active recording id");

    let pausing = manager
        .begin_pause_recording(&recording_id, 20)
        .expect("begin pause");
    assert_eq!(pausing.state, RecorderState::Pausing);
    assert!(pausing.status_revision > started.status_revision);
    assert_eq!(
        pausing.active_recording_id.as_deref(),
        Some(recording_id.as_str())
    );
    assert_eq!(
        pausing.active_segment_id.as_deref(),
        started.active_segment_id.as_deref()
    );
    assert_eq!(
        pausing.finalization.as_ref().map(|status| &status.action),
        Some(&super::types::RecorderFinalizationAction::Pause)
    );
    assert_eq!(
        pausing
            .finalization
            .as_ref()
            .and_then(|status| status.segment_id.as_deref()),
        started.active_segment_id.as_deref()
    );

    let concurrent = manager
        .begin_stop_recording(&recording_id, 21)
        .expect_err("pending pause should reject concurrent stop");
    assert_eq!(concurrent.code, "recorder_already_active");

    let paused = manager
        .complete_pause_recording(&storage, &recording_id, 25)
        .expect("complete pause");
    assert_eq!(paused.state, RecorderState::Paused);
    assert_eq!(
        paused.active_recording_id.as_deref(),
        Some(recording_id.as_str())
    );
    assert_eq!(paused.active_segment_id, None);
    assert_eq!(paused.finalization, None);

    let segments = storage
        .with_connection(|connection| storage::list_recording_segments(connection, &recording_id))
        .expect("list segments");
    assert_eq!(segments[0].state, "finalized");
    assert_eq!(segments[0].duration_ms, Some(15));

    let _ = fs::remove_dir_all(app_data_dir);
}

#[test]
fn manager_begin_and_complete_stop_exposes_async_stopping_state() {
    let app_data_dir = test_app_data_dir();
    let manager = RecorderManager::new(app_data_dir.clone());
    let storage = test_storage();
    let scene_id = saved_test_scene(&storage);

    let started = start_test_recording(&manager, &storage, start_input(&scene_id), 10)
        .expect("start recording");
    let recording_id = started
        .active_recording_id
        .clone()
        .expect("active recording id");

    let stopping = manager
        .begin_stop_recording(&recording_id, 20)
        .expect("begin stop");
    assert_eq!(stopping.state, RecorderState::Stopping);
    assert!(stopping.status_revision > started.status_revision);
    assert_eq!(
        stopping.active_recording_id.as_deref(),
        Some(recording_id.as_str())
    );
    assert_eq!(
        stopping.active_segment_id.as_deref(),
        started.active_segment_id.as_deref()
    );
    assert_eq!(
        stopping.finalization.as_ref().map(|status| &status.action),
        Some(&super::types::RecorderFinalizationAction::Stop)
    );

    let concurrent = manager
        .begin_pause_recording(&recording_id, 21)
        .expect_err("pending stop should reject concurrent pause");
    assert_eq!(concurrent.code, "recorder_already_active");

    let stopped = manager
        .complete_stop_recording(&storage, &recording_id, 25)
        .expect("complete stop");
    assert_eq!(stopped.state, RecorderState::Idle);
    assert_eq!(stopped.active_recording_id, None);
    assert_eq!(stopped.finalization, None);

    let recording = storage
        .with_connection(|connection| storage::get_recording(connection, &recording_id))
        .expect("read recording");
    let segments = storage
        .with_connection(|connection| storage::list_recording_segments(connection, &recording_id))
        .expect("list segments");
    assert_eq!(recording.status, "completed");
    assert_eq!(segments[0].state, "finalized");
    assert_eq!(segments[0].duration_ms, Some(15));

    let _ = fs::remove_dir_all(app_data_dir);
}

#[test]
fn manager_stop_while_idle_preserves_backend_health() {
    let manager = RecorderManager::new(test_app_data_dir());
    let backend_health = available_test_backend_health();
    manager
        .status_with_backend_health(backend_health.clone(), 10)
        .expect("set backend health");

    let stopped = manager
        .begin_stop_recording(&Uuid::now_v7().to_string(), 20)
        .expect("idempotent stop while idle");

    assert_eq!(stopped.state, RecorderState::Idle);
    assert_eq!(stopped.backend_health, backend_health);
}

#[test]
fn manager_stop_after_recording_preserves_backend_health() {
    let app_data_dir = test_app_data_dir();
    let manager = RecorderManager::new(app_data_dir.clone());
    let storage = test_storage();
    let scene_id = saved_test_scene(&storage);
    let backend_health = available_test_backend_health();
    manager
        .status_with_backend_health(backend_health.clone(), 10)
        .expect("set backend health");
    let started = start_test_recording(&manager, &storage, start_input(&scene_id), 20)
        .expect("start recording");
    let recording_id = started
        .active_recording_id
        .clone()
        .expect("active recording id");

    let stopped =
        stop_test_recording(&manager, &storage, &recording_id, 30).expect("stop active recording");

    assert_eq!(stopped.state, RecorderState::Idle);
    assert_eq!(stopped.backend_health, backend_health);

    let _ = fs::remove_dir_all(app_data_dir);
}

#[test]
fn manager_rejects_invalid_transitions() {
    let app_data_dir = test_app_data_dir();
    let manager = RecorderManager::new(app_data_dir.clone());
    let storage = test_storage();
    let scene_id = saved_test_scene(&storage);

    let pause_error = manager
        .begin_pause_recording(&Uuid::now_v7().to_string(), 1)
        .expect_err("pause while idle should fail");
    let resume_error = resume_test_recording(&manager, &storage, &Uuid::now_v7().to_string(), 1)
        .expect_err("resume while idle should fail");

    start_test_recording(&manager, &storage, start_input(&scene_id), 10).expect("start recording");
    let start_error = start_test_recording(&manager, &storage, start_input(&scene_id), 11)
        .expect_err("start while active should fail");

    assert_eq!(pause_error.code, "recorder_not_recording");
    assert_eq!(resume_error.code, "recorder_not_paused");
    assert_eq!(start_error.code, "recorder_already_active");

    let _ = fs::remove_dir_all(app_data_dir);
}

#[test]
fn manager_serializes_concurrent_starts() {
    let app_data_dir = test_app_data_dir();
    let manager = Arc::new(RecorderManager::new(app_data_dir.clone()));
    let storage = Arc::new(test_storage());
    let scene_id = saved_test_scene(&storage);
    let worker_count = 8;
    let barrier = Arc::new(Barrier::new(worker_count));

    let handles = (0..worker_count)
        .map(|_| {
            let barrier = Arc::clone(&barrier);
            let manager = Arc::clone(&manager);
            let scene_id = scene_id.clone();
            let storage = Arc::clone(&storage);

            thread::spawn(move || {
                barrier.wait();
                start_test_recording(&manager, &storage, start_input(&scene_id), 100)
            })
        })
        .collect::<Vec<_>>();
    let results = handles
        .into_iter()
        .map(|handle| handle.join().expect("start worker should not panic"))
        .collect::<Vec<_>>();
    let successes = results
        .iter()
        .filter_map(|result| result.as_ref().ok())
        .collect::<Vec<_>>();
    let errors = results
        .iter()
        .filter_map(|result| result.as_ref().err())
        .collect::<Vec<_>>();

    assert_eq!(successes.len(), 1);
    assert_eq!(errors.len(), worker_count - 1);
    assert!(errors
        .iter()
        .all(|error| error.code == "recorder_already_active"));

    let status = manager.status().expect("read manager status");
    let active_recording_id = status
        .active_recording_id
        .clone()
        .expect("manager should have active recording");
    let recording_count: i64 = storage
        .with_connection(|connection| {
            connection.query_row("SELECT COUNT(*) FROM recordings", [], |row| row.get(0))
        })
        .expect("count recordings");
    let recording_segment_count: i64 = storage
        .with_connection(|connection| {
            connection.query_row("SELECT COUNT(*) FROM recording_segments", [], |row| {
                row.get(0)
            })
        })
        .expect("count recording segments");

    assert_eq!(
        active_recording_id,
        successes[0]
            .active_recording_id
            .clone()
            .expect("successful start should return active recording")
    );
    assert_eq!(recording_count, 1);
    assert_eq!(recording_segment_count, 1);

    let _ = fs::remove_dir_all(app_data_dir);
}

#[test]
fn manager_rejects_missing_scene_before_creating_recording() {
    let app_data_dir = test_app_data_dir();
    let manager = RecorderManager::new(app_data_dir.clone());
    let storage = test_storage();
    let scene_id = Uuid::now_v7().to_string();

    let error = start_test_recording(&manager, &storage, start_input(&scene_id), 10)
        .expect_err("missing scene should fail");
    let recording_count: i64 = storage
        .with_connection(|connection| {
            connection.query_row("SELECT COUNT(*) FROM recordings", [], |row| row.get(0))
        })
        .expect("count recordings");

    assert_eq!(error.code, "recorder_scene_not_found");
    assert_eq!(recording_count, 0);
    assert!(!app_data_dir.join("recordings").exists());
}

#[test]
fn manager_selecting_state_rejects_concurrent_starts() {
    let manager = RecorderManager::new(test_app_data_dir());

    let selecting = manager
        .begin_selecting("scene-1", 10)
        .expect("begin selecting");
    let error = manager
        .begin_selecting("scene-2", 11)
        .expect_err("second selection should fail");

    assert_eq!(selecting.state, RecorderState::Selecting);
    assert_eq!(selecting.active_scene_id.as_deref(), Some("scene-1"));
    assert_eq!(error.code, "recorder_already_active");
}

#[test]
fn pending_live_start_preparation_does_not_create_recording_rows() {
    let app_data_dir = test_app_data_dir();
    let manager = RecorderManager::new(app_data_dir.clone());
    let storage = test_storage();
    let scene_id = saved_test_scene(&storage);

    manager
        .begin_selecting(&scene_id, 10)
        .expect("begin selecting");
    let pending = manager
        .prepare_recording_start(&storage, start_input(&scene_id), 10)
        .expect("prepare pending recording");
    let recording_count = storage
        .with_connection(|connection| {
            connection.query_row("SELECT COUNT(*) FROM recordings", [], |row| {
                row.get::<_, i64>(0)
            })
        })
        .expect("count recordings");

    assert_eq!(pending.scene.id, scene_id);
    assert_eq!(recording_count, 0);

    let _ = fs::remove_dir_all(app_data_dir);
}

#[test]
fn pending_live_start_uses_saved_record_settings() {
    let app_data_dir = test_app_data_dir();
    let manager = RecorderManager::new(app_data_dir.clone());
    let storage = test_storage();
    let scene_id = saved_test_scene(&storage);
    storage
        .with_connection_mut(|connection| {
            storage::save_app_settings(
                connection,
                &[
                    storage::SaveAppSettingInput {
                        key: "record.fps".to_string(),
                        value_json: "0.5".to_string(),
                        is_sensitive: false,
                        timestamp_ms: 5,
                    },
                    storage::SaveAppSettingInput {
                        key: "record.videoBitrateKbps".to_string(),
                        value_json: "2500".to_string(),
                        is_sensitive: false,
                        timestamp_ms: 5,
                    },
                    storage::SaveAppSettingInput {
                        key: "storage.retentionDays".to_string(),
                        value_json: "14".to_string(),
                        is_sensitive: false,
                        timestamp_ms: 5,
                    },
                    storage::SaveAppSettingInput {
                        key: "storage.sizeLimitBytes".to_string(),
                        value_json: (75_u64 * 1024 * 1024 * 1024).to_string(),
                        is_sensitive: false,
                        timestamp_ms: 5,
                    },
                ],
            )
        })
        .expect("save app settings");

    manager
        .begin_selecting(&scene_id, 10)
        .expect("begin selecting");
    let pending = manager
        .prepare_recording_start(&storage, start_input(&scene_id), 10)
        .expect("prepare pending recording");

    assert_eq!(pending.segment_input.fps, 0.5);
    assert_eq!(pending.segment_input.video_bitrate_kbps, 2_500);

    let _ = fs::remove_dir_all(app_data_dir);
}

#[test]
fn recording_start_pads_odd_canvas_dimensions_without_mutating_scene() {
    let app_data_dir = test_app_data_dir();
    let manager = RecorderManager::new(app_data_dir.clone());
    let storage = test_storage();
    let scene_id = Uuid::now_v7().to_string();
    let mut scene = test_scene(&scene_id);
    scene.canvas_width = 2559;
    scene.canvas_height = 1439;
    save_recorder_scene_for_storage(&storage, SaveRecorderSceneInput { scene }, 1)
        .expect("save odd scene");

    manager
        .begin_selecting(&scene_id, 10)
        .expect("begin selecting");
    let pending = manager
        .prepare_recording_start(&storage, start_input(&scene_id), 10)
        .expect("prepare pending recording");
    let stored_scene = storage
        .with_connection(|connection| storage::get_recorder_scene(connection, &scene_id))
        .expect("read stored scene");

    assert_eq!(stored_scene.canvas_width, 2559);
    assert_eq!(stored_scene.canvas_height, 1439);
    assert_eq!(pending.segment_input.canvas_width, 2560);
    assert_eq!(pending.segment_input.canvas_height, 1440);

    let _ = fs::remove_dir_all(app_data_dir);
}

#[test]
fn manager_caches_selected_capture_sources() {
    let manager = RecorderManager::new(test_app_data_dir());

    let selected = manager
        .set_capture_sources(vec![CaptureSource {
            display_name: "Monitor 1".to_string(),
            height: Some(1440),
            id: "monitor-1".to_string(),
            kind: CaptureSourceKind::Monitor,
            logical_height: None,
            logical_width: None,
            scale_factor: None,
            width: Some(2560),
        }])
        .expect("set capture sources");
    let cached = manager.capture_sources().expect("read capture sources");

    assert_eq!(selected, cached);
    assert_eq!(cached.sources[0].id, "monitor-1");
}

#[test]
fn public_lifecycle_commands_use_media_writer_manager_contracts() {
    let app = tauri::test::mock_app();
    let app_data_dir = test_app_data_dir();
    let storage = test_storage();
    app.manage(storage);
    app.manage(RecorderManager::new(app_data_dir.clone()));

    let start_error = tauri::async_runtime::block_on(start_recording(
        start_input("missing-scene"),
        app.handle().clone(),
        app.state::<storage::AppStorage>(),
        app.state::<RecorderManager>(),
    ))
    .expect_err("public start should require an existing scene");
    let pause_error = pause_recording(
        PauseRecordingInput {
            recording_id: Uuid::now_v7().to_string(),
        },
        app.handle().clone(),
        app.state::<RecorderManager>(),
    )
    .expect_err("public pause should remain unavailable");
    let resume_error = tauri::async_runtime::block_on(resume_recording(
        ResumeRecordingInput {
            recording_id: Uuid::now_v7().to_string(),
        },
        app.handle().clone(),
        app.state::<storage::AppStorage>(),
        app.state::<RecorderManager>(),
    ))
    .expect_err("public resume should remain unavailable");
    let stop_status = stop_recording(
        StopRecordingInput {
            recording_id: Uuid::now_v7().to_string(),
        },
        app.handle().clone(),
        app.state::<RecorderManager>(),
    )
    .expect("public stop while idle is idempotent");

    assert_eq!(start_error.code, "recorder_scene_not_found");
    assert_eq!(pause_error.code, "recorder_not_recording");
    assert_eq!(resume_error.code, "recorder_not_paused");
    assert_eq!(stop_status.state, RecorderState::Idle);
    let _ = fs::remove_dir_all(app_data_dir);
}

#[test]
fn empty_scene_preview_start_resets_pending_preview_before_platform_start() {
    let manager = RecorderManager::new(test_app_data_dir());
    let scene_id = Uuid::now_v7().to_string();
    let PreviewStartDecision::Pending { generation, .. } = manager
        .begin_preview_start(&scene_id, 10)
        .expect("begin preview")
    else {
        panic!("preview should be pending");
    };

    let preview_status = stop_empty_preview_start(
        &manager,
        &empty_test_scene(&scene_id),
        &scene_id,
        generation,
    )
    .expect("empty scene should stop pending preview")
    .expect("empty scene should return idle status");

    assert_eq!(
        preview_status.state,
        super::types::RecorderPreviewState::Idle
    );
    assert_eq!(preview_status.scene_id, None);
    assert!(!manager.preview_start_is_current(&scene_id, generation));
}

#[test]
fn public_commands_record_pause_resume_stop_with_fake_capture_and_real_writer() {
    assert_fake_capture_public_lifecycle(false, Duration::from_millis(250));
}

#[test]
fn public_commands_record_pause_resume_stop_with_independent_fake_capture_and_real_writer() {
    assert_fake_capture_public_lifecycle(true, Duration::from_millis(250));
}

#[test]
fn public_commands_record_pause_resume_stop_with_sparse_independent_fake_capture_and_real_writer() {
    assert_fake_capture_public_lifecycle(true, Duration::from_millis(750));
}

fn assert_fake_capture_public_lifecycle(independent_video: bool, capture_duration: Duration) {
    let app = tauri::test::mock_app();
    let app_data_dir = test_app_data_dir();
    let storage = test_storage();
    let scene_id = Uuid::now_v7().to_string();
    let mut scene = fake_test_scene(&scene_id);
    scene.independent_video = independent_video;
    if independent_video {
        scene.sources[0].rect.width = 160.0;
        scene.sources[0].rect.height = 90.0;
        let mut second = scene.sources[0].clone();
        second.id = Uuid::now_v7().to_string();
        second.rect.width = 128.0;
        second.rect.height = 72.0;
        scene.sources.push(second);
    }
    save_recorder_scene_for_storage(&storage, SaveRecorderSceneInput { scene }, 10)
        .expect("save fake scene");
    app.manage(storage);
    app.manage(RecorderManager::new(app_data_dir.clone()));

    let started = tauri::async_runtime::block_on(start_recording(
        start_input(&scene_id),
        app.handle().clone(),
        app.state::<storage::AppStorage>(),
        app.state::<RecorderManager>(),
    ))
    .expect("start fake recording");
    let recording_id = started
        .active_recording_id
        .clone()
        .expect("active recording id");
    thread::sleep(capture_duration);

    let pausing = pause_recording(
        PauseRecordingInput {
            recording_id: recording_id.clone(),
        },
        app.handle().clone(),
        app.state::<RecorderManager>(),
    )
    .expect("pause fake recording");
    assert!(pausing.finalization.is_some());
    wait_for_recorder_state(app.state::<RecorderManager>(), RecorderState::Paused);

    let first_segments = app
        .state::<storage::AppStorage>()
        .with_connection(|connection| storage::list_recording_segments(connection, &recording_id))
        .expect("list paused segments");
    assert_eq!(first_segments.len(), 1);
    assert_finalized_probeable_segment(&first_segments[0]);

    let resumed = tauri::async_runtime::block_on(resume_recording(
        ResumeRecordingInput {
            recording_id: recording_id.clone(),
        },
        app.handle().clone(),
        app.state::<storage::AppStorage>(),
        app.state::<RecorderManager>(),
    ))
    .expect("resume fake recording");
    assert_eq!(resumed.state, RecorderState::Recording);
    thread::sleep(capture_duration);

    let stopping = stop_recording(
        StopRecordingInput {
            recording_id: recording_id.clone(),
        },
        app.handle().clone(),
        app.state::<RecorderManager>(),
    )
    .expect("stop fake recording");
    assert!(stopping.finalization.is_some());
    wait_for_recorder_state(app.state::<RecorderManager>(), RecorderState::Idle);

    let recording = app
        .state::<storage::AppStorage>()
        .with_connection(|connection| storage::get_recording(connection, &recording_id))
        .expect("read stopped recording");
    let segments = app
        .state::<storage::AppStorage>()
        .with_connection(|connection| storage::list_recording_segments(connection, &recording_id))
        .expect("list stopped segments");

    assert_eq!(recording.status, "completed");
    let stopped_at_ms = recording.ended_at_ms.expect("recording stop timestamp");
    assert_eq!(segments.len(), 2);
    for segment in &segments {
        assert!(
            segment.started_at_ms <= stopped_at_ms,
            "segment cannot start after recording stopped"
        );
    }
    assert_finalized_probeable_segment(&segments[0]);
    assert_finalized_probeable_segment(&segments[1]);
    if independent_video {
        for segment in &segments {
            let probe: serde_json::Value =
                serde_json::from_str(segment.probe_json.as_deref().expect("segment probe"))
                    .expect("parse independent video probe");
            let mut dimensions: Vec<_> = probe["videos"]
                .as_array()
                .expect("independent video streams")
                .iter()
                .map(|video| (video["width"].as_u64(), video["height"].as_u64()))
                .collect();
            dimensions.sort();
            assert_eq!(
                dimensions,
                vec![(Some(128), Some(72)), (Some(160), Some(90))]
            );
        }
    }

    let _ = fs::remove_dir_all(app_data_dir);
}

#[derive(Debug)]
struct TestPreviewSession {
    stopped: Arc<AtomicUsize>,
}

impl CapturePreviewSession for TestPreviewSession {
    fn stop(self: Box<Self>) -> Result<(), super::types::RecorderError> {
        self.stopped.fetch_add(1, Ordering::SeqCst);

        Ok(())
    }
}

fn test_storage() -> storage::AppStorage {
    storage::test_storage(PathBuf::from("/tmp/daydream-test.sqlite3"))
}

fn test_app_data_dir() -> PathBuf {
    std::env::temp_dir().join(format!("daydream-recorder-{}", Uuid::now_v7()))
}

fn available_test_backend_health() -> super::types::RecorderBackendHealth {
    let mut backend_health = super::manager::unavailable_backend_health();
    backend_health.media_writer.available = true;
    backend_health.media_writer.error = None;
    backend_health.pipewire.available = true;
    backend_health.pipewire.error = None;
    backend_health.portal.available = true;
    backend_health.portal.error = None;

    backend_health
}

fn saved_test_scene(storage: &storage::AppStorage) -> String {
    let scene_id = Uuid::now_v7().to_string();
    save_recorder_scene_for_storage(
        storage,
        SaveRecorderSceneInput {
            scene: test_scene(&scene_id),
        },
        1,
    )
    .expect("save test scene");

    scene_id
}

fn start_input(scene_id: &str) -> StartRecordingInput {
    StartRecordingInput {
        setup: None,
        desktop_audio_source_id: None,
        microphone_audio_source_id: None,
        quality_preset: RecorderQualityPreset::BalancedOcr,
        scene_id: scene_id.to_string(),
    }
}

fn start_test_recording(
    manager: &RecorderManager,
    storage: &storage::AppStorage,
    input: StartRecordingInput,
    timestamp_ms: u64,
) -> Result<RecorderStatus, super::types::RecorderError> {
    manager.begin_selecting(&input.scene_id, timestamp_ms)?;
    let pending = match manager.prepare_recording_start(storage, input, timestamp_ms) {
        Ok(pending) => pending,
        Err(error) => {
            let _ = manager.clear_selecting_to_idle(timestamp_ms);
            return Err(error);
        }
    };
    if let Err(error) = manager.create_pending_recording_rows(storage, &pending) {
        let _ = manager.clear_selecting_to_idle(timestamp_ms);
        return Err(error);
    }

    manager.activate_pending_recording_without_runtime_for_test(pending, timestamp_ms)
}

fn pause_test_recording(
    manager: &RecorderManager,
    storage: &storage::AppStorage,
    recording_id: &str,
    timestamp_ms: u64,
) -> Result<RecorderStatus, super::types::RecorderError> {
    let status = manager.begin_pause_recording(recording_id, timestamp_ms)?;
    if status.finalization.is_none() {
        return Ok(status);
    }

    manager.complete_pause_recording(storage, recording_id, timestamp_ms)
}

fn resume_test_recording(
    manager: &RecorderManager,
    storage: &storage::AppStorage,
    recording_id: &str,
    timestamp_ms: u64,
) -> Result<RecorderStatus, super::types::RecorderError> {
    let active = manager.begin_resume_selecting(recording_id, timestamp_ms)?;
    let pending = match manager.prepare_resume_segment_start(storage, active.clone(), timestamp_ms)
    {
        Ok(pending) => pending,
        Err(error) => {
            let _ = manager.restore_paused_after_selecting(storage, active, timestamp_ms);
            return Err(error);
        }
    };
    if let Err(error) = manager.create_pending_segment_rows(storage, &pending) {
        let _ =
            manager.restore_paused_after_selecting(storage, pending.active.clone(), timestamp_ms);
        return Err(error);
    }

    manager.activate_pending_segment_without_runtime_for_test(pending, timestamp_ms)
}

fn stop_test_recording(
    manager: &RecorderManager,
    storage: &storage::AppStorage,
    recording_id: &str,
    timestamp_ms: u64,
) -> Result<RecorderStatus, super::types::RecorderError> {
    let status = manager.begin_stop_recording(recording_id, timestamp_ms)?;
    if status.finalization.is_none() {
        return Ok(status);
    }

    manager.complete_stop_recording(storage, recording_id, timestamp_ms)
}

fn test_scene(scene_id: &str) -> RecorderScene {
    RecorderScene {
        independent_video: false,
        canvas_height: 1440,
        canvas_width: 2560,
        desktop_audio_source_id: Some("desktop-monitor".to_string()),
        id: scene_id.to_string(),
        layout_mode: RecorderSceneLayoutMode::Horizontal,
        microphone_audio_source_id: Some("mic".to_string()),
        name: "Test scene".to_string(),
        sources: vec![RecorderSceneSource {
            crop: CropRect {
                bottom: 0.0,
                left: 0.0,
                right: 0.0,
                top: 0.0,
            },
            display_name: "Display 1".to_string(),
            id: Uuid::now_v7().to_string(),
            preserve_aspect_ratio: true,
            rect: CanvasRect {
                height: 1440.0,
                width: 2560.0,
                x: 0.0,
                y: 0.0,
            },
            source_id: "monitor-1".to_string(),
            source_height: Some(1440.0),
            source_kind: CaptureSourceKind::Monitor,
            source_width: Some(2560.0),
            z_index: 0,
        }],
    }
}

fn empty_test_scene(scene_id: &str) -> RecorderScene {
    RecorderScene {
        independent_video: false,
        canvas_height: 1440,
        canvas_width: 2560,
        desktop_audio_source_id: None,
        id: scene_id.to_string(),
        layout_mode: RecorderSceneLayoutMode::Horizontal,
        microphone_audio_source_id: None,
        name: "Empty scene".to_string(),
        sources: Vec::new(),
    }
}

fn fake_test_scene(scene_id: &str) -> RecorderScene {
    RecorderScene {
        independent_video: false,
        canvas_height: 180,
        canvas_width: 320,
        desktop_audio_source_id: None,
        id: scene_id.to_string(),
        layout_mode: RecorderSceneLayoutMode::Horizontal,
        microphone_audio_source_id: None,
        name: "Fake recorder scene".to_string(),
        sources: vec![RecorderSceneSource {
            crop: CropRect {
                bottom: 0.0,
                left: 0.0,
                right: 0.0,
                top: 0.0,
            },
            display_name: "Fake monitor".to_string(),
            id: Uuid::now_v7().to_string(),
            preserve_aspect_ratio: true,
            rect: CanvasRect {
                height: 180.0,
                width: 320.0,
                x: 0.0,
                y: 0.0,
            },
            source_id: "fake-monitor-1".to_string(),
            source_height: Some(180.0),
            source_kind: CaptureSourceKind::Monitor,
            source_width: Some(320.0),
            z_index: 0,
        }],
    }
}

fn wait_for_recorder_state(manager: tauri::State<'_, RecorderManager>, expected: RecorderState) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let status = manager.status().expect("read recorder status");
        if status.state == expected && status.finalization.is_none() {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for recorder state {expected:?}; latest status: {status:?}"
        );
        thread::sleep(Duration::from_millis(25));
    }
}

fn assert_finalized_probeable_segment(segment: &storage::RecordingSegmentRow) {
    assert_eq!(segment.state, "finalized");
    assert_eq!(segment.error_code, None);
    assert!(segment.duration_ms.unwrap_or_default() > 0);
    let metadata = fs::metadata(&segment.file_path).expect("read media segment metadata");
    assert!(metadata.len() > 0, "segment should not be empty");
    let probe: serde_json::Value = serde_json::from_str(
        segment
            .probe_json
            .as_deref()
            .expect("finalized segment should have probe metadata"),
    )
    .expect("parse segment probe json");
    assert_eq!(
        probe.get("available").and_then(|value| value.as_bool()),
        Some(true)
    );
    assert!(
        probe.get("video").is_some_and(|value| value.is_object()),
        "probe should include video metadata: {probe}"
    );
}
