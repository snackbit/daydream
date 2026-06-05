use std::{
    env,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Barrier, Mutex},
    thread,
};

use serde_json::json;
use tauri::Manager as _;
use uuid::Uuid;

use super::commands::{
    get_recorder_scene, pause_recording, resume_recording, start_recording, stop_recording,
};
use super::errors::unavailable_error;
use super::manager::RecorderManager;
use super::storage_mapping::save_recorder_scene_for_storage;
use super::types::{
    CanvasRect, CaptureSource, CaptureSourceKind, CropRect, PauseRecordingInput,
    RecorderErrorCategory, RecorderQualityPreset, RecorderScene, RecorderSceneLayoutMode,
    RecorderSceneSource, RecorderState, ResumeRecordingInput, SaveRecorderSceneInput,
    StartRecordingInput, StopRecordingInput,
};
use crate::storage;

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
    assert!(!status.backend_health.ffmpeg.available);
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
fn manager_start_creates_recording_directory_metadata_and_segment() {
    let app_data_dir = test_app_data_dir();
    let manager = RecorderManager::new(app_data_dir.clone());
    let storage = test_storage();
    let scene_id = saved_test_scene(&storage);

    let status = manager
        .start_internal(&storage, start_input(&scene_id), 100)
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

    let started = manager
        .start_internal(&storage, start_input(&scene_id), 10)
        .expect("start recording");
    let recording_id = started
        .active_recording_id
        .clone()
        .expect("active recording id");

    let paused = manager.pause_internal(&storage, 25).expect("pause");
    assert_eq!(paused.state, RecorderState::Paused);
    assert_eq!(paused.active_segment_id, None);

    let resumed = manager.resume_internal(&storage, 30).expect("resume");
    assert_eq!(resumed.state, RecorderState::Recording);
    assert!(resumed.active_segment_id.is_some());

    let stopped = manager.stop_internal(&storage, 42).expect("stop");
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

    let started = manager
        .start_internal(&storage, start_input(&scene_id), 10)
        .expect("start recording");
    let recording_id = started
        .active_recording_id
        .clone()
        .expect("active recording id");
    manager.pause_internal(&storage, 20).expect("pause");
    manager.stop_internal(&storage, 30).expect("stop");

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

    let started = manager
        .start_internal(&storage, start_input(&scene_id), 10)
        .expect("start recording");
    let recording_id = started
        .active_recording_id
        .clone()
        .expect("active recording id");

    let pausing = manager
        .begin_pause_recording(&recording_id, 20)
        .expect("begin pause");
    assert_eq!(pausing.state, RecorderState::Paused);
    assert_eq!(
        pausing.active_recording_id.as_deref(),
        Some(recording_id.as_str())
    );
    assert_eq!(pausing.active_segment_id, None);
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

    let started = manager
        .start_internal(&storage, start_input(&scene_id), 10)
        .expect("start recording");
    let recording_id = started
        .active_recording_id
        .clone()
        .expect("active recording id");

    let stopping = manager
        .begin_stop_recording(&recording_id, 20)
        .expect("begin stop");
    assert_eq!(stopping.state, RecorderState::Idle);
    assert_eq!(stopping.active_recording_id, None);
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
    let storage = test_storage();
    let backend_health = available_test_backend_health();
    manager
        .status_with_backend_health(backend_health.clone(), 10)
        .expect("set backend health");

    let stopped = manager
        .stop_recording(&storage, &Uuid::now_v7().to_string(), 20)
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
    manager
        .start_internal(&storage, start_input(&scene_id), 20)
        .expect("start recording");

    let stopped = manager
        .stop_internal(&storage, 30)
        .expect("stop active recording");

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
        .pause_internal(&storage, 1)
        .expect_err("pause while idle should fail");
    let resume_error = manager
        .resume_internal(&storage, 1)
        .expect_err("resume while idle should fail");

    manager
        .start_internal(&storage, start_input(&scene_id), 10)
        .expect("start recording");
    let start_error = manager
        .start_internal(&storage, start_input(&scene_id), 11)
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
                manager.start_internal(&storage, start_input(&scene_id), 100)
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

    let error = manager
        .start_internal(&storage, start_input(&scene_id), 10)
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
    let fake_ffmpeg_path = fake_failing_ffmpeg_path();
    let fake_ffmpeg_dir = fake_ffmpeg_path
        .parent()
        .expect("fake ffmpeg parent")
        .to_path_buf();
    let manager = RecorderManager::new(app_data_dir.clone());
    let storage = test_storage();
    let scene_id = saved_test_scene(&storage);

    with_ffmpeg_path(&fake_ffmpeg_path, || {
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
    });

    let _ = fs::remove_dir_all(app_data_dir);
    let _ = fs::remove_dir_all(fake_ffmpeg_dir);
}

#[test]
fn ffmpeg_failure_clears_runtime_and_preserves_failed_segment() {
    let app_data_dir = test_app_data_dir();
    let fake_ffmpeg_path = fake_failing_ffmpeg_path();
    let fake_ffmpeg_dir = fake_ffmpeg_path
        .parent()
        .expect("fake ffmpeg parent")
        .to_path_buf();
    let manager = RecorderManager::new(app_data_dir.clone());
    let storage = test_storage();
    let scene_id = saved_test_scene(&storage);

    with_ffmpeg_path(&fake_ffmpeg_path, || {
        let started = manager
            .start_recording(&storage, start_input(&scene_id), 10)
            .expect("start recording with fake ffmpeg");
        let recording_id = started
            .active_recording_id
            .clone()
            .expect("active recording id");

        let pause_error = manager
            .pause_recording(&storage, &recording_id, 20)
            .expect_err("fake ffmpeg should fail during pause");
        assert!(
            pause_error.code == "recorder_ffmpeg_failed"
                || pause_error.code == "recorder_ffmpeg_frame_write_failed"
        );

        let failed_status = manager.status().expect("read failed manager status");
        assert_eq!(failed_status.state, RecorderState::Error);
        assert_eq!(failed_status.active_recording_id, None);
        assert_eq!(failed_status.active_segment_id, None);
        assert_eq!(
            failed_status
                .last_error
                .as_ref()
                .map(|error| error.code.as_str()),
            Some(pause_error.code.as_str()),
        );

        let recording = storage
            .with_connection(|connection| storage::get_recording(connection, &recording_id))
            .expect("read failed recording");
        let segments = storage
            .with_connection(|connection| {
                storage::list_recording_segments(connection, &recording_id)
            })
            .expect("list failed segments");
        assert_eq!(recording.status, "failed");
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].state, "failed");
        let failed_error_code = segments[0].error_code.clone();
        let failed_ended_at_ms = segments[0].ended_at_ms;

        let stopped = manager
            .stop_recording(&storage, &recording_id, 30)
            .expect("stop after terminal failure should be idempotent");
        assert_eq!(stopped.state, RecorderState::Idle);

        let recording_after_stop = storage
            .with_connection(|connection| storage::get_recording(connection, &recording_id))
            .expect("read recording after stop");
        let segments_after_stop = storage
            .with_connection(|connection| {
                storage::list_recording_segments(connection, &recording_id)
            })
            .expect("list segments after stop");
        assert_eq!(recording_after_stop.status, "failed");
        assert_eq!(segments_after_stop[0].state, "failed");
        assert_eq!(segments_after_stop[0].error_code, failed_error_code);
        assert_eq!(segments_after_stop[0].ended_at_ms, failed_ended_at_ms);
    });

    let _ = fs::remove_dir_all(app_data_dir);
    let _ = fs::remove_dir_all(fake_ffmpeg_dir);
}

#[test]
fn resume_ffmpeg_start_failure_clears_runtime_and_preserves_failed_segment() {
    let app_data_dir = test_app_data_dir();
    let fake_ffmpeg_path = fake_probe_then_missing_ffmpeg_path();
    let fake_ffmpeg_dir = fake_ffmpeg_path
        .parent()
        .expect("fake ffmpeg parent")
        .to_path_buf();
    let manager = RecorderManager::new(app_data_dir.clone());
    let storage = test_storage();
    let scene_id = saved_test_scene(&storage);

    let started = manager
        .start_internal(&storage, start_input(&scene_id), 10)
        .expect("start recording");
    let recording_id = started
        .active_recording_id
        .clone()
        .expect("active recording id");
    manager
        .pause_internal(&storage, 20)
        .expect("pause recording");

    with_ffmpeg_path(&fake_ffmpeg_path, || {
        let resume_error = manager
            .resume_recording(&storage, &recording_id, 30)
            .expect_err("resume should fail when FFmpeg cannot start");
        assert_eq!(resume_error.code, "recorder_ffmpeg_spawn_failed");

        let failed_status = manager.status().expect("read failed manager status");
        assert_eq!(failed_status.state, RecorderState::Error);
        assert_eq!(failed_status.active_recording_id, None);
        assert_eq!(failed_status.active_segment_id, None);
        assert_eq!(
            failed_status
                .last_error
                .as_ref()
                .map(|error| error.code.as_str()),
            Some("recorder_ffmpeg_spawn_failed"),
        );

        let recording = storage
            .with_connection(|connection| storage::get_recording(connection, &recording_id))
            .expect("read failed recording");
        let segments = storage
            .with_connection(|connection| {
                storage::list_recording_segments(connection, &recording_id)
            })
            .expect("list failed segments");
        assert_eq!(recording.status, "failed");
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].state, "finalized");
        assert_eq!(segments[1].state, "failed");
        assert_eq!(
            segments[1].error_code.as_deref(),
            Some("recorder_ffmpeg_spawn_failed"),
        );
        let failed_ended_at_ms = segments[1].ended_at_ms;

        let stopped = manager
            .stop_recording(&storage, &recording_id, 40)
            .expect("stop after terminal failure should be idempotent");
        assert_eq!(stopped.state, RecorderState::Idle);

        let recording_after_stop = storage
            .with_connection(|connection| storage::get_recording(connection, &recording_id))
            .expect("read recording after stop");
        let segments_after_stop = storage
            .with_connection(|connection| {
                storage::list_recording_segments(connection, &recording_id)
            })
            .expect("list segments after stop");
        assert_eq!(recording_after_stop.status, "failed");
        assert_eq!(segments_after_stop[1].state, "failed");
        assert_eq!(segments_after_stop[1].ended_at_ms, failed_ended_at_ms);
    });

    let _ = fs::remove_dir_all(app_data_dir);
    let _ = fs::remove_dir_all(fake_ffmpeg_dir);
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
fn public_lifecycle_commands_use_ffmpeg_backed_manager_contracts() {
    let app = tauri::test::mock_app();
    let app_data_dir = test_app_data_dir();
    let missing_ffmpeg_path = app_data_dir.join("missing-ffmpeg");
    let storage = test_storage();
    let scene_id = saved_test_scene(&storage);
    app.manage(storage);
    app.manage(RecorderManager::new(app_data_dir.clone()));

    with_ffmpeg_path(&missing_ffmpeg_path, || {
        let start_error = tauri::async_runtime::block_on(start_recording(
            start_input(&scene_id),
            app.state::<storage::AppStorage>(),
            app.state::<RecorderManager>(),
        ))
        .expect_err("public start should require available FFmpeg");
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

        assert_eq!(start_error.code, "recorder_ffmpeg_unavailable");
        assert_eq!(pause_error.code, "recorder_not_recording");
        assert_eq!(resume_error.code, "recorder_not_paused");
        assert_eq!(stop_status.state, RecorderState::Idle);
    });
    let _ = fs::remove_dir_all(app_data_dir);
}

static FFMPEG_ENV_LOCK: Mutex<()> = Mutex::new(());

struct EnvVarGuard {
    key: &'static str,
    previous: Option<OsString>,
}

impl EnvVarGuard {
    fn set_path(key: &'static str, path: &Path) -> Self {
        let previous = env::var_os(key);
        env::set_var(key, path);

        Self { key, previous }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        if let Some(previous) = &self.previous {
            env::set_var(self.key, previous);
        } else {
            env::remove_var(self.key);
        }
    }
}

fn with_ffmpeg_path<T>(path: &Path, run: impl FnOnce() -> T) -> T {
    let _lock = FFMPEG_ENV_LOCK.lock().expect("ffmpeg env lock");
    let _guard = EnvVarGuard::set_path("DAYDREAM_FFMPEG_PATH", path);

    run()
}

fn test_storage() -> storage::AppStorage {
    storage::test_storage(PathBuf::from("/tmp/daydream-test.sqlite3"))
}

fn test_app_data_dir() -> PathBuf {
    std::env::temp_dir().join(format!("daydream-recorder-{}", Uuid::now_v7()))
}

fn fake_failing_ffmpeg_path() -> PathBuf {
    let directory = test_app_data_dir();
    fs::create_dir_all(&directory).expect("create fake ffmpeg directory");
    let path = directory.join("ffmpeg");
    fs::write(
        &path,
        r#"#!/bin/sh
case "$*" in
  *-version*) echo 'ffmpeg version fake'; exit 0 ;;
  *-devices*) printf ' D  lavfi\n DE pulse\n'; exit 0 ;;
  *-formats*) printf ' DE rawvideo\n  E matroska\n'; exit 0 ;;
  *-encoders*) printf ' V....D libx264\n A..... libopus\n'; exit 0 ;;
esac
exit 1
"#,
    )
    .expect("write fake ffmpeg");
    make_executable(&path);

    path
}

fn fake_probe_then_missing_ffmpeg_path() -> PathBuf {
    let directory = test_app_data_dir();
    fs::create_dir_all(&directory).expect("create fake ffmpeg directory");
    let path = directory.join("ffmpeg");
    fs::write(
        &path,
        r#"#!/bin/sh
case "$*" in
  *-version*) echo 'ffmpeg version fake'; exit 0 ;;
  *-devices*) printf ' D  lavfi\n DE pulse\n'; exit 0 ;;
  *-formats*) printf ' DE rawvideo\n  E matroska\n'; exit 0 ;;
  *-encoders*) printf ' V....D libx264\n A..... libopus\n'; rm "$0"; exit 0 ;;
esac
exit 1
"#,
    )
    .expect("write fake ffmpeg");
    make_executable(&path);

    path
}

fn available_test_backend_health() -> super::types::RecorderBackendHealth {
    let mut backend_health = super::manager::unavailable_backend_health();
    backend_health.ffmpeg.available = true;
    backend_health.ffmpeg.error = None;
    backend_health.pipewire.available = true;
    backend_health.pipewire.error = None;
    backend_health.portal.available = true;
    backend_health.portal.error = None;

    backend_health
}

#[cfg(unix)]
fn make_executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(path)
        .expect("fake ffmpeg metadata")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).expect("make fake ffmpeg executable");
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) {}

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
        desktop_audio_source_id: None,
        fps: 1.0,
        microphone_audio_source_id: None,
        quality_preset: RecorderQualityPreset::BalancedOcr,
        scene_id: scene_id.to_string(),
    }
}

fn test_scene(scene_id: &str) -> RecorderScene {
    RecorderScene {
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
