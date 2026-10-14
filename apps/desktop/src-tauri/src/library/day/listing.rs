//! Library cards read capture and Recall metadata, never refresh evidence.
use crate::{
    recall::{
        self,
        types::{ListRecallDaysInput, RecallDayInput},
    },
    storage::{self, AppStorage},
};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use sha2::{Digest, Sha256};

mod configuration;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LibraryDayCard {
    privacy_revision: i64,
    day_id: String,
    recording_count: usize,
    evidence_count: usize,
    reviewed: bool,
    status: String,
    captured_duration_ms: i64,
    thumbnail_path: Option<String>,
    thumbnail_recording_id: Option<String>,
    redaction_count: usize,
    recording_active: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LibraryDaysPage {
    privacy_revision: i64,
    days: Vec<LibraryDayCard>,
    next_before_day_id: Option<String>,
    generation_ready: bool,
    generation_unavailable_reason: Option<String>,
}

pub(crate) fn list_days(
    storage: &AppStorage,
    input: &ListRecallDaysInput,
) -> Result<LibraryDaysPage, String> {
    // Configuration metadata only: never open the keyring or initialize a model
    // just to enumerate recordings. Runtime readiness belongs to Recall actions.
    storage
        .with_read_connection(|connection| {
            let transaction = connection.unchecked_transaction()?;
            let page = read_page(&transaction, input)?;
            transaction.commit()?;
            if storage::recall_privacy_revision(connection)? != page.privacy_revision {
                return Err(rusqlite::Error::InvalidQuery);
            }
            Ok(page)
        })
        .map_err(|_| "Library days could not be loaded. Try again.".into())
}

fn read_page(
    connection: &Connection,
    input: &ListRecallDaysInput,
) -> rusqlite::Result<LibraryDaysPage> {
    let profile = configuration::summary_profile(connection);
    let rows = recall::day::capture_rows(connection)?;
    let dates = recall::day::capture_dates(&rows, input, chrono::Utc::now().timestamp_millis())
        .map_err(|_| rusqlite::Error::InvalidQuery)?;
    let mut dates = dates.into_iter().rev();
    let ids: Vec<_> = dates.by_ref().take(input.limit.clamp(1, 100)).collect();
    let has_more = dates.next().is_some();
    let privacy_revision = storage::recall_privacy_revision(connection)?;
    let mut days = Vec::with_capacity(ids.len());
    for day_id in ids {
        let day_input = RecallDayInput {
            day_id,
            timezone: input.timezone.clone(),
        };
        let (start, end) = recall::day::day_bounds(&day_input.day_id, &day_input.timezone)
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let evidence_count = usize::try_from(connection.query_row(
            "SELECT COUNT(*) FROM recall_evidence WHERE started_at_ms<?2 AND ended_at_ms>?1",
            params![start, end],
            |row| row.get::<_, i64>(0),
        )?)
        .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let key = recall::day::day_key(&day_input);
        let dirty: bool = connection.query_row(
                "SELECT EXISTS(SELECT 1 FROM recall_dirty_recordings d JOIN recording_segments s ON s.recording_id=d.recording_id WHERE s.started_at_ms<?2 AND COALESCE(s.started_at_ms+s.duration_ms,s.ended_at_ms,?2)>?1)",
                params![start,end], |row| row.get(0),
            )?;
        let has_review: bool = connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM recall_day_reviews WHERE day_key=?1)",
            [&key],
            |row| row.get(0),
        )?;
        let fingerprint = if has_review && !dirty {
            Some(evidence_fingerprint(connection, start, end)?)
        } else {
            None
        };
        let snapshot = recall::day::day_snapshot(
            connection,
            &day_input,
            &rows,
            evidence_count,
            fingerprint.as_deref(),
        )
        .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let card = snapshot.day;
        let status = summary_status(
            connection,
            &key,
            (start, end),
            evidence_count,
            dirty,
            profile.as_deref().ok(),
        )?;
        days.push(LibraryDayCard {
            privacy_revision,
            day_id: card.day_id,
            recording_count: card.recording_count,
            evidence_count: card.evidence_count,
            reviewed: card.reviewed,
            status,
            captured_duration_ms: card.captured_duration_ms,
            thumbnail_path: card.thumbnail_path,
            thumbnail_recording_id: card.thumbnail_recording_id,
            redaction_count: card.redaction_count,
            recording_active: card.recording_active,
        });
    }
    Ok(LibraryDaysPage {
        next_before_day_id: if has_more {
            days.last().map(|day| day.day_id.clone())
        } else {
            None
        },
        days,
        privacy_revision,
        generation_ready: profile.is_ok(),
        generation_unavailable_reason: profile.err(),
    })
}

fn evidence_fingerprint(connection: &Connection, start: i64, end: i64) -> rusqlite::Result<String> {
    let mut query = connection.prepare("SELECT id,revision FROM recall_evidence WHERE started_at_ms<?2 AND ended_at_ms>?1 ORDER BY started_at_ms,id")?;
    let mut hash = Sha256::new();
    for row in query.query_map(params![start, end], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })? {
        let (id, revision) = row?;
        hash.update((id.len() as u64).to_le_bytes());
        hash.update(id.as_bytes());
        hash.update(revision.as_bytes());
    }
    Ok(format!("{:x}", hash.finalize()))
}

fn summary_status(
    connection: &Connection,
    key: &str,
    (start, end): (i64, i64),
    evidence_count: usize,
    dirty: bool,
    profile: Option<&str>,
) -> rusqlite::Result<String> {
    let generation = connection
        .query_row(
            "SELECT status,profile_fingerprint FROM recall_generations WHERE day_key=?1",
            [key],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()?;
    if let Some((status, configuration)) = &generation {
        if status == "ready" {
            let (consumed,matched): (i64,i64) = connection.query_row(
                "SELECT COUNT(*),COUNT(e.id) FROM recall_generation_inputs i LEFT JOIN recall_evidence e ON e.id=i.evidence_id AND e.revision=i.revision AND e.started_at_ms<?3 AND e.ended_at_ms>?2 WHERE i.day_key=?1",
                params![key,start,end],|row|Ok((row.get(0)?,row.get(1)?)),
            )?;
            if consumed > 0 && consumed == matched {
                return Ok(if !dirty
                    && usize::try_from(consumed).ok() == Some(evidence_count)
                    && profile == Some(configuration.as_str())
                {
                    "ready"
                } else {
                    "partial"
                }
                .into());
            }
        }
    }
    let running: bool = connection.query_row("SELECT EXISTS(SELECT 1 FROM recall_jobs WHERE day_key=?1 AND status IN('running','pending'))",[key],|row|row.get(0))?;
    Ok(if profile.is_none() {
        "unconfigured"
    } else if evidence_count == 0 {
        "insufficient"
    } else if running {
        "generating"
    } else if generation.is_some() {
        "invalidated"
    } else {
        "partial"
    }
    .into())
}

#[tauri::command(rename = "listLibraryDays")]
pub(crate) async fn list_library_days(
    input: ListRecallDaysInput,
    app: tauri::AppHandle,
) -> Result<LibraryDaysPage, String> {
    use tauri::Manager;
    let activity = crate::maintenance::runtime::admit(&app, "library-days")?;
    tauri::async_runtime::spawn_blocking(move || {
        let _activity = activity;
        list_days(&app.state::<AppStorage>(), &input)
    })
    .await
    .map_err(|_| "Library days could not be loaded. Try again.".to_string())?
}

#[cfg(test)]
mod tests;
