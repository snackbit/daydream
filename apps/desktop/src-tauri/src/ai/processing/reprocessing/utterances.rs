use super::super::super::*;

pub(super) fn utterance_ranges(
    connection: &rusqlite::Connection,
    recording_id: &str,
    transcripts: &[TranscriptSegmentRow],
) -> rusqlite::Result<BTreeMap<String, Vec<(i64, i64)>>> {
    let mut by_role = BTreeMap::<String, Vec<(i64, i64)>>::new();
    for transcript in transcripts {
        by_role
            .entry(transcript.source_role.clone())
            .or_default()
            .push((transcript.start_offset_ms, transcript.end_offset_ms));
    }
    let mut query=connection.prepare("SELECT t.kind,p.start_offset_ms+r.start_ms,p.start_offset_ms+r.end_ms FROM processing_audio_ranges r JOIN processing_chunks p ON p.id=r.chunk_id JOIN recording_tracks t ON t.id=p.track_id WHERE p.recording_id=?1 AND p.source_kind='audio'")?;
    for row in query.query_map([recording_id], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, i64>(1)?,
            r.get::<_, i64>(2)?,
        ))
    })? {
        let (role, start, end) = row?;
        by_role.entry(role).or_default().push((start, end));
    }
    // Canonical speech may span media fragments. Its submitted context is not
    // itself a new utterance, but connects forced cuts of the same speech.
    let mut query=connection.prepare("SELECT t.kind,u.start_ms,u.end_ms,u.right_open FROM processing_speech_units u JOIN processing_jobs j ON j.id=u.job_id JOIN recording_tracks t ON t.id=u.track_id WHERE j.recording_id=?1 AND u.submitted_end_ms>u.submitted_start_ms ORDER BY u.job_id,u.track_id,u.start_ms")?;
    let mut previous: Option<(String, i64, i64, bool)> = None;
    for row in query.query_map([recording_id], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, i64>(1)?,
            r.get::<_, i64>(2)?,
            r.get::<_, bool>(3)?,
        ))
    })? {
        let (role, start, end, open) = row?;
        if let Some((old_role, old_start, old_end, old_open)) = previous.take() {
            if old_role == role && old_end == start && old_open {
                previous = Some((role, old_start, end, open));
                continue;
            }
            by_role
                .entry(old_role)
                .or_default()
                .push((old_start, old_end));
        }
        previous = Some((role, start, end, open));
    }
    if let Some((role, start, end, _)) = previous {
        by_role.entry(role).or_default().push((start, end));
    }
    for ranges in by_role.values_mut() {
        ranges.sort_unstable();
        let mut merged: Vec<(i64, i64)> = Vec::new();
        for &(start, end) in ranges.iter() {
            if let Some(last) = merged.last_mut() {
                if start < last.1 {
                    last.1 = last.1.max(end);
                    continue;
                }
            }
            merged.push((start, end));
        }
        *ranges = merged;
    }
    Ok(by_role)
}
pub(super) fn expand(start: i64, end: i64, ranges: &[(i64, i64)]) -> (i64, i64) {
    let mut result = (start, end);
    for &(from, to) in &ranges[ranges.partition_point(|r| r.1 <= start)..] {
        if from >= result.1 {
            break;
        }
        result.0 = result.0.min(from);
        result.1 = result.1.max(to);
    }
    result
}
