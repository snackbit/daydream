use super::super::super::*;
use super::types::*;

#[derive(Default)]
pub(super) struct PendingVisualOutput {
    pub(super) frame_similarity_entries: Vec<CreateFrameSimilarityEntryInput>,
    pub(super) ocr_regions: Vec<CreateVisualRegionInput>,
    pub(super) vlm_observations: Vec<CreateVisualRegionInput>,
}

pub(super) fn ocr_region_inputs(
    chunk: &ProcessingChunkRow,
    frame: &ExtractedVisualFrame,
    regions: &[OcrCandidate],
) -> Vec<CreateVisualRegionInput> {
    regions
        .iter()
        .map(|region| CreateVisualRegionInput {
            chunk_id: chunk.id.clone(),
            confidence: region.confidence,
            frame_timestamp_ms: frame.frame_timestamp_ms,
            id: Uuid::now_v7().to_string(),
            job_id: chunk.job_id.clone(),
            model: Some("tesseract".to_string()),
            observation_kind: None,
            prompt_version: None,
            provider: "tesseract".to_string(),
            recording_id: chunk.recording_id.clone(),
            region_height: region.region.height,
            region_width: region.region.width,
            region_x: region.region.x,
            region_y: region.region.y,
            source_frame_ref: frame.source_frame_ref.clone(),
            text: region.text.clone(),
        })
        .collect()
}

pub(super) fn frame_similarity_input(
    chunk: &ProcessingChunkRow,
    frame: &ExtractedVisualFrame,
) -> CreateFrameSimilarityEntryInput {
    CreateFrameSimilarityEntryInput {
        chunk_id: Some(chunk.id.clone()),
        id: Uuid::now_v7().to_string(),
        job_id: chunk.job_id.clone(),
        last_processed_timestamp_ms: frame.frame_timestamp_ms,
        recording_id: chunk.recording_id.clone(),
        region_height: frame.region.height,
        region_width: frame.region.width,
        region_x: frame.region.x,
        region_y: frame.region.y,
        signature: frame.signature.clone(),
        source_frame_ref: frame.source_frame_ref.clone(),
    }
}

pub(super) fn queue_visual_frame_states(
    app: &AppHandle,
    storage: &AppStorage,
    chunk: &ProcessingChunkRow,
    frames: &[ExtractedVisualFrame],
) -> Result<Vec<VisualFrameProcessingStateRow>, AiError> {
    let timestamp_ms = now_ms();
    let rows = storage
        .with_connection(|connection| {
            frames
                .iter()
                .map(|frame| {
                    storage::upsert_visual_frame_processing_state(
                        connection,
                        &CreateVisualFrameProcessingStateInput {
                            chunk_id: chunk.id.clone(),
                            created_at_ms: timestamp_ms,
                            error_code: None,
                            frame_timestamp_ms: frame.frame_timestamp_ms,
                            id: Uuid::now_v7().to_string(),
                            job_id: chunk.job_id.clone(),
                            output_count: 0,
                            progress: 0.0,
                            recording_id: chunk.recording_id.clone(),
                            region_height: frame.region.height,
                            region_width: frame.region.width,
                            region_x: frame.region.x,
                            region_y: frame.region.y,
                            segment_id: chunk.segment_id.clone(),
                            source_frame_ref: frame.source_frame_ref.clone(),
                            source_kind: chunk.source_kind.clone(),
                            status: "queued".to_string(),
                            updated_at_ms: timestamp_ms,
                        },
                    )
                })
                .collect::<rusqlite::Result<Vec<_>>>()
        })
        .map_err(storage_error)?;
    emit_processing_state(app, &chunk.recording_id);
    Ok(rows)
}

pub(super) fn mark_visual_frame_state(
    app: &AppHandle,
    storage: &AppStorage,
    chunk: &ProcessingChunkRow,
    frame_state_id: &str,
    status: &str,
    progress: f64,
    error_code: Option<&str>,
    output_count: i64,
) -> Result<(), AiError> {
    storage
        .with_connection(|connection| {
            storage::update_visual_frame_processing_state_status(
                connection,
                frame_state_id,
                status,
                progress,
                error_code,
                output_count,
                now_ms(),
            )
            .map(|_row| ())
        })
        .map_err(storage_error)?;
    emit_processing_state(app, &chunk.recording_id);
    Ok(())
}

pub(super) fn vlm_observation_inputs(
    chunk: &ProcessingChunkRow,
    config: &VisionRequestConfig,
    observations: Vec<PendingVlmObservation>,
) -> Vec<CreateVisualRegionInput> {
    observations
        .into_iter()
        .map(|observation| CreateVisualRegionInput {
            chunk_id: chunk.id.clone(),
            confidence: observation.confidence,
            frame_timestamp_ms: observation.frame_timestamp_ms,
            id: Uuid::now_v7().to_string(),
            job_id: chunk.job_id.clone(),
            model: Some(config.model.clone()),
            observation_kind: Some(observation.observation_kind),
            prompt_version: Some(VLM_PROMPT_VERSION.to_string()),
            provider: config.provider.clone(),
            recording_id: chunk.recording_id.clone(),
            region_height: observation.region.height,
            region_width: observation.region.width,
            region_x: observation.region.x,
            region_y: observation.region.y,
            source_frame_ref: observation.source_frame_ref,
            text: observation.text,
        })
        .collect()
}

pub(super) fn persist_visual_output(
    storage: &AppStorage,
    chunk: &ProcessingChunkRow,
    output: PendingVisualOutput,
) -> Result<(), AiError> {
    let ocr_region_count = output.ocr_regions.len();
    let vlm_observation_count = output.vlm_observations.len();
    let frame_similarity_entry_count = output.frame_similarity_entries.len();
    tracing::debug!(
        recording_id = %chunk.recording_id,
        job_id = %chunk.job_id,
        chunk_id = %chunk.id,
        source_kind = %chunk.source_kind,
        ocr_region_count,
        vlm_observation_count,
        frame_similarity_entry_count,
        "ai_processing_visual_persist_started"
    );
    let started = Instant::now();
    let result = storage.with_connection_mut(|connection| {
        let transaction = connection.transaction()?;
        storage::delete_visual_output_for_chunks(&transaction, std::slice::from_ref(&chunk.id))?;
        for region in output.ocr_regions {
            storage::create_ocr_region(&transaction, &region)?;
        }
        for observation in output.vlm_observations {
            storage::create_vlm_observation(&transaction, &observation)?;
        }
        for entry in output.frame_similarity_entries {
            storage::create_frame_similarity_entry(&transaction, &entry)?;
        }
        transaction.commit()?;
        Ok(())
    });
    match result {
        Ok(()) => {
            tracing::info!(
                recording_id = %chunk.recording_id,
                job_id = %chunk.job_id,
                chunk_id = %chunk.id,
                source_kind = %chunk.source_kind,
                ocr_region_count,
                vlm_observation_count,
                frame_similarity_entry_count,
                elapsed_ms = elapsed_ms(started),
                "ai_processing_visual_persist_completed"
            );
            Ok(())
        }
        Err(error) => {
            tracing::warn!(
                recording_id = %chunk.recording_id,
                job_id = %chunk.job_id,
                chunk_id = %chunk.id,
                source_kind = %chunk.source_kind,
                ocr_region_count,
                vlm_observation_count,
                frame_similarity_entry_count,
                error = %error,
                elapsed_ms = elapsed_ms(started),
                "ai_processing_visual_persist_failed"
            );
            Err(storage_error(error))
        }
    }
}
