use crate::transcription::VerboseTranscription;

/// Reconcile only the shared source interval. Both decodes then use the same
/// timestamp anchor, so midpoint ownership cannot keep a boundary word twice.
pub(super) fn reconcile(
    left: &mut VerboseTranscription,
    left_base: i64,
    right: &mut VerboseTranscription,
    right_base: i64,
    shared_start: i64,
    shared_end: i64,
) {
    let from = shared_start as f64 / 1000.0;
    let to = shared_end as f64 / 1000.0;
    let lb = left_base as f64 / 1000.0;
    let rb = right_base as f64 / 1000.0;
    let mut cursor = 0;
    for word in &left.words {
        let start = lb + word.start;
        let end = lb + word.end;
        if start >= to || end <= from {
            continue;
        }
        let wanted = normalized(&word.word);
        if wanted.is_empty() {
            continue;
        }
        if let Some(index) = (cursor..right.words.len())
            .filter(|index| {
                let candidate = &right.words[*index];
                let cs = rb + candidate.start;
                let ce = rb + candidate.end;
                cs < to
                    && ce > from
                    && ((cs + ce - start - end) / 2.0).abs() <= 0.75
                    && normalized(&candidate.word) == wanted
            })
            .min_by(|a, b| {
                let distance = |index: usize| {
                    ((rb + right.words[index].start + rb + right.words[index].end - start - end)
                        / 2.0)
                        .abs()
                };
                distance(*a).total_cmp(&distance(*b))
            })
        {
            let original = &right.words[index];
            for segment in &mut right.segments {
                if original.start >= segment.start && original.end <= segment.end {
                    segment.start = segment.start.min(start - rb);
                    segment.end = segment.end.max(end - rb);
                }
            }
            right.words[index].start = start - rb;
            right.words[index].end = end - rb;
            cursor = index + 1;
        }
    }
    // Coarse providers can anchor identical segments; unmatched segments remain
    // intact. There is no invented timing or unrestricted text deduplication.
    if left.words.is_empty() || right.words.is_empty() {
        let mut cursor = 0;
        for segment in &left.segments {
            let start = lb + segment.start;
            let end = lb + segment.end;
            if start >= to || end <= from {
                continue;
            }
            if let Some(index) = (cursor..right.segments.len()).find(|index| {
                let candidate = &right.segments[*index];
                let cs = rb + candidate.start;
                let ce = rb + candidate.end;
                cs < to
                    && ce > from
                    && (cs - start).abs() <= 0.75
                    && (ce - end).abs() <= 0.75
                    && normalized(&candidate.text) == normalized(&segment.text)
            }) {
                right.segments[index].start = start - rb;
                right.segments[index].end = end - rb;
                cursor = index + 1;
            }
        }
        if let (Some(last), Some(first)) = (left.segments.last(), right.segments.first_mut()) {
            let ls = lb + last.start;
            let le = lb + last.end;
            let rs = rb + first.start;
            let re = rb + first.end;
            if ls < to && le > from && rs < to && re > from && ls < rs && le < re {
                let tail = word_spans(&last.text);
                let head = word_spans(&first.text);
                let overlap = (2..=tail.len().min(head.len())).rev().find(|length| {
                    tail[tail.len() - length..]
                        .iter()
                        .zip(&head[..*length])
                        .all(|((a, b), (c, d))| {
                            normalized(&last.text[*a..*b]) == normalized(&first.text[*c..*d])
                        })
                });
                if let Some(length) = overlap {
                    // Keep the provider's coarse timestamps and the untouched
                    // suffix. This is textual alignment, not invented word timing.
                    if let Some((start, _)) = head.get(length) {
                        first.text = first.text[*start..].to_string();
                    }
                }
            }
        }
    }
    // Word anchors can extend a provider's segment bounds slightly. Preserve
    // those real endpoints before applying ownership filtering.
    for segment in &mut right.segments {
        for word in &right.words {
            if word.start < segment.end && word.end > segment.start {
                segment.start = segment.start.min(word.start);
                segment.end = segment.end.max(word.end);
            }
        }
    }
}
fn normalized(text: &str) -> String {
    text.chars()
        .filter(|ch| ch.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}
fn word_spans(text: &str) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    let mut cursor = 0;
    for word in text.split_whitespace() {
        if let Some(offset) = text[cursor..].find(word) {
            let start = cursor + offset;
            cursor = start + word.len();
            spans.push((start, cursor));
        }
    }
    spans
}

pub(super) fn reconcile_published_neighbor(
    left: &mut VerboseTranscription,
    left_base: i64,
    right: &mut VerboseTranscription,
    right_base: i64,
    shared_start: i64,
    shared_end: i64,
) {
    // Published timestamps are authoritative: adjust the retried output, not
    // the neighbor that is already visible and must remain unchanged.
    reconcile(right, right_base, left, left_base, shared_start, shared_end);
    if !left.words.is_empty() || !right.words.is_empty() {
        return;
    }
    let (Some(last), Some(first)) = (left.segments.last_mut(), right.segments.first()) else {
        return;
    };
    let lb = left_base as f64 / 1000.0;
    let rb = right_base as f64 / 1000.0;
    let from = shared_start as f64 / 1000.0;
    let to = shared_end as f64 / 1000.0;
    if lb + last.start >= to
        || lb + last.end <= from
        || rb + first.start >= to
        || rb + first.end <= from
    {
        return;
    }
    let tail = word_spans(&last.text);
    let head = word_spans(&first.text);
    let overlap = (2..=tail.len().min(head.len())).rev().find(|length| {
        tail[tail.len() - length..]
            .iter()
            .zip(&head[..*length])
            .all(|((a, b), (c, d))| {
                normalized(&last.text[*a..*b]) == normalized(&first.text[*c..*d])
            })
    });
    if let Some(length) = overlap {
        if let Some((start, _)) = tail.get(tail.len() - length) {
            last.text = last.text[..*start].trim_end().to_string();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transcription::{VerboseTranscriptionSegment, VerboseTranscriptionWord};
    fn reply(start: f64, end: f64) -> VerboseTranscription {
        VerboseTranscription {
            duration: None,
            language: Some("en".into()),
            language_confidence: None,
            task: None,
            text: "Again.".into(),
            segments: vec![VerboseTranscriptionSegment {
                id: 0,
                start,
                end,
                text: "Again.".into(),
            }],
            words: vec![VerboseTranscriptionWord {
                start,
                end,
                word: "Again.".into(),
                probability: None,
            }],
        }
    }
    #[test]
    fn timestamp_disagreement_cannot_duplicate_a_boundary_word() {
        let mut left = reply(25.6, 26.2);
        let mut right = reply(1.9, 2.5);
        reconcile(&mut left, 0, &mut right, 24_000, 24_000, 28_000);
        let left =
            crate::transcription::constrain_owned_transcription(left, 28_000, &[(0.0, 26.0)]);
        let right =
            crate::transcription::constrain_owned_transcription(right, 30_000, &[(2.0, 28.0)]);
        assert_eq!(left.words.len() + right.words.len(), 1);
    }
    #[test]
    fn repeated_words_outside_the_shared_audio_are_preserved() {
        let mut left = reply(10.0, 10.5);
        let mut right = reply(2.0, 2.5);
        let original = right.clone();
        reconcile(&mut left, 0, &mut right, 24_000, 24_000, 28_000);
        assert_eq!(right, original);
    }
    #[test]
    fn coarse_segments_align_their_overlap_without_fabricating_word_times() {
        let mut left = reply(0.0, 28.0);
        left.words.clear();
        left.segments[0].text = "We discussed the synthetic example.".into();
        let mut right = reply(0.0, 30.0);
        right.words.clear();
        right.segments[0].text = "synthetic example. Then continued.".into();
        reconcile(&mut left, 0, &mut right, 24_000, 24_000, 28_000);
        assert_eq!(right.segments[0].text, "Then continued.");
        assert_eq!(right.segments[0].start, 0.0);
        assert!(right.words.is_empty());
    }
}
