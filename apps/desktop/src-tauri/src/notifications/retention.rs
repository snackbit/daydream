use crate::storage::{self, NotificationRow};

const NOTIFICATION_RETENTION_MS: i64 = 7 * 24 * 60 * 60 * 1_000;

pub(super) fn list_retained_notifications(
    connection: &rusqlite::Connection,
    timestamp_ms: i64,
) -> rusqlite::Result<Vec<NotificationRow>> {
    storage::delete_notifications_older_than(connection, timestamp_ms - NOTIFICATION_RETENTION_MS)?;
    storage::list_notifications(connection)
}
