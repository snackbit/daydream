mod actions;
mod cancel;
mod chunk_context;
mod chunk_runner;
mod completion;
mod dispatch;
mod enqueue;
mod execution;
mod job_state;
mod scheduler;
mod types;
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
pub(crate) use enqueue::enqueue_recording_ai_processing;
pub(in crate::ai) use enqueue::selected_transcription_model;
pub(in crate::ai) use scheduler::{recover_stale_processing, run_processing_worker_tick};
pub(in crate::ai) use types::AudioChunkProcessingOutcome;
