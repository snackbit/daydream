use crate::storage::NotificationRow;

use super::types::{Notification, NotificationAction, NotificationError, NotificationList};

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

pub(super) fn notification_list_from_rows(
    rows: Vec<NotificationRow>,
) -> Result<NotificationList, NotificationError> {
    rows.into_iter()
        .map(notification_from_row)
        .collect::<Result<Vec<_>, _>>()
        .map(|notifications| NotificationList { notifications })
}

pub(super) fn parse_actions(actions_json: &str) -> rusqlite::Result<Vec<NotificationAction>> {
    serde_json::from_str(actions_json).map_err(to_sql_error)
}

pub(super) fn to_sql_error(error: serde_json::Error) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(Box::new(error))
}

pub(super) fn storage_error(error: rusqlite::Error) -> NotificationError {
    NotificationError {
        code: "notification_storage_failed".to_string(),
        message: format!("Notification storage failed: {error}"),
    }
}
