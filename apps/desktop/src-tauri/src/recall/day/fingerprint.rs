//! Preserve the persisted review contract without reading evidence content.
use super::{day_key, RecallCoverageInterval, RecallDayInput};
use rusqlite::{Connection, OptionalExtension};
use sha2::{Digest, Sha256};

pub(super) fn review_state(
    connection: &Connection,
    input: &RecallDayInput,
    evidence_fingerprint: &str,
    coverage: &[RecallCoverageInterval],
) -> Result<(String, bool), String> {
    let mut input_hasher = Sha256::new();
    input_hasher.update(evidence_fingerprint);
    input_hasher.update(input.timezone.as_bytes());
    input_hasher.update(serde_json::to_vec(coverage).map_err(|_| "Invalid coverage.")?);
    let key = day_key(input);
    let mut statement=connection.prepare("SELECT session_id,correction,title_correction,category_correction FROM recall_session_corrections WHERE day_key=?1 ORDER BY session_id").map_err(|_|"Day corrections could not be read.")?;
    for row in statement
        .query_map([&key], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        })
        .map_err(|_| "Day corrections could not be read.")?
    {
        let (id, text, title, category) = row.map_err(|_| "Day corrections could not be read.")?;
        for value in [Some(id), text, title, category] {
            let value = value.unwrap_or_default();
            input_hasher.update((value.len() as u64).to_le_bytes());
            input_hasher.update(value.as_bytes());
        }
    }
    // Legacy review fingerprints include the exact serialized output. Hash it
    // without composing/validating sessions, suggestions, tasks or evidence.
    let generation: Option<(String, String)> = connection
        .query_row(
            "SELECT output_json,status FROM recall_generations WHERE day_key=?1",
            [&key],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|_| "Interpretation state could not be read.")?;
    if let Some((output, status)) = generation {
        input_hasher.update(output.as_bytes());
        input_hasher.update(status.as_bytes());
    }
    let configuration: Option<String> = connection
        .query_row(
            "SELECT value_json FROM app_settings WHERE key='ai.recall'",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|_| "Configuration could not be read.")?;
    input_hasher.update(configuration.unwrap_or_default().as_bytes());
    let fingerprint = format!("{:x}", input_hasher.finalize());
    let reviewed = connection
        .query_row(
            "SELECT fingerprint FROM recall_day_reviews WHERE day_key=?1",
            [&key],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|_| "Review state could not be read.")?
        .is_some_and(|value| value == fingerprint);
    Ok((fingerprint, reviewed))
}
