mod eta;
mod group_actions;
mod group_activity;
mod group_candidates;
pub(crate) mod group_commands;
mod group_details;
#[cfg(test)]
mod group_test_fixtures;
mod group_types;
mod groups;
pub(crate) mod operations;
mod projection;
mod types;
pub(crate) use group_types::*;
use tauri::{AppHandle, Emitter, Manager, Runtime};
pub(crate) use types::*;

#[tauri::command(rename = "listBackgroundTasks")]
pub async fn list_background_tasks(
    input: Option<ListBackgroundTasksInput>,
    app: AppHandle,
) -> Result<BackgroundTaskPage, crate::ai::AiError> {
    tauri::async_runtime::spawn_blocking(move || {
        projection::list_background_tasks(input, &app.state::<crate::storage::AppStorage>())
    })
    .await
    .map_err(|_| crate::ai::AiError {
        code: "ai_processing_task_worker_failed".into(),
        message: "Background task state could not be loaded.".into(),
    })?
}

#[tauri::command(rename = "getBackgroundTask")]
pub async fn get_background_task(
    input: GetBackgroundTaskInput,
    app: AppHandle,
) -> Result<BackgroundTaskDetail, crate::ai::AiError> {
    tauri::async_runtime::spawn_blocking(move || {
        projection::get_background_task(input, &app.state::<crate::storage::AppStorage>())
    })
    .await
    .map_err(|_| crate::ai::AiError {
        code: "ai_processing_task_worker_failed".into(),
        message: "Background task details could not be loaded.".into(),
    })?
}

pub(crate) fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(i64::MAX)
}

pub(crate) fn emit_changed<R: Runtime>(app: &tauri::AppHandle<R>, recording_id: Option<&str>) {
    let _ = app.emit(
        "background-tasks-changed",
        BackgroundTasksChanged {
            source_kind: None,
            recording_id: recording_id.map(str::to_owned),
            updated_at_ms: now_ms(),
        },
    );
}
