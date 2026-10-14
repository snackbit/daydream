//! Durable index admission, separate from embedding-space compatibility.
use rusqlite::{params, Connection, OptionalExtension};

pub(super) fn claim(
    connection: &Connection,
    space: &str,
    snapshot: &str,
    now: i64,
) -> rusqlite::Result<Option<String>> {
    super::super::jobs::retry_resolved_local_dependency(connection, space, snapshot, now)?;
    let candidate = connection.query_row(
        "SELECT id,status,next_attempt_at_ms FROM recall_jobs WHERE kind='index' AND config_fingerprint=?1 AND status IN('pending','running','paused','failed','cancelled') AND (status IN('running','paused','cancelled') OR (json_valid(config_json) AND json_extract(config_json,'$.embedding')=json_extract(?2,'$.embedding'))) ORDER BY created_at_ms DESC,id DESC LIMIT 1",
        params![space,snapshot], |row| Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?,row.get::<_,Option<i64>>(2)?)),
    ).optional()?;
    match candidate {
        Some((id, status, retry)) if status == "pending" && retry.is_none_or(|at| at <= now) => {
            let changed = connection.execute("UPDATE recall_jobs SET status='running',updated_at_ms=?2,started_at_ms=?2 WHERE id=?1 AND status='pending'",params![id,now])?;
            Ok((changed == 1).then_some(id))
        }
        Some(_) => Ok(None),
        None => {
            let id = uuid::Uuid::now_v7().to_string();
            connection.execute("INSERT INTO recall_jobs(id,day_key,kind,status,input_fingerprint,config_fingerprint,config_json,created_at_ms,updated_at_ms,started_at_ms) VALUES(?1,'','index','running','',?2,?3,?4,?4,?4)",params![id,space,snapshot,now])?;
            Ok(Some(id))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn backend_change_admits_current_work_without_overriding_user_stop() {
        for status in ["failed", "paused", "cancelled", "running"] {
            let storage = crate::storage::test_storage(std::env::temp_dir());
            storage.with_connection(|connection| {
                connection.execute("INSERT INTO recall_jobs(id,day_key,kind,status,input_fingerprint,config_fingerprint,config_json,error_code,created_at_ms,updated_at_ms) VALUES('old','','index',?1,'','same-space',?2,'recall_embedding_model_missing',1,1)",params![status,r#"{"embedding":{"provider":"local","localBackend":"cpu"}}"#])?;
                let id = claim(connection,"same-space",r#"{"embedding":{"provider":"local","localBackend":"vulkan"}}"#,2)?;
                assert_eq!(id.is_some(),status=="failed","{status}");
                if let Some(id) = id {
                    assert_ne!(id,"old");
                    assert_eq!(connection.query_row("SELECT status FROM recall_jobs WHERE id='old'",[],|row|row.get::<_,String>(0))?,"failed");
                    assert!(claim(connection,"same-space",r#"{"embedding":{"provider":"local","localBackend":"vulkan"}}"#,3)?.is_none());
                }
                Ok(())
            }).unwrap();
        }
    }
    #[test]
    fn resolved_dependency_reuses_matching_attempt_and_respects_retry_delay() {
        let storage = crate::storage::test_storage(std::env::temp_dir());
        storage.with_connection(|connection| {
            let snapshot=r#"{"embedding":{"provider":"local","localBackend":"cpu"}}"#;
            connection.execute("INSERT INTO recall_jobs(id,day_key,kind,status,input_fingerprint,config_fingerprint,config_json,error_code,created_at_ms,updated_at_ms) VALUES('old','','index','failed','','same-space',?1,'recall_embedding_model_missing',1,1)",[snapshot])?;
            assert_eq!(claim(connection,"same-space",snapshot,2)?,Some("old".into()));
            connection.execute("UPDATE recall_jobs SET status='pending',next_attempt_at_ms=10 WHERE id='old'",[])?;
            assert!(claim(connection,"same-space",snapshot,3)?.is_none());
            assert_eq!(claim(connection,"same-space",snapshot,10)?,Some("old".into()));
            Ok(())
        }).unwrap();
    }
}
