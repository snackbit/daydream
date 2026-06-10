use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

use crate::storage::{self, AppStorage, NotificationRow, SaveNotificationInput};

const NOTIFICATION_RETENTION_MS: i64 = 7 * 24 * 60 * 60 * 1_000;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NotificationAction {
    pub handler: String,
    pub id: String,
    pub label: String,
    pub payload: std::collections::BTreeMap<String, String>,
    pub style: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Notification {
    pub actions: Vec<NotificationAction>,
    pub created_at_ms: i64,
    pub dismissed_at_ms: Option<i64>,
    pub id: String,
    pub message: String,
    pub source: String,
    pub status: String,
    pub title: String,
    pub topic: String,
    pub updated_at_ms: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NotificationList {
    pub notifications: Vec<Notification>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DismissNotificationInput {
    pub notification_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AcknowledgeNotificationsInput {
    pub notification_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeleteNotificationsInput {
    pub notification_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExecuteNotificationActionInput {
    pub action_id: String,
    pub notification_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NotificationError {
    pub code: String,
    pub message: String,
}

#[tauri::command(rename = "getNotifications")]
pub fn get_notifications(
    storage: State<'_, AppStorage>,
) -> Result<NotificationList, NotificationError> {
    storage
        .with_connection(|connection| list_retained_notifications(connection, now_ms()))
        .map_err(storage_error)
        .and_then(notification_list_from_rows)
}

#[tauri::command(rename = "acknowledgeNotifications")]
pub fn acknowledge_notifications(
    input: AcknowledgeNotificationsInput,
    storage: State<'_, AppStorage>,
) -> Result<NotificationList, NotificationError> {
    storage
        .with_connection(|connection| {
            storage::acknowledge_notifications(connection, &input.notification_ids, now_ms())?;
            list_retained_notifications(connection, now_ms())
        })
        .map_err(storage_error)
        .and_then(notification_list_from_rows)
}

#[tauri::command(rename = "dismissNotification")]
pub fn dismiss_notification(
    input: DismissNotificationInput,
    storage: State<'_, AppStorage>,
) -> Result<NotificationList, NotificationError> {
    storage
        .with_connection(|connection| {
            storage::dismiss_notification(connection, &input.notification_id, now_ms())?;
            list_retained_notifications(connection, now_ms())
        })
        .map_err(storage_error)
        .and_then(notification_list_from_rows)
}

#[tauri::command(rename = "deleteNotifications")]
pub fn delete_notifications(
    input: DeleteNotificationsInput,
    storage: State<'_, AppStorage>,
) -> Result<NotificationList, NotificationError> {
    storage
        .with_connection(|connection| {
            storage::delete_notifications(connection, &input.notification_ids)?;
            list_retained_notifications(connection, now_ms())
        })
        .map_err(storage_error)
        .and_then(notification_list_from_rows)
}

#[tauri::command(rename = "deleteAllNotifications")]
pub fn delete_all_notifications(
    storage: State<'_, AppStorage>,
) -> Result<NotificationList, NotificationError> {
    storage
        .with_connection(|connection| {
            storage::delete_all_notifications(connection)?;
            list_retained_notifications(connection, now_ms())
        })
        .map_err(storage_error)
        .and_then(notification_list_from_rows)
}

#[tauri::command(rename = "executeNotificationAction")]
pub fn execute_notification_action(
    input: ExecuteNotificationActionInput,
    storage: State<'_, AppStorage>,
) -> Result<NotificationList, NotificationError> {
    storage
        .with_connection(|connection| {
            let notification = storage::get_notification(connection, &input.notification_id)?
                .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
            let actions = parse_actions(&notification.actions_json)?;
            let action = actions
                .into_iter()
                .find(|candidate| candidate.id == input.action_id)
                .ok_or(rusqlite::Error::QueryReturnedNoRows)?;

            execute_stored_action(connection, &action, now_ms())?;
            list_retained_notifications(connection, now_ms())
        })
        .map_err(storage_error)
        .and_then(notification_list_from_rows)
}

pub fn save_unavailable_notification(
    connection: &rusqlite::Connection,
    topic: &str,
    title: &str,
    message: &str,
    actions: &[NotificationAction],
    timestamp_ms: i64,
) -> rusqlite::Result<NotificationRow> {
    storage::save_notification(
        connection,
        &SaveNotificationInput {
            actions_json: serde_json::to_string(actions).map_err(to_sql_error)?,
            id: Uuid::now_v7().to_string(),
            message: message.to_string(),
            source: "ai".to_string(),
            status: "failed".to_string(),
            timestamp_ms,
            title: title.to_string(),
            topic: topic.to_string(),
        },
    )
}

pub fn notification_from_row(row: NotificationRow) -> Result<Notification, NotificationError> {
    Ok(Notification {
        actions: parse_actions(&row.actions_json).map_err(storage_error)?,
        created_at_ms: row.created_at_ms,
        dismissed_at_ms: row.dismissed_at_ms,
        id: row.id,
        message: row.message,
        source: row.source,
        status: row.status,
        title: row.title,
        topic: row.topic,
        updated_at_ms: row.updated_at_ms,
    })
}

fn execute_stored_action(
    connection: &rusqlite::Connection,
    action: &NotificationAction,
    timestamp_ms: i64,
) -> rusqlite::Result<()> {
    match action.handler.as_str() {
        "model-download-retry" => {
            let model_id = action
                .payload
                .get("modelId")
                .map_or("unknown", String::as_str);
            save_unavailable_notification(
                connection,
                "model-download",
                "Download unavailable",
                &format!("Model download is not implemented yet for {model_id}."),
                std::slice::from_ref(action),
                timestamp_ms,
            )?;
        }
        "model-download-cancel" => {
            let model_id = action
                .payload
                .get("modelId")
                .map_or("unknown", String::as_str);
            save_unavailable_notification(
                connection,
                "model-download",
                "Cancel unavailable",
                &format!("Model download cancellation is unavailable for {model_id}."),
                &[],
                timestamp_ms,
            )?;
        }
        "processing-job-cancel" => update_job_or_notify(
            connection,
            action,
            "cancelled",
            "Processing job cancelled",
            timestamp_ms,
        )?,
        "processing-job-pause" => update_job_or_notify(
            connection,
            action,
            "paused",
            "Processing job paused",
            timestamp_ms,
        )?,
        "processing-job-resume" => update_job_or_notify(
            connection,
            action,
            "queued",
            "Processing job queued",
            timestamp_ms,
        )?,
        "processing-job-retry" => retry_job_or_notify(connection, action, timestamp_ms)?,
        _ => return Err(rusqlite::Error::InvalidQuery),
    }

    Ok(())
}

fn list_retained_notifications(
    connection: &rusqlite::Connection,
    timestamp_ms: i64,
) -> rusqlite::Result<Vec<NotificationRow>> {
    storage::delete_notifications_older_than(connection, timestamp_ms - NOTIFICATION_RETENTION_MS)?;
    storage::list_notifications(connection)
}

fn update_job_or_notify(
    connection: &rusqlite::Connection,
    action: &NotificationAction,
    status: &str,
    title: &str,
    timestamp_ms: i64,
) -> rusqlite::Result<()> {
    let Some(job_id) = action.payload.get("jobId") else {
        save_unavailable_notification(
            connection,
            "processing-job",
            "Job action unavailable",
            "Notification action is missing its processing job reference.",
            &[],
            timestamp_ms,
        )?;
        return Ok(());
    };

    if storage::get_processing_job(connection, job_id)?.is_some() {
        storage::update_processing_job_status(connection, job_id, status, None, timestamp_ms)?;
        storage::save_notification(
            connection,
            &SaveNotificationInput {
                actions_json: "[]".to_string(),
                id: Uuid::now_v7().to_string(),
                message: format!("Processing job {job_id} is now {status}."),
                source: "ai".to_string(),
                status: "info".to_string(),
                timestamp_ms,
                title: title.to_string(),
                topic: "processing-job".to_string(),
            },
        )?;
    } else {
        save_unavailable_notification(
            connection,
            "processing-job",
            "Job action unavailable",
            "Processing job is not available anymore.",
            &[],
            timestamp_ms,
        )?;
    }

    Ok(())
}

fn retry_job_or_notify(
    connection: &rusqlite::Connection,
    action: &NotificationAction,
    timestamp_ms: i64,
) -> rusqlite::Result<()> {
    let Some(job_id) = action.payload.get("jobId") else {
        save_unavailable_notification(
            connection,
            "processing-job",
            "Job action unavailable",
            "Notification action is missing its processing job reference.",
            &[],
            timestamp_ms,
        )?;
        return Ok(());
    };

    if storage::get_processing_job(connection, job_id)?.is_some() {
        storage::retry_processing_job_chunks(connection, job_id, timestamp_ms)?;
        storage::save_notification(
            connection,
            &SaveNotificationInput {
                actions_json: "[]".to_string(),
                id: Uuid::now_v7().to_string(),
                message: format!("Processing job {job_id} is now queued."),
                source: "ai".to_string(),
                status: "info".to_string(),
                timestamp_ms,
                title: "Processing job queued for retry".to_string(),
                topic: "processing-job".to_string(),
            },
        )?;
    } else {
        save_unavailable_notification(
            connection,
            "processing-job",
            "Job action unavailable",
            "Processing job is not available anymore.",
            &[],
            timestamp_ms,
        )?;
    }

    Ok(())
}

fn notification_list_from_rows(
    rows: Vec<NotificationRow>,
) -> Result<NotificationList, NotificationError> {
    rows.into_iter()
        .map(notification_from_row)
        .collect::<Result<Vec<_>, _>>()
        .map(|notifications| NotificationList { notifications })
}

fn parse_actions(actions_json: &str) -> rusqlite::Result<Vec<NotificationAction>> {
    serde_json::from_str(actions_json).map_err(to_sql_error)
}

fn to_sql_error(error: serde_json::Error) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(Box::new(error))
}

fn storage_error(error: rusqlite::Error) -> NotificationError {
    NotificationError {
        code: "notification_storage_failed".to_string(),
        message: format!("Notification storage failed: {error}"),
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
