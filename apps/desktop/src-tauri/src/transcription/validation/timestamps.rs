use crate::transcription::VerboseTranscription;

/// Timestamps are relative to the submitted speech window, not its parent chunk.
pub(crate) fn constrain_transcription(
    mut value: VerboseTranscription,
    duration_ms: i64,
) -> VerboseTranscription {
    let limit = duration_ms.max(0) as f64 / 1_000.0;
    value.segments.retain_mut(|segment| {
        if !valid_overlap(segment.start, segment.end, limit) || segment.text.trim().is_empty() {
            return false;
        }
        segment.start = segment.start.max(0.0);
        segment.end = segment.end.min(limit);
        segment.end > segment.start
    });
    value.words.retain_mut(|word| {
        if !valid_overlap(word.start, word.end, limit)
            || word.word.trim().is_empty()
            || word
                .probability
                .is_some_and(|p| !p.is_finite() || !(0.0..=1.0).contains(&p))
        {
            return false;
        }
        word.start = word.start.max(0.0);
        word.end = word.end.min(limit);
        value
            .segments
            .iter()
            .any(|segment| word.start >= segment.start && word.end <= segment.end)
    });
    value.text = value
        .segments
        .iter()
        .map(|segment| segment.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    value.duration = Some(limit);
    value
}

fn valid_overlap(start: f64, end: f64, limit: f64) -> bool {
    start.is_finite() && end.is_finite() && end > start && end > 0.0 && start < limit
}

/// Neighboring context protects whole words. Assign a word once, using its
/// midpoint, while retaining its full timestamps inside the submitted window.
pub(crate) fn constrain_owned_transcription(
    value: VerboseTranscription,
    duration_ms: i64,
    owned: &[(f64, f64)],
) -> VerboseTranscription {
    let mut value = constrain_transcription(value, duration_ms);
    let belongs = |start: f64, end: f64| {
        owned.iter().any(|(from, to)| {
            let midpoint = (start + end) / 2.0;
            midpoint >= *from && midpoint < *to
        })
    };
    if value.words.is_empty() {
        value
            .segments
            .retain(|segment| belongs(segment.start, segment.end));
    } else {
        let original_words = value.words.clone();
        value.words.retain(|word| belongs(word.start, word.end));
        value.segments.retain_mut(|segment| {
            let words = original_words
                .iter()
                .filter(|word| word.start >= segment.start && word.end <= segment.end)
                .collect::<Vec<_>>();
            let kept = words
                .iter()
                .filter(|word| belongs(word.start, word.end))
                .copied()
                .collect::<Vec<_>>();
            if kept.is_empty() {
                return false;
            }
            if kept.len() != words.len() {
                // Align against the original text instead of inventing spaces
                // between word tokens (which would change CJK transcripts).
                let mut cursor = 0;
                let mut first = None;
                let mut last = 0;
                let mut aligned = true;
                for word in words {
                    let text = word.word.trim();
                    let Some(offset) = segment.text[cursor..].find(text) else {
                        aligned = false;
                        break;
                    };
                    let start = cursor + offset;
                    cursor = start + text.len();
                    if belongs(word.start, word.end) {
                        first.get_or_insert(start);
                        last = cursor;
                    }
                }
                if !aligned {
                    // A provider's token spelling can differ from its segment
                    // text. Keep the original segment with coarse ownership
                    // rather than losing an otherwise valid utterance.
                    return belongs(segment.start, segment.end);
                }
                let Some(first) = first else {
                    return false;
                };
                segment.text = segment.text[first..last].trim().to_string();
            }
            segment.start = kept.first().map(|word| word.start).unwrap_or(segment.start);
            segment.end = kept.last().map(|word| word.end).unwrap_or(segment.end);
            true
        });
    }
    value.words.retain(|word| {
        value
            .segments
            .iter()
            .any(|segment| word.start >= segment.start && word.end <= segment.end)
    });
    value.text = value
        .segments
        .iter()
        .map(|segment| segment.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    value
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transcription::{VerboseTranscriptionSegment, VerboseTranscriptionWord};
    #[test]
    fn short_reply_cannot_occupy_a_thirty_second_chunk() {
        let value = VerboseTranscription {
            duration: None,
            language: None,
            task: None,
            text: "Synthetic reply".into(),
            language_confidence: None,
            segments: vec![VerboseTranscriptionSegment {
                id: 0,
                start: 0.1,
                end: 30.0,
                text: "Synthetic reply".into(),
            }],
            words: vec![
                VerboseTranscriptionWord {
                    start: 0.1,
                    end: 0.5,
                    word: "Synthetic".into(),
                    probability: None,
                },
                VerboseTranscriptionWord {
                    start: 0.5,
                    end: 30.0,
                    word: "reply".into(),
                    probability: None,
                },
                VerboseTranscriptionWord {
                    start: 3.0,
                    end: 4.0,
                    word: "outside".into(),
                    probability: None,
                },
            ],
        };
        let value = constrain_transcription(value, 1_200);
        assert_eq!(value.segments[0].end, 1.2);
        assert_eq!(value.words.len(), 2);
        assert_eq!(value.words[1].end, 1.2);
    }
    #[test]
    fn invalid_and_wholly_outside_results_are_discarded() {
        let value = VerboseTranscription {
            duration: None,
            language: None,
            task: None,
            text: "Untrusted".into(),
            language_confidence: None,
            segments: vec![
                VerboseTranscriptionSegment {
                    id: 0,
                    start: f64::NAN,
                    end: 1.0,
                    text: "Invalid".into(),
                },
                VerboseTranscriptionSegment {
                    id: 1,
                    start: 2.0,
                    end: 3.0,
                    text: "Outside".into(),
                },
            ],
            words: vec![],
        };
        let value = constrain_transcription(value, 1_000);
        assert!(value.segments.is_empty() && value.words.is_empty() && value.text.is_empty());
    }
    #[test]
    fn a_reply_crossing_a_chunk_boundary_is_kept_once_with_its_real_endpoint() {
        let value = VerboseTranscription {
            duration: None,
            language: Some("en".into()),
            language_confidence: None,
            task: None,
            text: "Yes.".into(),
            segments: vec![VerboseTranscriptionSegment {
                id: 0,
                start: 0.3,
                end: 0.78,
                text: "Yes.".into(),
            }],
            words: vec![VerboseTranscriptionWord {
                start: 0.3,
                end: 0.78,
                word: "Yes.".into(),
                probability: None,
            }],
        };
        let before = constrain_owned_transcription(value.clone(), 930, &[(0.0, 0.5)]);
        let after = constrain_owned_transcription(value, 930, &[(0.5, 0.93)]);
        assert!(before.text.is_empty());
        assert_eq!(after.text, "Yes.");
        assert_eq!(after.words[0].end, 0.78);
    }
}
