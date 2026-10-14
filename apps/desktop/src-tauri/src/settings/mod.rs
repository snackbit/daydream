pub(crate) mod commands;
mod credentials;
mod defaults;
pub(crate) mod input_activity;
mod persistence;
mod recall_types;
mod recall_validation;
mod service;
mod types;
mod validation;
mod vision_readiness;

pub(crate) use credentials::import_model_registry;
pub(crate) use credentials::prune_unreferenced_secrets;
pub use defaults::*;
pub use service::load_app_settings;
pub use types::*;
pub(crate) use validation::validate_settings;
pub(crate) use vision_readiness::vision_ready;
pub(crate) fn use_fast_local_processing(
    storage: &crate::storage::AppStorage,
) -> Result<(), AppSettingsError> {
    let mut settings = load_app_settings(storage)?;
    settings.ai.ocr.enabled = true;
    settings.ai.ocr.engine = "tesseract".into();
    settings.ai.processing.visual_sampling_mode = "adaptive-v1".into();
    settings.ai.processing.resource_mode = "fast".into();
    settings.ai.processing.max_parallel_ocr_jobs = 4;
    let inputs = persistence::settings_to_storage_inputs(
        &settings,
        service::now_ms().min(i64::MAX as u64) as i64,
    )?;
    storage
        .with_connection_mut(|connection| {
            let transaction = connection.transaction()?;
            for input in inputs.into_iter().filter(|input| {
                matches!(
                    input.key.as_str(),
                    "ai.ocr.enabled"
                        | "ai.ocr.engine"
                        | "ai.processing.visualSamplingMode"
                        | "ai.processing.resourceMode"
                        | "ai.processing.maxParallelOcrJobs"
                )
            }) {
                crate::storage::save_app_setting(&transaction, &input)?;
            }
            transaction.commit()
        })
        .map_err(service::storage_error)
}

#[cfg(test)]
mod tests;
