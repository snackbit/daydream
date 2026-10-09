//! Recovery ledger and safe publication are separate from media repair.
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RecordingRecoveryCoverage {
    pub original_segment_id: String,
    pub recovered_segment_id: String,
    pub source_start_offset_ms: i64,
    pub source_end_offset_ms: i64,
    pub recovered_start_offset_ms: i64,
    pub recovered_end_offset_ms: i64,
    pub streams: Vec<RecordingRecoveryStreamCoverage>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RecordingRecoveryStreamCoverage {
    pub original_stream_index: i64,
    pub recovered_stream_index: i64,
    pub source_start_offset_ms: i64,
    pub source_end_offset_ms: i64,
}
pub(crate) fn start_operation(
    connection: &Connection,
    id: &str,
    original: &str,
    output_id: &str,
    timestamp: i64,
) -> rusqlite::Result<()> {
    let transaction = connection.unchecked_transaction()?;
    transaction.execute("UPDATE recording_recovery_operations SET status='interrupted' WHERE original_recording_id=?1 AND status='running'",[original])?;
    transaction.execute("INSERT INTO recording_recovery_operations(id,original_recording_id,pending_output_id,status,coverage_json,created_at_ms) VALUES(?1,?2,?3,'running','[]',?4)",params![id,original,output_id,timestamp])?;
    transaction.commit()
}
pub(crate) fn finish_operation(
    connection: &Connection,
    id: &str,
    recovered: &str,
    coverage: &[RecordingRecoveryCoverage],
) -> rusqlite::Result<()> {
    let coverage = serde_json::to_string(coverage).map_err(|_| rusqlite::Error::InvalidQuery)?;
    let changed = connection.execute("UPDATE recording_recovery_operations SET recovered_recording_id=?2,pending_output_id=NULL,status='completed',coverage_json=?3 WHERE id=?1 AND status='running' AND pending_output_id=?2",params![id,recovered,coverage])?;
    if changed != 1 {
        return Err(rusqlite::Error::InvalidQuery);
    }
    Ok(())
}
pub(crate) fn fail_operation(connection: &Connection, id: &str) -> rusqlite::Result<()> {
    connection.execute(
        "UPDATE recording_recovery_operations SET status='failed' WHERE id=?1 AND status='running'",
        [id],
    )?;
    Ok(())
}

pub(crate) fn pending_outputs(
    connection: &Connection,
    original: Option<&str>,
) -> rusqlite::Result<Vec<(String, String)>> {
    let mut statement = connection.prepare("SELECT id,pending_output_id FROM recording_recovery_operations WHERE pending_output_id IS NOT NULL AND status!='completed' AND (?1 IS NULL OR original_recording_id=?1)")?;
    let outputs = statement
        .query_map([original], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect();
    outputs
}
pub(crate) fn output_removed(
    connection: &Connection,
    operation: &str,
    output_id: &str,
) -> rusqlite::Result<()> {
    connection.execute("UPDATE recording_recovery_operations SET pending_output_id=NULL,status=CASE WHEN status='running' THEN 'interrupted' ELSE status END WHERE id=?1 AND pending_output_id=?2 AND status!='completed'", params![operation, output_id])?;
    Ok(())
}

/// Intersect privacy ranges with actual recovered coverage. Retained source
/// timeline positions make masks and timestamp provenance agree across gaps.
pub(crate) fn intersect_coverage(
    start: i64,
    end: i64,
    coverage: &[RecordingRecoveryCoverage],
) -> Vec<(i64, i64)> {
    coverage
        .iter()
        .filter_map(|range| {
            let intersection_start = start.max(range.source_start_offset_ms);
            let intersection_end = end.min(range.source_end_offset_ms);
            (intersection_end > intersection_start).then_some((
                range.recovered_start_offset_ms + intersection_start - range.source_start_offset_ms,
                range.recovered_start_offset_ms + intersection_end - range.source_start_offset_ms,
            ))
        })
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn privacy_ranges_follow_only_recovered_coverage_and_preserve_gaps() {
        let coverage = vec![
            RecordingRecoveryCoverage {
                original_segment_id: "source".into(),
                recovered_segment_id: "copy".into(),
                source_start_offset_ms: 500,
                source_end_offset_ms: 900,
                recovered_start_offset_ms: 500,
                recovered_end_offset_ms: 900,
                streams: vec![],
            },
            RecordingRecoveryCoverage {
                original_segment_id: "source2".into(),
                recovered_segment_id: "copy2".into(),
                source_start_offset_ms: 1200,
                source_end_offset_ms: 1500,
                recovered_start_offset_ms: 1200,
                recovered_end_offset_ms: 1500,
                streams: vec![],
            },
        ];
        assert_eq!(
            intersect_coverage(700, 1400, &coverage),
            vec![(700, 900), (1200, 1400)]
        );
        assert!(intersect_coverage(900, 1200, &coverage).is_empty());
    }
}
