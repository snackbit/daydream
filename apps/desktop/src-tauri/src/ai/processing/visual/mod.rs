use super::super::*;

mod extraction;
mod image;
mod ocr;
mod persistence;
mod types;
mod vlm;

use extraction::extract_visual_frames;
#[cfg(test)]
pub(in crate::ai) use extraction::{
    effective_frame_recognition_fps, frame_sample_interval_ms, full_checkpoint_for_frame,
};
#[cfg(test)]
pub(in crate::ai) use image::visual_delta_regions;
use ocr::run_tesseract_ocr;
#[cfg(test)]
pub(in crate::ai) use ocr::{
    parse_tesseract_tsv, run_tesseract_ocr_with_binary, tesseract_ocr_command_args_for_test,
};
use persistence::{
    frame_similarity_input, ocr_region_inputs, persist_visual_output, vlm_observation_inputs,
    PendingVisualOutput,
};
pub(in crate::ai) use types::{ExtractedVisualFrame, OcrCandidate, PreviousVisualObservation};
#[cfg(test)]
pub(in crate::ai) use types::{VisionRequestConfig, VisualImage};
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
        &segment,
        segment_playback_offset_ms,
        chunk,
        recording_fps,
        &app_settings,
        !is_vlm_chunk,
        &cancel,
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
    let mut previous_observations = if is_vlm_chunk {
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
        for (frame_index, frame) in frame_set.vlm_frames.iter().enumerate() {
            check_processing_cancel(&cancel)?;
            log_visual_frame_started(chunk, frame_index, frame);
            let ocr_started = Instant::now();
            let (ocr_regions, ocr_source) = persisted_ocr_candidates
                .get(&frame.frame_timestamp_ms)
                .filter(|regions| !regions.is_empty())
                .map(|regions| (regions.clone(), "persisted"))
                .unwrap_or_else(|| match run_tesseract_ocr(app, frame.path(), frame) {
                    Ok(regions) => (regions, "fallback-tesseract"),
                    Err(error) => {
                        tracing::warn!(
                            recording_id = %chunk.recording_id,
                            job_id = %chunk.job_id,
                            chunk_id = %chunk.id,
                            frame_index,
                            frame_timestamp_ms = frame.frame_timestamp_ms,
                            source_frame_ref = %frame.source_frame_ref,
                            error_code = %error.code,
                            "ai_processing_visual_vlm_ocr_context_failed"
                        );
                        (Vec::new(), "missing")
                    }
                });
            let frame_ocr_elapsed_ms = elapsed_ms(ocr_started);
            ocr_elapsed_ms = ocr_elapsed_ms.saturating_add(frame_ocr_elapsed_ms);
            ocr_candidate_count = ocr_candidate_count.saturating_add(ocr_regions.len());
            let recent_observations =
                recent_visual_observations(&previous_observations, frame.frame_timestamp_ms);
            let vlm_started = Instant::now();
            let observations =
                request_vlm_observations(config, frame, &ocr_regions, &recent_observations).await?;
            let frame_vlm_elapsed_ms = elapsed_ms(vlm_started);
            vlm_elapsed_ms = vlm_elapsed_ms.saturating_add(frame_vlm_elapsed_ms);
            vlm_observation_count = vlm_observation_count.saturating_add(observations.len());
            update_visual_chunk_progress(app, &storage, chunk, frame_index + 1, frame_count)?;
            tracing::info!(
                recording_id = %chunk.recording_id,
                job_id = %chunk.job_id,
                chunk_id = %chunk.id,
                source_kind = %chunk.source_kind,
                frame_index,
                frame_timestamp_ms = frame.frame_timestamp_ms,
                source_frame_ref = %frame.source_frame_ref,
                ocr_source,
                ocr_candidate_count = ocr_regions.len(),
                recent_observation_count = recent_observations.len(),
                vlm_observation_count = observations.len(),
                ocr_elapsed_ms = frame_ocr_elapsed_ms,
                vlm_elapsed_ms = frame_vlm_elapsed_ms,
                "ai_processing_visual_frame_completed"
            );
            previous_observations.extend(observations.iter().map(|observation| {
                PreviousVisualObservation {
                    frame_timestamp_ms: observation.frame_timestamp_ms,
                    kind: observation.observation_kind.clone(),
                    text: observation.text.clone(),
                }
            }));
            pending_output
                .vlm_observations
                .extend(vlm_observation_inputs(chunk, config, observations));
        }
    } else {
        let frame_count = frame_set.ocr_frames.len();
        for (frame_index, frame) in frame_set.ocr_frames.iter().enumerate() {
            check_processing_cancel(&cancel)?;
            log_visual_frame_started(chunk, frame_index, frame);
            let ocr_started = Instant::now();
            let ocr_regions = run_tesseract_ocr(app, frame.path(), frame)?;
            let frame_ocr_elapsed_ms = elapsed_ms(ocr_started);
            ocr_elapsed_ms = ocr_elapsed_ms.saturating_add(frame_ocr_elapsed_ms);
            ocr_candidate_count = ocr_candidate_count.saturating_add(ocr_regions.len());
            pending_output
                .ocr_regions
                .extend(ocr_region_inputs(chunk, frame, &ocr_regions));
            pending_output
                .frame_similarity_entries
                .push(frame_similarity_input(chunk, frame));
            update_visual_chunk_progress(app, &storage, chunk, frame_index + 1, frame_count)?;
            tracing::info!(
                recording_id = %chunk.recording_id,
                job_id = %chunk.job_id,
                chunk_id = %chunk.id,
                source_kind = %chunk.source_kind,
                frame_index,
                frame_timestamp_ms = frame.frame_timestamp_ms,
                ocr_candidate_count = ocr_regions.len(),
                ocr_elapsed_ms = frame_ocr_elapsed_ms,
                "ai_processing_visual_frame_completed"
            );
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

fn log_visual_frame_started(
    chunk: &ProcessingChunkRow,
    frame_index: usize,
    frame: &ExtractedVisualFrame,
) {
    tracing::debug!(
        recording_id = %chunk.recording_id,
        job_id = %chunk.job_id,
        chunk_id = %chunk.id,
        source_kind = %chunk.source_kind,
        frame_index,
        frame_timestamp_ms = frame.frame_timestamp_ms,
        source_frame_ref = %frame.source_frame_ref,
        region_x = frame.region.x,
        region_y = frame.region.y,
        region_width = frame.region.width,
        region_height = frame.region.height,
        full_checkpoint = frame.full_checkpoint,
        abrupt_transition = frame.abrupt_transition,
        "ai_processing_visual_frame_started"
    );
}

fn update_visual_chunk_progress(
    app: &AppHandle,
    storage: &AppStorage,
    chunk: &ProcessingChunkRow,
    completed_frame_count: usize,
    total_frame_count: usize,
) -> Result<(), AiError> {
    let progress = if total_frame_count == 0 {
        1.0
    } else {
        (completed_frame_count as f64 / total_frame_count as f64).clamp(0.05, 0.99)
    };
    storage
        .with_connection(|connection| {
            storage::update_processing_chunk_progress(connection, &chunk.id, progress, now_ms())
                .map(|_row| ())
        })
        .map_err(storage_error)?;
    emit_processing_state(app, &chunk.recording_id);
    Ok(())
}

fn previous_visual_observations(
    storage: &AppStorage,
    recording_id: &str,
) -> Result<Vec<PreviousVisualObservation>, AiError> {
    storage
        .with_connection(|connection| {
            let mut observations = Vec::new();
            observations.extend(
                storage::list_ocr_regions(connection, recording_id)?
                    .into_iter()
                    .map(|region| PreviousVisualObservation {
                        frame_timestamp_ms: region.frame_timestamp_ms,
                        kind: "ocr".to_string(),
                        text: region.text,
                    }),
            );
            observations.extend(
                storage::list_vlm_observations(connection, recording_id)?
                    .into_iter()
                    .map(|observation| PreviousVisualObservation {
                        frame_timestamp_ms: observation.frame_timestamp_ms,
                        kind: observation.observation_kind,
                        text: observation.text,
                    }),
            );
            observations.sort_by(|left, right| {
                left.frame_timestamp_ms
                    .cmp(&right.frame_timestamp_ms)
                    .then_with(|| left.kind.cmp(&right.kind))
                    .then_with(|| left.text.cmp(&right.text))
            });
            Ok(observations)
        })
        .map_err(storage_error)
}

fn persisted_ocr_candidates_by_timestamp(
    storage: &AppStorage,
    recording_id: &str,
) -> Result<BTreeMap<i64, Vec<OcrCandidate>>, AiError> {
    storage
        .with_connection(|connection| storage::list_ocr_regions(connection, recording_id))
        .map_err(storage_error)
        .map(|regions| {
            let mut by_timestamp = BTreeMap::<i64, Vec<OcrCandidate>>::new();
            for region in regions {
                by_timestamp
                    .entry(region.frame_timestamp_ms)
                    .or_default()
                    .push(OcrCandidate {
                        confidence: region.confidence,
                        region: VisualRegion {
                            height: region.region_height,
                            width: region.region_width,
                            x: region.region_x,
                            y: region.region_y,
                        },
                        text: region.text,
                    });
            }
            by_timestamp
        })
}

fn recent_visual_observations(
    observations: &[PreviousVisualObservation],
    frame_timestamp_ms: i64,
) -> Vec<PreviousVisualObservation> {
    let mut recent = observations
        .iter()
        .filter(|observation| observation.frame_timestamp_ms < frame_timestamp_ms)
        .cloned()
        .collect::<Vec<_>>();
    recent.sort_by(|left, right| {
        right
            .frame_timestamp_ms
            .cmp(&left.frame_timestamp_ms)
            .then_with(|| right.kind.cmp(&left.kind))
            .then_with(|| right.text.cmp(&left.text))
    });
    recent.truncate(3);
    recent.reverse();
    recent
}
