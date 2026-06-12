use tauri::State;

use crate::storage::{self, AppStorage};

use super::{
    actions::execute_stored_action,
    mapping::{notification_list_from_rows, parse_actions, storage_error},
    retention::list_retained_notifications,
    time::now_ms,
    types::{
        AcknowledgeNotificationsInput, DeleteNotificationsInput, DismissNotificationInput,
        ExecuteNotificationActionInput, NotificationError, NotificationList,
    },
};

pub(super) fn get_notifications(
    storage: State<'_, AppStorage>,
) -> Result<NotificationList, NotificationError> {
    storage
        .with_connection(|connection| list_retained_notifications(connection, now_ms()))
        .map_err(storage_error)
        .and_then(notification_list_from_rows)
}

pub(super) fn acknowledge_notifications(
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

pub(super) fn dismiss_notification(
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

pub(super) fn delete_notifications(
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

pub(super) fn delete_all_notifications(
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

pub(super) fn execute_notification_action(
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
