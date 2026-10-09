use super::*;
use crate::{
    ai::AiError,
    storage::{self, AppStorage, ProcessingJobRow},
};
use rusqlite::OptionalExtension;

fn storage_error(_: rusqlite::Error) -> AiError {
    AiError {
        code: "ai_storage_failed".into(),
        message: "Background task state could not be read.".into(),
    }
}

pub(crate) fn task_for_job(
    connection: &rusqlite::Connection,
    job: ProcessingJobRow,
) -> rusqlite::Result<BackgroundTask> {
    let mut statement=connection.prepare("SELECT source_kind,COUNT(*),SUM(status='succeeded'),SUM(status IN ('failed','completed-with-errors')),SUM(COALESCE(result_code,'') IN ('skipped-silence','skipped-redaction')),SUM(status='running'),SUM(status='queued'),AVG(progress) FROM processing_chunks WHERE job_id=?1 GROUP BY source_kind")?;
    let stages = statement
        .query_map([&job.id], |row| {
            Ok(BackgroundTaskStage {
                kind: row.get(0)?,
                total: row.get(1)?,
                completed: row.get(2)?,
                failed: row.get(3)?,
                skipped: row.get(4)?,
                running: row.get(5)?,
                queued: row.get(6)?,
                progress: row.get(7)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let total: i64 = stages.iter().map(|stage| stage.total).sum();
    let done: i64 = stages
        .iter()
        .map(|stage| stage.completed + stage.failed)
        .sum();
    let progress = if total == 0 {
        None
    } else {
        Some(
            stages
                .iter()
                .map(|stage| stage.progress * stage.total as f64)
                .sum::<f64>()
                / total as f64,
        )
    };
    let (started,next_retry):(Option<i64>,Option<i64>)=connection.query_row("SELECT MIN(r.started_at_ms),MIN(r.next_retry_at_ms) FROM processing_chunk_runtime r JOIN processing_chunks c ON c.id=r.chunk_id WHERE c.job_id=?1",[&job.id],|row|Ok((row.get(0)?,row.get(1)?)))?;
    let elapsed_ms = (if job.status == "running" {
        now_ms()
    } else {
        job.updated_at_ms
    } - started.unwrap_or(job.created_at_ms))
    .max(0);
    let eta_ms = progress
        .filter(|value| done >= 3 && *value > 0.0 && *value < 1.0 && job.status == "running")
        .map(|value| ((elapsed_ms as f64 * (1.0 - value) / value).ceil() as i64).max(0));
    let input_open = storage::processing_job_input_open(connection, &job.id)?;
    let eta_ms = if input_open { None } else { eta_ms };
    let phase:Option<String>=connection.query_row("SELECT r.phase FROM processing_chunks c LEFT JOIN processing_chunk_runtime r ON r.chunk_id=c.id WHERE c.job_id=?1 AND c.status IN ('running','queued') ORDER BY c.status='running' DESC,c.updated_at_ms DESC LIMIT 1",[&job.id],|row|row.get(0)).optional()?.flatten();
    let title: Option<String> = connection.query_row(
        "SELECT title FROM recordings WHERE id=?1",
        [&job.recording_id],
        |row| row.get(0),
    )?;
    Ok(BackgroundTask {
        id: job.id,
        kind: job.kind,
        title: title.unwrap_or_else(|| "Recording processing".into()),
        recording_id: Some(job.recording_id),
        model_id: None,
        day_id: None,
        status: job.status.clone(),
        phase: phase.unwrap_or_else(|| {
            if next_retry.is_some() {
                "retrying".into()
            } else if input_open && done == total {
                "waiting-for-recording".into()
            } else {
                job.status
            }
        }),
        progress,
        elapsed_ms,
        eta_ms,
        next_retry_at_ms: next_retry,
        input_open,
        error_code: job.error,
        updated_at_ms: job.updated_at_ms,
        stages,
    })
}

pub(super) fn list_background_tasks(
    input: Option<ListBackgroundTasksInput>,
    storage: &AppStorage,
) -> Result<BackgroundTaskPage, AiError> {
    let input = input.unwrap_or_default();
    let shared = input.scope.as_deref() == Some("shared");
    let limit = input.limit.unwrap_or(50).clamp(1, 200);
    let mut tasks=storage.with_connection(|connection| {
        let mut statement=connection.prepare("SELECT id FROM processing_jobs WHERE NOT ?5 AND (?1 IS NULL OR recording_id=?1) AND (?2 IS NULL OR (updated_at_ms,id)<(?2,?3)) ORDER BY updated_at_ms DESC,id DESC LIMIT ?4")?;
        let ids=statement.query_map(rusqlite::params![input.recording_id,input.cursor.as_ref().map(|cursor|cursor.updated_at_ms),input.cursor.as_ref().map(|cursor|&cursor.id),(limit+1) as i64,shared],|row|row.get::<_,String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
        let jobs=ids.into_iter().map(|id|storage::get_processing_job(connection,&id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)).collect::<rusqlite::Result<Vec<_>>>()?;
        let mut tasks=jobs.into_iter().map(|job|task_for_job(connection,job)).collect::<rusqlite::Result<Vec<_>>>()?;
        if input.recording_id.is_none() {
            let mut recall_query = connection.prepare("SELECT id FROM recall_jobs WHERE (NOT ?4 OR status IN('pending','running','paused','failed','completed-with-errors')) AND (?1 IS NULL OR (updated_at_ms,id)<(?1,?2)) ORDER BY updated_at_ms DESC,id DESC LIMIT ?3")?;
            let recall_ids = recall_query.query_map(rusqlite::params![input.cursor.as_ref().map(|cursor| cursor.updated_at_ms), input.cursor.as_ref().map(|cursor| &cursor.id), (limit + 1) as i64,shared], |row| row.get::<_, String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
            for id in recall_ids {
                let job = crate::recall::jobs::get(connection, &id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)?;
                tasks.push(task_for_recall_job(connection, job)?);
            }
            for model in storage::list_ai_model_assets(connection)? {
                if model.download_status=="missing" {continue;}
                if shared && matches!(model.download_status.as_str(),"available"|"cancelled") && model.self_test_status!="running" {continue;}
                let status=if model.self_test_status=="running" { "running" } else { match model.download_status.as_str(){"available"=>"succeeded","downloading"=>"running","queued"=>"queued","paused"=>"paused","cancelled"=>"cancelled",_=>"failed"} };
                let progress=model.total_bytes.filter(|total|*total>0).map(|total|(model.downloaded_bytes as f64/total as f64).clamp(0.0,1.0));
                tasks.push(BackgroundTask{id:format!("model-download:{}",model.id),kind:"model-download".into(),title:model.display_label,recording_id:None,model_id:Some(model.id),day_id:None,status:status.into(),phase:if model.self_test_status=="running"{"verifying".into()}else{model.download_status},progress,elapsed_ms:(model.updated_at_ms-model.created_at_ms).max(0),eta_ms:None,next_retry_at_ms:None,input_open:false,error_code:model.error_code,updated_at_ms:model.updated_at_ms,stages:Vec::new()});
            }
        }
        Ok(tasks)
    }).map_err(storage_error)?;
    if !shared {
        tasks.extend(super::operations::list(input.recording_id.as_deref()));
    }
    if let Some(cursor) = &input.cursor {
        tasks.retain(|task| {
            (task.updated_at_ms, task.id.as_str()) < (cursor.updated_at_ms, cursor.id.as_str())
        });
    }
    tasks.sort_by(|a, b| {
        b.updated_at_ms
            .cmp(&a.updated_at_ms)
            .then_with(|| b.id.cmp(&a.id))
    });
    let has_more = tasks.len() > limit;
    tasks.truncate(limit);
    let next_cursor = if has_more {
        tasks.last().map(|task| BackgroundTaskCursor {
            updated_at_ms: task.updated_at_ms,
            id: task.id.clone(),
        })
    } else {
        None
    };
    Ok(BackgroundTaskPage { tasks, next_cursor })
}

pub(super) fn get_background_task(
    input: GetBackgroundTaskInput,
    storage: &AppStorage,
) -> Result<BackgroundTaskDetail, AiError> {
    let recall_task = storage
        .with_connection(|connection| crate::recall::jobs::get(connection, &input.task_id))
        .map_err(storage_error)?;
    if let Some(job) = recall_task {
        return Ok(BackgroundTaskDetail {
            task: storage
                .with_connection(|connection| task_for_recall_job(connection, job))
                .map_err(storage_error)?,
            units: Vec::new(),
        });
    }
    if input.task_id.starts_with("model-download:") {
        let page = list_background_tasks(
            Some(ListBackgroundTasksInput {
                scope: None,
                limit: Some(200),
                recording_id: None,
                cursor: None,
            }),
            storage,
        )?;
        if let Some(task) = page.tasks.into_iter().find(|task| task.id == input.task_id) {
            return Ok(BackgroundTaskDetail {
                task,
                units: vec![],
            });
        }
    }
    if let Some(task) = super::operations::get(&input.task_id) {
        return Ok(BackgroundTaskDetail {
            task,
            units: vec![],
        });
    }
    storage
        .with_connection(|connection| {
            let job = storage::get_processing_job(connection, &input.task_id)?
                .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
            let task = task_for_job(connection, job)?;
            let chunks = storage::list_processing_chunks(connection, &input.task_id)?;
            let mut units = Vec::new();
            for chunk in chunks {
                let mut children = unit_children(connection, &chunk)?;
                if !children.is_empty()
                    && (chunk.error_code.is_none()
                        || children.iter().any(|unit| unit.status == "failed"))
                {
                    units.append(&mut children);
                    continue;
                }
                units.push(BackgroundTaskUnit {
                    id: chunk.id.clone(),
                    chunk_id: chunk.id,
                    source_kind: chunk.source_kind,
                    start_offset_ms: chunk.start_offset_ms,
                    end_offset_ms: chunk.end_offset_ms,
                    status: chunk.status,
                    progress: chunk.progress,
                    error_code: chunk.error_code,
                    attempt_count: chunk.attempt_count,
                });
                units.append(&mut children);
            }
            units.sort_by_key(|unit| {
                (
                    match unit.status.as_str() {
                        "running" => 0,
                        "failed" | "completed-with-errors" => 1,
                        "queued" => 2,
                        _ => 3,
                    },
                    unit.start_offset_ms,
                )
            });
            units.truncate(500);
            Ok(BackgroundTaskDetail { task, units })
        })
        .map_err(storage_error)
}

fn task_for_recall_job(
    connection: &rusqlite::Connection,
    job: crate::recall::types::RecallJob,
) -> rusqlite::Result<BackgroundTask> {
    let (started_at_ms, next_retry_at_ms): (i64, Option<i64>) = connection.query_row(
        "SELECT COALESCE(started_at_ms,created_at_ms),next_attempt_at_ms FROM recall_jobs WHERE id=?1",
        [&job.id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    let elapsed_ms = (if job.status == "running" {
        now_ms()
    } else {
        job.updated_at_ms
    })
    .saturating_sub(started_at_ms)
    .max(0);
    let title = match job.day_id.as_deref() {
        Some(day) => format!("Day recall · {day}"),
        None => "Recall evidence indexing".into(),
    };
    let status = if job.status == "pending" {
        "queued".into()
    } else {
        job.status.clone()
    };
    Ok(BackgroundTask {
        id: job.id,
        kind: format!("recall-{}", job.kind),
        title,
        recording_id: None,
        model_id: None,
        day_id: job.day_id,
        status,
        phase: job.status,
        progress: Some(job.progress),
        elapsed_ms,
        eta_ms: None,
        next_retry_at_ms,
        input_open: false,
        error_code: job.error_code,
        updated_at_ms: job.updated_at_ms,
        stages: Vec::new(),
    })
}

fn unit_children(
    connection: &rusqlite::Connection,
    chunk: &storage::ProcessingChunkRow,
) -> rusqlite::Result<Vec<BackgroundTaskUnit>> {
    if matches!(chunk.source_kind.as_str(), "ocr" | "vlm") {
        let mut statement=connection.prepare("SELECT id,frame_timestamp_ms,status,progress,error_code FROM visual_frame_processing_states WHERE chunk_id=?1 ORDER BY CASE status WHEN 'running' THEN 0 WHEN 'failed' THEN 1 WHEN 'queued' THEN 2 ELSE 3 END,frame_timestamp_ms LIMIT 500")?;
        let rows = statement.query_map([&chunk.id], |row| {
            Ok(BackgroundTaskUnit {
                id: row.get(0)?,
                chunk_id: chunk.id.clone(),
                source_kind: chunk.source_kind.clone(),
                start_offset_ms: Some(row.get(1)?),
                end_offset_ms: None,
                status: row.get(2)?,
                progress: row.get(3)?,
                error_code: row.get(4)?,
                attempt_count: chunk.attempt_count,
            })
        })?;
        return rows.collect();
    }
    if chunk.source_kind == "audio" {
        let mut statement=connection.prepare("SELECT r.start_ms,r.end_ms,r.status,r.error_code FROM processing_audio_ranges r WHERE r.chunk_id=?1 AND NOT EXISTS(SELECT 1 FROM processing_audio_ranges n WHERE n.chunk_id=r.chunk_id AND n.start_ms=r.start_ms AND n.end_ms=r.end_ms AND (n.updated_at_ms>r.updated_at_ms OR (n.updated_at_ms=r.updated_at_ms AND n.rowid>r.rowid))) ORDER BY r.status='failed' DESC,r.start_ms LIMIT 500")?;
        let rows = statement.query_map([&chunk.id], |row| {
            let start: i64 = row.get(0)?;
            let end: i64 = row.get(1)?;
            Ok(BackgroundTaskUnit {
                id: format!("speech:{}:{start}:{end}", chunk.id),
                chunk_id: chunk.id.clone(),
                source_kind: "audio".into(),
                start_offset_ms: chunk.start_offset_ms.map(|offset| offset + start),
                end_offset_ms: chunk.start_offset_ms.map(|offset| offset + end),
                status: row.get(2)?,
                progress: 1.0,
                error_code: row.get(3)?,
                attempt_count: chunk.attempt_count,
            })
        })?;
        return rows.collect();
    }
    Ok(vec![])
}

#[cfg(test)]
mod recall_tests {
    use super::*;

    #[test]
    fn recall_jobs_page_beyond_status_history_without_recording_ids() {
        let storage = crate::storage::test_storage(std::path::PathBuf::from(
            "/tmp/synthetic-recall-task-page",
        ));
        storage.with_connection(|connection| {
            for index in 0..105 {
                connection.execute("INSERT INTO recall_jobs(id,day_key,kind,status,input_fingerprint,config_fingerprint,created_at_ms,updated_at_ms) VALUES(?1,'2026-10-05@UTC','generation','pending','synthetic','synthetic',?2,?2)", rusqlite::params![format!("synthetic-{index:03}"), index])?;
            }
            Ok(())
        }).unwrap();
        let mut cursor = None;
        let mut seen = std::collections::BTreeSet::new();
        loop {
            let page = list_background_tasks(
                Some(ListBackgroundTasksInput {
                    scope: None,
                    recording_id: None,
                    limit: Some(25),
                    cursor,
                }),
                &storage,
            )
            .unwrap();
            for task in page.tasks {
                assert_eq!(task.kind, "recall-generation");
                assert_eq!(task.recording_id, None);
                assert_eq!(task.status, "queued");
                assert!(seen.insert(task.id));
            }
            cursor = page.next_cursor;
            if cursor.is_none() {
                break;
            }
        }
        assert_eq!(seen.len(), 105);
        let detail = get_background_task(
            GetBackgroundTaskInput {
                task_id: "synthetic-000".into(),
            },
            &storage,
        )
        .unwrap();
        assert_eq!(detail.task.id, "synthetic-000");
        assert!(detail.units.is_empty());
    }
}
