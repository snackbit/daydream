use super::super::*;

/// Activity consumers need both windows, without exposing events in their gap.
pub(crate) fn activity_for_recording_with_review(
    storage: &AppStorage,
    recording_id: &str,
    range: Option<(i64, i64)>,
    review: Option<(i64, i64)>,
) -> Result<RecordingActivity, AiError> {
    let mut result = if range.is_some() && review.is_some() {
        super::activity::activity_for_recording_in_windows(storage, recording_id, range, review)?
    } else if range.is_some() {
        super::activity_for_recording_in_range(storage, recording_id, range)?
    } else {
        super::activity_for_recording(storage, recording_id)?
    };
    if let Some((start, end)) = range {
        let overlaps = |from: i64, to: i64| {
            from < end && to > start || review.is_some_and(|(a, b)| from < b && to > a)
        };
        result
            .intervals
            .retain(|interval| overlaps(interval.start_offset_ms, interval.end_offset_ms));
        result
            .events
            .retain(|event| overlaps(event.offset_ms, event.offset_ms.saturating_add(1)));
        result
            .window_snapshots
            .retain(|snapshot| overlaps(snapshot.offset_ms, snapshot.offset_ms.saturating_add(1)));
        // Capture-state history preceding either page establishes its initial state.
    }
    Ok(result)
}

#[cfg(test)]
mod tests;
