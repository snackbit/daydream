use super::super::super::*;

pub(crate) fn reprocess_recording_audio(
    storage: &AppStorage,
    recording_id: &str,
) -> Result<ProcessingJob, AiError> {
    processing::reprocessing::whole_recording(
        storage,
        recording_id,
        processing::reprocessing::ReprocessingOperation::Transcription,
    )
}
