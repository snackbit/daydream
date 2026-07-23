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
            let chunks = storage::list_processing_chunks(connection, job_id)?;
            let chunk = chunks
                .iter()
                .filter(|chunk| chunk.status == "queued")
                .find(|chunk| chunk_ready_to_claim(chunk, &chunks))
                .cloned();
            let Some(chunk) = chunk else {
                return Ok(None);
            };
            storage::claim_processing_chunk(connection, &chunk.id, 0.05, now_ms())
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

fn chunk_ready_to_claim(chunk: &ProcessingChunkRow, chunks: &[ProcessingChunkRow]) -> bool {
    if chunk.source_kind != "vlm" {
        return true;
    }

    !chunks.iter().any(|candidate| {
        candidate.source_kind == "ocr"
            && candidate.status != "succeeded"
            && candidate.segment_id == chunk.segment_id
            && chunks_overlap(candidate, chunk)
    })
}

fn chunks_overlap(left: &ProcessingChunkRow, right: &ProcessingChunkRow) -> bool {
    let left_start = left.start_offset_ms.unwrap_or(0);
    let left_end = left.end_offset_ms.unwrap_or(left_start);
    let right_start = right.start_offset_ms.unwrap_or(0);
    let right_end = right.end_offset_ms.unwrap_or(right_start);

    left_start < right_end && right_start < left_end
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vlm_chunk_waits_for_overlapping_ocr_chunk() {
        let segment_id = Uuid::now_v7().to_string();
        let ocr = test_chunk("ocr", "running", 0, 1_000, &segment_id);
        let vlm = test_chunk("vlm", "queued", 0, 1_000, &segment_id);

        assert!(!chunk_ready_to_claim(&vlm, &[ocr.clone(), vlm.clone()]));

        let succeeded_ocr = ProcessingChunkRow {
            status: "succeeded".to_string(),
            ..ocr
        };
        assert!(chunk_ready_to_claim(&vlm, &[succeeded_ocr, vlm.clone()]));
    }

    fn test_chunk(
        source_kind: &str,
        status: &str,
        start_offset_ms: i64,
        end_offset_ms: i64,
        segment_id: &str,
    ) -> ProcessingChunkRow {
        ProcessingChunkRow {
            attempt_count: 0,
            created_at_ms: 1,
            end_offset_ms: Some(end_offset_ms),
            error_code: None,
            id: Uuid::now_v7().to_string(),
            job_id: Uuid::now_v7().to_string(),
            progress: 0.0,
            recording_id: Uuid::now_v7().to_string(),
            result_code: None,
            segment_id: Some(segment_id.to_string()),
            source_kind: source_kind.to_string(),
            start_offset_ms: Some(start_offset_ms),
            status: status.to_string(),
            track_id: None,
            updated_at_ms: 1,
        }
    }
}
