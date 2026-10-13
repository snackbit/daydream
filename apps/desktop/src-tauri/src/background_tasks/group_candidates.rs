//! SQL metadata selection owns filters/counts before any recording/file projection.
use super::*;
use rusqlite::{params, Connection};

fn count(row: &rusqlite::Row<'_>, column: usize) -> rusqlite::Result<usize> {
    let value: i64 = row.get(column)?;
    usize::try_from(value).map_err(|_| rusqlite::Error::IntegralValueOutOfRange(column, value))
}

/// Reused by both candidate aggregation and per-lane inspection/actions.
pub(super) const EFFECTIVE_LANES: &str = r#"
WITH job_lanes AS (
 SELECT j.recording_id,c.source_kind,j.id job_id,j.created_at_ms,j.status job_status,
 COUNT(*) total,SUM(c.status='running') running,SUM(c.status='queued') queued,
 SUM(c.status IN('failed','completed-with-errors')) failed,SUM(c.status='cancelled') cancelled,
 MAX(COALESCE(r.input_open,0)) input_open
 FROM processing_jobs j JOIN processing_chunks c ON c.job_id=j.id LEFT JOIN processing_job_runtime r ON r.job_id=j.id
 WHERE c.source_kind IN('audio','ocr','vlm','activity','waveform') AND (?1 IS NULL OR j.recording_id=?1)
 GROUP BY j.id,c.source_kind
), ranked_lanes AS (
 SELECT *,ROW_NUMBER() OVER(PARTITION BY recording_id,source_kind ORDER BY created_at_ms DESC,job_id DESC) run_order FROM job_lanes
), effective_lanes AS (
 SELECT * FROM ranked_lanes WHERE run_order=1 OR (job_status IN('running','queued','paused') AND running+queued>0)
)
"#;
const GROUP_STATES: &str = r#"
, lane_states AS (
 SELECT recording_id,
 MAX(job_status NOT IN('paused','failed') AND running>0) running,
 MAX(job_status NOT IN('paused','failed') AND running=0 AND queued>0) queued,
 MAX(job_status='paused') paused,MAX(job_status='failed') failed,
 MAX(job_status!='paused' AND running+queued=0 AND (failed>0 OR (job_status='completed-with-errors' AND cancelled>0))) errors,
 MAX(job_status!='paused' AND running+queued+failed=0 AND cancelled>0) cancelled,
 SUM(failed) failed_sections,MAX(input_open) input_open FROM effective_lanes GROUP BY recording_id
), empty_states AS (
 SELECT j.recording_id,MAX(j.status='running') running,MAX(j.status='queued') queued,MAX(j.status='paused') paused,
 MAX(j.status='failed') failed,MAX(j.status='completed-with-errors') errors,MAX(j.status='cancelled') cancelled,
 0 failed_sections,0 input_open FROM processing_jobs j
 WHERE (?1 IS NULL OR j.recording_id=?1) AND NOT EXISTS(SELECT 1 FROM effective_lanes l WHERE l.recording_id=j.recording_id) GROUP BY j.recording_id
), work_states AS (SELECT * FROM lane_states UNION ALL SELECT * FROM empty_states),
recording_updates AS (
 SELECT j.recording_id,MAX(MAX(j.updated_at_ms),r.created_at_ms) updated_at_ms FROM processing_jobs j JOIN recordings r ON r.id=j.recording_id
 WHERE (?1 IS NULL OR j.recording_id=?1) GROUP BY j.recording_id
), recording_states AS MATERIALIZED (
 SELECT u.recording_id,u.updated_at_ms,
 CASE WHEN w.running THEN 'running' WHEN w.queued THEN 'queued' WHEN w.paused THEN 'paused' WHEN w.failed THEN 'failed'
 WHEN w.errors THEN 'completed-with-errors' WHEN w.cancelled THEN 'cancelled' WHEN w.input_open THEN 'running' ELSE 'succeeded' END status,
 (w.failed OR w.errors OR w.failed_sections>0 OR EXISTS(SELECT 1 FROM recording_protection_spans p WHERE p.recording_id=u.recording_id AND p.status!='materialized')) attention
 FROM recording_updates u JOIN work_states w ON w.recording_id=u.recording_id
 WHERE (?6 IS NULL OR u.recording_id IN(SELECT value FROM json_each(?6)))
), counts AS (
 SELECT COUNT(*) total,COALESCE(SUM(status IN('running','queued','paused')),0) active,COALESCE(SUM(attention),0) attention,
 COALESCE(SUM(status NOT IN('running','queued','paused')),0) history,
 (SELECT COALESCE(SUM(running),0) FROM effective_lanes WHERE job_status NOT IN('paused','failed') AND recording_id IN(SELECT recording_id FROM recording_states)) running,
 (SELECT COALESCE(SUM(queued),0) FROM effective_lanes WHERE job_status NOT IN('paused','failed') AND recording_id IN(SELECT recording_id FROM recording_states)) queued FROM recording_states
), page AS (
 SELECT recording_id,updated_at_ms FROM recording_states
 WHERE (?2='all' OR (?2='active' AND status IN('running','queued','paused')) OR (?2='attention' AND attention) OR (?2='history' AND status NOT IN('running','queued','paused')))
 AND (?3 IS NULL OR (updated_at_ms,recording_id)<(?3,?4)) ORDER BY updated_at_ms DESC,recording_id DESC LIMIT ?5
)
SELECT page.recording_id,page.updated_at_ms,counts.total,counts.active,counts.attention,counts.history,counts.running,counts.queued FROM counts LEFT JOIN page ON TRUE ORDER BY page.updated_at_ms DESC,page.recording_id DESC
"#;

pub(super) struct Candidates {
    pub ids: Vec<String>,
    pub next_cursor: Option<BackgroundTaskCursor>,
    pub counts: RecordingProcessingCounts,
}
pub(super) fn select(
    connection: &Connection,
    input: &ListRecordingProcessingGroupsInput,
    limit: usize,
) -> rusqlite::Result<Candidates> {
    let sql = format!("{EFFECTIVE_LANES}{GROUP_STATES}");
    let mut query = connection.prepare(&sql)?;
    let scope = (!input.recording_ids.is_empty())
        .then(|| serde_json::to_string(&input.recording_ids))
        .transpose()
        .map_err(|_| rusqlite::Error::InvalidQuery)?;
    let rows = query
        .query_map(
            params![
                input.recording_id,
                input.filter.as_deref().unwrap_or("all"),
                input.cursor.as_ref().map(|cursor| cursor.updated_at_ms),
                input.cursor.as_ref().map(|cursor| &cursor.id),
                (limit + 1) as i64,
                scope
            ],
            |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, Option<i64>>(1)?,
                    RecordingProcessingCounts {
                        all: count(row, 2)?,
                        active: count(row, 3)?,
                        attention: count(row, 4)?,
                        history: count(row, 5)?,
                        running: count(row, 6)?,
                        queued: count(row, 7)?,
                    },
                ))
            },
        )?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let counts = rows
        .first()
        .map(|(_, _, counts)| RecordingProcessingCounts {
            all: counts.all,
            active: counts.active,
            attention: counts.attention,
            history: counts.history,
            running: counts.running,
            queued: counts.queued,
        })
        .unwrap_or_default();
    let mut page = rows
        .into_iter()
        .filter_map(|(id, time, _)| id.zip(time))
        .collect::<Vec<_>>();
    let has_more = page.len() > limit;
    page.truncate(limit);
    let next_cursor = if has_more {
        page.last().map(|(id, time)| BackgroundTaskCursor {
            id: id.clone(),
            updated_at_ms: *time,
        })
    } else {
        None
    };
    Ok(Candidates {
        ids: page.into_iter().map(|(id, _)| id).collect(),
        next_cursor,
        counts,
    })
}

pub(super) fn known_ids(
    connection: &Connection,
    input: &ListRecordingProcessingGroupsInput,
) -> rusqlite::Result<Vec<String>> {
    if input.known_recording_ids.is_empty() {
        return Ok(Vec::new());
    }
    let known = serde_json::to_string(&input.known_recording_ids)
        .map_err(|_| rusqlite::Error::InvalidQuery)?;
    let scope = (!input.recording_ids.is_empty())
        .then(|| serde_json::to_string(&input.recording_ids))
        .transpose()
        .map_err(|_| rusqlite::Error::InvalidQuery)?;
    let mut query=connection.prepare("SELECT DISTINCT recording_id FROM processing_jobs WHERE recording_id IN(SELECT value FROM json_each(?1)) AND (?2 IS NULL OR recording_id=?2) AND (?3 IS NULL OR recording_id IN(SELECT value FROM json_each(?3)))")?;
    let ids = query
        .query_map(params![known, input.recording_id, scope], |row| row.get(0))?
        .collect();
    ids
}

#[cfg(test)]
#[path = "group_candidate_tests.rs"]
mod tests;
