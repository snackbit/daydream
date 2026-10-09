//! Recording actions resolve current owners and preserve each job's result.
use super::*;
use crate::{
    ai::AiError,
    storage::{self, AppStorage},
};
use std::collections::BTreeSet;

pub(super) fn apply(
    input: &ControlRecordingProcessingGroupsInput,
    storage: &AppStorage,
    cancelled: impl Fn() -> bool,
) -> Result<RecordingProcessingActionResult, AiError> {
    if !["pause", "resume", "cancel", "retry"].contains(&input.action.as_str())
        || (input.source_kind.is_some() && input.action != "retry")
    {
        return Err(groups::failure());
    }
    if let Some(kind) = &input.source_kind {
        groups::validate_lane(kind)?;
    }
    let kinds = input
        .source_kind
        .as_deref()
        .map_or_else(|| groups::LANES.to_vec(), |kind| vec![kind]);
    let ids = storage
        .with_connection(|connection| {
            let mut ids = BTreeSet::new();
            for kind in kinds {
                ids.extend(groups::lane_job_ids(connection, &input.recording_id, kind)?);
            }
            Ok(ids)
        })
        .map_err(|_| groups::failure())?;
    let mut results = Vec::new();
    for id in ids {
        let current = storage
            .with_connection(|connection| storage::get_processing_job(connection, &id))
            .map_err(|_| groups::failure())?;
        let Some(current) = current else { continue };
        let eligible = match input.action.as_str() {
            "pause" => matches!(current.status.as_str(), "queued" | "running"),
            "resume" => current.status == "paused",
            "cancel" => matches!(current.status.as_str(), "queued" | "running" | "paused"),
            _ => true,
        };
        if !eligible {
            continue;
        }
        let result = if cancelled() {
            Err(AiError {
                code: "ai_maintenance_active".into(),
                message: "Processing actions stopped for maintenance. Refresh after it finishes."
                    .into(),
            })
        } else if input.action == "retry" {
            crate::ai::retry_processing_units_for_action(
                storage,
                &crate::ai::RetryProcessingUnitsInput {
                    job_id: id.clone(),
                    chunk_id: None,
                    unit_id: None,
                    source_kind: input.source_kind.clone(),
                    use_current_settings: input.use_current_settings,
                },
            )
        } else {
            crate::ai::processing::control_job(storage, &id, &input.action)
        };
        results.push(RecordingProcessingJobActionResult {
            job_id: id,
            succeeded: result.is_ok(),
            error_code: result.err().map(|error| error.code),
        });
    }
    Ok(RecordingProcessingActionResult { results })
}

#[cfg(test)]
#[path = "group_action_tests.rs"]
mod tests;
