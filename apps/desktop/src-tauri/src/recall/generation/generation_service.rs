use super::{generation_store, types::*};
use crate::storage::AppStorage;
use rusqlite::{Connection, OptionalExtension};

pub fn get_day(storage: &AppStorage, input: &RecallDayInput) -> Result<RecallDay, String> {
    let profile = super::worker::profile(storage).ok();
    storage
        .with_connection(|connection| {
            compose(
                connection,
                input,
                profile.as_ref().map(|profile| profile.fingerprint.as_str()),
            )
            .map_err(|_| rusqlite::Error::InvalidQuery)
        })
        .map_err(|_| "recall_day_unavailable".into())
}
pub(crate) fn compose(
    connection: &Connection,
    input: &RecallDayInput,
    profile: Option<&str>,
) -> Result<RecallDay, String> {
    crate::storage::refresh_evidence(connection).map_err(|_| "recall_storage_failed")?;
    let mut day = super::day::base_day(connection, input)?;
    let key = format!("{}@{}", input.day_id, input.timezone);
    let evidence =
        crate::storage::list_evidence_range(connection, day.day.start_ms, day.day.end_ms)
            .map_err(|_| "recall_storage_failed")?;
    let inputs = generation_store::inputs(&evidence);
    let mut has_output = false;
    let citations = |statement: &super::generation::Statement| -> Vec<RecallCitation> {
        statement
            .citations
            .iter()
            .filter_map(|citation| {
                evidence
                    .iter()
                    .find(|item| item.citation.evidence_id == citation.evidence_id)
                    .map(|item| item.citation.clone())
            })
            .collect()
    };
    if let Some(output) = generation_store::load(
        connection,
        &key,
        &inputs,
        Some(profile.unwrap_or("disabled")),
    )? {
        has_output = true;
        day.day.summary = Some(
            output
                .summary
                .iter()
                .map(|item| item.text.as_str())
                .collect::<Vec<_>>()
                .join("\n"),
        );
        day.day.status = if generation_store::is_fresh(connection, &key, &inputs, profile)? {
            "ready"
        } else {
            "partial"
        }
        .into();
        for session in &output.sessions {
            let corrections = connection.query_row("SELECT correction,title_correction,category_correction FROM recall_session_corrections WHERE day_key=?1 AND session_id=?2",rusqlite::params![key,session.id],|row|Ok((row.get::<_,Option<String>>(0)?,row.get::<_,Option<String>>(1)?,row.get::<_,Option<String>>(2)?))).optional().map_err(|_| "recall_storage_failed")?.unwrap_or_default();
            day.sessions.push(RecallSession {
                id: session.id.clone(),
                title: session.text.clone(),
                category: session.category.clone().unwrap_or_else(|| "other".into()),
                title_correction: corrections.1,
                category_correction: corrections.2,
                summary: session.text.clone(),
                start_ms: session
                    .citations
                    .iter()
                    .map(|c| c.start_ms)
                    .min()
                    .unwrap_or(day.day.start_ms)
                    .max(day.day.start_ms),
                end_ms: session
                    .citations
                    .iter()
                    .map(|c| c.end_ms)
                    .max()
                    .unwrap_or(day.day.end_ms)
                    .min(day.day.end_ms),
                citations: citations(session),
                correction: corrections.0,
            });
        }
        for suggestion in &output.suggestions {
            let key_action = super::generation::action_key(suggestion);
            let accepted_task_id = connection
                .query_row(
                    "SELECT id FROM recall_tasks WHERE day_key=?1 AND action_key=?2",
                    rusqlite::params![key, key_action],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|_| "recall_storage_failed")?;
            day.suggestions.push(RecallSuggestion {
                id: suggestion.id.clone(),
                title: suggestion.text.clone(),
                description: String::new(),
                citations: citations(suggestion),
                accepted_task_id,
                dismissed: connection
                    .query_row(
                        "SELECT status FROM recall_tasks WHERE day_key=?1 AND action_key=?2",
                        rusqlite::params![key, key_action],
                        |row| row.get::<_, String>(0),
                    )
                    .optional()
                    .map_err(|_| "recall_storage_failed")?
                    .is_some_and(|status| status == "dismissed"),
            });
        }
    }
    if !has_output {
        day.day.status=if profile.is_none(){"unconfigured"}else if inputs.is_empty(){"insufficient"}else {
            let stored=connection.query_row("SELECT status FROM recall_generations WHERE day_key=?1",[&key],|row|row.get::<_,String>(0)).optional().map_err(|_|"recall_storage_failed")?;
            let running=connection.query_row("SELECT COUNT(*) FROM recall_jobs WHERE day_key=?1 AND status IN('running','pending')",[&key],|row|row.get::<_,i64>(0)).map_err(|_|"recall_storage_failed")?;
            if running>0 {"generating"} else if stored.is_some(){"invalidated"}else{"partial"}
        }.into();
    }
    let session_ranges = day
        .sessions
        .iter()
        .map(|session| (session.start_ms, session.end_ms))
        .collect::<Vec<_>>();
    day.coverage = day
        .coverage
        .into_iter()
        .flat_map(|interval| {
            if interval.kind != "uninterpreted" {
                return vec![interval];
            }
            super::day::complement(interval.start_ms, interval.end_ms, &session_ranges)
                .into_iter()
                .map(|(start_ms, end_ms)| RecallCoverageInterval {
                    start_ms,
                    end_ms,
                    ..interval.clone()
                })
                .collect()
        })
        .collect();
    day.tasks = super::tasks::list(connection, &key)?;
    for task in &mut day.tasks {
        task.day_id = input.day_id.clone();
    }
    day.day.reviewed = connection
        .query_row(
            "SELECT fingerprint FROM recall_day_reviews WHERE day_key=?1",
            [&key],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|_| "recall_storage_failed")?
        .is_some_and(|value| value == day.day.input_fingerprint);
    let mut feedback = connection
        .prepare("SELECT target_id,rating,correction FROM recall_feedback WHERE day_key=?1")
        .map_err(|_| "recall_storage_failed")?;
    day.feedback = feedback
        .query_map([&key], |row| {
            Ok(RecallFeedback {
                target_id: row.get(0)?,
                rating: row.get(1)?,
                comment: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
            })
        })
        .map_err(|_| "recall_storage_failed")?
        .collect::<Result<_, _>>()
        .map_err(|_| "recall_storage_failed")?;
    Ok(day)
}

pub fn accept_suggestion(
    storage: &AppStorage,
    input: &AcceptRecallSuggestionInput,
    timezone: &str,
    now_ms: i64,
) -> Result<String, String> {
    decide_suggestion(storage, input, timezone, now_ms, "open")
}
pub fn dismiss_suggestion(
    storage: &AppStorage,
    input: &AcceptRecallSuggestionInput,
    timezone: &str,
    now_ms: i64,
) -> Result<String, String> {
    decide_suggestion(storage, input, timezone, now_ms, "dismissed")
}
fn decide_suggestion(
    storage: &AppStorage,
    input: &AcceptRecallSuggestionInput,
    timezone: &str,
    now_ms: i64,
    status: &str,
) -> Result<String, String> {
    storage
        .with_connection(|connection| {
            let day_input = RecallDayInput {
                day_id: input.day_id.clone(),
                timezone: timezone.into(),
            };
            let day = super::day::base_day(connection, &day_input)
                .map_err(|_| rusqlite::Error::InvalidQuery)?;
            let key = format!("{}@{}", input.day_id, timezone);
            crate::storage::refresh_evidence(connection)?;
            let evidence =
                crate::storage::list_evidence_range(connection, day.day.start_ms, day.day.end_ms)?;
            let consumed = generation_store::inputs(&evidence);
            let output = generation_store::load(connection, &key, &consumed, None)
                .map_err(|_| rusqlite::Error::InvalidQuery)?
                .ok_or(rusqlite::Error::InvalidQuery)?;
            let suggestion = output
                .suggestions
                .iter()
                .find(|item| item.id == input.suggestion_id)
                .ok_or(rusqlite::Error::InvalidQuery)?;
            let mut lineage_query = connection.prepare(
                "SELECT evidence_id,revision FROM recall_generation_inputs WHERE day_key=?1",
            )?;
            let lineage = lineage_query
                .query_map([&key], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })?
                .collect::<Result<std::collections::HashSet<_>, _>>()?;
            let consumed = consumed
                .into_iter()
                .filter(|input| lineage.contains(&(input.id.clone(), input.revision.clone())))
                .collect::<Vec<_>>();
            let citations = suggestion
                .citations
                .iter()
                .filter_map(|citation| {
                    evidence
                        .iter()
                        .find(|item| item.citation.evidence_id == citation.evidence_id)
                        .map(|item| item.citation.clone())
                })
                .collect();
            super::tasks::accept_with_status(
                connection, &key, suggestion, &consumed, citations, now_ms, status,
            )
            .map_err(|_| rusqlite::Error::InvalidQuery)
        })
        .map_err(|_| "recall_suggestion_unavailable".into())
}
