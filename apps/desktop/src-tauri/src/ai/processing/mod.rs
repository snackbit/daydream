mod activity;
mod audio;
mod planning;
mod temp_files;
mod visual;
mod worker;

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
pub(in crate::ai) use planning::*;
pub(in crate::ai) use temp_files::{
    processing_temp_audio_path, processing_temp_audio_range_path, processing_temp_visual_directory,
    TempAudioFile, TempProcessingDirectory,
};
pub(in crate::ai) use visual::process_visual_chunk_inner;
#[cfg(test)]
pub(in crate::ai) use visual::{
    effective_frame_recognition_fps, frame_sample_interval_ms, full_checkpoint_for_frame,
    parse_tesseract_tsv, run_tesseract_ocr_with_binary, validate_vlm_observations,
    visual_delta_regions, ExtractedVisualFrame, VisualImage, VlmObservationCandidate,
    VlmObservationPayload, VlmObservationRegion,
};
pub(crate) use worker::enqueue_recording_ai_processing;
pub(in crate::ai) use worker::{
    recover_stale_processing, retry_processing_job_for_action, run_processing_worker_tick,
    selected_transcription_model, update_processing_job_for_action, AudioChunkProcessingOutcome,
};
#[cfg(test)]
pub(in crate::ai) use worker::{summarize_processing_completion, ProcessingCompletion};
