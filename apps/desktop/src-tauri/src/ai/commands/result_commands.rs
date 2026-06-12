use super::super::*;

#[tauri::command(rename = "getRecordingAiProcessingState")]
pub fn get_recording_ai_processing_state(
    input: RecordingAiResultInput,
    storage: State<'_, AppStorage>,
) -> Result<RecordingAiProcessingState, AiError> {
    processing_state_for_recording(&storage, &input.recording_id)
}

#[tauri::command(rename = "getRecordingTranscript")]
pub fn get_recording_transcript(
    input: GetRecordingTranscriptInput,
    storage: State<'_, AppStorage>,
) -> Result<RecordingTranscript, AiError> {
    transcript_for_recording(&storage, &input.recording_id)
}

#[tauri::command(rename = "getRecordingActivity")]
pub fn get_recording_activity(
    input: RecordingAiResultInput,
    storage: State<'_, AppStorage>,
) -> Result<RecordingActivity, AiError> {
    activity_for_recording(&storage, &input.recording_id)
}

#[tauri::command(rename = "getRecordingVisualObservations")]
pub fn get_recording_visual_observations(
    input: RecordingAiResultInput,
    storage: State<'_, AppStorage>,
) -> Result<RecordingVisualObservations, AiError> {
    visual_observations_for_recording(&storage, &input.recording_id)
}
