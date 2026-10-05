mod activity;
mod audio;
mod ocr_documents;
mod planning;
pub(crate) use ocr_documents::capture_ocr_job_config;
#[cfg(test)]
pub(in crate::ai) use ocr_documents::infer_or_reuse_frame;
mod temp_files;
mod visual;
mod worker;
#[cfg(test)]
pub(crate) use worker::cancel_job_if_inactive;

pub(in crate::ai) fn ensure_chunk_active(
    storage: &crate::storage::AppStorage,
    chunk: &crate::storage::ProcessingChunkRow,
) -> Result<(), super::AiError> {
    if storage
        .with_connection(|connection| crate::recording_privacy::chunk_active(connection, chunk))
        .map_err(super::storage_error)?
    {
        return Ok(());
    }
    Err(super::ai_error(
        "ai_processing_cancelled",
        "Processing stopped because its source is no longer eligible.",
    ))
}

pub(in crate::ai) use activity::process_activity_chunk_inner;
#[cfg(test)]
pub(in crate::ai) use activity::{
    activity_intervals_from_events, frame_delta_score, merge_activity_intervals, planned_interval,
};
pub(in crate::ai) use audio::process_audio_chunk_inner;
#[cfg(test)]
pub(in crate::ai) use audio::{
    ffmpeg_audio_extract_args, persist_transcriptions_for_chunk, transcript_segment_offsets_ms,
    transcription_words_for_segment, PendingTranscription,
};
pub(crate) use ocr_documents::reprocess_recording_ocr;
pub(in crate::ai) use planning::*;
pub(in crate::ai) use temp_files::{
    processing_temp_audio_path, processing_temp_audio_range_path, processing_temp_visual_directory,
    TempAudioFile, TempProcessingDirectory,
};
pub(in crate::ai) use visual::process_visual_chunk_inner;
#[cfg(test)]
pub(in crate::ai) use visual::{
    effective_frame_recognition_fps, frame_sample_interval_ms, full_checkpoint_for_frame,
    parse_tesseract_tsv, persist_visual_output_for_test, previous_visual_observations,
    run_tesseract_ocr_with_binary, tesseract_ocr_command_args_for_test,
    validate_vlm_observation_text, vision_chat_completions_url, visual_delta_regions,
    vlm_chat_completions_body, ExtractedVisualFrame, OcrCandidate, PendingVisualOutputForTest,
    PreviousVisualObservation, VisionRequestConfig, VisualImage,
};
pub(crate) use worker::cancel_recording_processing_for_privacy;
pub(crate) use worker::enqueue_recording_ai_processing;
#[cfg(test)]
pub(in crate::ai) use worker::{
    claim_next_processing_chunk, summarize_processing_completion, ProcessingCompletion,
};
pub(in crate::ai) use worker::{
    recover_stale_processing, retry_processing_job_for_action, run_processing_worker_tick,
    selected_transcription_model, update_processing_job_for_action, AudioChunkProcessingOutcome,
};
