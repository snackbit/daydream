use crate::{
    recorder::types::ProtectableWindow,
    settings::{
        ProtectedWindowAction, ProtectedWindowRule, ProtectedWindowRuleAction,
        ProtectedWindowTitleMatcherKind, RecordWindowProtectionSettings,
    },
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct MatchedRule {
    pub(super) action: ProtectedWindowAction,
    pub(super) label: String,
    pub(super) rule_id: String,
}

pub(super) fn match_window(
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

pub(super) fn rule_matches_window(rule: &ProtectedWindowRule, window: &ProtectableWindow) -> bool {
    let app_matches = optional_match(rule.app_identity.as_deref(), window.app_identity.as_deref());
    let process_matches =
        optional_match(rule.process_name.as_deref(), window.process_name.as_deref());
    let title_matches = rule.title_matcher.as_ref().is_some_and(|matcher| {
        window
            .title
            .as_deref()
            .is_some_and(|title| match matcher.kind {
                ProtectedWindowTitleMatcherKind::Contains => {
                    if matcher.case_sensitive {
                        title.contains(&matcher.value)
                    } else {
                        title.to_lowercase().contains(&matcher.value.to_lowercase())
                    }
                }
                ProtectedWindowTitleMatcherKind::Regex => regex::RegexBuilder::new(&matcher.value)
                    .case_insensitive(!matcher.case_sensitive)
                    .build()
                    .is_ok_and(|regex| regex.is_match(title)),
            })
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
