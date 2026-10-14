//! Estimates use measured lane throughput, never an average across unlike work.
use rusqlite::{params, Connection};

pub(super) fn lane(
    connection: &Connection,
    jobs: &[String],
    kind: &str,
    now: i64,
) -> rusqlite::Result<Option<i64>> {
    let jobs = serde_json::to_string(jobs).map_err(|_| rusqlite::Error::InvalidQuery)?;
    let mut pending=connection.prepare("SELECT c.progress,c.status,COALESCE(c.end_offset_ms-c.start_offset_ms,1),COALESCE(r.last_progress_at_ms,r.started_at_ms),COALESCE(r.config_json,j.config_json,'') FROM processing_chunks c LEFT JOIN processing_chunk_runtime r ON r.chunk_id=c.id LEFT JOIN processing_job_runtime j ON j.job_id=c.job_id WHERE c.job_id IN(SELECT value FROM json_each(?1)) AND c.source_kind=?2 AND c.status IN('running','queued')")?;
    let work = pending
        .query_map(params![jobs, kind], |row| {
            Ok((
                row.get::<_, f64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, Option<i64>>(3)?,
                row.get::<_, String>(4)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    if work.is_empty() {
        return Ok(Some(0));
    }
    let active = work.iter().filter(|row| row.1 == "running").count().max(1);
    let mut total = 0.;
    let mut measured = std::collections::HashMap::new();
    let mut query=connection.prepare("SELECT CAST(c.updated_at_ms-r.started_at_ms AS REAL)/MAX(COALESCE(c.end_offset_ms-c.start_offset_ms,1),1) FROM processing_chunks c JOIN processing_chunk_runtime r ON r.chunk_id=c.id LEFT JOIN processing_job_runtime j ON j.job_id=c.job_id WHERE c.source_kind=?1 AND c.status='succeeded' AND r.started_at_ms IS NOT NULL AND c.updated_at_ms>r.started_at_ms AND COALESCE(r.retry_count,0)=0 AND COALESCE(r.config_json,j.config_json,'')=?2 ORDER BY c.updated_at_ms DESC LIMIT 8")?;
    for (progress, status, duration, last, configuration) in work {
        let rate = if let Some(rate) = measured.get(&configuration) {
            *rate
        } else {
            let mut rates = query
                .query_map(params![kind, &configuration], |row| row.get::<_, f64>(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            if rates.len() < 3 {
                return Ok(None);
            }
            rates.sort_by(f64::total_cmp);
            let rate = rates[rates.len() / 2];
            measured.insert(configuration, rate);
            rate
        };
        if status == "running"
            && last.is_none_or(|last| {
                now - last > (rate * duration.max(1) as f64 * 2.).max(120_000.) as i64
            })
        {
            return Ok(None);
        }
        total += duration.max(1) as f64 * (1. - progress.clamp(0., 1.)) * rate;
    }
    Ok(Some((total / active as f64).ceil() as i64))
}

pub(super) fn job(connection: &Connection, id: &str, now: i64) -> rusqlite::Result<Option<i64>> {
    let mut query=connection.prepare("SELECT DISTINCT source_kind FROM processing_chunks WHERE job_id=?1 AND status IN('running','queued')")?;
    let kinds = query
        .query_map([id], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut longest = 0;
    for kind in kinds {
        let Some(eta) = lane(connection, &[id.into()], &kind, now)? else {
            return Ok(None);
        };
        longest = longest.max(eta);
    }
    Ok(Some(longest))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::background_tasks::group_test_fixtures::{
        fixture, fixture_id, job as fixture_job, recording,
    };
    #[test]
    fn expensive_ocr_does_not_inherit_cheap_activity_throughput_and_stalls_hide_eta() {
        let storage = fixture();
        storage.with_connection(|connection| {
            recording(connection,"a");
            for (kind,duration) in [("ocr",60_000),("activity",100)] {
                for index in 0..3 {
                    let name=format!("{kind}-{index}");
                    fixture_job(connection,"a",&name,40_000,kind,"succeeded");
                    connection.execute("UPDATE processing_chunks SET updated_at_ms=100000 WHERE job_id=?1",[fixture_id(&name)])?;
                    connection.execute("INSERT INTO processing_chunk_runtime(chunk_id,started_at_ms) VALUES(?1,?2)",params![fixture_id(&format!("{name}-chunk")),100_000-duration])?;
                }
                fixture_job(connection,"a",kind,200_000,kind,"running");
                connection.execute("UPDATE processing_chunks SET progress=0.5 WHERE job_id=?1",[fixture_id(kind)])?;
                connection.execute("INSERT INTO processing_chunk_runtime(chunk_id,started_at_ms,last_progress_at_ms) VALUES(?1,200000,200000)",[fixture_id(&format!("{kind}-chunk"))])?;
            }
            assert_eq!(lane(connection,&[fixture_id("ocr")],"ocr",200_001)?,Some(30_000));
            assert_eq!(lane(connection,&[fixture_id("activity")],"activity",200_001)?,Some(50));
            assert_eq!(lane(connection,&[fixture_id("ocr")],"ocr",400_001)?,None);
            Ok(())
        }).unwrap();
    }
}
