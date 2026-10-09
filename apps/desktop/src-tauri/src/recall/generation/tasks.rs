use super::{
    generation::{action_key, Input, Statement},
    types::{RecallCitation, RecallTask, UpdateRecallTaskInput},
};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Sources {
    inputs: Vec<Input>,
    citations: Vec<RecallCitation>,
    #[serde(default)]
    withheld: bool,
}

pub fn accept_with_status(
    connection: &Connection,
    day_key: &str,
    statement: &Statement,
    inputs: &[Input],
    citations: Vec<RecallCitation>,
    now_ms: i64,
    status: &str,
) -> Result<String, String> {
    if !matches!(status, "open" | "dismissed") {
        return Err("recall_task_invalid".into());
    }
    let key = action_key(statement);
    let existing: Option<String> = connection
        .query_row(
            "SELECT id FROM recall_tasks WHERE day_key=?1 AND action_key=?2",
            params![day_key, key],
            |row| row.get(0),
        )
        .optional()
        .map_err(|_| "recall_storage_failed")?;
    // Exact matches preserve every local edit/status, including dismissed tasks.
    if let Some(id) = existing {
        if status == "dismissed" {
            connection
                .execute(
                    "UPDATE recall_tasks SET status='dismissed',updated_at=?2 WHERE id=?1",
                    params![id, now_ms],
                )
                .map_err(|_| "recall_storage_failed")?;
        }
        return Ok(id);
    }
    let id = uuid::Uuid::now_v7().to_string();
    let source = serde_json::to_string(&Sources {
        inputs: inputs
            .iter()
            .cloned()
            .map(|mut input| {
                input.text.clear();
                input
            })
            .collect(),
        citations,
        withheld: false,
    })
    .map_err(|_| "recall_input_invalid")?;
    connection.execute("INSERT INTO recall_tasks(id,day_key,action_key,source_json,title,status,updated_at) VALUES(?1,?2,?3,?4,?5,?7,?6)", params![id,day_key,key,source,statement.text,now_ms,status]).map_err(|_| "recall_storage_failed")?;
    Ok(id)
}

pub fn update(
    connection: &Connection,
    input: &UpdateRecallTaskInput,
    now_ms: i64,
) -> Result<(), String> {
    if input.title.trim().is_empty()
        || input.title.len() > 2_000
        || input.notes.len() > 8_000
        || !matches!(input.status.as_str(), "open" | "done" | "dismissed")
    {
        return Err("recall_task_invalid".into());
    }
    let changed = connection
        .execute(
            "UPDATE recall_tasks SET edited_title=CASE WHEN ?2=COALESCE(edited_title,title) THEN edited_title ELSE ?2 END,notes=?3,status=?4,updated_at=?5 WHERE id=?1",
            params![
                input.task_id,
                input.title,
                input.notes,
                input.status,
                now_ms
            ],
        )
        .map_err(|_| "recall_storage_failed")?;
    if changed != 1 {
        return Err("recall_task_missing".into());
    }
    Ok(())
}

pub fn list(connection: &Connection, day_key: &str) -> Result<Vec<RecallTask>, String> {
    crate::storage::refresh_evidence(connection).map_err(|_| "recall_storage_failed")?;
    let mut query = connection.prepare("SELECT id,title,edited_title,notes,status,source_json,updated_at FROM recall_tasks WHERE day_key=?1 ORDER BY updated_at,id").map_err(|_| "recall_storage_failed")?;
    let rows = query
        .query_map([day_key], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, i64>(6)?,
            ))
        })
        .map_err(|_| "recall_storage_failed")?;
    let mut tasks = Vec::new();
    for row in rows {
        let (id, title, edited, notes, status, json, updated_at_ms) =
            row.map_err(|_| "recall_storage_failed")?;
        let sources: Sources = serde_json::from_str(&json).map_err(|_| "recall_task_invalid")?;
        let mut withheld = sources.withheld;
        for input in &sources.inputs {
            let revision: Option<String> = connection
                .query_row(
                    "SELECT revision FROM recall_evidence WHERE id=?1",
                    [&input.id],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|_| "recall_storage_failed")?;
            if revision.as_deref() != Some(input.revision.as_str()) {
                withheld = true;
                break;
            }
        }
        // Local edits stay durable; withholding avoids recreating hidden source text.
        tasks.push(RecallTask {
            id,
            day_id: day_key.into(),
            title: if withheld {
                String::new()
            } else {
                edited.unwrap_or(title)
            },
            notes: if withheld { String::new() } else { notes },
            status,
            citations: if withheld { vec![] } else { sources.citations },
            source_withheld: withheld,
            updated_at_ms,
        });
    }
    Ok(tasks)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn status_only_changes_do_not_preserve_generated_titles_after_source_deletion() {
        use crate::storage;
        let storage = storage::test_storage(std::path::PathBuf::from(
            "/tmp/daydream-recall-task-status-test",
        ));
        storage
            .with_connection(|connection| {
                let fixture = storage::seed_recall_fixture(connection);
                storage::refresh_evidence(connection)?;
                let evidence = storage::list_evidence_range(connection, 0, i64::MAX)?;
                let inputs = super::super::generation_store::inputs(&evidence);
                let statement = Statement {
                    id: "task".into(),
                    text: "Generated synthetic title".into(),
                    category: None,
                    citations: vec![],
                };
                let unedited =
                    accept_with_status(connection, "day", &statement, &inputs, vec![], 1, "open")
                        .unwrap();
                update(
                    connection,
                    &UpdateRecallTaskInput {
                        task_id: unedited.clone(),
                        title: statement.text.clone(),
                        notes: String::new(),
                        status: "done".into(),
                    },
                    2,
                )
                .unwrap();
                assert!(connection
                    .query_row(
                        "SELECT edited_title FROM recall_tasks WHERE id=?1",
                        [&unedited],
                        |row| row.get::<_, Option<String>>(0)
                    )?
                    .is_none());
                let edited_statement = Statement {
                    text: "Another generated title".into(),
                    ..statement
                };
                let edited = accept_with_status(
                    connection,
                    "day",
                    &edited_statement,
                    &inputs,
                    vec![],
                    1,
                    "open",
                )
                .unwrap();
                update(
                    connection,
                    &UpdateRecallTaskInput {
                        task_id: edited.clone(),
                        title: "My genuine local edit".into(),
                        notes: "My local note".into(),
                        status: "done".into(),
                    },
                    2,
                )
                .unwrap();
                storage::delete_recording(connection, &fixture.recording)?;
                let unedited_row: (String, Option<String>, String) = connection.query_row(
                    "SELECT title,edited_title,status FROM recall_tasks WHERE id=?1",
                    [&unedited],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )?;
                assert_eq!(unedited_row, (String::new(), None, "done".into()));
                let edited_row: (String, Option<String>, String) = connection.query_row(
                    "SELECT title,edited_title,notes FROM recall_tasks WHERE id=?1",
                    [&edited],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )?;
                assert_eq!(
                    edited_row,
                    (
                        String::new(),
                        Some("My genuine local edit".into()),
                        "My local note".into()
                    )
                );
                Ok(())
            })
            .unwrap();
    }
    #[test]
    fn accepting_exact_regeneration_preserves_dismissal_and_local_edits() {
        let connection = Connection::open_in_memory().unwrap();
        connection.execute_batch("CREATE TABLE recall_tasks(id TEXT PRIMARY KEY,day_key TEXT,action_key TEXT,source_json TEXT,title TEXT,edited_title TEXT,notes TEXT DEFAULT '',status TEXT,updated_at INTEGER,UNIQUE(day_key,action_key));").unwrap();
        let statement = Statement {
            id: "suggestion1".into(),
            text: "Review synthetic document".into(),
            citations: vec![],
            category: None,
        };
        let id =
            accept_with_status(&connection, "day", &statement, &[], vec![], 1, "open").unwrap();
        update(
            &connection,
            &UpdateRecallTaskInput {
                task_id: id.clone(),
                title: "My local edit".into(),
                notes: "Local note".into(),
                status: "dismissed".into(),
            },
            2,
        )
        .unwrap();
        assert_eq!(
            accept_with_status(&connection, "day", &statement, &[], vec![], 3, "open").unwrap(),
            id
        );
        let stored: (String, String, String) = connection
            .query_row(
                "SELECT edited_title,notes,status FROM recall_tasks WHERE id=?1",
                [&id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(
            stored,
            (
                "My local edit".into(),
                "Local note".into(),
                "dismissed".into()
            )
        );
        let changed = Statement {
            text: "Email synthetic document".into(),
            ..statement
        };
        assert_ne!(
            accept_with_status(&connection, "day", &changed, &[], vec![], 4, "open").unwrap(),
            id
        );
    }
    #[test]
    fn redaction_of_uncited_consumed_input_withholds_task_and_preserves_local_edits() {
        use crate::storage;
        let storage =
            storage::test_storage(std::path::PathBuf::from("/tmp/daydream-recall-task-test"));
        storage
            .with_connection(|connection| {
                let fixture = storage::seed_recall_fixture(connection);
                storage::refresh_evidence(connection)?;
                let evidence = storage::list_evidence_range(connection, 0, i64::MAX)?;
                assert!(!evidence.is_empty());
                let inputs = super::super::generation_store::inputs(&evidence);
                let statement = Statement {
                    id: "task".into(),
                    text: "Synthetic action".into(),
                    citations: vec![],
                    category: None,
                };
                let task =
                    accept_with_status(connection, "day", &statement, &inputs, vec![], 1, "open")
                        .unwrap();
                update(
                    connection,
                    &UpdateRecallTaskInput {
                        task_id: task.clone(),
                        title: "Preserved edit".into(),
                        notes: "Preserved note".into(),
                        status: "done".into(),
                    },
                    2,
                )
                .unwrap();
                storage::create_redaction(
                    connection,
                    &storage::CreateRedactionInput {
                        id: uuid::Uuid::now_v7().to_string(),
                        recording_id: fixture.recording.clone(),
                        start_offset_ms: 0,
                        end_offset_ms: 5000,
                        track_ids_json: format!("[\"{}\"]", fixture.track),
                        created_at_ms: 3,
                        reason: None,
                    },
                )?;
                let tasks = list(connection, "day").unwrap();
                assert!(tasks[0].source_withheld);
                assert!(tasks[0].title.is_empty());
                assert!(tasks[0].notes.is_empty());
                assert_eq!(tasks[0].status, "done");
                let edits: (String, String) = connection.query_row(
                    "SELECT edited_title,notes FROM recall_tasks WHERE id=?1",
                    [task],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )?;
                assert_eq!(edits, ("Preserved edit".into(), "Preserved note".into()));
                Ok(())
            })
            .unwrap();
    }
}
