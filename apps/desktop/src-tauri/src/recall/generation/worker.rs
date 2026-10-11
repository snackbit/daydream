use super::{generation::Profile, types::*};
use crate::{
    settings,
    storage::{self, AppStorage},
};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    sync::atomic::AtomicBool,
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::Manager;

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(i64::MAX as u128) as i64
}
pub fn key(
    storage: &AppStorage,
    auth: &str,
    reference: Option<&str>,
) -> Result<Option<String>, String> {
    if auth == "none" {
        return Ok(None);
    }
    if auth != "api-key" {
        return Err("recall_profile_invalid".into());
    }
    let reference = reference.ok_or("recall_key_missing")?;
    let envelope = storage
        .with_connection(|connection| storage::get_app_setting(connection, reference))
        .map_err(|_| "recall_storage_failed")?
        .ok_or("recall_key_missing")?;
    crate::secret_crypto::decrypt_sensitive_setting_with_platform_key(&envelope.value_json)
        .map(Some)
        .map_err(|_| "recall_key_unavailable".into())
}
pub fn profile(storage: &AppStorage) -> Result<Profile, String> {
    let settings =
        settings::load_app_settings(storage).map_err(|_| "recall_settings_unavailable")?;
    profile_from(storage, &settings.ai.recall.generation, None)
}
pub fn profile_from(
    storage: &AppStorage,
    settings: &settings::RecallExternalProfile,
    draft: Option<String>,
) -> Result<Profile, String> {
    if !settings.enabled {
        return Err("recall_generation_disabled".into());
    }
    if let Some(route) = &settings.model_route {
        if route.subscription
            && route
                .subscription_id
                .as_deref()
                .is_none_or(|id| !crate::model_registry::subscription::configured(storage, id))
        {
            return Err("recall_provider_subscription_reconnect".into());
        }
    }
    let api_key = if settings.auth_mode == "none"
        || settings
            .model_route
            .as_ref()
            .is_some_and(|route| route.subscription_id.is_some())
    {
        None
    } else if draft.is_some() {
        draft
    } else {
        key(
            storage,
            &settings.auth_mode,
            settings.api_key_ref.as_deref(),
        )?
    };
    let fingerprint = hex::encode(Sha256::digest(
        serde_json::to_vec(settings).map_err(|_| "recall_profile_invalid")?,
    ));
    Ok(Profile {
        route: settings.model_route.clone(),
        storage: Some(storage.clone()),
        enabled: settings.enabled,
        base_url: settings.base_url.clone().ok_or("recall_profile_invalid")?,
        model: settings.model.clone().ok_or("recall_profile_invalid")?,
        api_key,
        fingerprint,
    })
}
pub fn regenerate(storage: &AppStorage, input: &RecallDayInput) -> Result<String, String> {
    let profile = profile(storage)?;
    let day = super::generation_service::get_day(storage, input)?;
    let key = format!("{}@{}", input.day_id, input.timezone);
    let settings =
        settings::load_app_settings(storage).map_err(|_| "recall_settings_unavailable")?;
    let snapshot =
        serde_json::to_string(&settings.ai.recall).map_err(|_| "recall_profile_invalid")?;
    storage
        .with_connection(|connection| {
            let id = super::jobs::enqueue(
                connection,
                &key,
                &day.day.input_fingerprint,
                &profile.fingerprint,
                now_ms(),
            )
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
            connection.execute(
                "UPDATE recall_jobs SET config_json=?2,manual=1 WHERE id=?1",
                rusqlite::params![id, snapshot],
            )?;
            Ok(id)
        })
        .map_err(|_| "recall_storage_failed".into())
}
pub fn start_worker(app: tauri::AppHandle) {
    super::index_worker::start(app.clone());
    tauri::async_runtime::spawn(async move {
        let storage = app.state::<AppStorage>();
        let _ = storage.with_connection(|connection| super::jobs::recover(connection, now_ms()));
        let mut observed: HashMap<String, (String, i64)> = HashMap::new();
        let mut last_started: HashMap<String, i64> = HashMap::new();
        let mut cached_cards = Vec::new();
        let mut last_revision = None;
        let mut scheduled = std::collections::HashSet::new();
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
            let Ok(_activity) = crate::maintenance::runtime::admit(&app, "recall-generation")
            else {
                continue;
            };
            let Ok(settings) = settings::load_app_settings(&storage) else {
                continue;
            };
            let Ok(timezone) = super::day::local_timezone() else {
                continue;
            };
            let active_profile = match profile(&storage) {
                Ok(profile) => profile,
                Err(error) => {
                    if error != "recall_generation_disabled" {
                        tracing::warn!(
                            error_code = crate::telemetry::failures::safe_error_code(&error),
                            stage = "profile",
                            "recall_generation_unavailable"
                        );
                    }
                    let status = if error == "recall_generation_disabled" {
                        "cancelled"
                    } else {
                        "failed"
                    };
                    let changed=storage.with_connection(|connection|connection.execute("UPDATE recall_jobs SET status=?1,error_code=?2,updated_at_ms=?3 WHERE kind='generation' AND status='pending'",rusqlite::params![status,error,now_ms()])).unwrap_or(0);
                    if changed > 0 {
                        super::mutations::emit_changed(&app, None)
                    }
                    continue;
                }
            };
            let Ok(revision) = storage.with_connection(storage::refresh_evidence) else {
                continue;
            };
            let changed = last_revision.as_ref() != Some(&(revision, timezone.clone()));
            if changed {
                let Ok(cards) = discover_days(&storage, &timezone) else {
                    continue;
                };
                cached_cards = cards;
                last_revision = Some((revision, timezone.clone()));
            }
            let now = now_ms();
            if settings.ai.recall.auto_generate {
                for card in cached_cards.iter().rev() {
                    let key = format!("{}@{}", card.day_id, timezone);
                    let fingerprint = if !changed && observed.contains_key(&key) {
                        observed[&key].0.clone()
                    } else {
                        match super::generation_service::get_day(
                            &storage,
                            &RecallDayInput {
                                day_id: card.day_id.clone(),
                                timezone: timezone.clone(),
                            },
                        ) {
                            Ok(day) => match storage
                                .with_connection(|connection| {
                                    storage::list_evidence_range(
                                        connection,
                                        day.day.start_ms,
                                        day.day.end_ms,
                                    )
                                })
                                .ok()
                                .and_then(|items| {
                                    super::generation::input_fingerprint(
                                        &super::generation_store::inputs(&items),
                                    )
                                    .ok()
                                }) {
                                Some(fingerprint) => fingerprint,
                                None => continue,
                            },
                            Err(_) => continue,
                        }
                    };
                    let entry = observed
                        .entry(key.clone())
                        .or_insert((fingerprint.clone(), now));
                    if entry.0 != fingerprint {
                        *entry = (fingerprint.clone(), now);
                    }
                    if card.evidence_count == 0
                        || now.saturating_sub(entry.1) < super::scheduler::QUIET_MS
                    {
                        continue;
                    }
                    let schedule_key = (
                        key.clone(),
                        fingerprint.clone(),
                        active_profile.fingerprint.clone(),
                    );
                    if scheduled.contains(&schedule_key) {
                        continue;
                    }
                    let needs=storage.with_connection(|connection| Ok(connection.query_row("SELECT 1 FROM recall_generations WHERE day_key=?1 AND input_fingerprint=?2 AND profile_fingerprint=?3 AND status='ready'",rusqlite::params![key,fingerprint,active_profile.fingerprint], |_|Ok(())).is_err())).unwrap_or(false);
                    let queued=storage.with_connection(|connection|connection.query_row("SELECT COUNT(*) FROM recall_jobs WHERE day_key=?1 AND input_fingerprint=?2 AND config_fingerprint=?3 AND status IN('pending','running','paused','failed')",rusqlite::params![key,fingerprint,active_profile.fingerprint],|row|row.get::<_,i64>(0))).unwrap_or(1);
                    if needs && queued == 0 {
                        let enqueued = storage.with_connection(|connection| {
                            let id = super::jobs::enqueue(
                                connection,
                                &key,
                                &fingerprint,
                                &active_profile.fingerprint,
                                now,
                            )
                            .map_err(|_| rusqlite::Error::InvalidQuery)?;
                            connection.execute(
                                "UPDATE recall_jobs SET config_json=?2 WHERE id=?1",
                                rusqlite::params![
                                    id,
                                    serde_json::to_string(&settings.ai.recall).unwrap_or_default()
                                ],
                            )?;
                            Ok(id)
                        });
                        if enqueued.is_ok() {
                            scheduled.insert(schedule_key);
                        }
                    } else {
                        scheduled.insert(schedule_key);
                    }
                }
            }
            let candidate=storage.with_connection(|connection| {
                use rusqlite::OptionalExtension;
                connection.query_row("SELECT id,day_key,manual,config_fingerprint FROM recall_jobs WHERE kind='generation' AND status='pending' AND (next_attempt_at_ms IS NULL OR next_attempt_at_ms<=?1) ORDER BY created_at_ms DESC LIMIT 1",[now],|row|Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?,row.get::<_,bool>(2)?,row.get::<_,String>(3)?))).optional()
            }).ok().flatten();
            let Some((job, key, manual, config)) = candidate else {
                continue;
            };
            if config != active_profile.fingerprint {
                let _=storage.with_connection(|connection|connection.execute("UPDATE recall_jobs SET status='cancelled',error_code='recall_profile_changed' WHERE id=?1",[&job]));
                continue;
            }
            let recording = app
                .state::<crate::recorder::RecorderManager>()
                .status()
                .map(|status| status.state == crate::recorder::RecorderState::Recording)
                .unwrap_or(true);
            let durable_last = storage
                .with_connection(|connection| {
                    connection.query_row(
                        "SELECT MAX(started_at_ms) FROM recall_jobs WHERE day_key=?1",
                        [&key],
                        |row| row.get::<_, Option<i64>>(0),
                    )
                })
                .ok()
                .flatten();
            let last = last_started.get(&key).copied().or(durable_last);
            let changed_at = observed.get(&key).map(|entry| entry.1).unwrap_or(now);
            if !super::scheduler::generation_due(now, changed_at, last, recording, manual) {
                continue;
            }
            let Some((date, zone)) = key.split_once('@') else {
                continue;
            };
            let input = RecallDayInput {
                day_id: date.into(),
                timezone: zone.into(),
            };
            let Ok(day) = super::generation_service::get_day(&storage, &input) else {
                continue;
            };
            let consumed = storage
                .with_connection(|connection| {
                    storage::list_evidence_range(connection, day.day.start_ms, day.day.end_ms)
                })
                .map(|items| super::generation_store::inputs(&items));
            let Ok(consumed) = consumed else { continue };
            let fingerprint = super::generation::input_fingerprint(&consumed).unwrap_or_default();
            let started=storage.with_connection(|connection|connection.execute("UPDATE recall_jobs SET status='running',config_fingerprint=?2,updated_at_ms=?3,started_at_ms=?3,input_fingerprint=?4 WHERE id=?1 AND status='pending'",rusqlite::params![job,active_profile.fingerprint,now,fingerprint])).unwrap_or(0);
            if started != 1 {
                continue;
            }
            super::mutations::emit_changed(&app, Some(date));
            last_started.insert(key.clone(), now);
            let cancelled = AtomicBool::new(false);
            let generation = super::generation_windows::generate(
                &storage,
                &key,
                &job,
                &active_profile,
                &consumed,
                &cancelled,
            );
            tokio::pin!(generation);
            let result = loop {
                tokio::select! {
                    result=&mut generation=>break result,
                    _=tokio::time::sleep(std::time::Duration::from_secs(1))=> {
                        let running=storage.with_connection(|connection|connection.query_row("SELECT status FROM recall_jobs WHERE id=?1",[&job],|row|row.get::<_,String>(0))).is_ok_and(|status|status=="running");
                        let current_profile=profile(&storage);
                        let current_inputs=storage.with_connection(|connection| {storage::refresh_evidence(connection)?;storage::list_evidence_range(connection,day.day.start_ms,day.day.end_ms)}).map(|items|super::generation_store::inputs(&items));
                        let stale=app.state::<crate::maintenance::RuntimeActivityRegistry>().is_maintenance_active() || !running || current_profile.as_ref().map(|current|current.fingerprint!=active_profile.fingerprint).unwrap_or(true)
                            || current_inputs.ok().and_then(|current|super::generation::input_fingerprint(&current).ok())!=super::generation::input_fingerprint(&consumed).ok();
                        if stale {cancelled.store(true,std::sync::atomic::Ordering::Release);break Err("recall_cancelled".into());}
                    }
                }
            };
            let result: Result<(), String> = match result {
                Ok(output) => {
                    let current_profile = profile_from(
                        &storage,
                        &settings::load_app_settings(&storage)
                            .map(|settings| settings.ai.recall.generation)
                            .unwrap_or(settings.ai.recall.generation.clone()),
                        None,
                    );
                    if current_profile
                        .as_ref()
                        .map(|current| current.fingerprint != active_profile.fingerprint)
                        .unwrap_or(true)
                    {
                        Err("recall_profile_changed".into())
                    } else {
                        storage
                            .with_connection_mut(|connection| {
                                storage::refresh_evidence(connection)?;
                                let current = storage::list_evidence_range(
                                    connection,
                                    day.day.start_ms,
                                    day.day.end_ms,
                                )?;
                                super::generation_store::publish(
                                    connection,
                                    super::generation_store::PublicationContext {
                                        day_key: &key,
                                        job_id: &job,
                                        profile: &active_profile,
                                        now_ms: now_ms(),
                                    },
                                    &consumed,
                                    &super::generation_store::inputs(&current),
                                    &output,
                                )
                                .map_err(|_| rusqlite::Error::InvalidQuery)
                            })
                            .map_err(|_| "recall_input_changed".into())
                    }
                }
                Err(error) => Err(error),
            };
            if let Err(error) = result {
                if error == "recall_cancelled" || error == "recall_profile_changed" {
                    tracing::info!(job_id = %job, "recall_generation_cancelled");
                } else {
                    tracing::warn!(job_id = %job, error_code = crate::telemetry::failures::safe_error_code(&error), "recall_generation_failed");
                }
                let _ = storage.with_connection(|connection| {
                    super::jobs::fail(connection, &job, &error, now_ms())
                });
            }
            super::mutations::emit_changed(&app, Some(date));
        }
    });
}

fn discover_days(storage: &AppStorage, timezone: &str) -> Result<Vec<RecallDayCard>, String> {
    let mut cards = Vec::new();
    let mut before = None;
    loop {
        let batch = super::day::list_days(
            storage,
            &ListRecallDaysInput {
                before_day_id: before.clone(),
                limit: 100,
                timezone: timezone.into(),
            },
        )?;
        let exhausted = batch.len() < 100;
        let next = batch.last().map(|card| card.day_id.clone());
        if next.is_some() && next == before {
            return Err("recall_day_pagination_invalid".into());
        }
        cards.extend(batch);
        if exhausted {
            return Ok(cards);
        }
        before = next;
    }
}
