use super::super::*;
pub(in crate::ai) use concurrency::{
    acquire_ocr_cancellable as acquire_ocr_permit_for_documents,
    acquire_vlm_cancellable as acquire_vlm_permit_for_documents,
};
pub(in crate::ai) use image::decode_png_rgba as decode_png_rgba_for_ocr;
pub(in crate::ai) use vlm::{
    verify_current_vision_configuration,
    vision_chat_completions_url as vision_chat_completions_url_for_ocr,
    vision_request_config as vision_request_config_for_ocr,
};

mod change_processing;
mod change_protocol;
#[cfg(test)]
mod change_protocol_tests;
mod concurrency;
mod context;
mod extraction;
mod frame_jobs;
mod image;
mod ocr;
mod persistence;
mod provider_configuration;
pub(in crate::ai) mod shared_inference;
mod types;
mod vlm;

use concurrency::{acquire_ocr_cancellable, acquire_ocr_permit, acquire_vlm_cancellable};
#[cfg(test)]
pub(in crate::ai) use context::previous_visual_observations;
use context::{persisted_ocr_candidates_by_timestamp, recent_visual_observations};
#[cfg(test)]
pub(in crate::ai) use extraction::frame_sample_interval_ms;
#[cfg(test)]
pub(in crate::ai) use extraction::{effective_frame_recognition_fps, full_checkpoint_for_frame};
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
pub(in crate::ai) use types::VisualImage;
pub(in crate::ai) use types::{
    ExtractedVisualFrame, OcrCandidate, PendingVlmObservation, PreviousVisualObservation,
    VisionRequestConfig,
};
use vlm::{request_vlm_observations, vision_request_config};
mod configuration;

pub(in crate::ai) async fn probe_provider(
    config: &VisionRequestConfig,
    path: &Path,
    cancel: Arc<AtomicBool>,
) -> Result<(), AiError> {
    let frame = types::ExtractedVisualFrame {
        abrupt_transition: false,
        after_context: None,
        before_context: None,
        frame_timestamp_ms: 0,
        full_checkpoint: true,
        height: 64,
        path: path.into(),
        region: VisualRegion {
            x: 0,
            y: 0,
            width: 128,
            height: 64,
        },
        signature: "synthetic-provider-test".into(),
        source_frame_ref: "synthetic-provider-test".into(),
        width: 128,
    };
    request_vlm_observations(config, &frame, &[], &[], cancel)
        .await
        .map(|_| ())
}
#[cfg(test)]
pub(in crate::ai) use vlm::{
    validate_vlm_observation_text, vision_chat_completions_url, vlm_chat_completions_body,
};

pub(in crate::ai) async fn process_visual_chunk_inner(
    app: &AppHandle,
    chunk: &ProcessingChunkRow,
    cancel: Arc<AtomicBool>,
    lease: crate::recording_privacy::ProcessingLease,
) -> Result<(), AiError> {
    if chunk.source_kind == "ocr" {
        return super::ocr_documents::process_ocr_document_chunk(app, chunk, cancel, lease).await;
    }
    if shared_inference::enabled(&app.state::<AppStorage>(), &chunk.job_id)? {
        return change_processing::process(app, chunk, cancel, lease).await;
    }
    let started = Instant::now();
    let storage = app.state::<AppStorage>();
    ensure_chunk_active(&storage, chunk)?;
    let app_settings = configuration::load_visual_settings(&storage, chunk)?;
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
            Ok((
                source_segment(connection, chunk, segment)?,
                segment_playback_offset_ms,
                recording.default_fps,
            ))
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

    let extraction_app = app.clone();
    let extraction_chunk = chunk.clone();
    let extraction_settings = app_settings.clone();
    let extraction_cancel = cancel.clone();
    let extraction_lease = lease.clone();
    let mut frame_set = tauri::async_runtime::spawn_blocking(move || {
        let _lease = extraction_lease;
        extract_visual_frames(
            &extraction_app,
            VisualExtractionRequest {
                segment: &segment,
                segment_playback_offset_ms,
                chunk: &extraction_chunk,
                recording_fps,
                app_settings: &extraction_settings,
                include_ocr_regions: !is_vlm_chunk,
                cancel: &extraction_cancel,
            },
        )
    })
    .await
    .map_err(|_| {
        ai_error(
            "ai_processing_visual_extraction_worker_failed",
            "Visual extraction worker failed.",
        )
    })??;
    let selection = super::worker::load_unit_selection(&storage, &chunk.id)?;
    let succeeded=storage.with_connection(|connection|{
        let mut statement=connection.prepare("SELECT frame_timestamp_ms FROM visual_frame_processing_states WHERE chunk_id=?1 AND status='succeeded'")?;
        let rows=statement.query_map([&chunk.id],|row|row.get::<_,i64>(0))?;rows.collect::<rusqlite::Result<std::collections::BTreeSet<_>>>()
    }).map_err(storage_error)?;
    frame_set.vlm_frames.retain(|frame| {
        !succeeded.contains(&frame.frame_timestamp_ms)
            && selection
                .frame_timestamps
                .as_ref()
                .is_none_or(|timestamps| timestamps.contains(&frame.frame_timestamp_ms))
    });
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
        let observations = context::previous_visual_observations_for_source(
            &storage,
            &chunk.recording_id,
            chunk.track_id.as_deref(),
        )?;
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
        let candidates = persisted_ocr_candidates_by_timestamp(
            &storage,
            &chunk.recording_id,
            chunk.track_id.as_deref(),
        )?;
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
                lease: lease.clone(),
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
    if !is_vlm_chunk {
        persist_visual_output(&storage, chunk, pending_output)?;
    }
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
