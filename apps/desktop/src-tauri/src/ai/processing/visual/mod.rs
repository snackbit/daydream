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
pub(in crate::ai) use ocr::{parse_tesseract_tsv, run_tesseract_ocr_with_binary};
use persistence::{
    frame_similarity_input, ocr_region_inputs, persist_visual_output, vlm_observation_inputs,
    PendingVisualOutput,
};
#[cfg(test)]
pub(in crate::ai) use types::{
    ExtractedVisualFrame, VisualImage, VlmObservationCandidate, VlmObservationPayload,
    VlmObservationRegion,
};
#[cfg(test)]
pub(in crate::ai) use vlm::validate_vlm_observations;
use vlm::{request_vlm_observations, vision_provider_ready, vision_request_config};

pub(in crate::ai) async fn process_visual_chunk_inner(
    app: &AppHandle,
    chunk: &ProcessingChunkRow,
    cancel: Arc<AtomicBool>,
) -> Result<(), AiError> {
    let storage = app.state::<AppStorage>();
    let app_settings = settings::load_app_settings(&storage).map_err(settings_error)?;
    let is_vlm_chunk = chunk.source_kind == "vlm";
    if chunk.source_kind == "ocr" && !app_settings.ai.ocr.enabled {
        return Ok(());
    }
    if is_vlm_chunk && !vision_provider_ready(&app_settings) {
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

    let frame_set = extract_visual_frames(
        app,
        &segment,
        segment_playback_offset_ms,
        chunk,
        recording_fps,
        &app_settings,
        &cancel,
    )?;

    let vision_config = if is_vlm_chunk {
        Some(vision_request_config(&storage, &app_settings)?)
    } else {
        None
    };
    let mut pending_output = PendingVisualOutput::default();

    for frame in frame_set.frames {
        check_processing_cancel(&cancel)?;
        let ocr_regions = run_tesseract_ocr(frame.path(), &frame)?;
        if is_vlm_chunk {
            let config = vision_config.as_ref().ok_or_else(|| {
                ai_error(
                    "ai_vision_remote_unconfigured",
                    "Vision provider is not configured.",
                )
            })?;
            let observations = request_vlm_observations(config, &frame, &ocr_regions).await?;
            pending_output
                .vlm_observations
                .extend(vlm_observation_inputs(chunk, config, observations));
        } else {
            pending_output
                .ocr_regions
                .extend(ocr_region_inputs(chunk, &frame, &ocr_regions));
            pending_output
                .frame_similarity_entries
                .push(frame_similarity_input(chunk, &frame));
        }
    }

    persist_visual_output(&storage, chunk, pending_output)?;

    Ok(())
}
