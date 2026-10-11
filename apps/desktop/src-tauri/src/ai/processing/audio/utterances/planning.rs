use super::super::*;

pub(super) const OWNED_WINDOW_MS: i64 = 26_000;
pub(super) const CONTEXT_MS: i64 = 2_000;
pub(super) struct PlanningWindow {
    pub cursor: i64,
    pub scan_start: i64,
    pub scan_end: i64,
    pub run_end: i64,
    pub open: bool,
}

/// Coverage includes silence; submission ranges contain speech and bounded context.
pub(super) fn plan_units(
    chunk: &ProcessingChunkRow,
    track: &RecordingTrackRow,
    window: PlanningWindow,
    speech: &[whisper_backend::SpeechRange],
    fingerprint: &str,
) -> Vec<storage::SpeechUnit> {
    let PlanningWindow {
        cursor,
        scan_start,
        scan_end,
        run_end,
        open,
    } = window;
    let limit = (cursor + OWNED_WINDOW_MS).min(run_end);
    let mut covered = cursor;
    let mut units = Vec::new();
    let unit = |from: i64, to: i64, submitted_start: i64, submitted_end: i64, right_open: bool| {
        storage::SpeechUnit {
            id: Uuid::now_v7().to_string(),
            job_id: chunk.job_id.clone(),
            track_id: track.id.clone(),
            owner_chunk_id: chunk.id.clone(),
            start_ms: from,
            end_ms: to,
            submitted_start_ms: submitted_start,
            submitted_end_ms: submitted_end,
            right_open,
            status: "queued".into(),
            fingerprint: fingerprint.into(),
            output_json: None,
        }
    };
    for range in speech {
        let start = scan_start + range.start_ms;
        let end = scan_start + range.end_ms;
        if end <= cursor || start >= limit {
            continue;
        }
        // An unfinished tail waits for a finalized neighbor; earlier utterances can publish.
        if open && scan_end == run_end && end + 400 > run_end && end <= limit {
            break;
        }
        let submitted_start = (start - 300).max(scan_start);
        let gap_end = submitted_start.max(covered).min(limit);
        if gap_end > covered {
            units.push(unit(covered, gap_end, gap_end, gap_end, false));
            covered = gap_end;
        }
        // A short utterance near the nominal window edge can finish inside the
        // lookahead. Do not split it just because leading silence used the budget.
        let closed = end + 400 <= scan_end || (!open && scan_end == run_end);
        let owned_end = if closed {
            (end + 150).min(covered + OWNED_WINDOW_MS).min(run_end)
        } else {
            limit
        };
        let right_open = end > owned_end;
        let submitted_end = if right_open {
            (limit + CONTEXT_MS).min(scan_end)
        } else {
            (end + 150).min(scan_end)
        };
        let submitted_start = if start < cursor {
            (cursor - CONTEXT_MS).max(scan_start)
        } else {
            submitted_start
        };
        units.push(unit(
            covered,
            owned_end,
            submitted_start,
            submitted_end,
            right_open,
        ));
        covered = owned_end;
        if right_open {
            break;
        }
    }
    let safe_end = if open && scan_end == run_end {
        speech
            .iter()
            .find(|range| {
                scan_start + range.end_ms + 400 > run_end && scan_start + range.start_ms < limit
            })
            .map(|range| (scan_start + range.start_ms - 300).max(covered))
            .unwrap_or(limit)
    } else {
        limit
    };
    if covered < safe_end {
        units.push(unit(covered, safe_end, safe_end, safe_end, false));
    }
    units
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn speech_crosses_media_cuts_and_long_speech_uses_bounded_overlap() {
        let store = storage::test_storage(PathBuf::new());
        let (chunk, track) = store
            .with_connection(|c| Ok(crate::ai::tests::test_transcription_chunk(c)))
            .unwrap();
        let units = plan_units(
            &chunk,
            &track,
            PlanningWindow {
                cursor: 0,
                scan_start: 0,
                scan_end: 28_000,
                run_end: 60_000,
                open: false,
            },
            &[whisper_backend::SpeechRange {
                start_ms: 5_000,
                end_ms: 28_000,
            }],
            "v2",
        );
        let speech = units.last().unwrap();
        assert_eq!(speech.end_ms, 26_000);
        assert_eq!(speech.submitted_end_ms, 28_000);
        assert!(speech.right_open);
        let next = plan_units(
            &chunk,
            &track,
            PlanningWindow {
                cursor: 26_000,
                scan_start: 24_000,
                scan_end: 54_000,
                run_end: 60_000,
                open: false,
            },
            &[whisper_backend::SpeechRange {
                start_ms: 0,
                end_ms: 30_000,
            }],
            "v2",
        );
        assert_eq!(next[0].submitted_start_ms, 24_000);
        assert_eq!(
            next[0].submitted_end_ms - next[0].submitted_start_ms,
            30_000
        );
    }
    #[test]
    fn unfinished_live_speech_waits_but_stop_flushes_it() {
        let store = storage::test_storage(PathBuf::new());
        let (chunk, track) = store
            .with_connection(|c| Ok(crate::ai::tests::test_transcription_chunk(c)))
            .unwrap();
        let speech = [whisper_backend::SpeechRange {
            start_ms: 500,
            end_ms: 2_000,
        }];
        let waiting = plan_units(
            &chunk,
            &track,
            PlanningWindow {
                cursor: 0,
                scan_start: 0,
                scan_end: 2_000,
                run_end: 2_000,
                open: true,
            },
            &speech,
            "v2",
        );
        assert!(waiting
            .iter()
            .all(|unit| unit.submitted_start_ms == unit.submitted_end_ms));
        let finished = plan_units(
            &chunk,
            &track,
            PlanningWindow {
                cursor: 0,
                scan_start: 0,
                scan_end: 2_000,
                run_end: 2_000,
                open: false,
            },
            &speech,
            "v2",
        );
        assert!(finished
            .iter()
            .any(|unit| unit.submitted_end_ms > unit.submitted_start_ms));
    }
    #[test]
    fn leading_silence_does_not_force_a_cut_through_a_short_utterance() {
        let store = storage::test_storage(PathBuf::new());
        let (chunk, track) = store
            .with_connection(|c| Ok(crate::ai::tests::test_transcription_chunk(c)))
            .unwrap();
        let units = plan_units(
            &chunk,
            &track,
            PlanningWindow {
                cursor: 0,
                scan_start: 0,
                scan_end: 28_000,
                run_end: 60_000,
                open: false,
            },
            &[whisper_backend::SpeechRange {
                start_ms: 25_000,
                end_ms: 27_000,
            }],
            "v2",
        );
        let speech = units
            .iter()
            .filter(|unit| unit.submitted_end_ms > unit.submitted_start_ms)
            .collect::<Vec<_>>();
        assert_eq!(speech.len(), 1);
        assert_eq!(speech[0].end_ms, 27_150);
        assert!(!speech[0].right_open);
        assert!(units
            .iter()
            .all(|unit| unit.end_ms - unit.start_ms <= OWNED_WINDOW_MS));
    }
}
