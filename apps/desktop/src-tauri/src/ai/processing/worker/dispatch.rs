use super::super::super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ProcessingChunkSource {
    Activity,
    Audio,
    Visual,
}

impl ProcessingChunkSource {
    pub(super) fn from_chunk(chunk: &ProcessingChunkRow) -> Result<Self, AiError> {
        match chunk.source_kind.as_str() {
            "activity" => Ok(Self::Activity),
            "audio" => Ok(Self::Audio),
            "ocr" | "vlm" => Ok(Self::Visual),
            _ => Err(ai_error(
                "ai_processing_chunk_source_unsupported",
                "Processing chunk source is not supported.",
            )),
        }
    }
}

pub(super) fn claim_next_processing_chunk(
    storage: &AppStorage,
    job_id: &str,
) -> Result<Option<ProcessingChunkRow>, AiError> {
    let next_chunk = storage
        .with_connection(|connection| {
            storage::claim_next_queued_processing_chunk(connection, job_id, 0.05, now_ms())
        })
        .map_err(storage_error)?;

    if let Some(chunk) = &next_chunk {
        tracing::info!(
            recording_id = %chunk.recording_id,
            job_id = %job_id,
            chunk_id = %chunk.id,
            source_kind = %chunk.source_kind,
            track_id = chunk.track_id.as_deref().unwrap_or(""),
            segment_id = chunk.segment_id.as_deref().unwrap_or(""),
            start_offset_ms = ?chunk.start_offset_ms,
            end_offset_ms = ?chunk.end_offset_ms,
            attempt_count = chunk.attempt_count,
            "ai_processing_chunk_claimed"
        );
    }

    Ok(next_chunk)
}
