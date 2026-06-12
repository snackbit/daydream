use serde::{Deserialize, Serialize};

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
