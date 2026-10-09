use super::types::RecallJob;
use rusqlite::{params, Connection};

pub fn enqueue(
    connection: &Connection,
    day_key: &str,
    fingerprint: &str,
    config: &str,
    now_ms: i64,
) -> Result<String, String> {
    // Only the newest pending generation remains durable; active work is untouched.
    connection.execute("UPDATE recall_jobs SET status='cancelled',updated_at_ms=?1 WHERE kind='generation' AND status='pending' AND day_key=?2", params![now_ms,day_key]).map_err(|_| "recall_storage_failed")?;
    let id = uuid::Uuid::now_v7().to_string();
    connection.execute("INSERT INTO recall_jobs(id,day_key,kind,status,input_fingerprint,config_fingerprint,created_at_ms,updated_at_ms) VALUES(?1,?2,'generation','pending',?3,?4,?5,?5)",params![id,day_key,fingerprint,config,now_ms]).map_err(|_| "recall_storage_failed")?;
    Ok(id)
}
pub fn recover(connection: &Connection, now_ms: i64) -> rusqlite::Result<()> {
    connection.execute(
        "UPDATE recall_jobs SET status='pending',updated_at_ms=?1 WHERE status='running'",
        [now_ms],
    )?;
    Ok(())
}
pub fn retry_resolved_local_dependency(
    connection: &Connection,
    config: &str,
    snapshot: &str,
    now_ms: i64,
) -> rusqlite::Result<usize> {
    connection.execute("UPDATE recall_jobs SET status='pending',error_code=NULL,next_attempt_at_ms=NULL,attempt_count=0,updated_at_ms=?2 WHERE kind='index' AND status='failed' AND config_fingerprint=?1 AND json_valid(config_json) AND json_extract(config_json,'$.embedding')=json_extract(?3,'$.embedding') AND error_code IN ('recall_embedding_model_missing','recall_embedding_worker_unavailable','recall_embedding_model_test_failed','recall_embedding_vulkan_unavailable')", params![config, now_ms, snapshot])
}

#[cfg(test)]
mod dependency_tests {
    use super::*;
    #[test]
    fn resolved_dependency_retries_only_current_failed_jobs() {
        let connection = Connection::open_in_memory().unwrap();
        connection.execute_batch("CREATE TABLE recall_jobs(id TEXT,day_key TEXT,kind TEXT,status TEXT,input_fingerprint TEXT,config_fingerprint TEXT,config_json TEXT DEFAULT '{\"embedding\":{\"provider\":\"local\"}}',error_code TEXT,created_at_ms INTEGER,updated_at_ms INTEGER,next_attempt_at_ms INTEGER,attempt_count INTEGER DEFAULT 0);").unwrap();
        for (id, status, config, error) in [
            (
                "missing",
                "failed",
                "current",
                "recall_embedding_model_missing",
            ),
            (
                "paused",
                "paused",
                "current",
                "recall_embedding_model_missing",
            ),
            (
                "cancelled",
                "cancelled",
                "current",
                "recall_embedding_model_missing",
            ),
            ("stale", "failed", "old", "recall_embedding_model_missing"),
            (
                "backend",
                "failed",
                "current",
                "recall_embedding_model_missing",
            ),
            (
                "other",
                "failed",
                "current",
                "recall_embedding_invalid_response",
            ),
        ] {
            connection.execute("INSERT INTO recall_jobs(id,day_key,kind,status,input_fingerprint,config_fingerprint,error_code,created_at_ms,updated_at_ms) VALUES(?1,'','index',?2,'',?3,?4,1,1)", params![id,status,config,error]).unwrap();
        }
        connection.execute("UPDATE recall_jobs SET config_json='{\"embedding\":{\"provider\":\"local\",\"localBackend\":\"vulkan\"}}' WHERE id='backend'", []).unwrap();
        assert_eq!(
            retry_resolved_local_dependency(
                &connection,
                "current",
                r#"{"embedding":{"provider":"local"}}"#,
                2
            )
            .unwrap(),
            1
        );
        let states: Vec<(String, String)> = connection
            .prepare("SELECT id,status FROM recall_jobs ORDER BY id")
            .unwrap()
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        assert_eq!(
            states,
            vec![
                ("backend".into(), "failed".into()),
                ("cancelled".into(), "cancelled".into()),
                ("missing".into(), "pending".into()),
                ("other".into(), "failed".into()),
                ("paused".into(), "paused".into()),
                ("stale".into(), "failed".into())
            ]
        );
    }
}
pub fn list(connection: &Connection) -> rusqlite::Result<Vec<RecallJob>> {
    let mut query = connection.prepare("SELECT id,day_key,kind,status,progress,error_code,updated_at_ms FROM recall_jobs ORDER BY updated_at_ms DESC LIMIT 100")?;
    let rows = query
        .query_map([], |row| {
            Ok(RecallJob {
                id: row.get(0)?,
                day_id: {
                    let key: String = row.get(1)?;
                    if key.is_empty() {
                        None
                    } else {
                        Some(key.split('@').next().unwrap_or(&key).into())
                    }
                },
                kind: row.get(2)?,
                status: row.get(3)?,
                progress: row.get(4)?,
                error_code: row.get(5)?,
                updated_at_ms: row.get(6)?,
            })
        })?
        .collect();
    rows
}
pub fn get(connection: &Connection, id: &str) -> rusqlite::Result<Option<RecallJob>> {
    use rusqlite::OptionalExtension;
    connection.query_row("SELECT id,day_key,kind,status,progress,error_code,updated_at_ms FROM recall_jobs WHERE id=?1",[id],|row| {
        let key:String=row.get(1)?;
        Ok(RecallJob{id:row.get(0)?,day_id:if key.is_empty(){None}else{Some(key.split('@').next().unwrap_or(&key).into())},kind:row.get(2)?,status:row.get(3)?,progress:row.get(4)?,error_code:row.get(5)?,updated_at_ms:row.get(6)?})
    }).optional()
}
pub fn control(connection: &Connection, id: &str, action: &str, now_ms: i64) -> Result<(), String> {
    let status = match action {
        "cancel" => "cancelled",
        "pause" => "paused",
        "retry" | "resume" => "pending",
        _ => return Err("recall_job_action_invalid".into()),
    };
    let changed = connection.execute("UPDATE recall_jobs SET status=?2,error_code=NULL,next_attempt_at_ms=NULL,attempt_count=CASE WHEN ?2='pending' THEN 0 ELSE attempt_count END,manual=CASE WHEN ?2='pending' THEN 1 ELSE manual END,updated_at_ms=?3 WHERE id=?1",params![id,status,now_ms]).map_err(|_| "recall_storage_failed")?;
    if changed != 1 {
        return Err("recall_job_missing".into());
    }
    Ok(())
}
pub fn fail(connection: &Connection, id: &str, error: &str, now_ms: i64) -> rusqlite::Result<()> {
    let transient = matches!(
        error,
        "recall_provider_network"
            | "recall_provider_timeout"
            | "recall_provider_rate_limited"
            | "recall_provider_server_failed"
            | "recall_embedding_connection_failed"
            | "recall_embedding_response_failed"
            | "recall_embedding_rate_limited"
            | "recall_embedding_server_failed"
            | "recall_embedding_timeout"
    );
    connection.execute("UPDATE recall_jobs SET attempt_count=attempt_count+1,status=CASE WHEN ?2 AND attempt_count<2 THEN 'pending' ELSE 'failed' END,next_attempt_at_ms=CASE WHEN ?2 AND attempt_count<2 THEN ?3+(attempt_count+1)*30000 ELSE NULL END,error_code=?4,updated_at_ms=?3 WHERE id=?1 AND status='running'",params![id,transient,now_ms,error])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn db() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        connection.execute_batch("CREATE TABLE recall_jobs(id TEXT PRIMARY KEY,day_key TEXT,kind TEXT,status TEXT,input_fingerprint TEXT,config_fingerprint TEXT,manual INTEGER DEFAULT 0,attempt_count INTEGER DEFAULT 0,next_attempt_at_ms INTEGER,progress REAL DEFAULT 0,error_code TEXT,created_at_ms INTEGER,updated_at_ms INTEGER);").unwrap();
        connection
    }
    #[test]
    fn embedding_rate_limit_is_persisted_for_retry_and_healthy_retry_completes() {
        let connection = db();
        let id = enqueue(&connection, "", "input", "config", 0).unwrap();
        connection
            .execute(
                "UPDATE recall_jobs SET kind='index',status='running' WHERE id=?1",
                [&id],
            )
            .unwrap();
        fail(&connection, &id, "recall_embedding_rate_limited", 1000).unwrap();
        let row: (String, i64, i64) = connection
            .query_row(
                "SELECT status,attempt_count,next_attempt_at_ms FROM recall_jobs WHERE id=?1",
                [&id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(row, ("pending".into(), 1, 31000));
        connection
            .execute(
                "UPDATE recall_jobs SET status='running' WHERE id=?1 AND next_attempt_at_ms<=31000",
                [&id],
            )
            .unwrap();
        connection.execute("UPDATE recall_jobs SET status='succeeded',progress=1,error_code=NULL WHERE id=?1 AND status='running'",[&id]).unwrap();
        let job = get(&connection, &id).unwrap().unwrap();
        assert_eq!(job.status, "succeeded");
        assert!(job.error_code.is_none());
        for code in [
            "recall_embedding_connection_failed",
            "recall_embedding_server_failed",
            "recall_embedding_timeout",
            "recall_embedding_response_failed",
        ] {
            connection
                .execute(
                    "UPDATE recall_jobs SET status='running',attempt_count=0 WHERE id=?1",
                    [&id],
                )
                .unwrap();
            fail(&connection, &id, code, 1000).unwrap();
            assert_eq!(get(&connection, &id).unwrap().unwrap().status, "pending");
        }
    }
    #[test]
    fn transient_calls_retry_only_three_times_and_pause_is_preserved() {
        let connection = db();
        let id = enqueue(&connection, "day", "input", "config", 0).unwrap();
        for attempt in 0..3 {
            connection
                .execute("UPDATE recall_jobs SET status='running' WHERE id=?1", [&id])
                .unwrap();
            fail(&connection, &id, "recall_provider_network", attempt * 1000).unwrap();
            let status: String = connection
                .query_row("SELECT status FROM recall_jobs WHERE id=?1", [&id], |row| {
                    row.get(0)
                })
                .unwrap();
            assert_eq!(status, if attempt < 2 { "pending" } else { "failed" });
        }
        control(&connection, &id, "pause", 4000).unwrap();
        fail(&connection, &id, "recall_provider_network", 5000).unwrap();
        assert_eq!(list(&connection).unwrap()[0].status, "paused");
    }
    #[test]
    fn newest_pending_replaces_only_pending_work() {
        let connection = db();
        let active = enqueue(&connection, "day", "a", "config", 0).unwrap();
        connection
            .execute(
                "UPDATE recall_jobs SET status='running' WHERE id=?1",
                [&active],
            )
            .unwrap();
        let older = enqueue(&connection, "day", "b", "config", 1).unwrap();
        let newest = enqueue(&connection, "day", "c", "config", 2).unwrap();
        let jobs = list(&connection).unwrap();
        assert_eq!(
            jobs.iter().find(|job| job.id == active).unwrap().status,
            "running"
        );
        assert_eq!(
            jobs.iter().find(|job| job.id == older).unwrap().status,
            "cancelled"
        );
        assert_eq!(
            jobs.iter().find(|job| job.id == newest).unwrap().status,
            "pending"
        );
    }
}
