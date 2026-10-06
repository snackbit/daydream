use super::super::*;

#[tauri::command(rename = "reprocessRecordingTranscription")]
pub fn reprocess_recording_transcription(
    app: AppHandle,
    input: RecordingAiResultInput,
    storage: State<'_, AppStorage>,
) -> Result<ProcessingJob, AiError> {
    let job = reprocess_recording_audio(&storage, &input.recording_id)?;
    emit_processing_state(&app, &input.recording_id);
    Ok(job)
}

#[tauri::command(rename = "retryProcessingUnits")]
pub fn retry_processing_units(
    app: AppHandle,
    input: RetryProcessingUnitsInput,
    storage: State<'_, AppStorage>,
) -> Result<ProcessingJob, AiError> {
    let job = retry_processing_units_for_action(&storage, &input)?;
    emit_processing_state(&app, &job.recording_id);
    Ok(job)
}

#[tauri::command(rename = "listProcessingJobs")]
pub fn list_processing_jobs(
    input: Option<ListProcessingJobsInput>,
    storage: State<'_, AppStorage>,
) -> Result<ProcessingJobList, AiError> {
    let recording_id = input.and_then(|value| value.recording_id);
    storage
        .with_connection(|connection| {
            storage::list_processing_jobs(connection, recording_id.as_deref())
        })
        .map_err(storage_error)
        .map(processing_job_list_from_rows)
}

#[tauri::command(rename = "pauseProcessingJob")]
pub fn pause_processing_job(
    app: AppHandle,
    input: ProcessingJobActionInput,
    storage: State<'_, AppStorage>,
) -> Result<ProcessingJob, AiError> {
    tracing::info!(job_id = %input.job_id, action = "paused", "ai_processing_action_requested");
    let job = update_processing_job_for_action(&storage, &input.job_id, "paused")?;
    emit_processing_state(&app, &job.recording_id);
    Ok(job)
}

#[tauri::command(rename = "resumeProcessingJob")]
pub fn resume_processing_job(
    app: AppHandle,
    input: ProcessingJobActionInput,
    storage: State<'_, AppStorage>,
) -> Result<ProcessingJob, AiError> {
    tracing::info!(job_id = %input.job_id, action = "queued", "ai_processing_action_requested");
    let job = update_processing_job_for_action(&storage, &input.job_id, "queued")?;
    emit_processing_state(&app, &job.recording_id);
    Ok(job)
}

#[tauri::command(rename = "cancelProcessingJob")]
pub fn cancel_processing_job(
    app: AppHandle,
    input: ProcessingJobActionInput,
    storage: State<'_, AppStorage>,
) -> Result<ProcessingJob, AiError> {
    tracing::info!(job_id = %input.job_id, action = "cancelled", "ai_processing_action_requested");
    let job = update_processing_job_for_action(&storage, &input.job_id, "cancelled")?;
    emit_processing_state(&app, &job.recording_id);
    Ok(job)
}

#[tauri::command(rename = "retryProcessingJob")]
pub fn retry_processing_job(
    app: AppHandle,
    input: ProcessingJobActionInput,
    storage: State<'_, AppStorage>,
) -> Result<ProcessingJob, AiError> {
    tracing::info!(job_id = %input.job_id, action = "retry", "ai_processing_action_requested");
    let job = retry_processing_job_for_action(&storage, &input.job_id)?;
    emit_processing_state(&app, &job.recording_id);
    Ok(job)
}

#[tauri::command(rename = "startRecordingAiProcessing")]
pub fn start_recording_ai_processing(
    app: AppHandle,
    input: StartRecordingAiProcessingInput,
    storage: State<'_, AppStorage>,
) -> Result<ProcessingJob, AiError> {
    let job = enqueue_recording_ai_processing(&app, &storage, &input.recording_id)?;
    Ok(processing_job_from_row_with_storage(&storage, job))
}
