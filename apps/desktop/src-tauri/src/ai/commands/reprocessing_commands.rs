use super::super::*;
use processing::reprocessing;
#[tauri::command(rename = "previewRecordingReprocessing")]
pub fn preview_recording_reprocessing(
    input: reprocessing::PreviewRecordingReprocessingInput,
    storage: State<'_, AppStorage>,
) -> Result<reprocessing::RecordingReprocessingPreview, AiError> {
    reprocessing::preview(&storage, &input)
}
#[tauri::command(rename = "startRecordingReprocessing")]
pub async fn start_recording_reprocessing(
    app: AppHandle,
    input: reprocessing::StartRecordingReprocessingInput,
    storage: State<'_, AppStorage>,
) -> Result<ProcessingJob, AiError> {
    let _admission = super::processing_commands::admit_processing(&app, "reprocess-recording")?;
    let _privacy = crate::recording_privacy::mutation_lease(&input.recording_id).await;
    let id = input.recording_id.clone();
    let job = reprocessing::start(&storage, input)?;
    emit_processing_state(&app, &id);
    Ok(job)
}
