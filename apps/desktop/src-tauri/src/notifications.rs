mod actions;
mod commands;
mod mapping;
mod retention;
#[cfg(test)]
mod tests;
mod time;
mod types;

use tauri::State;

use crate::storage::AppStorage;

pub use mapping::notification_from_row;
pub use types::{
    AcknowledgeNotificationsInput, DeleteNotificationsInput, DismissNotificationInput,
    ExecuteNotificationActionInput, Notification, NotificationAction, NotificationError,
    NotificationList,
};

#[tauri::command(rename = "getNotifications")]
pub fn get_notifications(
    storage: State<'_, AppStorage>,
) -> Result<NotificationList, NotificationError> {
    commands::get_notifications(storage)
}

#[tauri::command(rename = "acknowledgeNotifications")]
pub fn acknowledge_notifications(
    input: AcknowledgeNotificationsInput,
    storage: State<'_, AppStorage>,
) -> Result<NotificationList, NotificationError> {
    commands::acknowledge_notifications(input, storage)
}

#[tauri::command(rename = "dismissNotification")]
pub fn dismiss_notification(
    input: DismissNotificationInput,
    storage: State<'_, AppStorage>,
) -> Result<NotificationList, NotificationError> {
    commands::dismiss_notification(input, storage)
}

#[tauri::command(rename = "deleteNotifications")]
pub fn delete_notifications(
    input: DeleteNotificationsInput,
    storage: State<'_, AppStorage>,
) -> Result<NotificationList, NotificationError> {
    commands::delete_notifications(input, storage)
}

#[tauri::command(rename = "deleteAllNotifications")]
pub fn delete_all_notifications(
    storage: State<'_, AppStorage>,
) -> Result<NotificationList, NotificationError> {
    commands::delete_all_notifications(storage)
}

#[tauri::command(rename = "executeNotificationAction")]
pub fn execute_notification_action(
    input: ExecuteNotificationActionInput,
    app: tauri::AppHandle,
    storage: State<'_, AppStorage>,
) -> Result<NotificationList, NotificationError> {
    let _activity =
        crate::maintenance::runtime::admit(&app, "notification-action").map_err(|message| {
            NotificationError {
                code: "notification_maintenance_active".into(),
                message,
            }
        })?;
    commands::execute_notification_action(input, storage)
}
