use super::super::*;

mod concurrency;
mod context;
mod extraction;
mod frame_jobs;
mod image;
mod ocr;
mod persistence;
mod types;
mod vlm;

use concurrency::{acquire_ocr_permit, acquire_vlm_permit};
pub(in crate::ai) use context::previous_visual_observations;
use context::{persisted_ocr_candidates_by_timestamp, recent_visual_observations};
#[cfg(test)]
pub(in crate::ai) use extraction::{
    effective_frame_recognition_fps, frame_sample_interval_ms, full_checkpoint_for_frame,
};
use extraction::{extract_visual_frames, VisualExtractionRequest};
use frame_jobs::{process_ocr_frames, process_vlm_frames, VlmFrameBatch};
#[cfg(test)]
pub(in crate::ai) use image::visual_delta_regions;
use ocr::run_tesseract_ocr;
#[cfg(test)]
pub(in crate::ai) use ocr::{
    parse_tesseract_tsv, run_tesseract_ocr_with_binary, tesseract_ocr_command_args_for_test,
};
use persistence::{
    frame_similarity_input, mark_visual_frame_state, ocr_region_inputs, persist_visual_output,
    queue_visual_frame_states, vlm_observation_inputs, PendingVisualOutput, VisualFrameUpdate,
};
#[cfg(test)]
pub(in crate::ai) use persistence::{
    persist_visual_output as persist_visual_output_for_test,
    PendingVisualOutput as PendingVisualOutputForTest,
};
#[cfg(test)]
pub(in crate::ai) use types::VisualImage;
pub(in crate::ai) use types::{
    ExtractedVisualFrame, OcrCandidate, PendingVlmObservation, PreviousVisualObservation,
    VisionRequestConfig,
};
use vlm::{request_vlm_observations, vision_provider_ready, vision_request_config};
#[cfg(test)]
pub(in crate::ai) use vlm::{
    validate_vlm_observation_text, vision_chat_completions_url, vlm_chat_completions_body,
};

pub(in crate::ai) async fn process_visual_chunk_inner(
    app: &AppHandle,
    chunk: &ProcessingChunkRow,
    cancel: Arc<AtomicBool>,
) -> Result<(), AiError> {
    let started = Instant::now();
    let storage = app.state::<AppStorage>();
    ensure_chunk_active(&storage, chunk)?;
    let app_settings = settings::load_app_settings(&storage).map_err(settings_error)?;
    let is_vlm_chunk = chunk.source_kind == "vlm";
    if chunk.source_kind == "ocr" && !app_settings.ai.ocr.enabled {
        tracing::info!(
            recording_id = %chunk.recording_id,
            job_id = %chunk.job_id,
            chunk_id = %chunk.id,
            source_kind = %chunk.source_kind,
            "ai_processing_visual_chunk_skipped_ocr_disabled"
        );
        return Ok(());
    }
    if is_vlm_chunk && !vision_provider_ready(&app_settings) {
        tracing::info!(
            recording_id = %chunk.recording_id,
            job_id = %chunk.job_id,
            chunk_id = %chunk.id,
            source_kind = %chunk.source_kind,
            vision_provider = %app_settings.ai.vision.provider,
            ocr_enabled = app_settings.ai.ocr.enabled,
            has_base_url = app_settings.ai.vision.base_url.is_some(),
            has_api_key_ref = app_settings.ai.vision.api_key_ref.is_some(),
            has_model = app_settings.ai.vision.model.is_some(),
            "ai_processing_visual_chunk_skipped_vlm_unconfigured"
        );
        return Ok(());
    }

    let chunk_start = chunk.start_offset_ms.ok_or_else(|| {
        ai_error(
            "ai_processing_chunk_offset_missing",
            "Processing chunk start offset is missing.",
        )
    })?;
    let chunk_end = chunk.end_offset_ms.ok_or_else(|| {
        ai_error(
            "ai_processing_chunk_offset_missing",
            "Processing chunk end offset is missing.",
        )
    })?;
    if chunk_end <= chunk_start {
        tracing::warn!(
            recording_id = %chunk.recording_id,
            job_id = %chunk.job_id,
            chunk_id = %chunk.id,
            source_kind = %chunk.source_kind,
            start_offset_ms = chunk_start,
            end_offset_ms = chunk_end,
            "ai_processing_visual_chunk_invalid_duration"
        );
        return Err(ai_error(
            "ai_processing_chunk_duration_invalid",
            "Processing chunk duration is invalid.",
        ));
    }

    check_processing_cancel(&cancel)?;
    let (segment, segment_playback_offset_ms, recording_fps) = storage
        .with_connection(|connection| {
            let segment_id = chunk
                .segment_id
                .as_deref()
                .ok_or(rusqlite::Error::InvalidQuery)?;
            let recording = storage::get_recording(connection, &chunk.recording_id)?;
            let segments = storage::list_recording_segments(connection, &chunk.recording_id)?;
            let segment_playback_offset_ms = playback_offset_for_segment(&segments, segment_id)
                .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
            let segment = segments
                .into_iter()
                .find(|candidate| candidate.id == segment_id)
                .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
            Ok((segment, segment_playback_offset_ms, recording.default_fps))
        })
        .map_err(storage_error)?;

    tracing::info!(
        recording_id = %chunk.recording_id,
        job_id = %chunk.job_id,
        chunk_id = %chunk.id,
        source_kind = %chunk.source_kind,
        segment_id = %segment.id,
        segment_playback_offset_ms,
        recording_fps,
        chunk_start_ms = chunk_start,
        chunk_end_ms = chunk_end,
        "ai_processing_visual_chunk_loaded"
    );

    let frame_set = extract_visual_frames(
        app,
        VisualExtractionRequest {
            segment: &segment,
            segment_playback_offset_ms,
            chunk,
            recording_fps,
            app_settings: &app_settings,
            include_ocr_regions: !is_vlm_chunk,
            cancel: &cancel,
        },
    )?;
    tracing::info!(
        recording_id = %chunk.recording_id,
        job_id = %chunk.job_id,
        chunk_id = %chunk.id,
        source_kind = %chunk.source_kind,
        ocr_frame_count = frame_set.ocr_frames.len(),
        vlm_frame_count = frame_set.vlm_frames.len(),
        "ai_processing_visual_frames_ready"
    );

    let vision_config = if is_vlm_chunk {
        let config_started = Instant::now();
        let config = vision_request_config(&storage, &app_settings)?;
        tracing::info!(
            recording_id = %chunk.recording_id,
            job_id = %chunk.job_id,
            chunk_id = %chunk.id,
            source_kind = %chunk.source_kind,
            provider = %config.provider,
            model = %config.model,
            base_url = %config.base_url,
            elapsed_ms = elapsed_ms(config_started),
            "ai_processing_visual_vlm_config_loaded"
        );
        Some(config)
    } else {
        None
    };
    let previous_observations = if is_vlm_chunk {
        let previous_started = Instant::now();
        let observations = previous_visual_observations(&storage, &chunk.recording_id)?;
        tracing::info!(
            recording_id = %chunk.recording_id,
            job_id = %chunk.job_id,
            chunk_id = %chunk.id,
            previous_observation_count = observations.len(),
            elapsed_ms = elapsed_ms(previous_started),
            "ai_processing_visual_previous_observations_loaded"
        );
        observations
    } else {
        Vec::new()
    };
    let persisted_ocr_candidates = if is_vlm_chunk {
        let candidates_started = Instant::now();
        let candidates = persisted_ocr_candidates_by_timestamp(&storage, &chunk.recording_id)?;
        tracing::info!(
            recording_id = %chunk.recording_id,
            job_id = %chunk.job_id,
            chunk_id = %chunk.id,
            persisted_ocr_frame_count = candidates.len(),
            persisted_ocr_candidate_count = candidates.values().map(Vec::len).sum::<usize>(),
            elapsed_ms = elapsed_ms(candidates_started),
            "ai_processing_visual_persisted_ocr_loaded"
        );
        candidates
    } else {
        BTreeMap::new()
    };
    let mut pending_output = PendingVisualOutput::default();
    let mut ocr_candidate_count = 0usize;
    let mut ocr_elapsed_ms = 0u64;
    let mut vlm_observation_count = 0usize;
    let mut vlm_elapsed_ms = 0u64;

    if is_vlm_chunk {
        let config = vision_config.as_ref().ok_or_else(|| {
            ai_error(
                "ai_vision_remote_unconfigured",
                "Vision provider is not configured.",
            )
        })?;
        let frame_count = frame_set.vlm_frames.len();
        if frame_count == 0 {
            tracing::info!(
                recording_id = %chunk.recording_id,
                job_id = %chunk.job_id,
                chunk_id = %chunk.id,
                source_kind = %chunk.source_kind,
                "ai_processing_visual_vlm_no_full_frames"
            );
        }
        let frame_results = process_vlm_frames(
            app,
            &storage,
            chunk,
            VlmFrameBatch {
                config,
                frames: frame_set.vlm_frames.clone(),
                persisted_ocr_candidates,
                previous_observations,
                max_parallel_ocr_jobs: app_settings.ai.processing.max_parallel_ocr_jobs as usize,
                max_parallel_vlm_jobs: app_settings.ai.processing.max_parallel_vlm_jobs as usize,
                cancel: cancel.clone(),
            },
        )
        .await?;
        for result in frame_results {
            ocr_elapsed_ms = ocr_elapsed_ms.saturating_add(result.ocr_elapsed_ms);
            vlm_elapsed_ms = vlm_elapsed_ms.saturating_add(result.vlm_elapsed_ms);
            ocr_candidate_count = ocr_candidate_count.saturating_add(result.ocr_candidate_count);
            vlm_observation_count = vlm_observation_count.saturating_add(result.observations.len());
            pending_output
                .vlm_observations
                .extend(vlm_observation_inputs(chunk, config, result.observations));
        }
    } else {
        let frame_results = process_ocr_frames(
            app,
            &storage,
            chunk,
            frame_set.ocr_frames.clone(),
            app_settings.ai.processing.max_parallel_ocr_jobs as usize,
            cancel.clone(),
        )
        .await?;
        for result in frame_results {
            ocr_elapsed_ms = ocr_elapsed_ms.saturating_add(result.ocr_elapsed_ms);
            ocr_candidate_count = ocr_candidate_count.saturating_add(result.regions.len());
            pending_output.ocr_regions.extend(ocr_region_inputs(
                chunk,
                &result.frame,
                &result.regions,
            ));
            pending_output
                .frame_similarity_entries
                .push(frame_similarity_input(chunk, &result.frame));
        }
    }

    let persist_ocr_region_count = pending_output.ocr_regions.len();
    let persist_vlm_observation_count = pending_output.vlm_observations.len();
    let persist_similarity_entry_count = pending_output.frame_similarity_entries.len();
    let persist_started = Instant::now();
    persist_visual_output(&storage, chunk, pending_output)?;
    let persist_elapsed_ms = elapsed_ms(persist_started);

    tracing::info!(
        recording_id = %chunk.recording_id,
        job_id = %chunk.job_id,
        chunk_id = %chunk.id,
        source_kind = %chunk.source_kind,
        ocr_candidate_count,
        vlm_observation_count,
        persisted_ocr_region_count = persist_ocr_region_count,
        persisted_vlm_observation_count = persist_vlm_observation_count,
        persisted_similarity_entry_count = persist_similarity_entry_count,
        ocr_elapsed_ms,
        vlm_elapsed_ms,
        persist_elapsed_ms,
        elapsed_ms = elapsed_ms(started),
        "ai_processing_visual_chunk_completed"
    );

    Ok(())
}
