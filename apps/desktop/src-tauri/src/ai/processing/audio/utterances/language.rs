//! Detect every window; stabilize changes with independent speech evidence.
use crate::{transcription::VerboseTranscription, whisper_backend};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::OnceLock};

pub(super) const HISTORY_MS: i64 = 300_000;
const CONFIRMATIONS: u8 = 3;
const MAX_PENDING: usize = 8;

#[derive(Clone, Default, Debug, Serialize, Deserialize, PartialEq)]
pub(super) struct State {
    pub confirmed: Option<String>,
    candidate: Option<String>,
    votes: u8,
    pending: Vec<String>,
    end_ms: i64,
    confirmed_evidence: Vec<String>,
    candidate_evidence: Vec<String>,
}
impl State {
    pub(super) fn confirmed_units(&self) -> impl Iterator<Item = &str> {
        self.confirmed_evidence.iter().map(String::as_str)
    }
    pub(super) fn supporting_units(&self) -> impl Iterator<Item = &str> {
        self.confirmed_evidence
            .iter()
            .chain(&self.candidate_evidence)
            .map(String::as_str)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct Decision {
    pub state: State,
    pub detected: Option<String>,
    pub output_language: Option<String>,
    pub correction: Option<String>,
    pub repairs: Vec<String>,
}

pub(super) fn normalize(value: &str) -> Option<String> {
    static LANGUAGES: OnceLock<HashMap<String, String>> = OnceLock::new();
    LANGUAGES
        .get_or_init(|| {
            whisper_backend::supported_languages()
                .into_iter()
                .flat_map(|language| {
                    [
                        (language.id.clone(), language.id.clone()),
                        (language.label.to_lowercase(), language.id),
                    ]
                })
                .collect()
        })
        .get(&value.trim().to_lowercase())
        .cloned()
}

pub(super) fn plan(
    mut state: State,
    unit_id: &str,
    start: i64,
    end: i64,
    value: &VerboseTranscription,
) -> Decision {
    if start.saturating_sub(state.end_ms) > HISTORY_MS {
        state = State::default();
    }
    let detected = value.language.as_deref().and_then(normalize);
    let has_text = value
        .segments
        .iter()
        .any(|segment| !segment.text.trim().is_empty());
    let substantial = value
        .segments
        .iter()
        .flat_map(|segment| segment.text.chars())
        .filter(|letter| letter.is_alphabetic())
        .take(12)
        .count()
        >= 12;
    let confident = value
        .language_confidence
        .is_none_or(|confidence| confidence.is_finite() && (0.6..=1.0).contains(&confidence));
    let mut repairs = Vec::new();
    if has_text && detected != state.confirmed {
        state.pending.push(unit_id.into());
        if state.pending.len() > MAX_PENDING {
            state.pending.remove(0);
        }
    }
    if substantial && confident {
        if let Some(language) = &detected {
            if state.confirmed.as_ref() == Some(language) {
                state.candidate = None;
                state.votes = 0;
                state.pending.clear();
                state.candidate_evidence.clear();
                state.confirmed_evidence.push(unit_id.into());
                if state.confirmed_evidence.len() > usize::from(CONFIRMATIONS) {
                    state.confirmed_evidence.remove(0);
                }
            } else {
                if state.candidate.as_ref() == Some(language) {
                    state.votes += 1;
                } else {
                    state.candidate = Some(language.clone());
                    state.votes = 1;
                    state.candidate_evidence.clear();
                }
                state.candidate_evidence.push(unit_id.into());
                if state.votes >= CONFIRMATIONS {
                    state.confirmed = Some(language.clone());
                    state.candidate = None;
                    state.votes = 0;
                    state.confirmed_evidence = std::mem::take(&mut state.candidate_evidence);
                    repairs = std::mem::take(&mut state.pending)
                        .into_iter()
                        .filter(|id| id != unit_id)
                        .collect();
                }
            }
        }
    }
    state.end_ms = end;
    let correction = if has_text && detected.is_some() && detected != state.confirmed {
        state.confirmed.clone()
    } else {
        None
    };
    Decision {
        state,
        output_language: detected.clone(),
        detected,
        correction,
        repairs,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sample(language: &str, text: &str) -> VerboseTranscription {
        let mut result = crate::ai::tests::test_verbose_transcription(0., 1., text, vec![]);
        result.language = Some(language.into());
        result.language_confidence = Some(0.9);
        result
    }
    #[test]
    fn ambiguous_acknowledgement_does_not_pin_after_a_gap_and_is_repaired_after_consensus() {
        let mut state = State::default();
        let weak = plan(state, "ack", 20_000, 21_000, &sample("en", "Mm-hmm."));
        assert_eq!(weak.state.confirmed, None);
        state = weak.state;
        for number in 1..=3 {
            let decision = plan(
                state,
                &format!("german-{number}"),
                21_000 + number * 1000,
                22_000 + number * 1000,
                &sample("German", "Dies ist ein synthetischer Beispielsatz."),
            );
            assert_eq!(
                decision.state.confirmed.as_deref(),
                if number == 3 { Some("de") } else { None }
            );
            if number == 3 {
                assert!(decision.repairs.contains(&"ack".into()));
            }
            state = decision.state;
        }
    }
    #[test]
    fn real_change_needs_three_windows_and_repairs_the_tentative_outputs() {
        let mut state = State {
            confirmed: Some("de".into()),
            ..State::default()
        };
        for number in 1..=3 {
            let decision = plan(
                state,
                &format!("english-{number}"),
                number * 1000,
                number * 1000 + 1000,
                &sample("English", "This is a synthetic English sentence."),
            );
            assert_eq!(
                decision.correction.as_deref(),
                if number < 3 { Some("de") } else { None }
            );
            if number == 3 {
                assert_eq!(decision.repairs, ["english-1", "english-2"]);
                assert_eq!(decision.state.confirmed.as_deref(), Some("en"));
            }
            state = decision.state;
        }
    }
    #[test]
    fn weak_or_invalid_evidence_never_confirms_and_history_is_bounded() {
        let mut state = State::default();
        for number in 0..20 {
            let decision = plan(
                state,
                &number.to_string(),
                number * 1000,
                number * 1000 + 1000,
                &sample("en", "Yes."),
            );
            assert!(decision.state.confirmed.is_none());
            assert!(decision.state.pending.len() <= 8);
            state = decision.state;
        }
        let mut invalid = sample("de", "Synthetic substantial speech.");
        invalid.language_confidence = Some(f64::NAN);
        assert_eq!(
            plan(state.clone(), "invalid", 20_000, 21_000, &invalid)
                .state
                .votes,
            0
        );
        invalid.language_confidence = Some(0.9);
        invalid.language = Some("unsupported".into());
        assert_eq!(
            plan(state, "unknown", 21_000, 22_000, &invalid).state.votes,
            0
        );
        let previous = State {
            confirmed: Some("de".into()),
            ..State::default()
        };
        assert!(plan(
            previous,
            "late",
            HISTORY_MS + 1,
            HISTORY_MS + 1000,
            &sample("en", "Yes.")
        )
        .state
        .confirmed
        .is_none());
    }
}
