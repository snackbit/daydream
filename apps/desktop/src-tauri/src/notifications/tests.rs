use uuid::Uuid;

use crate::storage::NotificationRow;

use super::{actions::execute_stored_action, notification_from_row, NotificationAction};

#[test]
fn notification_row_parses_action_descriptors() {
    let notification = notification_from_row(NotificationRow {
        actions_json: r#"[{"id":"retry","label":"Retry","handler":"model-download-retry","payload":{"modelId":"small"},"style":"secondary"}]"#.to_string(),
        created_at_ms: 1,
        dismissed_at_ms: None,
        id: Uuid::now_v7().to_string(),
        message: "message".to_string(),
        source: "ai".to_string(),
        status: "failed".to_string(),
        title: "title".to_string(),
        topic: "model-download".to_string(),
        updated_at_ms: 1,
    })
    .expect("parse notification");

    assert_eq!(notification.actions[0].handler, "model-download-retry");
    assert_eq!(
        notification.actions[0].payload.get("modelId"),
        Some(&"small".to_string())
    );
}

#[test]
fn processing_job_retry_action_requeues_terminal_chunks() {
    let storage = crate::storage::test_storage(std::path::PathBuf::new());
    let recording_id = Uuid::now_v7().to_string();
    let job_id = Uuid::now_v7().to_string();
    let chunk_id = Uuid::now_v7().to_string();

    storage
        .with_connection(|connection| {
            crate::storage::create_recording(
                connection,
                &crate::storage::CreateRecordingInput {
                    active_scene_id: None,
                    created_at_ms: 1,
                    default_fps: 1.0,
                    id: recording_id.clone(),
                    notes: None,
                    output_root: None,
                    quality_label: "balanced-ocr".to_string(),
                    storage_dir: "/tmp/daydream".to_string(),
                },
            )?;
            crate::storage::create_processing_job(
                connection,
                &crate::storage::CreateProcessingJobInput {
                    created_at_ms: 1,
                    error: Some("chunk failed".to_string()),
                    id: job_id.clone(),
                    kind: "ai-processing".to_string(),
                    recording_id: recording_id.clone(),
                    status: "failed".to_string(),
                    updated_at_ms: 2,
                },
            )?;
            crate::storage::create_processing_chunk(
                connection,
                &crate::storage::CreateProcessingChunkInput {
                    attempt_count: 1,
                    created_at_ms: 1,
                    end_offset_ms: Some(1_000),
                    error_code: Some("processing-failed".to_string()),
                    id: chunk_id.clone(),
                    job_id: job_id.clone(),
                    progress: 0.5,
                    recording_id: recording_id.clone(),
                    result_code: None,
                    segment_id: None,
                    source_kind: "activity".to_string(),
                    start_offset_ms: Some(0),
                    status: "failed".to_string(),
                    track_id: None,
                    updated_at_ms: 2,
                },
            )?;

            let mut payload = std::collections::BTreeMap::new();
            payload.insert("jobId".to_string(), job_id.clone());
            execute_stored_action(
                connection,
                &NotificationAction {
                    handler: "processing-job-retry".to_string(),
                    id: "retry".to_string(),
                    label: "Retry".to_string(),
                    payload,
                    style: "default".to_string(),
                },
                99,
            )?;

            let job = crate::storage::get_processing_job(connection, &job_id)?
                .expect("processing job should exist");
            let queued_chunks = crate::storage::list_processing_chunks_by_status(
                connection,
                &job_id,
                &["queued"],
                None,
            )?;
            let notifications = crate::storage::list_notifications(connection)?;

            assert_eq!(job.status, "queued");
            assert_eq!(job.error, None);
            assert_eq!(queued_chunks.len(), 1);
            assert_eq!(queued_chunks[0].id, chunk_id);
            assert_eq!(queued_chunks[0].error_code, None);
            assert!(notifications
                .iter()
                .any(|notification| notification.title == "Processing job queued for retry"));

            Ok(())
        })
        .expect("execute retry action");
}
