use super::*;
use crate::{ai::AiError, storage::AppStorage};

fn admission(
    app: &AppHandle,
    kind: &str,
) -> Result<Option<crate::maintenance::runtime::RuntimeActivityLease>, AiError> {
    crate::maintenance::runtime::admit(app, kind).map_err(|_| AiError {
        code: "ai_maintenance_active".into(),
        message: "Maintenance is in progress. Retry when it finishes.".into(),
    })
}
#[tauri::command(rename = "listRecordingProcessingGroups")]
pub async fn list_recording_processing_groups(
    app: AppHandle,
    input: Option<ListRecordingProcessingGroupsInput>,
) -> Result<RecordingProcessingGroupPage, AiError> {
    let lease = admission(&app, "processing-group-read")?;
    tauri::async_runtime::spawn_blocking(move || {
        let _lease = lease;
        groups::list(input.unwrap_or_default(), &app.state::<AppStorage>())
    })
    .await
    .map_err(|_| groups::failure())?
}
#[tauri::command(rename = "getRecordingProcessingLane")]
pub async fn get_recording_processing_lane(
    app: AppHandle,
    input: GetRecordingProcessingLaneInput,
) -> Result<RecordingProcessingLaneDetail, AiError> {
    let lease = admission(&app, "processing-lane-read")?;
    tauri::async_runtime::spawn_blocking(move || {
        let _lease = lease;
        group_details::get(input, &app.state::<AppStorage>())
    })
    .await
    .map_err(|_| groups::failure())?
}
#[tauri::command(rename = "controlRecordingProcessingGroups")]
pub async fn control_recording_processing_groups(
    app: AppHandle,
    input: ControlRecordingProcessingGroupsInput,
) -> Result<RecordingProcessingActionResult, AiError> {
    let lease = admission(&app, "processing-group-control")?;
    tauri::async_runtime::spawn_blocking(move || {
        let _lease = lease;
        let result = group_actions::apply(&input, &app.state::<AppStorage>(), || {
            _lease
                .as_ref()
                .is_some_and(|lease| lease.cancellation().is_cancelled())
        });
        // Studio and Processing reconcile through their established events, including partial success.
        crate::ai::emit_processing_state(&app, &input.recording_id);
        result
    })
    .await
    .map_err(|_| groups::failure())?
}
