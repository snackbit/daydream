use super::types::{RecallEvidence, ResolveRecallEvidenceInput};
use crate::storage::{self, AppStorage};
pub use crate::storage::{privacy_revision, read_evidence, refresh_evidence};

pub fn resolve(
    storage: &AppStorage,
    input: &ResolveRecallEvidenceInput,
) -> Result<Option<RecallEvidence>, String> {
    if input.evidence_id.is_empty() || input.evidence_id.len() > 512 {
        return Err("Invalid source reference.".into());
    }
    storage
        .with_connection(|connection| {
            storage::refresh_evidence(connection)?;
            storage::read_evidence(connection, &input.evidence_id)
        })
        .map_err(|_| "Source text could not be read.".into())
}
