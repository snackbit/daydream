use super::{
    matching::{match_window, MatchedRule},
    persistence::{close_span, create_span},
};
use crate::{
    recorder::{
        errors::dependency_error,
        types::{ProtectableWindowProviderStatus, RecorderError, WindowMetadataSnapshot},
    },
    settings::{ProtectedWindowAction, RecordWindowProtectionSettings},
    storage::AppStorage,
};

#[derive(Default)]
pub(super) struct ProtectionState {
    candidate: Option<(MatchedRule, u64)>,
    open: Option<OpenProtection>,
}
struct OpenProtection {
    action: ProtectedWindowAction,
    last_matched_at_ms: u64,
    span_id: String,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum ProtectionDecision {
    None,
    Pause,
    Resume,
}

impl ProtectionState {
    pub(super) fn classified_through_ms(&self, started: u64, now: u64) -> i64 {
        self.candidate
            .as_ref()
            .map(|(_, since)| *since)
            .unwrap_or(now)
            .saturating_sub(started)
            .try_into()
            .unwrap_or(i64::MAX)
    }
    pub(super) fn observe(
        &mut self,
        storage: &AppStorage,
        recording_id: &str,
        started_at_ms: u64,
        settings: &RecordWindowProtectionSettings,
        timestamp_ms: u64,
        snapshot: WindowMetadataSnapshot,
    ) -> Result<ProtectionDecision, RecorderError> {
        // Unknown metadata is never evidence that a protected window disappeared.
        // Stop this recording rather than record an interval we cannot classify.
        if snapshot.status != ProtectableWindowProviderStatus::Available {
            return Err(dependency_error(
                "recorder_window_protection_lost",
                "Focused-window detection was lost. Recording stopped to protect private content.",
            ));
        }
        let matched = snapshot
            .focused_window
            .as_ref()
            .and_then(|window| match_window(settings, window));
        match (&mut self.open, matched) {
            (Some(open), Some(rule)) => {
                open.last_matched_at_ms = timestamp_ms;
                self.candidate = None;
                if rule.action == ProtectedWindowAction::Pause
                    && open.action != ProtectedWindowAction::Pause
                {
                    open.action = ProtectedWindowAction::Pause;
                    return Ok(ProtectionDecision::Pause);
                }
            }
            (Some(open), None) => {
                if timestamp_ms.saturating_sub(open.last_matched_at_ms)
                    >= u64::from(settings.merge_gap_ms)
                {
                    let end = open
                        .last_matched_at_ms
                        .saturating_add(settings.merge_gap_ms.into())
                        .saturating_sub(started_at_ms)
                        .try_into()
                        .unwrap_or(i64::MAX);
                    close_span(
                        storage,
                        &open.span_id,
                        end,
                        timestamp_ms.try_into().unwrap_or(i64::MAX),
                    )
                    .map_err(protection_storage_error)?;
                    let resume = open.action == ProtectedWindowAction::Pause;
                    self.open = None;
                    self.candidate = None;
                    return Ok(if resume {
                        ProtectionDecision::Resume
                    } else {
                        ProtectionDecision::None
                    });
                }
            }
            (None, Some(rule)) => {
                let since = self
                    .candidate
                    .as_ref()
                    .filter(|(candidate, _)| candidate.rule_id == rule.rule_id)
                    .map(|(_, since)| *since)
                    .unwrap_or(timestamp_ms);
                if timestamp_ms.saturating_sub(since) >= u64::from(settings.dwell_ms) {
                    let id = create_span(
                        storage,
                        recording_id,
                        started_at_ms,
                        since,
                        timestamp_ms,
                        &rule,
                    )
                    .map_err(protection_storage_error)?;
                    let pause = rule.action == ProtectedWindowAction::Pause;
                    self.open = Some(OpenProtection {
                        action: rule.action,
                        last_matched_at_ms: timestamp_ms,
                        span_id: id,
                    });
                    self.candidate = None;
                    return Ok(if pause {
                        ProtectionDecision::Pause
                    } else {
                        ProtectionDecision::None
                    });
                }
                self.candidate = Some((rule, since));
            }
            (None, None) => self.candidate = None,
        }
        Ok(ProtectionDecision::None)
    }
}

pub(super) fn protection_storage_error(_: rusqlite::Error) -> RecorderError {
    dependency_error("recorder_window_protection_storage_failed", "Protected-window metadata could not be saved. Recording stopped; processing remains blocked until protection is repaired.")
}
