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
    let remote_only = app
        .state::<AppStorage>()
        .with_connection(|connection| {
            let mut ids = std::collections::BTreeSet::new();
            for id in owned.keys() {
                if crate::ai::processing::observations_only(connection, id)? {
                    ids.insert(id.clone());
                }
            }
            Ok(ids)
        })
        .unwrap_or_default();
    let remote_capacity = settings.as_ref().map_or(1, |settings| {
        settings.ai.processing.max_parallel_jobs.max(1)
    }) as usize;
    let provider = crate::recall::provider_config::build_embedding_provider(app);
    let index_error = crate::recall::status::embedding_unavailable_reason(app, &provider)
        .ok()
        .flatten();
    let index_job=app.state::<AppStorage>().with_connection(|connection| {
        use rusqlite::OptionalExtension;
        let snapshot=serde_json::to_string(&settings.as_ref().ok_or(rusqlite::Error::InvalidQuery)?.ai.recall).map_err(|_|rusqlite::Error::InvalidQuery)?;
        connection.query_row("SELECT status,error_code FROM recall_jobs WHERE kind='index' AND (status IN('paused','cancelled') OR (json_valid(config_json) AND json_extract(config_json,'$.embedding')=json_extract(?1,'$.embedding'))) ORDER BY created_at_ms DESC,id DESC LIMIT 1",[snapshot],|row|Ok((row.get::<_,String>(0)?,row.get::<_,Option<String>>(1)?))).optional()
    }).ok().flatten();
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
        if group.index_coverage.applicability == "available" {
            let coverage = &mut group.index_coverage;
            if let Some(error) = &index_error {
                coverage.status = Some("blocked".into());
                coverage.error_code = Some(error.clone());
            } else if coverage.pending == 0 {
                coverage.status = Some("ready".into());
            } else if let Some((status, error)) = &index_job {
                coverage.status = Some(
                    if status == "pending" {
                        "queued"
                    } else {
                        status
                    }
                    .into(),
                );
                if status == "failed" {
                    coverage.error_code = error.clone();
                }
            } else {
                coverage.status = Some("queued".into());
            }
        }
        for lane in &mut group.lanes {
            for activity in &mut lane.activity {
                let active = owned.contains_key(&activity.job_id);
                activity.worker_active = Some(active);
                let full = if lane.kind == "waveform" {
                    owned.values().any(|kind| kind == "waveform")
                } else if remote_only.contains(&activity.job_id) {
                    remote_only.len() >= remote_capacity
                } else {
                    owned
                        .iter()
                        .filter(|(id, kind)| {
                            kind.as_str() != "waveform" && !remote_only.contains(*id)
                        })
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
    if input.action == "optimize-visual" {
        if input.source_kind.is_some() {
            return Err(groups::failure());
        }
        let _lease = lease;
        let job =
            crate::ai::processing::visual_recovery::optimize(&app, &input.recording_id).await?;
        return Ok(RecordingProcessingActionResult {
            results: vec![RecordingProcessingJobActionResult {
                job_id: job.id,
                succeeded: true,
                error_code: None,
            }],
        });
    }
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
