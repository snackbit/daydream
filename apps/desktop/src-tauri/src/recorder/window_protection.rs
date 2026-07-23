use std::{thread, time::Duration};

use tauri::{AppHandle, Manager, Runtime};
use uuid::Uuid;

use crate::{
    settings::{
        ProtectedWindowAction, ProtectedWindowRule, ProtectedWindowRuleAction,
        ProtectedWindowTitleMatcherKind, RecordWindowProtectionSettings,
    },
    storage::{
        self, AppStorage, CreateRecordingProtectionSpanInput, CreateRedactionInput,
        RecordingProtectionSpanRow,
    },
};

use super::{
    manager::{now_ms, RecorderManager},
    platform,
    types::{ProtectableWindow, RecorderState},
};

const POLL_MS: u64 = 500;

#[derive(Clone, Debug, PartialEq, Eq)]
struct MatchedRule {
    action: ProtectedWindowAction,
    label: String,
    rule_id: String,
}

#[derive(Debug)]
struct OpenProtection {
    action: ProtectedWindowAction,
    auto_pause_requested: bool,
    last_matched_at_ms: u64,
    rule_id: String,
    span_id: String,
}

pub(crate) fn start_window_protection_monitor<R: Runtime>(
    app: AppHandle<R>,
    recording_id: String,
    recording_started_at_ms: u64,
    settings: RecordWindowProtectionSettings,
) {
    if !settings.enabled || settings.rules.iter().all(|rule| !rule.enabled) {
        return;
    }

    let _ = thread::Builder::new()
        .name("daydream-window-protection".to_string())
        .spawn(move || {
            run_window_protection_monitor(app, recording_id, recording_started_at_ms, settings);
        });
}

pub(crate) fn materialize_closed_protection_spans(
    storage: &AppStorage,
    recording_id: &str,
) -> rusqlite::Result<()> {
    let spans = storage.with_connection(|connection| {
        storage::list_recording_protection_spans(connection, recording_id)
    })?;
    for span in spans
        .into_iter()
        .filter(|span| span.status == "closed" && span.redaction_id.is_none())
    {
        materialize_span(storage, &span)?;
    }

    Ok(())
}

pub(crate) fn close_open_protection_spans(
    storage: &AppStorage,
    recording_id: &str,
    end_offset_ms: i64,
    timestamp_ms: i64,
) -> rusqlite::Result<()> {
    let spans = storage.with_connection(|connection| {
        storage::list_open_recording_protection_spans(connection, recording_id)
    })?;
    for span in spans {
        storage.with_connection(|connection| {
            storage::close_recording_protection_span(
                connection,
                &span.id,
                end_offset_ms.max(span.start_offset_ms),
                timestamp_ms,
            )
        })?;
    }
    materialize_closed_protection_spans(storage, recording_id)
}

fn run_window_protection_monitor<R: Runtime>(
    app: AppHandle<R>,
    recording_id: String,
    recording_started_at_ms: u64,
    settings: RecordWindowProtectionSettings,
) {
    let storage = app.state::<AppStorage>();
    let manager = app.state::<RecorderManager>();
    let mut candidate: Option<(MatchedRule, u64)> = None;
    let mut open: Option<OpenProtection> = None;

    loop {
        let timestamp_ms = now_ms();
        let Ok(status) = manager.status() else {
            break;
        };
        if status.active_recording_id.as_deref() != Some(recording_id.as_str())
            || matches!(status.state, RecorderState::Idle | RecorderState::Error)
        {
            break;
        }

        let focused = platform::current_focused_window();
        let matched = focused
            .as_ref()
            .and_then(|window| match_window(&settings, window));

        match (&mut open, matched) {
            (Some(open_span), Some(matched_rule)) if open_span.rule_id == matched_rule.rule_id => {
                open_span.last_matched_at_ms = timestamp_ms;
            }
            (Some(open_span), Some(matched_rule)) => {
                if close_open_span_if_ready(
                    &storage,
                    open_span,
                    timestamp_ms,
                    recording_started_at_ms,
                    settings.merge_gap_ms.into(),
                ) {
                    if open_span.action == ProtectedWindowAction::Pause
                        && open_span.auto_pause_requested
                    {
                        super::commands::request_auto_resume(&app, &recording_id);
                    }
                    open = None;
                    candidate = Some((matched_rule, timestamp_ms));
                }
            }
            (Some(open_span), None) => {
                if close_open_span_if_ready(
                    &storage,
                    open_span,
                    timestamp_ms,
                    recording_started_at_ms,
                    settings.merge_gap_ms.into(),
                ) {
                    if open_span.action == ProtectedWindowAction::Pause
                        && open_span.auto_pause_requested
                    {
                        super::commands::request_auto_resume(&app, &recording_id);
                    }
                    open = None;
                }
            }
            (None, Some(matched_rule)) => match &candidate {
                Some((candidate_rule, since_ms))
                    if candidate_rule.rule_id == matched_rule.rule_id =>
                {
                    if timestamp_ms.saturating_sub(*since_ms) >= u64::from(settings.dwell_ms) {
                        if let Some(mut open_span) = open_span(
                            &storage,
                            &recording_id,
                            recording_started_at_ms,
                            *since_ms,
                            timestamp_ms,
                            &matched_rule,
                        ) {
                            if matched_rule.action == ProtectedWindowAction::Pause {
                                open_span.auto_pause_requested =
                                    super::commands::request_auto_pause(&app, &recording_id);
                            }
                            open = Some(open_span);
                        }
                    }
                }
                _ => candidate = Some((matched_rule, timestamp_ms)),
            },
            (None, None) => candidate = None,
        }

        thread::sleep(Duration::from_millis(POLL_MS));
    }
}

fn match_window(
    settings: &RecordWindowProtectionSettings,
    window: &ProtectableWindow,
) -> Option<MatchedRule> {
    settings
        .rules
        .iter()
        .filter(|rule| rule.enabled)
        .find(|rule| rule_matches_window(rule, window))
        .map(|rule| MatchedRule {
            action: match rule.action {
                ProtectedWindowRuleAction::Inherit => settings.default_action.clone(),
                ProtectedWindowRuleAction::Pause => ProtectedWindowAction::Pause,
                ProtectedWindowRuleAction::Redact => ProtectedWindowAction::Redact,
            },
            label: rule.label.clone(),
            rule_id: rule.id.clone(),
        })
}

fn rule_matches_window(rule: &ProtectedWindowRule, window: &ProtectableWindow) -> bool {
    let app_matches = optional_match(rule.app_identity.as_deref(), window.app_identity.as_deref());
    let process_matches =
        optional_match(rule.process_name.as_deref(), window.process_name.as_deref());
    let title_matches =
        rule.title_matcher
            .as_ref()
            .is_some_and(|matcher| match window.title.as_deref() {
                Some(title) => match matcher.kind {
                    ProtectedWindowTitleMatcherKind::Contains => {
                        contains_title(title, &matcher.value, matcher.case_sensitive)
                    }
                    ProtectedWindowTitleMatcherKind::Regex => {
                        regex::RegexBuilder::new(&matcher.value)
                            .case_insensitive(!matcher.case_sensitive)
                            .build()
                            .is_ok_and(|regex| regex.is_match(title))
                    }
                },
                None => false,
            });

    app_matches || process_matches || title_matches
}

fn optional_match(expected: Option<&str>, actual: Option<&str>) -> bool {
    let Some(expected) = expected.map(str::trim).filter(|value| !value.is_empty()) else {
        return false;
    };
    actual
        .map(str::trim)
        .is_some_and(|actual| actual.eq_ignore_ascii_case(expected))
}

fn contains_title(title: &str, needle: &str, case_sensitive: bool) -> bool {
    if case_sensitive {
        title.contains(needle)
    } else {
        title.to_lowercase().contains(&needle.to_lowercase())
    }
}

fn open_span(
    storage: &AppStorage,
    recording_id: &str,
    recording_started_at_ms: u64,
    candidate_since_ms: u64,
    timestamp_ms: u64,
    matched_rule: &MatchedRule,
) -> Option<OpenProtection> {
    let span_id = Uuid::now_v7().to_string();
    let start_offset_ms = candidate_since_ms
        .saturating_sub(recording_started_at_ms)
        .try_into()
        .unwrap_or(i64::MAX);
    let row = storage
        .with_connection(|connection| {
            storage::create_recording_protection_span(
                connection,
                &CreateRecordingProtectionSpanInput {
                    action: action_label(&matched_rule.action).to_string(),
                    id: span_id.clone(),
                    opened_at_ms: timestamp_ms.try_into().unwrap_or(i64::MAX),
                    recording_id: recording_id.to_string(),
                    rule_id: matched_rule.rule_id.clone(),
                    rule_label: matched_rule.label.clone(),
                    start_offset_ms,
                },
            )
        })
        .ok()?;

    Some(OpenProtection {
        action: matched_rule.action.clone(),
        auto_pause_requested: false,
        last_matched_at_ms: timestamp_ms,
        rule_id: matched_rule.rule_id.clone(),
        span_id: row.id,
    })
}

fn close_open_span_if_ready(
    storage: &AppStorage,
    open: &OpenProtection,
    timestamp_ms: u64,
    recording_started_at_ms: u64,
    merge_gap_ms: u64,
) -> bool {
    if timestamp_ms.saturating_sub(open.last_matched_at_ms) < merge_gap_ms {
        return false;
    }
    let end_offset_ms = open
        .last_matched_at_ms
        .saturating_add(merge_gap_ms)
        .saturating_sub(recording_started_at_ms)
        .try_into()
        .unwrap_or(i64::MAX);
    let closed_at_ms = timestamp_ms.try_into().unwrap_or(i64::MAX);
    let Ok(span) = storage.with_connection(|connection| {
        storage::close_recording_protection_span(
            connection,
            &open.span_id,
            end_offset_ms,
            closed_at_ms,
        )
    }) else {
        return true;
    };
    let _ = materialize_span(storage, &span);

    true
}

fn materialize_span(
    storage: &AppStorage,
    span: &RecordingProtectionSpanRow,
) -> rusqlite::Result<()> {
    let Some(end_offset_ms) = span.end_offset_ms else {
        return Ok(());
    };
    if end_offset_ms <= span.start_offset_ms {
        return Ok(());
    }
    let redaction_id = Uuid::now_v7().to_string();
    storage.with_connection(|connection| {
        storage::create_redaction(
            connection,
            &CreateRedactionInput {
                created_at_ms: span.closed_at_ms.unwrap_or(span.opened_at_ms),
                end_offset_ms,
                id: redaction_id.clone(),
                reason: Some(format!("Protected window: {}", span.rule_label)),
                recording_id: span.recording_id.clone(),
                start_offset_ms: span.start_offset_ms,
                track_ids_json: "[]".to_string(),
            },
        )?;
        storage::mark_recording_protection_span_materialized(connection, &span.id, &redaction_id)?;
        Ok(())
    })?;

    Ok(())
}

fn action_label(action: &ProtectedWindowAction) -> &'static str {
    match action {
        ProtectedWindowAction::Pause => "pause",
        ProtectedWindowAction::Redact => "redact",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn window(process_name: &str, title: &str) -> ProtectableWindow {
        ProtectableWindow {
            app_identity: Some(process_name.to_string()),
            id: "window-1".to_string(),
            platform: "test".to_string(),
            process_id: Some(1),
            process_name: Some(process_name.to_string()),
            title: Some(title.to_string()),
        }
    }

    #[test]
    fn rule_matches_process_or_title() {
        let rule = ProtectedWindowRule {
            action: ProtectedWindowRuleAction::Inherit,
            app_identity: None,
            enabled: true,
            id: "rule-1".to_string(),
            label: "Game".to_string(),
            process_name: Some("game".to_string()),
            title_matcher: Some(crate::settings::ProtectedWindowTitleMatcher {
                case_sensitive: false,
                kind: ProtectedWindowTitleMatcherKind::Contains,
                value: "private".to_string(),
            }),
        };

        assert!(rule_matches_window(&rule, &window("game", "Menu")));
        assert!(rule_matches_window(&rule, &window("other", "Private chat")));
        assert!(!rule_matches_window(&rule, &window("other", "Notes")));
    }
}
