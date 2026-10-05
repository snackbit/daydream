mod activity;
mod processing;
mod transcript;
mod visual;
mod vlm;

pub(in crate::ai) use activity::activity_for_recording;
pub(crate) use processing::emit_processing_state;
#[cfg(test)]
pub(in crate::ai) use processing::processing_chunk_from_row;
pub(crate) use processing::processing_state_for_recording;
pub(in crate::ai) use processing::{
    processing_chunk_status_counts, processing_job_from_row_with_storage,
    processing_job_list_from_rows,
};
pub(in crate::ai) use transcript::transcript_for_recording;
#[cfg(test)]
pub(in crate::ai) use transcript::transcription_chunk_state;
pub(crate) use transcript::visible_transcript_segments_for_recording;
pub(crate) use visual::{
    visible_visual_observation_rows_for_recording, visual_observations_for_recording,
};
#[cfg(test)]
pub(in crate::ai) use vlm::vlm_state_for_recording;
