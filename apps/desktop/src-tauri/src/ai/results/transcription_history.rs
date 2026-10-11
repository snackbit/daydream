//! A scoped pass owns only its covered intervals, not an entire audio source.
use super::super::*;
type Coverage = BTreeMap<(Option<String>, Option<String>), Vec<(i64, i64)>>;
pub(super) fn transcription_history(
    connection: &rusqlite::Connection,
    recording_id: &str,
    current: &[ProcessingChunk],
) -> rusqlite::Result<Vec<ProcessingChunk>> {
    let mut history = current.to_vec();
    let mut covered = Coverage::new();
    let mut ids = current
        .iter()
        .map(|chunk| chunk.id.clone())
        .collect::<BTreeSet<_>>();
    for chunk in current.iter().filter(|chunk| chunk.source_kind == "audio") {
        cover(&mut covered, chunk);
    }
    let mut jobs = storage::list_processing_jobs(connection, Some(recording_id))?;
    // A retry's update time must not let an old job claim newer scopes.
    jobs.sort_by(|a, b| {
        b.created_at_ms
            .cmp(&a.created_at_ms)
            .then_with(|| b.id.cmp(&a.id))
    });
    for job in jobs {
        for row in storage::list_processing_chunks(connection, &job.id)? {
            if row.source_kind != "audio" || !ids.insert(row.id.clone()) {
                continue;
            }
            let chunk = super::processing::processing_chunk_from_row(row);
            let start = chunk.start_offset_ms.unwrap_or(0);
            let end = chunk.end_offset_ms.unwrap_or(start);
            let ranges = covered
                .entry((chunk.track_id.clone(), chunk.segment_id.clone()))
                .or_default();
            let mut cursor = start;
            for &(left, right) in ranges.iter() {
                if left > cursor || cursor >= end {
                    break;
                }
                cursor = cursor.max(right);
            }
            if cursor < end || end <= start {
                cover(&mut covered, &chunk);
                history.push(chunk);
            }
        }
    }
    Ok(history)
}

fn cover(covered: &mut Coverage, chunk: &ProcessingChunk) {
    let start = chunk.start_offset_ms.unwrap_or(0);
    let end = chunk.end_offset_ms.unwrap_or(start);
    if end <= start {
        return;
    }
    let ranges = covered
        .entry((chunk.track_id.clone(), chunk.segment_id.clone()))
        .or_default();
    ranges.push((start, end));
    ranges.sort_unstable();
    let mut merged: Vec<(i64, i64)> = Vec::with_capacity(ranges.len());
    for &(left, right) in ranges.iter() {
        if let Some(last) = merged.last_mut().filter(|last| left <= last.1) {
            last.1 = last.1.max(right);
        } else {
            merged.push((left, right));
        }
    }
    *ranges = merged;
}
