use uuid::Uuid;

use crate::storage::{self, NotificationRow, SaveNotificationInput};

use super::{mapping::to_sql_error, types::NotificationAction};

pub(super) fn execute_stored_action(
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

fn save_unavailable_notification(
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
