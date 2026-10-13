use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use rusqlite::Connection;
use uuid::Uuid;

use super::{
    acknowledge_notifications, apply_migrations, cancel_visual_frame_processing_states_for_chunk,
    claim_next_queued_processing_chunk, collect_database_diagnostics_from_connection,
    create_activity_interval, create_frame_similarity_entry, create_ocr_region,
    create_processing_chunk, create_recording, create_recording_segment, create_redaction,
    create_transcript_segment, create_transcript_word, create_vlm_observation,
    delete_all_notifications, delete_notifications, delete_notifications_older_than,
    delete_portal_restore_token, delete_recording, delete_redaction, dismiss_notification,
    enable_connection_pragmas, fail_recording_segment, finalize_recording_segment, get_app_setting,
    get_latest_recorder_scene, get_notification, get_recorder_scene, get_recording,
    list_app_settings, list_notifications, list_processing_chunks_by_status,
    list_processing_jobs_by_status, list_recording_segments, list_redactions,
    list_visual_frame_processing_states, mark_portal_restore_token_used, mark_recording_completed,
    mark_recording_started, migrations, recover_interrupted_recordings, rename_recording,
    reset_app_settings, retry_processing_job_chunks, save_ai_model_asset, save_app_settings,
    save_notification, save_recorder_scene, save_unseen_notification,
    update_processing_chunk_result_code, update_recording_status,
    update_visual_frame_processing_state_status, upsert_portal_restore_token,
    upsert_visual_frame_processing_state, AppStorage, CreateActivityIntervalInput,
    CreateFrameSimilarityEntryInput, CreateProcessingChunkInput, CreateRecordingInput,
    CreateRecordingSegmentInput, CreateRedactionInput, CreateTranscriptSegmentInput,
    CreateTranscriptWordInput, CreateVisualFrameProcessingStateInput, CreateVisualRegionInput,
    FailRecordingSegmentInput, FinalizeRecordingSegmentInput, SaveAiModelAssetInput,
    SaveAppSettingInput, SaveNotificationInput, SaveRecorderSceneInput,
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

    assert_eq!(schema_version, 39);
    assert_eq!(table_counts["app_settings"], 0);
    assert_eq!(table_counts["ai_model_assets"], 0);
    assert_eq!(table_counts["notifications"], 0);
    assert_eq!(table_counts["recordings"], 0);
    assert_eq!(table_counts["recording_tracks"], 0);
    assert_eq!(table_counts["recording_segments"], 0);
    assert_eq!(table_counts["recorder_scenes"], 0);
    assert_eq!(table_counts["recorder_scene_sources"], 0);
    assert_eq!(table_counts["recorder_portal_restore_tokens"], 0);
    assert_eq!(table_counts["redactions"], 0);
    assert_eq!(table_counts["recording_protection_spans"], 0);
    assert_eq!(table_counts["capture_events"], 0);
    assert_eq!(table_counts["processing_jobs"], 0);
    assert_eq!(table_counts["processing_chunks"], 0);
    assert_eq!(table_counts["transcript_segments"], 0);
    assert_eq!(table_counts["transcript_words"], 0);
    assert_eq!(table_counts["frame_activity_intervals"], 0);
    assert_eq!(table_counts["ocr_regions"], 0);
    assert_eq!(table_counts["vlm_observations"], 0);
    assert_eq!(table_counts["frame_similarity_entries"], 0);
    assert_eq!(table_counts["visual_frame_processing_states"], 0);
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
fn notifications_persist_actions_and_dismissal_state() {
    let connection = migrated_memory_connection();

    let saved = save_notification(
        &connection,
        &SaveNotificationInput {
            actions_json: r#"[{"id":"retry","label":"Retry","handler":"model-download-retry","payload":{"modelId":"small"},"style":"secondary"}]"#.to_string(),
            id: Uuid::now_v7().to_string(),
            message: "Model download is not implemented in this gate.".to_string(),
            source: "ai".to_string(),
            status: "failed".to_string(),
            timestamp_ms: 10,
            title: "Download unavailable".to_string(),
            topic: "model-download".to_string(),
        },
    )
    .expect("save notification");

    assert_eq!(saved.source, "ai");
    assert_eq!(saved.topic, "model-download");
    assert!(saved.actions_json.contains("model-download-retry"));
    assert_eq!(
        list_notifications(&connection)
            .expect("list notifications")
            .len(),
        1
    );

    let dismissed = dismiss_notification(&connection, &saved.id, 20).expect("dismiss notification");

    assert_eq!(dismissed.dismissed_at_ms, Some(20));
    assert_eq!(dismissed.updated_at_ms, 10);
    assert_eq!(
        get_notification(&connection, &saved.id)
            .expect("read notification")
            .expect("notification row")
            .dismissed_at_ms,
        Some(20)
    );
}

#[test]
fn unseen_notification_upsert_clears_dismissal_state() {
    let connection = migrated_memory_connection();
    let id = Uuid::now_v7().to_string();

    save_notification(
        &connection,
        &SaveNotificationInput {
            actions_json: "[]".to_string(),
            id: id.clone(),
            message: "Started.".to_string(),
            source: "ai".to_string(),
            status: "running".to_string(),
            timestamp_ms: 10,
            title: "Model download started".to_string(),
            topic: "model-download".to_string(),
        },
    )
    .expect("save notification");
    dismiss_notification(&connection, &id, 20).expect("dismiss notification");

    let updated = save_unseen_notification(
        &connection,
        &SaveNotificationInput {
            actions_json: "[]".to_string(),
            id,
            message: "Cancelled.".to_string(),
            source: "ai".to_string(),
            status: "cancelled".to_string(),
            timestamp_ms: 30,
            title: "Model download cancelled".to_string(),
            topic: "model-download".to_string(),
        },
    )
    .expect("upsert notification");

    assert_eq!(updated.dismissed_at_ms, None);
    assert_eq!(updated.created_at_ms, 10);
    assert_eq!(updated.updated_at_ms, 30);
    assert_eq!(updated.status, "cancelled");
}

#[test]
fn notifications_acknowledge_supplied_unseen_rows_without_deleting_history() {
    let connection = migrated_memory_connection();

    let first = save_notification(
        &connection,
        &SaveNotificationInput {
            actions_json: "[]".to_string(),
            id: Uuid::now_v7().to_string(),
            message: "First notification.".to_string(),
            source: "system".to_string(),
            status: "info".to_string(),
            timestamp_ms: 10,
            title: "First".to_string(),
            topic: "status".to_string(),
        },
    )
    .expect("save first notification");
    let second = save_notification(
        &connection,
        &SaveNotificationInput {
            actions_json: "[]".to_string(),
            id: Uuid::now_v7().to_string(),
            message: "Second notification.".to_string(),
            source: "system".to_string(),
            status: "warning".to_string(),
            timestamp_ms: 20,
            title: "Second".to_string(),
            topic: "status".to_string(),
        },
    )
    .expect("save second notification");

    acknowledge_notifications(&connection, std::slice::from_ref(&first.id), 30)
        .expect("acknowledge notification");

    let rows = list_notifications(&connection).expect("list notifications");
    assert_eq!(rows.len(), 2);
    assert_eq!(
        get_notification(&connection, &first.id)
            .expect("read first")
            .expect("first row")
            .dismissed_at_ms,
        Some(30)
    );
    assert_eq!(
        get_notification(&connection, &second.id)
            .expect("read second")
            .expect("second row")
            .dismissed_at_ms,
        None
    );

    acknowledge_notifications(&connection, &[first.id], 40).expect("acknowledge already seen");

    assert_eq!(
        get_notification(&connection, &second.id)
            .expect("read second after duplicate ack")
            .expect("second row")
            .dismissed_at_ms,
        None
    );
}

#[test]
fn notifications_can_be_permanently_deleted_and_pruned_by_age() {
    let connection = migrated_memory_connection();

    let old = save_notification(
        &connection,
        &SaveNotificationInput {
            actions_json: "[]".to_string(),
            id: Uuid::now_v7().to_string(),
            message: "Old notification.".to_string(),
            source: "system".to_string(),
            status: "info".to_string(),
            timestamp_ms: 10,
            title: "Old".to_string(),
            topic: "status".to_string(),
        },
    )
    .expect("save old notification");
    let recent = save_notification(
        &connection,
        &SaveNotificationInput {
            actions_json: "[]".to_string(),
            id: Uuid::now_v7().to_string(),
            message: "Recent notification.".to_string(),
            source: "system".to_string(),
            status: "info".to_string(),
            timestamp_ms: 100,
            title: "Recent".to_string(),
            topic: "status".to_string(),
        },
    )
    .expect("save recent notification");

    assert_eq!(
        delete_notifications(&connection, std::slice::from_ref(&recent.id))
            .expect("delete one notification"),
        1
    );
    assert!(get_notification(&connection, &recent.id)
        .expect("read deleted notification")
        .is_none());
    assert_eq!(list_notifications(&connection).expect("list rows").len(), 1);

    let replacement = save_notification(
        &connection,
        &SaveNotificationInput {
            actions_json: "[]".to_string(),
            id: Uuid::now_v7().to_string(),
            message: "Replacement notification.".to_string(),
            source: "system".to_string(),
            status: "info".to_string(),
            timestamp_ms: 200,
            title: "Replacement".to_string(),
            topic: "status".to_string(),
        },
    )
    .expect("save replacement notification");

    assert_eq!(
        delete_notifications_older_than(&connection, 100).expect("prune old rows"),
        1
    );
    assert!(get_notification(&connection, &old.id)
        .expect("read pruned notification")
        .is_none());
    assert!(get_notification(&connection, &replacement.id)
        .expect("read replacement notification")
        .is_some());

    assert_eq!(
        delete_all_notifications(&connection).expect("delete all notifications"),
        1
    );
    assert!(list_notifications(&connection)
        .expect("list rows")
        .is_empty());
}

#[test]
fn app_settings_round_trip_and_reset() {
    let mut connection = migrated_memory_connection();

    assert_eq!(
        list_app_settings(&connection).expect("read empty app settings"),
        Vec::new()
    );

    let saved = save_app_settings(
        &mut connection,
        &[
            SaveAppSettingInput {
                key: "record.fps".to_string(),
                value_json: "0.2".to_string(),
                is_sensitive: false,
                timestamp_ms: 10,
            },
            SaveAppSettingInput {
                key: "storage.sizeLimitBytes".to_string(),
                value_json: "53687091200".to_string(),
                is_sensitive: false,
                timestamp_ms: 10,
            },
        ],
    )
    .expect("save app settings");

    assert_eq!(saved.len(), 2);
    assert_eq!(saved[0].key, "record.fps");
    assert_eq!(saved[0].value_json, "0.2");
    assert!(!saved[0].is_sensitive);
    assert_eq!(saved[0].updated_at_ms, 10);

    let updated = save_app_settings(
        &mut connection,
        &[SaveAppSettingInput {
            key: "record.fps".to_string(),
            value_json: "5.0".to_string(),
            is_sensitive: false,
            timestamp_ms: 20,
        }],
    )
    .expect("update app settings");
    let record_fps = updated
        .iter()
        .find(|row| row.key == "record.fps")
        .expect("record fps row");

    assert_eq!(record_fps.value_json, "5.0");
    assert_eq!(record_fps.created_at_ms, 10);
    assert_eq!(record_fps.updated_at_ms, 20);

    reset_app_settings(&connection).expect("reset app settings");

    assert_eq!(
        list_app_settings(&connection).expect("read reset app settings"),
        Vec::new()
    );
}

#[test]
fn app_settings_can_mark_encrypted_sensitive_rows() {
    let mut connection = migrated_memory_connection();

    save_app_settings(
        &mut connection,
        &[SaveAppSettingInput {
            key: "ai.transcription.remoteApiKey".to_string(),
            value_json: r#"{"v":1,"alg":"AES-256-GCM","nonce":"bm9uY2U=","ciphertext":"Y2lwaGVy"}"#
                .to_string(),
            is_sensitive: true,
            timestamp_ms: 10,
        }],
    )
    .expect("save encrypted setting row");

    let row = get_app_setting(&connection, "ai.transcription.remoteApiKey")
        .expect("read encrypted setting")
        .expect("encrypted setting row");

    assert!(row.is_sensitive);
    assert!(!row.value_json.contains("sk-"));
    assert!(row.value_json.contains("AES-256-GCM"));
}

#[test]
fn app_settings_json_must_be_valid() {
    let mut connection = migrated_memory_connection();

    let error = save_app_settings(
        &mut connection,
        &[SaveAppSettingInput {
            key: "record.fps".to_string(),
            value_json: "{not-json".to_string(),
            is_sensitive: false,
            timestamp_ms: 1,
        }],
    )
    .expect_err("invalid json should fail sqlite constraints");

    assert!(error.to_string().contains("CHECK constraint failed"));
}

#[test]
fn app_settings_multi_row_save_is_atomic() {
    let mut connection = migrated_memory_connection();

    save_app_settings(
        &mut connection,
        &[SaveAppSettingInput {
            key: "record.fps".to_string(),
            value_json: "0.2".to_string(),
            is_sensitive: false,
            timestamp_ms: 1,
        }],
    )
    .expect("save initial app setting");

    let error = save_app_settings(
        &mut connection,
        &[
            SaveAppSettingInput {
                key: "record.fps".to_string(),
                value_json: "5.0".to_string(),
                is_sensitive: false,
                timestamp_ms: 2,
            },
            SaveAppSettingInput {
                key: "storage.sizeLimitBytes".to_string(),
                value_json: "{not-json".to_string(),
                is_sensitive: false,
                timestamp_ms: 2,
            },
        ],
    )
    .expect_err("invalid later row should fail whole settings save");

    assert!(error.to_string().contains("CHECK constraint failed"));
    assert_eq!(
        get_app_setting(&connection, "record.fps")
            .expect("read record fps")
            .expect("record fps row")
            .value_json,
        "0.2"
    );
    assert!(get_app_setting(&connection, "storage.sizeLimitBytes")
        .expect("read storage size")
        .is_none());
}

#[test]
fn existing_flat_app_settings_migrate_to_key_value_rows() {
    let mut connection = Connection::open_in_memory().expect("open in-memory sqlite database");
    enable_connection_pragmas(&connection).expect("enable sqlite pragmas");
    migrations()
        .to_version(&mut connection, 8)
        .expect("apply v8 migration");

    connection
        .execute(
            r#"
            INSERT INTO app_settings (
                id,
                record_fps,
                record_video_bitrate_kbps,
                storage_retention_days,
                storage_size_limit_bytes,
                updated_at_ms
            ) VALUES (1, 0.5, 2500, 30, 80530636800, 99)
            "#,
            [],
        )
        .expect("insert v8 app settings");

    apply_migrations(&mut connection).expect("apply v9 migration");

    assert_eq!(
        get_app_setting(&connection, "record.fps")
            .expect("read record fps")
            .expect("record fps row")
            .value_json,
        "0.5"
    );
    assert_eq!(
        get_app_setting(&connection, "storage.sizeLimitBytes")
            .expect("read storage size")
            .expect("storage size row")
            .value_json,
        "80530636800"
    );
}

#[test]
fn processing_jobs_accept_m2_kinds_after_migration() {
    let connection = migrated_memory_connection();
    let recording_id = create_test_recording(&connection);

    for kind in [
        "activity",
        "ai-processing",
        "chunking",
        "digest",
        "embedding",
        "frame-extraction",
        "ocr",
        "transcription",
        "vlm",
    ] {
        connection
            .execute(
                r#"
                INSERT INTO processing_jobs (
                    id,
                    recording_id,
                    kind,
                    status,
                    created_at_ms,
                    updated_at_ms
                ) VALUES (?1, ?2, ?3, 'paused', 1, 1)
                "#,
                (&Uuid::now_v7().to_string(), &recording_id, kind),
            )
            .expect("insert m2 processing job");
    }

    let error = connection
        .execute(
            r#"
            INSERT INTO processing_jobs (
                id,
                recording_id,
                kind,
                status,
                created_at_ms,
                updated_at_ms
            ) VALUES (?1, ?2, 'unknown', 'queued', 1, 1)
            "#,
            (&Uuid::now_v7().to_string(), &recording_id),
        )
        .expect_err("unknown job kind should fail");

    assert!(error.to_string().contains("CHECK constraint failed"));
}

#[test]
fn processing_status_helpers_support_unbounded_queries_without_overflowing_limit() {
    let connection = migrated_memory_connection();
    let recording_id = create_test_recording(&connection);
    let job_id = Uuid::now_v7().to_string();

    for index in 0..3 {
        connection
            .execute(
                r#"
                INSERT INTO processing_jobs (
                    id,
                    recording_id,
                    kind,
                    status,
                    created_at_ms,
                    updated_at_ms
                ) VALUES (?1, ?2, 'ai-processing', 'running', ?3, ?3)
                "#,
                (
                    if index == 0 {
                        job_id.clone()
                    } else {
                        Uuid::now_v7().to_string()
                    },
                    &recording_id,
                    index + 1,
                ),
            )
            .expect("insert running processing job");
    }

    for index in 0..3 {
        create_processing_chunk(
            &connection,
            &CreateProcessingChunkInput {
                attempt_count: 0,
                created_at_ms: index + 1,
                end_offset_ms: Some((index + 1) * 1_000),
                error_code: None,
                id: Uuid::now_v7().to_string(),
                job_id: job_id.clone(),
                progress: 0.0,
                recording_id: recording_id.clone(),
                result_code: None,
                segment_id: None,
                source_kind: "audio".to_string(),
                start_offset_ms: Some(index * 1_000),
                status: "failed".to_string(),
                track_id: None,
                updated_at_ms: index + 1,
            },
        )
        .expect("insert failed processing chunk");
    }

    let jobs = list_processing_jobs_by_status(&connection, "ai-processing", &["running"], None)
        .expect("list unbounded jobs");
    let chunks = list_processing_chunks_by_status(&connection, &job_id, &["failed"], None)
        .expect("list unbounded chunks");
    let limited_jobs =
        list_processing_jobs_by_status(&connection, "ai-processing", &["running"], Some(2))
            .expect("list limited jobs");
    let huge_limit_chunks =
        list_processing_chunks_by_status(&connection, &job_id, &["failed"], Some(usize::MAX))
            .expect("list chunks with huge bounded limit");

    assert_eq!(jobs.len(), 3);
    assert_eq!(chunks.len(), 3);
    assert_eq!(limited_jobs.len(), 2);
    assert_eq!(huge_limit_chunks.len(), 3);
}

#[test]
fn claim_next_queued_processing_chunk_marks_one_chunk_running() {
    let connection = migrated_memory_connection();
    let recording_id = create_test_recording(&connection);
    let job_id = Uuid::now_v7().to_string();
    let chunk_id = Uuid::now_v7().to_string();

    connection
        .execute(
            r#"
            INSERT INTO processing_jobs (
                id,
                recording_id,
                kind,
                status,
                created_at_ms,
                updated_at_ms
            ) VALUES (?1, ?2, 'ai-processing', 'running', 1, 1)
            "#,
            (&job_id, &recording_id),
        )
        .expect("insert processing job");
    create_processing_chunk(
        &connection,
        &CreateProcessingChunkInput {
            attempt_count: 0,
            created_at_ms: 1,
            end_offset_ms: Some(1_000),
            error_code: None,
            id: chunk_id.clone(),
            job_id: job_id.clone(),
            progress: 0.0,
            recording_id,
            result_code: None,
            segment_id: None,
            source_kind: "audio".to_string(),
            start_offset_ms: Some(0),
            status: "queued".to_string(),
            track_id: None,
            updated_at_ms: 1,
        },
    )
    .expect("insert queued processing chunk");

    let claimed = claim_next_queued_processing_chunk(&connection, &job_id, 0.05, 10)
        .expect("claim queued chunk")
        .expect("claimed chunk");
    let claimed_again = claim_next_queued_processing_chunk(&connection, &job_id, 0.05, 11)
        .expect("claim no queued chunk");
    let running_chunks = list_processing_chunks_by_status(&connection, &job_id, &["running"], None)
        .expect("list running chunks");
    let queued_chunks = list_processing_chunks_by_status(&connection, &job_id, &["queued"], None)
        .expect("list queued chunks");

    assert_eq!(claimed.id, chunk_id);
    assert_eq!(claimed.status, "running");
    assert_eq!(claimed.progress, 0.05);
    assert_eq!(claimed.attempt_count, 1);
    assert!(claimed_again.is_none());
    assert_eq!(running_chunks.len(), 1);
    assert_eq!(queued_chunks.len(), 0);
}

#[test]
fn visual_frame_processing_state_tracks_progress_and_cleans_with_chunk() {
    let connection = migrated_memory_connection();
    let recording_id = create_test_recording(&connection);
    let job_id = Uuid::now_v7().to_string();
    let chunk_id = Uuid::now_v7().to_string();

    connection
        .execute(
            r#"
            INSERT INTO processing_jobs (
                id,
                recording_id,
                kind,
                status,
                created_at_ms,
                updated_at_ms
            ) VALUES (?1, ?2, 'ai-processing', 'running', 1, 1)
            "#,
            (&job_id, &recording_id),
        )
        .expect("insert processing job");
    create_processing_chunk(
        &connection,
        &CreateProcessingChunkInput {
            attempt_count: 0,
            created_at_ms: 1,
            end_offset_ms: Some(1_000),
            error_code: None,
            id: chunk_id.clone(),
            job_id: job_id.clone(),
            progress: 0.0,
            recording_id: recording_id.clone(),
            result_code: None,
            segment_id: None,
            source_kind: "ocr".to_string(),
            start_offset_ms: Some(0),
            status: "running".to_string(),
            track_id: None,
            updated_at_ms: 1,
        },
    )
    .expect("insert running processing chunk");

    let frame = upsert_visual_frame_processing_state(
        &connection,
        &CreateVisualFrameProcessingStateInput {
            chunk_id: chunk_id.clone(),
            created_at_ms: 2,
            error_code: None,
            frame_timestamp_ms: 500,
            id: Uuid::now_v7().to_string(),
            job_id: job_id.clone(),
            output_count: 0,
            progress: 0.0,
            recording_id: recording_id.clone(),
            region_height: 720,
            region_width: 1280,
            region_x: 0,
            region_y: 0,
            segment_id: None,
            source_frame_ref: "segment:one:offset:500:frame:0".to_string(),
            source_kind: "ocr".to_string(),
            status: "queued".to_string(),
            updated_at_ms: 2,
        },
    )
    .expect("insert frame state");

    let running = update_visual_frame_processing_state_status(
        &connection,
        &frame.id,
        "running",
        0.5,
        None,
        0,
        3,
    )
    .expect("mark frame running");
    let succeeded = update_visual_frame_processing_state_status(
        &connection,
        &frame.id,
        "succeeded",
        1.0,
        None,
        4,
        4,
    )
    .expect("mark frame succeeded");
    cancel_visual_frame_processing_states_for_chunk(&connection, &chunk_id, 5)
        .expect("cancel active frame states");
    let rows =
        list_visual_frame_processing_states(&connection, &recording_id).expect("list frame states");

    assert_eq!(running.status, "running");
    assert_eq!(succeeded.status, "succeeded");
    assert_eq!(succeeded.output_count, 4);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].status, "succeeded");

    super::delete_visual_output_for_chunks(&connection, std::slice::from_ref(&chunk_id))
        .expect("delete visual outputs");
    let rows = list_visual_frame_processing_states(&connection, &recording_id)
        .expect("list frame states after cleanup");
    assert!(rows.is_empty());
}

#[test]
fn processing_chunk_result_code_round_trips_and_clears_on_claim() {
    let connection = migrated_memory_connection();
    let recording_id = create_test_recording(&connection);
    let job_id = Uuid::now_v7().to_string();
    let chunk_id = Uuid::now_v7().to_string();

    connection
        .execute(
            r#"
            INSERT INTO processing_jobs (
                id,
                recording_id,
                kind,
                status,
                created_at_ms,
                updated_at_ms
            ) VALUES (?1, ?2, 'ai-processing', 'running', 1, 1)
            "#,
            (&job_id, &recording_id),
        )
        .expect("insert processing job");
    create_processing_chunk(
        &connection,
        &CreateProcessingChunkInput {
            attempt_count: 0,
            created_at_ms: 1,
            end_offset_ms: Some(1_000),
            error_code: None,
            id: chunk_id.clone(),
            job_id: job_id.clone(),
            progress: 0.0,
            recording_id,
            result_code: None,
            segment_id: None,
            source_kind: "audio".to_string(),
            start_offset_ms: Some(0),
            status: "queued".to_string(),
            track_id: None,
            updated_at_ms: 1,
        },
    )
    .expect("insert queued processing chunk");

    let updated =
        update_processing_chunk_result_code(&connection, &chunk_id, Some("skipped-silence"), 2)
            .expect("update result code");

    assert_eq!(updated.result_code.as_deref(), Some("skipped-silence"));

    let claimed = claim_next_queued_processing_chunk(&connection, &job_id, 0.05, 3)
        .expect("claim queued chunk")
        .expect("claimed chunk");

    assert_eq!(claimed.id, chunk_id);
    assert_eq!(claimed.result_code, None);
}

#[test]
fn retry_processing_job_requeues_terminal_chunks_and_clears_their_outputs() {
    let connection = migrated_memory_connection();
    let recording_id = create_test_recording(&connection);
    let segment_id = Uuid::now_v7().to_string();
    let track_id = Uuid::now_v7().to_string();
    let job_id = Uuid::now_v7().to_string();
    let failed_chunk_id = Uuid::now_v7().to_string();
    let cancelled_chunk_id = Uuid::now_v7().to_string();
    let succeeded_chunk_id = Uuid::now_v7().to_string();

    connection
        .execute(
            r#"
            INSERT INTO recording_tracks (
                id,
                recording_id,
                kind,
                label,
                created_at_ms
            ) VALUES (?1, ?2, 'microphone', 'Microphone', 1)
            "#,
            (&track_id, &recording_id),
        )
        .expect("insert track");
    create_recording_segment(
        &connection,
        &CreateRecordingSegmentInput {
            file_path: "/tmp/daydream/0001.mkv".to_string(),
            id: segment_id.clone(),
            recording_id: recording_id.clone(),
            segment_index: 1,
            started_at_ms: 10,
        },
    )
    .expect("create segment");
    connection
        .execute(
            r#"
            INSERT INTO processing_jobs (
                id,
                recording_id,
                kind,
                status,
                error,
                created_at_ms,
                updated_at_ms
            ) VALUES (?1, ?2, 'ai-processing', 'failed', 'chunk failed', 1, 2)
            "#,
            (&job_id, &recording_id),
        )
        .expect("insert failed job");

    for (index, chunk_id, status, source_kind) in [
        (0, &failed_chunk_id, "failed", "audio"),
        (1, &cancelled_chunk_id, "cancelled", "activity"),
        (2, &succeeded_chunk_id, "succeeded", "audio"),
    ] {
        create_processing_chunk(
            &connection,
            &CreateProcessingChunkInput {
                attempt_count: 1,
                created_at_ms: index + 1,
                end_offset_ms: Some((index + 1) * 1_000),
                error_code: if status == "succeeded" {
                    None
                } else {
                    Some("processing-failed".to_string())
                },
                id: chunk_id.clone(),
                job_id: job_id.clone(),
                progress: if status == "succeeded" { 1.0 } else { 0.5 },
                recording_id: recording_id.clone(),
                result_code: None,
                segment_id: Some(segment_id.clone()),
                source_kind: source_kind.to_string(),
                start_offset_ms: Some(index * 1_000),
                status: status.to_string(),
                track_id: Some(track_id.clone()),
                updated_at_ms: index + 2,
            },
        )
        .expect("insert processing chunk");
    }

    for chunk_id in [&failed_chunk_id, &succeeded_chunk_id] {
        create_transcript_segment(
            &connection,
            &CreateTranscriptSegmentInput {
                chunk_id: chunk_id.clone(),
                confidence: Some(0.8),
                created_at_ms: 5,
                end_offset_ms: 900,
                id: Uuid::now_v7().to_string(),
                job_id: job_id.clone(),
                model: Some("small".to_string()),
                provider: "local-whisper-cpp".to_string(),
                recording_id: recording_id.clone(),
                segment_id: segment_id.clone(),
                source_role: "microphone".to_string(),
                start_offset_ms: 100,
                text: "hello".to_string(),
                track_id: track_id.clone(),
            },
        )
        .expect("insert transcript segment");
    }

    for chunk_id in [&cancelled_chunk_id, &succeeded_chunk_id] {
        create_activity_interval(
            &connection,
            &CreateActivityIntervalInput {
                chunk_id: Some(chunk_id.clone()),
                end_offset_ms: 1_000,
                id: Uuid::now_v7().to_string(),
                job_id: job_id.clone(),
                recording_id: recording_id.clone(),
                regions_json: Some("[]".to_string()),
                score: Some(0.7),
                source_kind: "merged".to_string(),
                start_offset_ms: 0,
                state: "active".to_string(),
            },
        )
        .expect("insert activity interval");
    }

    for chunk_id in [&failed_chunk_id, &succeeded_chunk_id] {
        create_ocr_region(
            &connection,
            &CreateVisualRegionInput {
                chunk_id: chunk_id.clone(),
                confidence: Some(0.9),
                frame_timestamp_ms: 500,
                id: Uuid::now_v7().to_string(),
                job_id: job_id.clone(),
                model: Some("tesseract".to_string()),
                observation_kind: None,
                prompt_version: None,
                provider: "tesseract".to_string(),
                recording_id: recording_id.clone(),
                region_height: 20,
                region_width: 30,
                region_x: 1,
                region_y: 2,
                source_frame_ref: "segment:one:offset:500:frame:0".to_string(),
                text: "Daydream".to_string(),
            },
        )
        .expect("insert OCR region");
        create_vlm_observation(
            &connection,
            &CreateVisualRegionInput {
                chunk_id: chunk_id.clone(),
                confidence: Some(0.8),
                frame_timestamp_ms: 500,
                id: Uuid::now_v7().to_string(),
                job_id: job_id.clone(),
                model: Some("vision".to_string()),
                observation_kind: Some("corrected-ocr".to_string()),
                prompt_version: Some("m2-test".to_string()),
                provider: "openai-compatible".to_string(),
                recording_id: recording_id.clone(),
                region_height: 20,
                region_width: 30,
                region_x: 1,
                region_y: 2,
                source_frame_ref: "segment:one:offset:500:frame:0".to_string(),
                text: "Daydream corrected".to_string(),
            },
        )
        .expect("insert VLM observation");
        create_frame_similarity_entry(
            &connection,
            &CreateFrameSimilarityEntryInput {
                chunk_id: Some(chunk_id.clone()),
                id: Uuid::now_v7().to_string(),
                job_id: job_id.clone(),
                last_processed_timestamp_ms: 500,
                recording_id: recording_id.clone(),
                region_height: 20,
                region_width: 30,
                region_x: 1,
                region_y: 2,
                signature: format!("signature-{chunk_id}"),
                source_frame_ref: "segment:one:offset:500:frame:0".to_string(),
            },
        )
        .expect("insert frame similarity entry");
    }

    let retried =
        retry_processing_job_chunks(&connection, &job_id, 99).expect("retry processing job");

    let queued_chunks = list_processing_chunks_by_status(&connection, &job_id, &["queued"], None)
        .expect("list queued chunks");
    let succeeded_chunks =
        list_processing_chunks_by_status(&connection, &job_id, &["succeeded"], None)
            .expect("list succeeded chunks");
    let count_by_chunk = |table_name: &str, chunk_id: &str| -> i64 {
        connection
            .query_row(
                &format!("SELECT COUNT(*) FROM {table_name} WHERE chunk_id = ?1"),
                [chunk_id],
                |row| row.get(0),
            )
            .expect("count rows by chunk")
    };

    assert_eq!(retried.status, "queued");
    assert_eq!(retried.error, None);
    assert_eq!(queued_chunks.len(), 2);
    assert!(queued_chunks
        .iter()
        .any(|chunk| chunk.id == failed_chunk_id && chunk.error_code.is_none()));
    assert!(queued_chunks
        .iter()
        .any(|chunk| chunk.id == cancelled_chunk_id && chunk.error_code.is_none()));
    assert_eq!(succeeded_chunks.len(), 1);
    assert_eq!(succeeded_chunks[0].id, succeeded_chunk_id);
    assert_eq!(count_by_chunk("transcript_segments", &failed_chunk_id), 0);
    assert_eq!(
        count_by_chunk("transcript_segments", &succeeded_chunk_id),
        1
    );
    assert_eq!(
        count_by_chunk("frame_activity_intervals", &cancelled_chunk_id),
        0
    );
    assert_eq!(
        count_by_chunk("frame_activity_intervals", &succeeded_chunk_id),
        1
    );
    assert_eq!(count_by_chunk("ocr_regions", &failed_chunk_id), 0);
    assert_eq!(count_by_chunk("ocr_regions", &succeeded_chunk_id), 1);
    assert_eq!(count_by_chunk("vlm_observations", &failed_chunk_id), 0);
    assert_eq!(count_by_chunk("vlm_observations", &succeeded_chunk_id), 1);
    assert_eq!(
        count_by_chunk("frame_similarity_entries", &failed_chunk_id),
        0
    );
    assert_eq!(
        count_by_chunk("frame_similarity_entries", &succeeded_chunk_id),
        1
    );
}

#[test]
fn ai_storage_helpers_preserve_derived_output_provenance() {
    let connection = migrated_memory_connection();
    let recording_id = create_test_recording(&connection);
    let segment_id = Uuid::now_v7().to_string();
    let track_id = Uuid::now_v7().to_string();
    let job_id = Uuid::now_v7().to_string();
    let chunk_id = Uuid::now_v7().to_string();

    connection
        .execute(
            r#"
            INSERT INTO recording_tracks (
                id,
                recording_id,
                kind,
                label,
                created_at_ms
            ) VALUES (?1, ?2, 'microphone', 'Microphone', 1)
            "#,
            (&track_id, &recording_id),
        )
        .expect("insert track");
    create_recording_segment(
        &connection,
        &CreateRecordingSegmentInput {
            file_path: "/tmp/daydream/0001.mkv".to_string(),
            id: segment_id.clone(),
            recording_id: recording_id.clone(),
            segment_index: 1,
            started_at_ms: 10,
        },
    )
    .expect("create segment");
    connection
        .execute(
            r#"
            INSERT INTO processing_jobs (
                id,
                recording_id,
                kind,
                status,
                created_at_ms,
                updated_at_ms
            ) VALUES (?1, ?2, 'ai-processing', 'running', 1, 1)
            "#,
            (&job_id, &recording_id),
        )
        .expect("insert job");

    save_ai_model_asset(
        &connection,
        &SaveAiModelAssetInput {
            checksum: Some("55356645c2b361a969dfd0ef2c5a50d530afd8d5".to_string()),
            checksum_algorithm: Some("sha1".to_string()),
            display_label: "Whisper small".to_string(),
            downloaded_bytes: 1,
            download_status: "available".to_string(),
            etag: Some("etag".to_string()),
            error_code: None,
            id: "small".to_string(),
            local_path: Some("/models/small.bin".to_string()),
            provider: "local-whisper-cpp".to_string(),
            self_test_status: "passed".to_string(),
            size_bytes: Some(1),
            source_url: Some(
                "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin"
                    .to_string(),
            ),
            timestamp_ms: 1,
            total_bytes: Some(1),
            verification_status: "passed".to_string(),
        },
    )
    .expect("save model asset");
    let chunk = create_processing_chunk(
        &connection,
        &CreateProcessingChunkInput {
            attempt_count: 1,
            created_at_ms: 1,
            end_offset_ms: Some(1_000),
            error_code: None,
            id: chunk_id.clone(),
            job_id: job_id.clone(),
            progress: 0.5,
            recording_id: recording_id.clone(),
            result_code: None,
            segment_id: Some(segment_id.clone()),
            source_kind: "audio".to_string(),
            start_offset_ms: Some(0),
            status: "running".to_string(),
            track_id: Some(track_id.clone()),
            updated_at_ms: 2,
        },
    )
    .expect("create chunk");
    let transcript = create_transcript_segment(
        &connection,
        &CreateTranscriptSegmentInput {
            chunk_id: chunk_id.clone(),
            confidence: Some(0.8),
            created_at_ms: 2,
            end_offset_ms: 900,
            id: Uuid::now_v7().to_string(),
            job_id: job_id.clone(),
            model: Some("small".to_string()),
            provider: "local-whisper-cpp".to_string(),
            recording_id: recording_id.clone(),
            segment_id: segment_id.clone(),
            source_role: "microphone".to_string(),
            start_offset_ms: 100,
            text: "hello".to_string(),
            track_id: track_id.clone(),
        },
    )
    .expect("create transcript segment");
    create_transcript_word(
        &connection,
        &CreateTranscriptWordInput {
            end_offset_ms: 200,
            id: Uuid::now_v7().to_string(),
            probability: Some(0.9),
            start_offset_ms: 100,
            text: "hello".to_string(),
            transcript_segment_id: transcript.id.clone(),
        },
    )
    .expect("create word");
    create_activity_interval(
        &connection,
        &CreateActivityIntervalInput {
            chunk_id: Some(chunk_id.clone()),
            end_offset_ms: 1_000,
            id: Uuid::now_v7().to_string(),
            job_id: job_id.clone(),
            recording_id: recording_id.clone(),
            regions_json: Some("[]".to_string()),
            score: Some(0.7),
            source_kind: "merged".to_string(),
            start_offset_ms: 0,
            state: "active".to_string(),
        },
    )
    .expect("create activity interval");
    create_ocr_region(
        &connection,
        &CreateVisualRegionInput {
            chunk_id: chunk_id.clone(),
            confidence: Some(0.95),
            frame_timestamp_ms: 500,
            id: Uuid::now_v7().to_string(),
            job_id: job_id.clone(),
            model: Some("tesseract".to_string()),
            observation_kind: None,
            prompt_version: None,
            provider: "tesseract".to_string(),
            recording_id: recording_id.clone(),
            region_height: 20,
            region_width: 100,
            region_x: 10,
            region_y: 20,
            source_frame_ref: "segment-1@500".to_string(),
            text: "Daydream".to_string(),
        },
    )
    .expect("create ocr region");
    create_vlm_observation(
        &connection,
        &CreateVisualRegionInput {
            chunk_id: chunk_id.clone(),
            confidence: None,
            frame_timestamp_ms: 500,
            id: Uuid::now_v7().to_string(),
            job_id: job_id.clone(),
            model: Some("vision".to_string()),
            observation_kind: Some("screen-observation".to_string()),
            prompt_version: Some("m2-test".to_string()),
            provider: "openai-compatible".to_string(),
            recording_id: recording_id.clone(),
            region_height: 20,
            region_width: 100,
            region_x: 10,
            region_y: 20,
            source_frame_ref: "segment-1@500".to_string(),
            text: "Editor window".to_string(),
        },
    )
    .expect("create vlm observation");
    create_frame_similarity_entry(
        &connection,
        &CreateFrameSimilarityEntryInput {
            chunk_id: Some(chunk_id.clone()),
            id: Uuid::now_v7().to_string(),
            job_id: job_id.clone(),
            last_processed_timestamp_ms: 500,
            recording_id: recording_id.clone(),
            region_height: 20,
            region_width: 100,
            region_x: 10,
            region_y: 20,
            signature: "phash".to_string(),
            source_frame_ref: "segment-1@500".to_string(),
        },
    )
    .expect("create frame similarity entry");

    assert_eq!(chunk.job_id, job_id);
    assert_eq!(chunk.recording_id, recording_id);
    assert_eq!(transcript.chunk_id, chunk_id);
    assert_eq!(transcript.job_id, job_id);
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
fn redactions_store_lane_scoped_track_metadata() {
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

    let redaction = create_redaction(
        &connection,
        &CreateRedactionInput {
            created_at_ms: 3,
            end_offset_ms: 2_000,
            id: Uuid::now_v7().to_string(),
            reason: Some("private".to_string()),
            recording_id: recording_id.clone(),
            start_offset_ms: 1_000,
            track_ids_json: r#"["desktop-audio-1","video-0"]"#.to_string(),
        },
    )
    .expect("create redaction");

    let redactions = list_redactions(&connection, &recording_id).expect("list redactions");

    assert_eq!(redactions, vec![redaction]);
    assert_eq!(
        redactions[0].track_ids_json,
        r#"["desktop-audio-1","video-0"]"#
    );
}

#[test]
fn deleting_redactions_is_scoped_to_recording() {
    let connection = migrated_memory_connection();
    let recording_id = Uuid::now_v7().to_string();
    let other_recording_id = Uuid::now_v7().to_string();
    let redaction_id = Uuid::now_v7().to_string();

    for id in [&recording_id, &other_recording_id] {
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
                [id],
            )
            .expect("insert recording");
    }
    create_redaction(
        &connection,
        &CreateRedactionInput {
            created_at_ms: 3,
            end_offset_ms: 2_000,
            id: redaction_id.clone(),
            reason: None,
            recording_id: recording_id.clone(),
            start_offset_ms: 1_000,
            track_ids_json: r#"["video-0"]"#.to_string(),
        },
    )
    .expect("create redaction");

    assert!(
        !delete_redaction(&connection, &other_recording_id, &redaction_id)
            .expect("delete other recording redaction")
    );
    assert_eq!(
        list_redactions(&connection, &recording_id)
            .expect("list redactions")
            .len(),
        1
    );
    assert!(delete_redaction(&connection, &recording_id, &redaction_id).expect("delete redaction"));
    assert!(list_redactions(&connection, &recording_id)
        .expect("list redactions")
        .is_empty());
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

    assert_eq!(schema_version, 39);
    assert_eq!(table_counts["recordings"], 1);
    assert_eq!(migrated.quality_label, "ocr");
    assert_eq!(migrated.active_scene_id, None);
    assert_eq!(migrated.output_root, None);
    assert_eq!(migrated.title, None);
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
    assert_eq!(recording.title, None);

    connection
        .execute("DELETE FROM recorder_scenes WHERE id = ?1", [&scene_id])
        .expect("delete scene");

    let recording = get_recording(&connection, &recording_id).expect("read recording");
    assert_eq!(recording.active_scene_id, None);
}

#[test]
fn recording_title_can_be_renamed_and_cleared_after_completion() {
    let connection = migrated_memory_connection();
    let recording_id = Uuid::now_v7().to_string();

    create_recording(
        &connection,
        &CreateRecordingInput {
            id: recording_id.clone(),
            active_scene_id: None,
            created_at_ms: 2,
            default_fps: 1.0,
            notes: None,
            output_root: Some("/tmp/daydream/recordings".to_string()),
            quality_label: "balanced-ocr".to_string(),
            storage_dir: "/tmp/daydream".to_string(),
        },
    )
    .expect("create recording");
    mark_recording_started(&connection, &recording_id, 2).expect("mark started");
    mark_recording_completed(&connection, &recording_id, 10).expect("mark completed");

    let renamed =
        rename_recording(&connection, &recording_id, Some("Workday review")).expect("rename");
    assert_eq!(renamed.title.as_deref(), Some("Workday review"));

    let cleared = rename_recording(&connection, &recording_id, None).expect("clear title");
    assert_eq!(cleared.title, None);
}

#[test]
fn recording_title_rename_rejects_active_recordings() {
    let connection = migrated_memory_connection();
    let recording_id = Uuid::now_v7().to_string();

    create_recording(
        &connection,
        &CreateRecordingInput {
            id: recording_id.clone(),
            active_scene_id: None,
            created_at_ms: 2,
            default_fps: 1.0,
            notes: None,
            output_root: Some("/tmp/daydream/recordings".to_string()),
            quality_label: "balanced-ocr".to_string(),
            storage_dir: "/tmp/daydream".to_string(),
        },
    )
    .expect("create recording");
    mark_recording_started(&connection, &recording_id, 2).expect("mark started");

    let error = rename_recording(&connection, &recording_id, Some("Active"))
        .expect_err("active recording rename should fail");

    assert!(error.to_string().contains("not renameable"));
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
        connection: Arc::new(Mutex::new(connection)),
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

#[test]
fn processing_resilience_migration_preserves_populated_job_children() {
    for historical_column_order in [false, true] {
        let mut connection = Connection::open_in_memory().unwrap();
        enable_connection_pragmas(&connection).unwrap();
        migrations().to_version(&mut connection, 20).unwrap();
        if historical_column_order {
            // Early databases received result_code after timestamps and track_id.
            connection.execute_batch("ALTER TABLE processing_chunks DROP COLUMN result_code; ALTER TABLE processing_chunks ADD COLUMN result_code TEXT;").unwrap();
        }
        let recording_id = create_test_recording(&connection);
        let job_id = Uuid::now_v7().to_string();
        let chunk_id = Uuid::now_v7().to_string();
        super::create_processing_job(
            &connection,
            &super::CreateProcessingJobInput {
                id: job_id.clone(),
                recording_id: recording_id.clone(),
                kind: "ai-processing".into(),
                status: "running".into(),
                created_at_ms: 1,
                updated_at_ms: 1,
                error: None,
            },
        )
        .unwrap();
        create_processing_chunk(
            &connection,
            &CreateProcessingChunkInput {
                id: chunk_id.clone(),
                job_id: job_id.clone(),
                recording_id: recording_id.clone(),
                source_kind: "activity".into(),
                segment_id: None,
                track_id: None,
                start_offset_ms: Some(0),
                end_offset_ms: Some(1_000),
                status: "succeeded".into(),
                attempt_count: 1,
                progress: 1.0,
                error_code: None,
                result_code: None,
                created_at_ms: 1,
                updated_at_ms: 1,
            },
        )
        .unwrap();
        super::save_processing_job_ocr_config(&connection, &job_id, "{}").unwrap();
        apply_migrations(&mut connection).unwrap();
        assert!(super::get_processing_job(&connection, &job_id)
            .unwrap()
            .is_some());
        let migrated_chunk = super::get_processing_chunk(&connection, &chunk_id)
            .unwrap()
            .unwrap();
        assert_eq!(migrated_chunk.created_at_ms, 1);
        assert_eq!(migrated_chunk.updated_at_ms, 1);
        assert_eq!(migrated_chunk.track_id, None);
        assert_eq!(
            super::get_processing_job_ocr_config(&connection, &job_id)
                .unwrap()
                .as_deref(),
            Some("{}")
        );
        super::update_processing_job_status(
            &connection,
            &job_id,
            "completed-with-errors",
            Some("ai_transcription_vad_model_missing"),
            2,
        )
        .unwrap();
        super::update_processing_chunk_status(
            &connection,
            &chunk_id,
            "completed-with-errors",
            1.0,
            Some("ai_transcription_vad_model_missing"),
            2,
        )
        .unwrap();
        assert!(connection
            .prepare("PRAGMA foreign_key_check")
            .unwrap()
            .query([])
            .unwrap()
            .next()
            .unwrap()
            .is_none());
        assert_eq!(
            connection
                .query_row("PRAGMA foreign_keys", [], |row| row.get::<_, i64>(0))
                .unwrap(),
            1
        );
    }
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
            CREATE TABLE recordings (
                id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
                active_scene_id TEXT CHECK (active_scene_id IS NULL OR length(active_scene_id) = 36),
                created_at_ms INTEGER NOT NULL CHECK (created_at_ms >= 0),
                started_at_ms INTEGER CHECK (started_at_ms IS NULL OR started_at_ms >= 0),
                ended_at_ms INTEGER CHECK (ended_at_ms IS NULL OR ended_at_ms >= 0),
                status TEXT NOT NULL CHECK (status IN ('planned', 'recording', 'paused', 'completed', 'failed', 'deleted')),
                default_fps REAL NOT NULL CHECK (default_fps > 0),
                quality_label TEXT NOT NULL CHECK (quality_label IN ('draft', 'standard', 'high', 'ocr', 'balanced-ocr')),
                storage_dir TEXT NOT NULL,
                output_root TEXT,
                primary_video_path TEXT,
                notes TEXT
            );

            CREATE TABLE recording_tracks (
                id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
                recording_id TEXT NOT NULL CHECK (length(recording_id) = 36),
                kind TEXT NOT NULL CHECK (kind IN ('screen', 'desktop-audio', 'microphone')),
                label TEXT NOT NULL,
                source_id TEXT,
                path TEXT,
                codec TEXT,
                created_at_ms INTEGER NOT NULL CHECK (created_at_ms >= 0),
                FOREIGN KEY (recording_id) REFERENCES recordings(id) ON DELETE CASCADE
            );

            CREATE TABLE redactions (
                id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
                recording_id TEXT NOT NULL CHECK (length(recording_id) = 36),
                start_offset_ms INTEGER NOT NULL CHECK (start_offset_ms >= 0),
                end_offset_ms INTEGER NOT NULL CHECK (end_offset_ms > start_offset_ms),
                reason TEXT,
                created_at_ms INTEGER NOT NULL CHECK (created_at_ms >= 0),
                FOREIGN KEY (recording_id) REFERENCES recordings(id) ON DELETE CASCADE
            );

            CREATE TABLE capture_events (
                id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
                recording_id TEXT NOT NULL CHECK (length(recording_id) = 36),
                occurred_at_ms INTEGER NOT NULL CHECK (occurred_at_ms >= 0),
                offset_ms INTEGER NOT NULL CHECK (offset_ms >= 0),
                kind TEXT NOT NULL CHECK (kind IN ('keyboard-activity', 'mouse-activity', 'focused-window', 'open-windows', 'capture-state')),
                payload_json TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(payload_json)),
                FOREIGN KEY (recording_id) REFERENCES recordings(id) ON DELETE CASCADE
            );

            CREATE TABLE processing_jobs (
                id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
                recording_id TEXT NOT NULL CHECK (length(recording_id) = 36),
                kind TEXT NOT NULL CHECK (kind IN ('chunking', 'frame-extraction', 'ocr', 'transcription', 'vlm', 'embedding', 'digest')),
                status TEXT NOT NULL CHECK (status IN ('queued', 'running', 'succeeded', 'failed', 'cancelled')),
                created_at_ms INTEGER NOT NULL CHECK (created_at_ms >= 0),
                updated_at_ms INTEGER NOT NULL CHECK (updated_at_ms >= created_at_ms),
                error TEXT,
                FOREIGN KEY (recording_id) REFERENCES recordings(id) ON DELETE CASCADE
            );

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

            CREATE TABLE recording_segments (
                id TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
                recording_id TEXT NOT NULL CHECK (length(recording_id) = 36),
                segment_index INTEGER NOT NULL CHECK (segment_index > 0),
                file_path TEXT NOT NULL,
                started_at_ms INTEGER NOT NULL CHECK (started_at_ms >= 0),
                ended_at_ms INTEGER CHECK (ended_at_ms IS NULL OR ended_at_ms >= started_at_ms),
                duration_ms INTEGER CHECK (duration_ms IS NULL OR duration_ms >= 0),
                state TEXT NOT NULL CHECK (state IN ('recording', 'finalized', 'failed')),
                video_stream_index INTEGER CHECK (video_stream_index IS NULL OR video_stream_index >= 0),
                desktop_audio_stream_index INTEGER CHECK (desktop_audio_stream_index IS NULL OR desktop_audio_stream_index >= 0),
                microphone_audio_stream_index INTEGER CHECK (microphone_audio_stream_index IS NULL OR microphone_audio_stream_index >= 0),
                ffmpeg_exit_code INTEGER,
                error_code TEXT,
                created_at_ms INTEGER NOT NULL CHECK (created_at_ms >= 0),
                updated_at_ms INTEGER NOT NULL CHECK (updated_at_ms >= created_at_ms),
                UNIQUE (recording_id, segment_index),
                FOREIGN KEY (recording_id) REFERENCES recordings(id) ON DELETE CASCADE
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
