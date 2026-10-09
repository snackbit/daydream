use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackgroundTaskStage {
    pub kind: String,
    pub total: i64,
    pub completed: i64,
    pub failed: i64,
    pub skipped: i64,
    pub running: i64,
    pub queued: i64,
    pub progress: f64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackgroundTask {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub recording_id: Option<String>,
    pub model_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub day_id: Option<String>,
    pub status: String,
    pub phase: String,
    pub progress: Option<f64>,
    pub elapsed_ms: i64,
    pub eta_ms: Option<i64>,
    pub next_retry_at_ms: Option<i64>,
    pub input_open: bool,
    pub error_code: Option<String>,
    pub updated_at_ms: i64,
    pub stages: Vec<BackgroundTaskStage>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackgroundTaskUnit {
    pub id: String,
    pub chunk_id: String,
    pub source_kind: String,
    pub start_offset_ms: Option<i64>,
    pub end_offset_ms: Option<i64>,
    pub status: String,
    pub progress: f64,
    pub error_code: Option<String>,
    pub attempt_count: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackgroundTaskCursor {
    pub updated_at_ms: i64,
    pub id: String,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListBackgroundTasksInput {
    pub scope: Option<String>,
    pub recording_id: Option<String>,
    pub limit: Option<usize>,
    pub cursor: Option<BackgroundTaskCursor>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackgroundTaskPage {
    pub tasks: Vec<BackgroundTask>,
    pub next_cursor: Option<BackgroundTaskCursor>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetBackgroundTaskInput {
    pub task_id: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackgroundTaskDetail {
    pub task: BackgroundTask,
    pub units: Vec<BackgroundTaskUnit>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackgroundTasksChanged {
    pub recording_id: Option<String>,
    pub updated_at_ms: i64,
}
