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
        let mut page = groups::list(input.unwrap_or_default(), &app.state::<AppStorage>())?;
        annotate_activity(
            &app,
            page.groups.iter_mut().chain(&mut page.reconciled_groups),
        );
        Ok(page)
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
        let mut detail = group_details::get(input, &app.state::<AppStorage>())?;
        let owned = app.state::<crate::ai::ProcessingWorkerOwners>().snapshot();
        for activity in &mut detail.lane.activity {
            activity.worker_active = Some(owned.contains_key(&activity.job_id));
        }
        Ok(detail)
    })
    .await
    .map_err(|_| groups::failure())?
}

fn annotate_activity<'a>(
    app: &AppHandle,
    groups: impl Iterator<Item = &'a mut RecordingProcessingGroup>,
) {
    let owned = app.state::<crate::ai::ProcessingWorkerOwners>().snapshot();
    let settings = crate::settings::load_app_settings(&app.state::<AppStorage>()).ok();
    let capacity = settings
        .map(|settings| {
            if settings.ai.processing.resource_mode == "background" {
                1
            } else {
                settings.ai.processing.max_parallel_jobs
            }
        })
        .unwrap_or(1) as usize;
    for group in groups {
        for lane in &mut group.lanes {
            for activity in &mut lane.activity {
                let active = owned.contains_key(&activity.job_id);
                activity.worker_active = Some(active);
                let full = if lane.kind == "waveform" {
                    owned.values().any(|kind| kind == "waveform")
                } else {
                    owned
                        .values()
                        .filter(|kind| kind.as_str() != "waveform")
                        .count()
                        >= capacity
                };
                if !active && full && activity.waiting_reason.as_deref() == Some("unknown") {
                    activity.waiting_reason = Some("capacity".into());
                }
            }
        }
    }
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
