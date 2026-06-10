use std::collections::BTreeMap;

use uuid::Uuid;

use super::{now_ms, storage_error, AiError};
use crate::{
    notifications::{self, Notification, NotificationAction},
    storage::{self, AppStorage, NotificationRow, ProcessingJobRow, SaveNotificationInput},
};

pub(super) struct ProcessingNotificationInput<'a> {
    pub actions: &'a [NotificationAction],
    pub job_id: Option<&'a str>,
    pub message: &'a str,
    pub recording_id: &'a str,
    pub status: &'a str,
    pub timestamp_ms: i64,
    pub title: &'a str,
}

pub(super) struct ModelDownloadNotificationInput<'a> {
    pub actions: &'a [NotificationAction],
    pub message: &'a str,
    pub model_id: &'a str,
    pub preferred_notification_id: Option<&'a str>,
    pub status: &'a str,
    pub timestamp_ms: i64,
    pub title: &'a str,
}

pub(super) fn save_model_notification(
    connection: &rusqlite::Connection,
    status: &str,
    topic: &str,
    title: &str,
    message: &str,
    actions: &[NotificationAction],
    timestamp_ms: i64,
) -> rusqlite::Result<NotificationRow> {
    let actions_json =
        serde_json::to_string(actions).map_err(|_error| rusqlite::Error::InvalidQuery)?;
    storage::save_notification(
        connection,
        &SaveNotificationInput {
            actions_json,
            id: Uuid::now_v7().to_string(),
            message: message.to_string(),
            source: "ai".to_string(),
            status: status.to_string(),
            timestamp_ms,
            title: title.to_string(),
            topic: topic.to_string(),
        },
    )
}

pub(super) fn save_job_running_notification(
    storage: &AppStorage,
    job: &ProcessingJobRow,
) -> Result<(), AiError> {
    storage
        .with_connection(|connection| {
            save_processing_notification(
                connection,
                ProcessingNotificationInput {
                    actions: &processing_running_actions(&job.id),
                    job_id: Some(&job.id),
                    message: "Audio transcription is running for this recording.",
                    recording_id: &job.recording_id,
                    status: "running",
                    timestamp_ms: now_ms(),
                    title: "AI processing running",
                },
            )
            .map(|_row| ())
        })
        .map_err(storage_error)
}

pub(super) fn save_job_progress_notification(
    storage: &AppStorage,
    job_id: &str,
) -> Result<(), AiError> {
    storage
        .with_connection(|connection| {
            let job = storage::get_processing_job(connection, job_id)?
                .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
            let (total, succeeded, average) =
                storage::processing_job_chunk_progress(connection, job_id)?;
            let percent = if total == 0 {
                0
            } else {
                (average * 100.0).round() as i64
            };
            save_processing_notification(
                connection,
                ProcessingNotificationInput {
                    actions: &processing_running_actions(job_id),
                    job_id: Some(job_id),
                    message: &format!(
                        "Audio transcription is {percent}% complete ({succeeded}/{total} chunks)."
                    ),
                    recording_id: &job.recording_id,
                    status: "running",
                    timestamp_ms: now_ms(),
                    title: "AI processing running",
                },
            )
            .map(|_row| ())
        })
        .map_err(storage_error)
}

pub(super) fn save_processing_notification(
    connection: &rusqlite::Connection,
    input: ProcessingNotificationInput<'_>,
) -> rusqlite::Result<NotificationRow> {
    let actions_json =
        serde_json::to_string(input.actions).map_err(|_error| rusqlite::Error::InvalidQuery)?;
    let notification_id =
        find_processing_notification_id(connection, input.recording_id, input.job_id)?
            .unwrap_or_else(|| Uuid::now_v7().to_string());

    storage::save_unseen_notification(
        connection,
        &SaveNotificationInput {
            actions_json,
            id: notification_id,
            message: input.message.to_string(),
            source: "ai".to_string(),
            status: input.status.to_string(),
            timestamp_ms: input.timestamp_ms,
            title: input.title.to_string(),
            topic: "processing-job".to_string(),
        },
    )
}

fn find_processing_notification_id(
    connection: &rusqlite::Connection,
    recording_id: &str,
    job_id: Option<&str>,
) -> rusqlite::Result<Option<String>> {
    let mut fallback = None;
    for row in storage::list_notifications(connection)? {
        if row.source != "ai" || row.topic != "processing-job" {
            continue;
        }
        let actions = serde_json::from_str::<Vec<NotificationAction>>(&row.actions_json)
            .map_err(|_error| rusqlite::Error::InvalidQuery)?;
        let has_job = job_id.is_some_and(|job_id| {
            actions.iter().any(|action| {
                action
                    .payload
                    .get("jobId")
                    .is_some_and(|candidate| candidate == job_id)
            })
        });
        if has_job {
            return Ok(Some(row.id));
        }
        if job_id.is_none() && row.message.contains(recording_id) {
            fallback.get_or_insert(row.id);
        }
    }

    Ok(fallback)
}

pub(super) fn processing_running_actions(job_id: &str) -> Vec<NotificationAction> {
    vec![
        processing_action(
            "pause",
            "Pause",
            "processing-job-pause",
            job_id,
            "secondary",
        ),
        processing_action(
            "cancel",
            "Cancel",
            "processing-job-cancel",
            job_id,
            "destructive",
        ),
    ]
}

pub(super) fn processing_paused_actions(job_id: &str) -> Vec<NotificationAction> {
    vec![
        processing_action(
            "resume",
            "Resume",
            "processing-job-resume",
            job_id,
            "default",
        ),
        processing_action(
            "cancel",
            "Cancel",
            "processing-job-cancel",
            job_id,
            "destructive",
        ),
    ]
}

pub(super) fn processing_retry_actions(job_id: &str) -> Vec<NotificationAction> {
    vec![processing_action(
        "retry",
        "Retry",
        "processing-job-retry",
        job_id,
        "default",
    )]
}

pub(super) fn processing_failed_actions(job_id: &str) -> Vec<NotificationAction> {
    processing_retry_actions(job_id)
}

fn processing_action(
    id: &str,
    label: &str,
    handler: &str,
    job_id: &str,
    style: &str,
) -> NotificationAction {
    let mut payload = BTreeMap::new();
    payload.insert("jobId".to_string(), job_id.to_string());
    NotificationAction {
        handler: handler.to_string(),
        id: id.to_string(),
        label: label.to_string(),
        payload,
        style: style.to_string(),
    }
}

pub(super) fn save_model_download_notification(
    connection: &rusqlite::Connection,
    input: ModelDownloadNotificationInput<'_>,
) -> rusqlite::Result<NotificationRow> {
    let actions_json =
        serde_json::to_string(input.actions).map_err(|_error| rusqlite::Error::InvalidQuery)?;
    let notification_id = find_model_download_notification_id(
        connection,
        input.model_id,
        input.preferred_notification_id,
    )?
    .unwrap_or_else(|| Uuid::now_v7().to_string());

    storage::save_unseen_notification(
        connection,
        &SaveNotificationInput {
            actions_json,
            id: notification_id,
            message: input.message.to_string(),
            source: "ai".to_string(),
            status: input.status.to_string(),
            timestamp_ms: input.timestamp_ms,
            title: input.title.to_string(),
            topic: "model-download".to_string(),
        },
    )
}

fn find_model_download_notification_id(
    connection: &rusqlite::Connection,
    model_id: &str,
    preferred_notification_id: Option<&str>,
) -> rusqlite::Result<Option<String>> {
    if let Some(notification_id) = preferred_notification_id {
        if let Some(row) = storage::get_notification(connection, notification_id)? {
            if row.source == "ai" && row.topic == "model-download" {
                return Ok(Some(row.id));
            }
        }
    }

    let mut fallback = None;
    for row in storage::list_notifications(connection)? {
        if row.source != "ai" || row.topic != "model-download" {
            continue;
        }

        let actions = serde_json::from_str::<Vec<NotificationAction>>(&row.actions_json)
            .map_err(|_error| rusqlite::Error::InvalidQuery)?;
        let has_model = actions.iter().any(|action| {
            action
                .payload
                .get("modelId")
                .is_some_and(|candidate| candidate == model_id)
        });

        if !has_model {
            continue;
        }

        if row.status == "running" {
            return Ok(Some(row.id));
        }

        fallback.get_or_insert(row.id);
    }

    Ok(fallback)
}

pub(super) fn notification_from_row(row: NotificationRow) -> Result<Notification, AiError> {
    notifications::notification_from_row(row).map_err(|error| AiError {
        code: error.code,
        message: error.message,
    })
}

pub(super) fn model_download_retry_action(model_id: &str) -> NotificationAction {
    NotificationAction {
        handler: "model-download-retry".to_string(),
        id: "retry".to_string(),
        label: "Retry".to_string(),
        payload: BTreeMap::from([("modelId".to_string(), model_id.to_string())]),
        style: "secondary".to_string(),
    }
}

pub(super) fn model_download_cancel_action(model_id: &str) -> NotificationAction {
    NotificationAction {
        handler: "model-download-cancel".to_string(),
        id: "cancel".to_string(),
        label: "Cancel".to_string(),
        payload: BTreeMap::from([("modelId".to_string(), model_id.to_string())]),
        style: "destructive".to_string(),
    }
}

pub(super) fn status_to_notification_status(status: &str) -> &'static str {
    match status {
        "queued" => "info",
        "downloading" => "running",
        "cancelled" => "cancelled",
        "failed" => "failed",
        "available" => "succeeded",
        _ => "info",
    }
}
