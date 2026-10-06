mod actions;
mod cancel;
mod chunk_context;
mod chunk_failure;
mod chunk_runner;
mod completion;
mod configuration;
pub(in crate::ai) use configuration::load_chunk_settings;
mod dispatch;
mod enqueue;
mod execution;
mod job_state;
mod readiness;
mod reprocess_audio;
mod retry_policy;
pub(crate) use reprocess_audio::reprocess_recording_audio;
mod scheduler;
mod types;
mod unit_selection;
pub(crate) use actions::cancel_recording_processing_for_privacy;
pub(in crate::ai) use actions::{
    retry_processing_job_for_action, update_processing_job_for_action,
};
#[cfg(test)]
pub(crate) use cancel::cancel_job_if_inactive;
#[cfg(test)]
pub(in crate::ai) use completion::{summarize_processing_completion, ProcessingCompletion};
#[cfg(test)]
pub(in crate::ai) use dispatch::claim_next_processing_chunk;
pub(crate) use enqueue::{
    admit_live_recording, enqueue_recording_ai_processing, finish_live_recording,
};
pub(in crate::ai) use readiness::check_transcription_readiness;
pub(in crate::ai) use readiness::selected_transcription_model;
pub(in crate::ai) use scheduler::{recover_stale_processing, run_processing_worker_tick};
pub(in crate::ai) use types::AudioChunkProcessingOutcome;
pub(in crate::ai) use unit_selection::load_unit_selection;
pub(crate) use unit_selection::{retry_processing_units_for_action, RetryProcessingUnitsInput};
