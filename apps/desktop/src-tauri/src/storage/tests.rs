use std::{path::PathBuf, sync::Mutex};

use rusqlite::Connection;
use uuid::Uuid;

use super::{
    apply_migrations, collect_database_diagnostics_from_connection, create_recording,
    create_recording_segment, delete_portal_restore_token, delete_recording,
    enable_connection_pragmas, fail_recording_segment, finalize_recording_segment,
    get_latest_recorder_scene, get_recorder_scene, get_recording, list_recording_segments,
    mark_portal_restore_token_used, mark_recording_completed, mark_recording_started, migrations,
    recover_interrupted_recordings, save_recorder_scene, update_recording_status,
    upsert_portal_restore_token, AppStorage, CreateRecordingInput, CreateRecordingSegmentInput,
    FailRecordingSegmentInput, FinalizeRecordingSegmentInput, SaveRecorderSceneInput,
    SaveRecorderSceneSourceInput,
};

fn migrated_memory_connection() -> Connection {
    let mut connection = Connection::open_in_memory().expect("open in-memory sqlite database");
    enable_connection_pragmas(&connection).expect("enable sqlite pragmas");
    apply_migrations(&mut connection).expect("apply migrations");

    connection
}

#[test]
fn migration_creates_core_tables_and_schema_version() {
    let connection = migrated_memory_connection();
    let (schema_version, table_counts) = collect_database_diagnostics_from_connection(&connection)
        .expect("collect database diagnostics");

    assert_eq!(schema_version, 7);
    assert_eq!(table_counts["recordings"], 0);
    assert_eq!(table_counts["recording_tracks"], 0);
    assert_eq!(table_counts["recording_segments"], 0);
    assert_eq!(table_counts["recorder_scenes"], 0);
    assert_eq!(table_counts["recorder_scene_sources"], 0);
    assert_eq!(table_counts["recorder_portal_restore_tokens"], 0);
    assert_eq!(table_counts["redactions"], 0);
    assert_eq!(table_counts["capture_events"], 0);
    assert_eq!(table_counts["processing_jobs"], 0);
}

#[test]
fn migration_enables_foreign_keys() {
    let connection = migrated_memory_connection();
    let enabled: i64 = connection
        .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
        .expect("query foreign_keys pragma");

    assert_eq!(enabled, 1);
}

#[test]
fn recording_status_constraint_rejects_unknown_values() {
    let connection = migrated_memory_connection();
    let id = Uuid::now_v7().to_string();

    let error = connection
        .execute(
            r#"
            INSERT INTO recordings (
                id,
                created_at_ms,
                status,
                default_fps,
                quality_label,
                storage_dir
            ) VALUES (?1, 1, 'unknown', 1.0, 'ocr', '/tmp/daydream')
            "#,
            [&id],
        )
        .expect_err("unknown recording status should fail");

    assert!(error.to_string().contains("CHECK constraint failed"));
}

#[test]
fn redaction_interval_must_have_positive_duration() {
    let connection = migrated_memory_connection();
    let recording_id = Uuid::now_v7().to_string();

    connection
        .execute(
            r#"
            INSERT INTO recordings (
                id,
                created_at_ms,
                status,
                default_fps,
                quality_label,
                storage_dir
            ) VALUES (?1, 1, 'completed', 1.0, 'ocr', '/tmp/daydream')
            "#,
            [&recording_id],
        )
        .expect("insert recording");

    let error = connection
        .execute(
            r#"
            INSERT INTO redactions (
                id,
                recording_id,
                start_offset_ms,
                end_offset_ms,
                created_at_ms
            ) VALUES (?1, ?2, 10, 10, 1)
            "#,
            [&Uuid::now_v7().to_string(), &recording_id],
        )
        .expect_err("zero-length redaction should fail");

    assert!(error.to_string().contains("CHECK constraint failed"));
}

#[test]
fn track_requires_existing_recording() {
    let connection = migrated_memory_connection();

    let error = connection
        .execute(
            r#"
            INSERT INTO recording_tracks (
                id,
                recording_id,
                kind,
                label,
                created_at_ms
            ) VALUES (?1, ?2, 'screen', 'Display 1', 1)
            "#,
            [&Uuid::now_v7().to_string(), &Uuid::now_v7().to_string()],
        )
        .expect_err("track without recording should fail");

    assert!(error.to_string().contains("FOREIGN KEY constraint failed"));
}

#[test]
fn existing_v1_database_migrates_to_v2_without_losing_recordings() {
    let mut connection = Connection::open_in_memory().expect("open in-memory sqlite database");
    enable_connection_pragmas(&connection).expect("enable sqlite pragmas");
    migrations()
        .to_version(&mut connection, 1)
        .expect("apply v1 migration");

    let recording_id = Uuid::now_v7().to_string();
    connection
        .execute(
            r#"
            INSERT INTO recordings (
                id,
                created_at_ms,
                status,
                default_fps,
                quality_label,
                storage_dir
            ) VALUES (?1, 1, 'planned', 1.0, 'ocr', '/tmp/daydream')
            "#,
            [&recording_id],
        )
        .expect("insert v1 recording");

    apply_migrations(&mut connection).expect("apply remaining migrations");

    let (schema_version, table_counts) = collect_database_diagnostics_from_connection(&connection)
        .expect("collect database diagnostics");
    let migrated = get_recording(&connection, &recording_id).expect("read migrated recording");

    assert_eq!(schema_version, 7);
    assert_eq!(table_counts["recordings"], 1);
    assert_eq!(migrated.quality_label, "ocr");
    assert_eq!(migrated.active_scene_id, None);
    assert_eq!(migrated.output_root, None);
}

#[test]
fn existing_v1_database_with_child_rows_migrates_to_v2() {
    let mut connection = Connection::open_in_memory().expect("open in-memory sqlite database");
    enable_connection_pragmas(&connection).expect("enable sqlite pragmas");
    migrations()
        .to_version(&mut connection, 1)
        .expect("apply v1 migration");

    let recording_id = Uuid::now_v7().to_string();
    connection
        .execute(
            r#"
            INSERT INTO recordings (
                id,
                created_at_ms,
                status,
                default_fps,
                quality_label,
                storage_dir
            ) VALUES (?1, 1, 'planned', 1.0, 'ocr', '/tmp/daydream')
            "#,
            [&recording_id],
        )
        .expect("insert v1 recording");
    connection
        .execute(
            r#"
            INSERT INTO recording_tracks (
                id,
                recording_id,
                kind,
                label,
                created_at_ms
            ) VALUES (?1, ?2, 'screen', 'Display 1', 1)
            "#,
            [&Uuid::now_v7().to_string(), &recording_id],
        )
        .expect("insert v1 track");

    apply_migrations(&mut connection).expect("apply remaining migrations");

    let track_count: i64 = connection
        .query_row("SELECT COUNT(*) FROM recording_tracks", [], |row| {
            row.get(0)
        })
        .expect("count tracks");

    assert_eq!(track_count, 1);
}

#[test]
fn recording_accepts_balanced_ocr_and_minimal_recorder_metadata() {
    let connection = migrated_memory_connection();
    let scene_id = Uuid::now_v7().to_string();
    let recording_id = Uuid::now_v7().to_string();

    connection
        .execute(
            r#"
            INSERT INTO recorder_scenes (
                id,
                name,
                canvas_width,
                canvas_height,
                layout_mode,
                created_at_ms,
                updated_at_ms
            ) VALUES (?1, 'Scene', 2560, 1440, 'horizontal', 1, 1)
            "#,
            [&scene_id],
        )
        .expect("insert scene");

    let recording = create_recording(
        &connection,
        &CreateRecordingInput {
            id: recording_id.clone(),
            active_scene_id: Some(scene_id.clone()),
            created_at_ms: 2,
            default_fps: 1.0,
            notes: None,
            output_root: Some("/tmp/daydream/recordings".to_string()),
            quality_label: "balanced-ocr".to_string(),
            storage_dir: "/tmp/daydream".to_string(),
        },
    )
    .expect("create recording");

    assert_eq!(recording.id, recording_id);
    assert_eq!(recording.active_scene_id, Some(scene_id.clone()));
    assert_eq!(
        recording.output_root,
        Some("/tmp/daydream/recordings".to_string()),
    );
    assert_eq!(recording.quality_label, "balanced-ocr");

    connection
        .execute("DELETE FROM recorder_scenes WHERE id = ?1", [&scene_id])
        .expect("delete scene");

    let recording = get_recording(&connection, &recording_id).expect("read recording");
    assert_eq!(recording.active_scene_id, None);
}

#[test]
fn scene_helper_persists_absolute_pixel_sources_sorted_by_z_index() {
    let mut connection = migrated_memory_connection();
    let scene_id = Uuid::now_v7().to_string();

    let scene = save_recorder_scene(
        &mut connection,
        &SaveRecorderSceneInput {
            canvas_height: 1440,
            canvas_width: 5120,
            desktop_audio_source_id: Some("desktop-monitor".to_string()),
            id: scene_id.clone(),
            layout_mode: "horizontal".to_string(),
            microphone_audio_source_id: Some("microphone".to_string()),
            name: "Workday".to_string(),
            sources: vec![
                SaveRecorderSceneSourceInput {
                    crop_bottom: 0,
                    crop_left: 0,
                    crop_right: 0,
                    crop_top: 0,
                    display_name: "Right".to_string(),
                    height: 1440,
                    id: Uuid::now_v7().to_string(),
                    preserve_aspect_ratio: true,
                    source_id: "monitor-2".to_string(),
                    source_height: Some(1440),
                    source_kind: "monitor".to_string(),
                    source_width: Some(2560),
                    width: 2560,
                    x: 2560,
                    y: 0,
                    z_index: 2,
                },
                SaveRecorderSceneSourceInput {
                    crop_bottom: 4,
                    crop_left: 1,
                    crop_right: 3,
                    crop_top: 2,
                    display_name: "Left".to_string(),
                    height: 1440,
                    id: Uuid::now_v7().to_string(),
                    preserve_aspect_ratio: false,
                    source_id: "monitor-1".to_string(),
                    source_height: Some(1440),
                    source_kind: "monitor".to_string(),
                    source_width: Some(2560),
                    width: 2560,
                    x: 0,
                    y: 0,
                    z_index: 1,
                },
            ],
            timestamp_ms: 10,
        },
    )
    .expect("save scene");

    assert_eq!(scene.id, scene_id);
    assert_eq!(scene.canvas_width, 5120);
    assert_eq!(
        scene.desktop_audio_source_id,
        Some("desktop-monitor".to_string()),
    );
    assert_eq!(
        scene.microphone_audio_source_id,
        Some("microphone".to_string()),
    );
    assert_eq!(scene.sources.len(), 2);
    assert_eq!(scene.sources[0].source_id, "monitor-1");
    assert_eq!(scene.sources[0].crop_left, 1);
    assert!(!scene.sources[0].preserve_aspect_ratio);
    assert_eq!(scene.sources[1].source_id, "monitor-2");
}

#[test]
fn saving_existing_scene_replaces_sources_atomically() {
    let mut connection = migrated_memory_connection();
    let scene_id = Uuid::now_v7().to_string();

    save_recorder_scene(
        &mut connection,
        &SaveRecorderSceneInput {
            canvas_height: 1440,
            canvas_width: 2560,
            desktop_audio_source_id: None,
            id: scene_id.clone(),
            layout_mode: "horizontal".to_string(),
            microphone_audio_source_id: None,
            name: "Initial".to_string(),
            sources: vec![scene_source("monitor-1", 0)],
            timestamp_ms: 1,
        },
    )
    .expect("save initial scene");

    save_recorder_scene(
        &mut connection,
        &SaveRecorderSceneInput {
            canvas_height: 1440,
            canvas_width: 2560,
            desktop_audio_source_id: None,
            id: scene_id.clone(),
            layout_mode: "custom".to_string(),
            microphone_audio_source_id: None,
            name: "Updated".to_string(),
            sources: vec![scene_source("monitor-2", 0)],
            timestamp_ms: 2,
        },
    )
    .expect("update scene");

    let scene = get_recorder_scene(&connection, &scene_id).expect("read scene");

    assert_eq!(scene.name, "Updated");
    assert_eq!(scene.layout_mode, "custom");
    assert_eq!(scene.sources.len(), 1);
    assert_eq!(scene.sources[0].source_id, "monitor-2");
}

#[test]
fn scene_helper_accepts_current_layout_modes() {
    let mut connection = migrated_memory_connection();

    for (index, layout_mode) in ["auto-16-9", "custom", "horizontal", "vertical"]
        .into_iter()
        .enumerate()
    {
        let scene_id = Uuid::now_v7().to_string();
        let scene = save_recorder_scene(
            &mut connection,
            &SaveRecorderSceneInput {
                canvas_height: 1440,
                canvas_width: 2560,
                desktop_audio_source_id: None,
                id: scene_id,
                layout_mode: layout_mode.to_string(),
                microphone_audio_source_id: None,
                name: format!("Layout {layout_mode}"),
                sources: vec![scene_source("monitor-1", index as i64)],
                timestamp_ms: index as i64 + 1,
            },
        )
        .expect("save scene with current layout mode");

        assert_eq!(scene.layout_mode, layout_mode);
    }
}

#[test]
fn existing_v4_scene_constraint_migrates_to_current_layout_modes() {
    let mut connection = Connection::open_in_memory().expect("open in-memory sqlite database");
    enable_connection_pragmas(&connection).expect("enable sqlite pragmas");
    create_v4_recorder_scene_tables(&connection);
    let existing_scene_id = Uuid::now_v7().to_string();
    connection
        .execute(
            r#"
            INSERT INTO recorder_scenes (
                id,
                name,
                canvas_width,
                canvas_height,
                layout_mode,
                created_at_ms,
                updated_at_ms
            ) VALUES (?1, 'Existing scene', 2560, 1440, 'horizontal', 1, 1)
            "#,
            [&existing_scene_id],
        )
        .expect("insert existing v4 scene");
    connection
        .pragma_update(None, "user_version", 4)
        .expect("set v4 schema version");

    apply_migrations(&mut connection).expect("apply current layout migration");

    let existing_scene =
        get_recorder_scene(&connection, &existing_scene_id).expect("read migrated v4 scene");
    assert_eq!(existing_scene.desktop_audio_source_id, None);
    assert_eq!(existing_scene.microphone_audio_source_id, None);

    let scene_id = Uuid::now_v7().to_string();
    let scene = save_recorder_scene(
        &mut connection,
        &SaveRecorderSceneInput {
            canvas_height: 1440,
            canvas_width: 2560,
            desktop_audio_source_id: None,
            id: scene_id,
            layout_mode: "auto-16-9".to_string(),
            microphone_audio_source_id: None,
            name: "Auto scene".to_string(),
            sources: vec![scene_source("monitor-1", 0)],
            timestamp_ms: 1,
        },
    )
    .expect("save scene after layout constraint migration");

    assert_eq!(scene.layout_mode, "auto-16-9");
}

#[test]
fn latest_scene_helper_returns_newest_saved_scene() {
    let mut connection = migrated_memory_connection();
    let older_scene_id = Uuid::now_v7().to_string();
    let newer_scene_id = Uuid::now_v7().to_string();

    save_recorder_scene(
        &mut connection,
        &SaveRecorderSceneInput {
            canvas_height: 720,
            canvas_width: 1280,
            desktop_audio_source_id: None,
            id: older_scene_id,
            layout_mode: "horizontal".to_string(),
            microphone_audio_source_id: None,
            name: "Older".to_string(),
            sources: vec![scene_source("monitor-1", 0)],
            timestamp_ms: 10,
        },
    )
    .expect("save older scene");
    save_recorder_scene(
        &mut connection,
        &SaveRecorderSceneInput {
            canvas_height: 1440,
            canvas_width: 2560,
            desktop_audio_source_id: None,
            id: newer_scene_id.clone(),
            layout_mode: "custom".to_string(),
            microphone_audio_source_id: None,
            name: "Newer".to_string(),
            sources: vec![scene_source("monitor-2", 0)],
            timestamp_ms: 20,
        },
    )
    .expect("save newer scene");

    let latest = get_latest_recorder_scene(&connection)
        .expect("read latest scene")
        .expect("latest scene");

    assert_eq!(latest.id, newer_scene_id);
    assert_eq!(latest.name, "Newer");
    assert_eq!(latest.sources[0].source_id, "monitor-2");
}

#[test]
fn latest_scene_helper_returns_none_without_scenes() {
    let connection = migrated_memory_connection();

    let latest = get_latest_recorder_scene(&connection).expect("read latest scene");

    assert_eq!(latest, None);
}

#[test]
fn app_storage_exposes_mutable_connection_for_scene_transactions() {
    let connection = migrated_memory_connection();
    let scene_id = Uuid::now_v7().to_string();
    let storage = AppStorage {
        connection: Mutex::new(connection),
        path: PathBuf::from("/tmp/daydream.sqlite3"),
    };

    let scene = storage
        .with_connection_mut(|connection| {
            save_recorder_scene(
                connection,
                &SaveRecorderSceneInput {
                    canvas_height: 1440,
                    canvas_width: 2560,
                    desktop_audio_source_id: None,
                    id: scene_id.clone(),
                    layout_mode: "horizontal".to_string(),
                    microphone_audio_source_id: None,
                    name: "Managed storage scene".to_string(),
                    sources: vec![scene_source("monitor-1", 0)],
                    timestamp_ms: 1,
                },
            )
        })
        .expect("save scene through managed storage");

    assert_eq!(scene.id, scene_id);
    assert_eq!(scene.sources.len(), 1);
}

#[test]
fn segment_helpers_create_finalize_fail_and_list_in_order() {
    let connection = migrated_memory_connection();
    let recording_id = create_test_recording(&connection);
    let first_segment_id = Uuid::now_v7().to_string();
    let second_segment_id = Uuid::now_v7().to_string();

    create_recording_segment(
        &connection,
        &CreateRecordingSegmentInput {
            file_path: "/tmp/daydream/0001.mkv".to_string(),
            id: first_segment_id.clone(),
            recording_id: recording_id.clone(),
            segment_index: 1,
            started_at_ms: 10,
        },
    )
    .expect("create first segment");
    create_recording_segment(
        &connection,
        &CreateRecordingSegmentInput {
            file_path: "/tmp/daydream/0002.mkv".to_string(),
            id: second_segment_id.clone(),
            recording_id: recording_id.clone(),
            segment_index: 2,
            started_at_ms: 30,
        },
    )
    .expect("create second segment");

    let finalized = finalize_recording_segment(
        &connection,
        &FinalizeRecordingSegmentInput {
            desktop_audio_stream_index: Some(1),
            duration_ms: 15,
            ended_at_ms: 25,
            id: first_segment_id,
            microphone_audio_stream_index: Some(2),
            probe_json: Some(
                serde_json::json!({
                    "available": true,
                    "durationMs": 15,
                    "video": {
                        "codecName": "h264",
                        "height": 720,
                        "index": 0,
                        "pixelFormat": "yuv420p",
                        "width": 1280
                    }
                })
                .to_string(),
            ),
            updated_at_ms: 26,
            video_stream_index: Some(0),
        },
    )
    .expect("finalize segment");
    let failed = fail_recording_segment(
        &connection,
        &FailRecordingSegmentInput {
            ended_at_ms: 38,
            error_code: "ffmpeg_failed".to_string(),
            ffmpeg_exit_code: Some(1),
            id: second_segment_id,
            updated_at_ms: 39,
        },
    )
    .expect("fail segment");

    let segments = list_recording_segments(&connection, &recording_id).expect("list segments");

    assert_eq!(finalized.state, "finalized");
    assert_eq!(finalized.video_stream_index, Some(0));
    assert!(finalized
        .probe_json
        .as_deref()
        .expect("probe json")
        .contains("\"yuv420p\""));
    assert_eq!(failed.state, "failed");
    assert_eq!(failed.error_code, Some("ffmpeg_failed".to_string()));
    assert_eq!(segments.len(), 2);
    assert_eq!(segments[0].segment_index, 1);
    assert_eq!(segments[1].segment_index, 2);
}

#[test]
fn recover_interrupted_recordings_marks_stale_active_rows_failed() {
    let connection = migrated_memory_connection();
    let planned_id = create_test_recording(&connection);
    let recording_id = create_test_recording(&connection);
    let paused_id = create_test_recording(&connection);
    let completed_id = create_test_recording(&connection);
    let active_segment_id = Uuid::now_v7().to_string();
    let paused_segment_id = Uuid::now_v7().to_string();

    mark_recording_started(&connection, &recording_id, 10).expect("start recording");
    create_recording_segment(
        &connection,
        &CreateRecordingSegmentInput {
            file_path: "/tmp/daydream/0001.mkv".to_string(),
            id: active_segment_id,
            recording_id: recording_id.clone(),
            segment_index: 1,
            started_at_ms: 10,
        },
    )
    .expect("create active segment");
    mark_recording_started(&connection, &paused_id, 20).expect("start paused recording");
    create_recording_segment(
        &connection,
        &CreateRecordingSegmentInput {
            file_path: "/tmp/daydream/paused-0001.mkv".to_string(),
            id: paused_segment_id.clone(),
            recording_id: paused_id.clone(),
            segment_index: 1,
            started_at_ms: 20,
        },
    )
    .expect("create paused segment");
    finalize_recording_segment(
        &connection,
        &FinalizeRecordingSegmentInput {
            desktop_audio_stream_index: None,
            duration_ms: 15,
            ended_at_ms: 35,
            id: paused_segment_id,
            microphone_audio_stream_index: None,
            probe_json: None,
            updated_at_ms: 36,
            video_stream_index: Some(0),
        },
    )
    .expect("finalize paused segment");
    update_recording_status(&connection, &paused_id, "paused").expect("pause recording");
    mark_recording_started(&connection, &completed_id, 30).expect("start completed recording");
    mark_recording_completed(&connection, &completed_id, 40).expect("complete recording");

    let recovered_count =
        recover_interrupted_recordings(&connection, 100).expect("recover interrupted recordings");

    assert_eq!(recovered_count, 3);
    assert_eq!(
        get_recording(&connection, &planned_id)
            .expect("read planned")
            .status,
        "failed"
    );
    assert_eq!(
        get_recording(&connection, &recording_id)
            .expect("read recording")
            .ended_at_ms,
        Some(10)
    );
    let paused = get_recording(&connection, &paused_id).expect("read paused");
    assert_eq!(paused.status, "failed");
    assert_eq!(
        paused.ended_at_ms,
        Some(35),
        "paused recovery must use the last finalized segment end, not app startup time"
    );
    assert_eq!(
        get_recording(&connection, &completed_id)
            .expect("read completed")
            .status,
        "completed"
    );

    let segments =
        list_recording_segments(&connection, &recording_id).expect("list recovered segments");
    assert_eq!(segments[0].state, "failed");
    assert_eq!(segments[0].ended_at_ms, Some(10));
    assert_eq!(segments[0].duration_ms, Some(0));
    assert_eq!(
        segments[0].error_code.as_deref(),
        Some("recording_interrupted_by_shutdown")
    );
}

#[test]
fn delete_recording_rejects_active_statuses() {
    let connection = migrated_memory_connection();
    let planned_id = create_test_recording(&connection);

    let error = delete_recording(&connection, &planned_id).expect_err("active delete should fail");

    assert!(error.to_string().contains("not deletable"));
    assert_eq!(
        get_recording(&connection, &planned_id)
            .expect("active row remains")
            .status,
        "planned"
    );
}

#[test]
fn duplicate_segment_index_per_recording_is_rejected() {
    let connection = migrated_memory_connection();
    let recording_id = create_test_recording(&connection);

    create_recording_segment(
        &connection,
        &CreateRecordingSegmentInput {
            file_path: "/tmp/daydream/0001.mkv".to_string(),
            id: Uuid::now_v7().to_string(),
            recording_id: recording_id.clone(),
            segment_index: 1,
            started_at_ms: 10,
        },
    )
    .expect("create first segment");

    let error = create_recording_segment(
        &connection,
        &CreateRecordingSegmentInput {
            file_path: "/tmp/daydream/0001-copy.mkv".to_string(),
            id: Uuid::now_v7().to_string(),
            recording_id,
            segment_index: 1,
            started_at_ms: 11,
        },
    )
    .expect_err("duplicate segment index should fail");

    assert!(error.to_string().contains("UNIQUE constraint failed"));
}

#[test]
fn segment_probe_json_must_be_valid_json() {
    let connection = migrated_memory_connection();
    let recording_id = create_test_recording(&connection);
    let segment_id = Uuid::now_v7().to_string();

    create_recording_segment(
        &connection,
        &CreateRecordingSegmentInput {
            file_path: "/tmp/daydream/0001.mkv".to_string(),
            id: segment_id.clone(),
            recording_id,
            segment_index: 1,
            started_at_ms: 10,
        },
    )
    .expect("create segment");

    let error = finalize_recording_segment(
        &connection,
        &FinalizeRecordingSegmentInput {
            desktop_audio_stream_index: None,
            duration_ms: 10,
            ended_at_ms: 20,
            id: segment_id,
            microphone_audio_stream_index: None,
            probe_json: Some("{not-json".to_string()),
            updated_at_ms: 20,
            video_stream_index: Some(0),
        },
    )
    .expect_err("invalid probe json should fail");

    assert!(error.to_string().contains("CHECK constraint failed"));
}

#[test]
fn portal_restore_token_helpers_keep_token_backend_only() {
    let connection = migrated_memory_connection();

    let first = upsert_portal_restore_token(
        &connection,
        "linux-screencast-monitor",
        "linux-screencast",
        "token-1",
        10,
    )
    .expect("insert restore token");

    assert_eq!(first.token, "token-1");
    assert_eq!(first.created_at_ms, 10);
    assert_eq!(first.updated_at_ms, 10);
    assert_eq!(first.last_used_at_ms, Some(10));

    let second = upsert_portal_restore_token(
        &connection,
        "linux-screencast-monitor",
        "linux-screencast",
        "token-2",
        20,
    )
    .expect("update restore token");

    assert_eq!(second.token, "token-2");
    assert_eq!(second.created_at_ms, 10);
    assert_eq!(second.updated_at_ms, 20);
    assert_eq!(second.last_used_at_ms, Some(20));

    mark_portal_restore_token_used(&connection, "linux-screencast-monitor", 30)
        .expect("mark restore token used");

    let restored = super::get_portal_restore_token(&connection, "linux-screencast-monitor")
        .expect("read restore token")
        .expect("restore token exists");

    assert_eq!(restored.token, "token-2");
    assert_eq!(restored.last_used_at_ms, Some(30));

    delete_portal_restore_token(&connection, "linux-screencast-monitor")
        .expect("delete restore token");

    let deleted = super::get_portal_restore_token(&connection, "linux-screencast-monitor")
        .expect("read deleted restore token");
    assert_eq!(deleted, None);
}

#[test]
fn recorder_constraints_reject_invalid_values() {
    let connection = migrated_memory_connection();
    let recording_id = create_test_recording(&connection);
    let scene_id = Uuid::now_v7().to_string();

    let segment_error = connection
        .execute(
            r#"
            INSERT INTO recording_segments (
                id,
                recording_id,
                segment_index,
                file_path,
                started_at_ms,
                ended_at_ms,
                state,
                created_at_ms,
                updated_at_ms
            ) VALUES (?1, ?2, 1, '/tmp/daydream/0001.mkv', 20, 19, 'recording', 20, 20)
            "#,
            [&Uuid::now_v7().to_string(), &recording_id],
        )
        .expect_err("segment end before start should fail");
    assert!(segment_error
        .to_string()
        .contains("CHECK constraint failed"));

    let scene_error = connection
        .execute(
            r#"
            INSERT INTO recorder_scenes (
                id,
                name,
                canvas_width,
                canvas_height,
                layout_mode,
                created_at_ms,
                updated_at_ms
            ) VALUES (?1, 'Scene', 2560, 1440, 'diagonal', 1, 1)
            "#,
            [&scene_id],
        )
        .expect_err("invalid layout mode should fail");
    assert!(scene_error.to_string().contains("CHECK constraint failed"));

    connection
        .execute(
            r#"
            INSERT INTO recorder_scenes (
                id,
                name,
                canvas_width,
                canvas_height,
                layout_mode,
                created_at_ms,
                updated_at_ms
            ) VALUES (?1, 'Scene', 2560, 1440, 'horizontal', 1, 1)
            "#,
            [&scene_id],
        )
        .expect("insert scene");

    let source_error = connection
        .execute(
            r#"
            INSERT INTO recorder_scene_sources (
                id,
                scene_id,
                source_id,
                source_kind,
                display_name,
                x,
                y,
                width,
                height,
                crop_left,
                crop_top,
                crop_right,
                crop_bottom,
                z_index,
                preserve_aspect_ratio,
                created_at_ms,
                updated_at_ms
            ) VALUES (?1, ?2, 'monitor-1', 'window', 'Display', 0, 0, 0, 1440, 0, 0, 0, 0, 0, 2, 1, 1)
            "#,
            [&Uuid::now_v7().to_string(), &scene_id],
        )
        .expect_err("invalid source kind, size, and boolean should fail");
    assert!(source_error.to_string().contains("CHECK constraint failed"));
}

#[test]
fn recorder_foreign_keys_are_enforced() {
    let connection = migrated_memory_connection();

    let segment_error = create_recording_segment(
        &connection,
        &CreateRecordingSegmentInput {
            file_path: "/tmp/daydream/0001.mkv".to_string(),
            id: Uuid::now_v7().to_string(),
            recording_id: Uuid::now_v7().to_string(),
            segment_index: 1,
            started_at_ms: 10,
        },
    )
    .expect_err("segment without recording should fail");
    assert!(segment_error
        .to_string()
        .contains("FOREIGN KEY constraint failed"));

    let source_error = connection
        .execute(
            r#"
            INSERT INTO recorder_scene_sources (
                id,
                scene_id,
                source_id,
                source_kind,
                display_name,
                x,
                y,
                width,
                height,
                crop_left,
                crop_top,
                crop_right,
                crop_bottom,
                z_index,
                preserve_aspect_ratio,
                created_at_ms,
                updated_at_ms
            ) VALUES (?1, ?2, 'monitor-1', 'monitor', 'Display', 0, 0, 2560, 1440, 0, 0, 0, 0, 0, 1, 1, 1)
            "#,
            [&Uuid::now_v7().to_string(), &Uuid::now_v7().to_string()],
        )
        .expect_err("source without scene should fail");
    assert!(source_error
        .to_string()
        .contains("FOREIGN KEY constraint failed"));
}

fn create_test_recording(connection: &Connection) -> String {
    let recording_id = Uuid::now_v7().to_string();

    create_recording(
        connection,
        &CreateRecordingInput {
            id: recording_id.clone(),
            active_scene_id: None,
            created_at_ms: 1,
            default_fps: 1.0,
            notes: None,
            output_root: None,
            quality_label: "balanced-ocr".to_string(),
            storage_dir: "/tmp/daydream".to_string(),
        },
    )
    .expect("create recording");

    recording_id
}

fn create_v4_recorder_scene_tables(connection: &Connection) {
    connection
        .execute_batch(
            r#"
            CREATE TABLE recorder_scenes (
                id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
                name TEXT NOT NULL,
                canvas_width INTEGER NOT NULL CHECK (canvas_width > 0),
                canvas_height INTEGER NOT NULL CHECK (canvas_height > 0),
                layout_mode TEXT NOT NULL CHECK (layout_mode IN ('horizontal', 'custom')),
                created_at_ms INTEGER NOT NULL CHECK (created_at_ms >= 0),
                updated_at_ms INTEGER NOT NULL CHECK (updated_at_ms >= created_at_ms)
            );

            CREATE TABLE recorder_scene_sources (
                id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
                scene_id TEXT NOT NULL CHECK (length(scene_id) = 36),
                source_id TEXT NOT NULL,
                source_kind TEXT NOT NULL CHECK (source_kind IN ('monitor')),
                display_name TEXT NOT NULL,
                x INTEGER NOT NULL,
                y INTEGER NOT NULL,
                width INTEGER NOT NULL CHECK (width > 0),
                height INTEGER NOT NULL CHECK (height > 0),
                source_width INTEGER CHECK (source_width IS NULL OR source_width > 0),
                source_height INTEGER CHECK (source_height IS NULL OR source_height > 0),
                crop_left INTEGER NOT NULL CHECK (crop_left >= 0),
                crop_top INTEGER NOT NULL CHECK (crop_top >= 0),
                crop_right INTEGER NOT NULL CHECK (crop_right >= 0),
                crop_bottom INTEGER NOT NULL CHECK (crop_bottom >= 0),
                z_index INTEGER NOT NULL,
                preserve_aspect_ratio INTEGER NOT NULL CHECK (preserve_aspect_ratio IN (0, 1)),
                created_at_ms INTEGER NOT NULL CHECK (created_at_ms >= 0),
                updated_at_ms INTEGER NOT NULL CHECK (updated_at_ms >= created_at_ms),
                FOREIGN KEY (scene_id) REFERENCES recorder_scenes(id) ON DELETE CASCADE
            );
            "#,
        )
        .expect("create v4 recorder scene tables");
}

fn scene_source(source_id: &str, z_index: i64) -> SaveRecorderSceneSourceInput {
    SaveRecorderSceneSourceInput {
        crop_bottom: 0,
        crop_left: 0,
        crop_right: 0,
        crop_top: 0,
        display_name: source_id.to_string(),
        height: 1440,
        id: Uuid::now_v7().to_string(),
        preserve_aspect_ratio: true,
        source_id: source_id.to_string(),
        source_height: Some(1440),
        source_kind: "monitor".to_string(),
        source_width: Some(2560),
        width: 2560,
        x: 0,
        y: 0,
        z_index,
    }
}
