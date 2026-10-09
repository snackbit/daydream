//! Local recall edits and job controls share privacy invalidation notifications.
use super::types::*;
use crate::storage::{self, AppStorage};
use rusqlite::OptionalExtension;
use tauri::{Emitter, Manager};

pub fn emit_changed<R: tauri::Runtime>(app: &tauri::AppHandle<R>, day_id: Option<&str>) {
    let storage = app.state::<AppStorage>();
    let revision = storage.with_connection(storage::recall_privacy_revision);
    if let Ok(privacy_revision) = revision {
        let _ = app.emit(
            "recall-state-changed",
            RecallStateChanged {
                day_id: day_id.map(str::to_owned),
                privacy_revision,
            },
        );
    }
    crate::background_tasks::emit_changed(app, None);
}

/// Storage triggers are authoritative. Observe their revision even when a
/// retention or protection mutation has no frontend command owner.
pub fn start_revision_observer(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut previous = None;
        loop {
            let revision = app
                .state::<AppStorage>()
                .with_connection(storage::recall_privacy_revision)
                .ok();
            if revision != previous {
                previous = revision;
                emit_changed(&app, None);
            }
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        }
    });
}

fn day_input(day_id: &str) -> Result<RecallDayInput, String> {
    let input = RecallDayInput {
        day_id: day_id.into(),
        timezone: super::day::local_timezone()?,
    };
    super::day::day_bounds(&input.day_id, &input.timezone)?;
    Ok(input)
}

fn find_job(storage: &AppStorage, id: &str) -> Result<RecallJob, String> {
    storage
        .with_connection(|connection| super::jobs::get(connection, id))
        .map_err(|_| "recall_storage_failed")?
        .ok_or_else(|| "recall_job_missing".into())
}

pub fn regenerate(app: &tauri::AppHandle, input: &RecallDayInput) -> Result<RecallJob, String> {
    super::day::day_bounds(&input.day_id, &input.timezone)?;
    let storage = app.state::<AppStorage>();
    let id = super::worker::regenerate(&storage, input)?;
    emit_changed(app, Some(&input.day_id));
    find_job(&storage, &id)
}

pub fn control(app: &tauri::AppHandle, input: &ControlRecallJobInput) -> Result<RecallJob, String> {
    let storage = app.state::<AppStorage>();
    storage
        .with_connection(|connection| {
            super::jobs::control(
                connection,
                &input.job_id,
                &input.action,
                super::worker::now_ms(),
            )
            .map_err(|_| rusqlite::Error::InvalidQuery)
        })
        .map_err(|_| "recall_job_control_failed")?;
    emit_changed(app, None);
    find_job(&storage, &input.job_id)
}

pub fn feedback(
    app: &tauri::AppHandle,
    mut input: SetRecallFeedbackInput,
) -> Result<RecallDay, String> {
    let day = day_input(&input.day_id)?;
    let storage = app.state::<AppStorage>();
    let current = super::generation_service::get_day(&storage, &day)?;
    if input.target_id != day.day_id
        && input.target_id != "day"
        && !current
            .sessions
            .iter()
            .any(|session| session.id == input.target_id)
        && !current
            .suggestions
            .iter()
            .any(|suggestion| suggestion.id == input.target_id)
    {
        return Err("recall_feedback_target_invalid".into());
    }
    input.day_id = super::day::day_key(&day);
    storage
        .with_connection(|connection| {
            super::review::set_feedback(connection, &input, super::worker::now_ms())
                .map_err(|_| rusqlite::Error::InvalidQuery)
        })
        .map_err(|_| "recall_feedback_failed")?;
    emit_changed(app, Some(&day.day_id));
    super::generation_service::get_day(&storage, &day)
}

pub fn correction(
    app: &tauri::AppHandle,
    mut input: SetRecallSessionCorrectionInput,
) -> Result<RecallDay, String> {
    let day = day_input(&input.day_id)?;
    let storage = app.state::<AppStorage>();
    if !super::generation_service::get_day(&storage, &day)?
        .sessions
        .iter()
        .any(|session| session.id == input.session_id)
    {
        return Err("recall_session_missing".into());
    }
    input.day_id = super::day::day_key(&day);
    storage
        .with_connection(|connection| {
            super::review::set_correction(connection, &input, super::worker::now_ms())
                .map_err(|_| rusqlite::Error::InvalidQuery)
        })
        .map_err(|_| "recall_correction_failed")?;
    emit_changed(app, Some(&day.day_id));
    super::generation_service::get_day(&storage, &day)
}

pub fn reviewed(
    app: &tauri::AppHandle,
    mut input: SetRecallDayReviewedInput,
) -> Result<RecallDay, String> {
    let day = day_input(&input.day_id)?;
    let storage = app.state::<AppStorage>();
    storage
        .with_connection(|connection| {
            storage::refresh_evidence(connection)?;
            let base = super::day::base_day(connection, &day)
                .map_err(|_| rusqlite::Error::InvalidQuery)?;
            input.day_id = super::day::day_key(&day);
            super::review::set_reviewed(connection, &input, &base.day.input_fingerprint)
                .map_err(|_| rusqlite::Error::InvalidQuery)
        })
        .map_err(|_| "recall_review_input_changed")?;
    emit_changed(app, Some(&day.day_id));
    super::generation_service::get_day(&storage, &day)
}

pub fn accept(
    app: &tauri::AppHandle,
    input: &AcceptRecallSuggestionInput,
) -> Result<RecallTask, String> {
    let day = day_input(&input.day_id)?;
    let storage = app.state::<AppStorage>();
    let id = super::generation_service::accept_suggestion(
        &storage,
        input,
        &day.timezone,
        super::worker::now_ms(),
    )?;
    emit_changed(app, Some(&day.day_id));
    get_task(&storage, &id)
}

pub fn dismiss(
    app: &tauri::AppHandle,
    input: &DismissRecallSuggestionInput,
) -> Result<RecallDay, String> {
    let day = day_input(&input.day_id)?;
    let storage = app.state::<AppStorage>();
    let accept = AcceptRecallSuggestionInput {
        day_id: input.day_id.clone(),
        suggestion_id: input.suggestion_id.clone(),
    };
    super::generation_service::dismiss_suggestion(
        &storage,
        &accept,
        &day.timezone,
        super::worker::now_ms(),
    )?;
    emit_changed(app, Some(&day.day_id));
    super::generation_service::get_day(&storage, &day)
}

pub fn update_task(
    app: &tauri::AppHandle,
    input: &UpdateRecallTaskInput,
) -> Result<RecallTask, String> {
    let storage = app.state::<AppStorage>();
    storage
        .with_connection(|connection| {
            super::tasks::update(connection, input, super::worker::now_ms())
                .map_err(|_| rusqlite::Error::InvalidQuery)
        })
        .map_err(|_| "recall_task_update_failed")?;
    emit_changed(app, None);
    get_task(&storage, &input.task_id)
}

fn get_task(storage: &AppStorage, id: &str) -> Result<RecallTask, String> {
    storage
        .with_connection(|connection| {
            storage::refresh_evidence(connection)?;
            let key = connection
                .query_row(
                    "SELECT day_key FROM recall_tasks WHERE id=?1",
                    [id],
                    |row| row.get::<_, String>(0),
                )
                .optional()?
                .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
            let mut task = super::tasks::list(connection, &key)
                .map_err(|_| rusqlite::Error::InvalidQuery)?
                .into_iter()
                .find(|task| task.id == id)
                .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
            task.day_id = key
                .split_once('@')
                .map_or(key.clone(), |(date, _)| date.into());
            Ok(task)
        })
        .map_err(|_| "recall_task_unavailable".into())
}
