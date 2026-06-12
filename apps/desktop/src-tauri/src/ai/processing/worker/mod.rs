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
pub(in crate::ai) use actions::{
    retry_processing_job_for_action, update_processing_job_for_action,
};
#[cfg(test)]
pub(in crate::ai) use completion::{summarize_processing_completion, ProcessingCompletion};
pub(crate) use enqueue::enqueue_recording_ai_processing;
pub(in crate::ai) use enqueue::selected_transcription_model;
pub(in crate::ai) use scheduler::{recover_stale_processing, run_processing_worker_tick};
pub(in crate::ai) use types::AudioChunkProcessingOutcome;
