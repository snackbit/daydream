//! Typed recall IPC; storage, provider and lifecycle policies live in services.
use super::types::*;
use crate::storage::AppStorage;
use tauri::{AppHandle, Manager};

async fn storage_command<T: Send + 'static>(
    app: AppHandle,
    work: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    let activity = crate::maintenance::runtime::admit(&app, "recall-command")?;
    tauri::async_runtime::spawn_blocking(move || {
        let _activity = activity;
        work()
    })
    .await
    .map_err(|_| "recall_worker_failed".to_string())?
}

#[tauri::command(rename = "listRecallDays")]
pub async fn list_recall_days(
    input: ListRecallDaysInput,
    app: AppHandle,
) -> Result<Vec<RecallDayCard>, String> {
    storage_command(app.clone(), move || {
        super::day::list_days(&app.state::<AppStorage>(), &input)
    })
    .await
}
#[tauri::command(rename = "getRecallDay")]
pub async fn get_recall_day(input: RecallDayInput, app: AppHandle) -> Result<RecallDay, String> {
    storage_command(app.clone(), move || {
        super::generation_service::get_day(&app.state::<AppStorage>(), &input)
    })
    .await
}
#[tauri::command(rename = "searchRecallEvidence")]
pub async fn search_recall_evidence(
    input: RecallSearchInput,
    app: AppHandle,
) -> Result<RecallSearchResult, String> {
    super::search_service::search_evidence(&app, &input).await
}
#[tauri::command(rename = "cancelRecallSearch")]
pub fn cancel_recall_search(app: AppHandle) -> Result<(), String> {
    super::search_service::cancel_search(&app)
}
#[tauri::command(rename = "resolveRecallEvidence")]
pub async fn resolve_recall_evidence(
    input: ResolveRecallEvidenceInput,
    app: AppHandle,
) -> Result<Option<RecallEvidence>, String> {
    storage_command(app.clone(), move || {
        super::evidence::resolve(&app.state::<AppStorage>(), &input)
    })
    .await
}
#[tauri::command(rename = "regenerateRecallDay")]
pub async fn regenerate_recall_day(
    input: RecallDayInput,
    app: AppHandle,
) -> Result<RecallJob, String> {
    storage_command(app.clone(), move || {
        super::mutations::regenerate(&app, &input)
    })
    .await
}
#[tauri::command(rename = "setRecallFeedback")]
pub async fn set_recall_feedback(
    input: SetRecallFeedbackInput,
    app: AppHandle,
) -> Result<RecallDay, String> {
    storage_command(app.clone(), move || super::mutations::feedback(&app, input)).await
}
#[tauri::command(rename = "setRecallSessionCorrection")]
pub async fn set_recall_session_correction(
    input: SetRecallSessionCorrectionInput,
    app: AppHandle,
) -> Result<RecallDay, String> {
    storage_command(app.clone(), move || {
        super::mutations::correction(&app, input)
    })
    .await
}
#[tauri::command(rename = "acceptRecallSuggestion")]
pub async fn accept_recall_suggestion(
    input: AcceptRecallSuggestionInput,
    app: AppHandle,
) -> Result<RecallTask, String> {
    storage_command(app.clone(), move || super::mutations::accept(&app, &input)).await
}
#[tauri::command(rename = "dismissRecallSuggestion")]
pub async fn dismiss_recall_suggestion(
    input: DismissRecallSuggestionInput,
    app: AppHandle,
) -> Result<RecallDay, String> {
    storage_command(app.clone(), move || super::mutations::dismiss(&app, &input)).await
}
#[tauri::command(rename = "updateRecallTask")]
pub async fn update_recall_task(
    input: UpdateRecallTaskInput,
    app: AppHandle,
) -> Result<RecallTask, String> {
    storage_command(app.clone(), move || {
        super::mutations::update_task(&app, &input)
    })
    .await
}
#[tauri::command(rename = "setRecallDayReviewed")]
pub async fn set_recall_day_reviewed(
    input: SetRecallDayReviewedInput,
    app: AppHandle,
) -> Result<RecallDay, String> {
    storage_command(app.clone(), move || super::mutations::reviewed(&app, input)).await
}
#[tauri::command(rename = "getRecallStatus")]
pub async fn get_recall_status(app: AppHandle) -> Result<RecallStatus, String> {
    storage_command(app.clone(), move || super::status::get_status(&app)).await
}
#[tauri::command(rename = "testRecallProvider")]
pub async fn test_recall_provider(
    input: TestRecallProviderInput,
    app: AppHandle,
) -> Result<RecallProviderTestResult, String> {
    let _activity = crate::maintenance::runtime::admit(&app, "recall-provider-test")?;
    super::status::test_provider(&app, &input).await
}
#[tauri::command(rename = "controlRecallJob")]
pub async fn control_recall_job(
    input: ControlRecallJobInput,
    app: AppHandle,
) -> Result<RecallJob, String> {
    storage_command(app.clone(), move || super::mutations::control(&app, &input)).await
}
