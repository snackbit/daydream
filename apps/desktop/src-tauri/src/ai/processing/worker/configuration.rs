use super::super::super::*;

pub(in crate::ai) fn load_job_settings(
    storage: &AppStorage,
    job_id: &str,
) -> Result<settings::AppSettings, AiError> {
    match storage
        .with_connection(|connection| storage::processing_job_config(connection, job_id))
        .map_err(storage_error)?
    {
        Some(json) => serde_json::from_str(&json).map_err(|_| {
            ai_error(
                "ai_processing_config_invalid",
                "The saved processing configuration is invalid.",
            )
        }),
        None => settings::load_app_settings(storage).map_err(settings_error),
    }
}

pub(in crate::ai) fn load_chunk_settings(
    storage: &AppStorage,
    chunk: &ProcessingChunkRow,
) -> Result<settings::AppSettings, AiError> {
    let mut settings = match storage
        .with_connection(|connection| storage::processing_chunk_config(connection, &chunk.id))
        .map_err(storage_error)?
    {
        Some(json) => serde_json::from_str(&json).map_err(|_| {
            ai_error(
                "ai_processing_config_invalid",
                "The saved source configuration is invalid.",
            )
        }),
        None => load_job_settings(storage, &chunk.job_id),
    }?;
    if settings::load_app_settings(storage)
        .map_err(settings_error)?
        .ai
        .processing
        .resource_mode
        == "background"
    {
        settings.ai.processing.max_parallel_ocr_jobs = 1;
        settings.ai.processing.max_parallel_vlm_jobs = 1;
    }
    Ok(settings)
}
