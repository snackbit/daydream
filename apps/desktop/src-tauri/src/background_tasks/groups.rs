//! Complete recording summaries. Pagination and filters operate on recordings.
use super::*;
use crate::{
    ai::AiError,
    settings,
    storage::{self, AppStorage},
};
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::BTreeSet;

pub(super) const LANES: [&str; 5] = ["audio", "ocr", "vlm", "activity", "waveform"];
pub(super) fn failure() -> AiError {
    AiError {
        code: "ai_processing_storage_failed".into(),
        message: "Processing activity could not be read. Retry to reconnect.".into(),
    }
}
pub(super) fn validate_lane(kind: &str) -> Result<(), AiError> {
    if LANES.contains(&kind) {
        Ok(())
    } else {
        Err(AiError {
            code: "ai_processing_selection_invalid".into(),
            message: "Choose a supported processing lane.".into(),
        })
    }
}

/// Newest run supplies headline outcomes; still-active older work remains visible.
pub(super) fn lane_job_ids(
    connection: &Connection,
    recording: &str,
    kind: &str,
) -> rusqlite::Result<Vec<String>> {
    let sql=format!("{} SELECT job_id FROM effective_lanes WHERE source_kind=?2 ORDER BY created_at_ms DESC,job_id DESC",group_candidates::EFFECTIVE_LANES);
    let mut query = connection.prepare(&sql)?;
    let ids = query
        .query_map(params![recording, kind], |row| row.get(0))?
        .collect();
    ids
}

pub(super) fn lane(
    connection: &Connection,
    recording: &str,
    kind: &str,
    settings: &settings::AppSettings,
) -> rusqlite::Result<RecordingProcessingLane> {
    let job_ids = lane_job_ids(connection, recording, kind)?;
    let mut lane = RecordingProcessingLane {
        kind: kind.into(),
        applicability: "available".into(),
        status: None,
        job_ids,
        total: 0,
        completed: 0,
        failed: 0,
        skipped: 0,
        cancelled: 0,
        running: 0,
        queued: 0,
        progress: None,
        input_open: false,
        error_code: None,
        next_retry_at_ms: None,
        activity: Vec::new(),
        eta_ms: None,
    };
    let mut progress_sum = 0.0;
    let mut statuses = Vec::new();
    for id in &lane.job_ids {
        let job = storage::get_processing_job(connection, id)?
            .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
        lane.input_open |= storage::processing_job_input_open(connection, id)?;
        let mut query = connection.prepare("SELECT COUNT(*),SUM(status='succeeded' AND COALESCE(result_code,'') NOT IN('skipped-silence','skipped-redaction')),SUM(status IN('failed','completed-with-errors')),SUM(status='succeeded' AND COALESCE(result_code,'') IN('skipped-silence','skipped-redaction')),SUM(status='cancelled'),SUM(status='running'),SUM(status='queued'),SUM(progress) FROM processing_chunks WHERE job_id=?1 AND source_kind=?2")?;
        let (total, completed, failed, skipped, cancelled, running, queued, progress): (
            i64,
            i64,
            i64,
            i64,
            i64,
            i64,
            i64,
            f64,
        ) = query.query_row(params![id, kind], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
                row.get(7)?,
            ))
        })?;
        lane.total += total;
        lane.completed += completed;
        lane.failed += failed;
        lane.skipped += skipped;
        lane.cancelled += cancelled;
        lane.running += running;
        lane.queued += queued;
        progress_sum += progress;
        // The parent may still be running another lane. This lane has its own state.
        statuses.push(
            if job.status == "paused" {
                "paused"
            } else if job.status == "failed" && failed + cancelled + running + queued > 0 {
                "failed"
            } else if running > 0 {
                "running"
            } else if queued > 0 {
                "queued"
            } else if failed > 0 || job.status == "completed-with-errors" && cancelled > 0 {
                "completed-with-errors"
            } else if cancelled > 0 {
                "cancelled"
            } else {
                "succeeded"
            }
            .to_string(),
        );
        let error: Option<String> = connection.query_row("SELECT error_code FROM processing_chunks WHERE job_id=?1 AND source_kind=?2 AND error_code IS NOT NULL ORDER BY updated_at_ms DESC,id DESC LIMIT 1", params![id,kind], |row| row.get(0)).optional()?.flatten();
        if lane.error_code.is_none() {
            let stopped = job.status == "failed" && failed + cancelled + running + queued > 0;
            lane.error_code = if stopped && error.as_deref() == Some("ai_processing_cancelled") {
                job.error.clone().or(error)
            } else {
                error.or_else(|| stopped.then(|| job.error.clone()).flatten())
            };
        }
        let next: Option<i64> = connection.query_row("SELECT MIN(r.next_retry_at_ms) FROM processing_chunk_runtime r JOIN processing_chunks c ON c.id=r.chunk_id WHERE c.job_id=?1 AND c.source_kind=?2 AND c.status IN('running','queued')",params![id,kind],|row|row.get(0))?;
        if let Some(next) = next {
            lane.next_retry_at_ms = Some(lane.next_retry_at_ms.map_or(next, |old| old.min(next)));
        }
    }
    if lane.total > 0 {
        lane.progress = Some((progress_sum / lane.total as f64).clamp(0.0, 1.0));
        lane.status = Some(combined_status(&statuses, false));
    }
    if !crate::recording_privacy::protection_ready(connection, recording)? {
        lane.applicability = "protected".into();
    } else if lane.total == 0 {
        lane.applicability = crate::ai::processing::applicability::source_applicability(
            connection, recording, kind, settings,
        )?;
    }
    lane.activity = super::group_activity::read(connection, &lane)?;
    if !lane.input_open
        && !matches!(
            lane.status.as_deref(),
            Some("paused" | "failed" | "cancelled")
        )
    {
        lane.eta_ms = super::eta::lane(connection, &lane.job_ids, kind, super::now_ms())?
            .filter(|eta| *eta > 0);
    }
    Ok(lane)
}

fn combined_status(statuses: &[String], input_open: bool) -> String {
    for status in [
        "running",
        "queued",
        "paused",
        "failed",
        "completed-with-errors",
        "cancelled",
    ] {
        if statuses.iter().any(|value| value == status) {
            return status.into();
        }
    }
    if input_open { "running" } else { "succeeded" }.into()
}

pub(super) fn group(
    connection: &Connection,
    recording: &str,
    settings: &settings::AppSettings,
) -> rusqlite::Result<RecordingProcessingGroup> {
    let row = storage::get_recording(connection, recording)?;
    let lanes = LANES
        .into_iter()
        .map(|kind| lane(connection, recording, kind, settings))
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let job_ids = lanes
        .iter()
        .flat_map(|lane| lane.job_ids.iter().cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let input_open = lanes.iter().any(|lane| lane.input_open);
    let mut updated = row.created_at_ms;
    let mut etas = Vec::new();
    let mut all_etas_known = true;
    let mut statuses = lanes
        .iter()
        .filter_map(|lane| lane.status.clone())
        .collect::<Vec<_>>();
    // Readiness failures can have no chunks; retain them as actionable recording work.
    for job in storage::list_processing_jobs(connection, Some(recording))? {
        updated = updated.max(job.updated_at_ms);
        if job_ids.contains(&job.id) {
            if matches!(job.status.as_str(), "running" | "queued" | "paused") {
                let task = projection::task_for_job(connection, job)?;
                match task.eta_ms {
                    Some(eta) => etas.push(eta),
                    None => all_etas_known = false,
                }
            }
        } else if lanes.iter().all(|lane| lane.total == 0) {
            statuses.push(job.status);
        }
    }
    let status = combined_status(&statuses, input_open);
    let total: i64 = connection.query_row(
        "SELECT COUNT(*) FROM recall_evidence WHERE recording_id=?1",
        [recording],
        |row| row.get(0),
    )?;
    let space = crate::recall::embedding::configured_space(&settings.ai.recall.embedding).ok();
    let indexed = match &space { Some(space)=>connection.query_row("SELECT COUNT(*) FROM recall_index i JOIN recall_evidence e ON e.id=i.evidence_id AND e.revision=i.source_revision WHERE e.recording_id=?1 AND i.embedding_space=?2",params![recording,space.fingerprint],|row|row.get::<_,i64>(0))?, None=>0 };
    let index_coverage = RecordingIndexCoverage {
        applicability: if space.is_none() {
            "disabled"
        } else if total == 0 {
            "no-eligible-input"
        } else {
            "available"
        }
        .into(),
        indexed,
        total,
        pending: total.saturating_sub(indexed),
        status: None,
        error_code: None,
    };
    Ok(RecordingProcessingGroup {
        recording_id: recording.into(),
        title: row.title.unwrap_or_else(|| "Untitled recording".into()),
        started_at_ms: row.started_at_ms,
        updated_at_ms: updated,
        status,
        input_open,
        eta_ms: if !input_open && all_etas_known {
            etas.into_iter().max()
        } else {
            None
        },
        job_ids,
        lanes,
        index_coverage,
    })
}

pub(super) fn list(
    input: ListRecordingProcessingGroupsInput,
    storage: &AppStorage,
) -> Result<RecordingProcessingGroupPage, AiError> {
    let filter = input.filter.as_deref().unwrap_or("all");
    if !["all", "active", "attention", "history"].contains(&filter)
        || input.known_recording_ids.len() > 2000
        || input.recording_ids.len() > 2000
    {
        return Err(failure());
    }
    let settings = settings::load_app_settings(storage).map_err(|_| failure())?;
    let limit = input.limit.unwrap_or(25).clamp(1, 100);
    storage
        .with_connection(|connection| {
            let candidates = group_candidates::select(connection, &input, limit)?;
            let known_ids = group_candidates::known_ids(connection, &input)?;
            let projected_ids = candidates
                .ids
                .iter()
                .chain(&known_ids)
                .cloned()
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
            // Only requested cards revalidate files/rebuild eligible evidence.
            storage::refresh_recording_evidence(connection, &projected_ids)?;
            let projected = projected_ids
                .iter()
                .map(|id| Ok((id.clone(), group(connection, id, &settings)?)))
                .collect::<rusqlite::Result<std::collections::BTreeMap<_, _>>>()?;
            let groups = candidates
                .ids
                .iter()
                .filter_map(|id| projected.get(id).cloned())
                .collect();
            let reconciled_groups = known_ids
                .iter()
                .filter_map(|id| projected.get(id).cloned())
                .collect();
            Ok(RecordingProcessingGroupPage {
                groups,
                next_cursor: candidates.next_cursor,
                counts: candidates.counts,
                reconciled_groups,
            })
        })
        .map_err(|_| failure())
}

#[cfg(test)]
#[path = "group_tests.rs"]
mod tests;
