use std::{
    collections::BTreeMap,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

use crate::{
    notifications::{self, Notification, NotificationAction},
    secret_crypto, settings,
    storage::{
        self, AiModelAssetRow, AppStorage, ProcessingJobRow, SaveAiModelAssetInput,
        SaveAppSettingInput,
    },
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiError {
    pub code: String,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiModelAsset {
    pub created_at_ms: i64,
    pub download_status: String,
    pub etag: Option<String>,
    pub id: String,
    pub label: String,
    pub local_path: Option<String>,
    pub provider: String,
    pub self_test_status: String,
    pub sha256: Option<String>,
    pub size_bytes: Option<i64>,
    pub updated_at_ms: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiModelAssetList {
    pub models: Vec<AiModelAsset>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DownloadAiModelInput {
    pub model_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeleteAiModelInput {
    pub model_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SelfTestAiModelInput {
    pub model_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SaveAiProviderSecretInput {
    pub plaintext: String,
    pub role: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeleteAiProviderSecretInput {
    pub secret_ref: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SaveAiProviderSecretResult {
    pub secret_ref: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiProviderStatus {
    pub local_transcription: String,
    pub remote_transcription: String,
    pub vision: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProcessingJob {
    pub created_at_ms: i64,
    pub error_code: Option<String>,
    pub id: String,
    pub kind: String,
    pub progress: f64,
    pub recording_id: String,
    pub status: String,
    pub updated_at_ms: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProcessingJobList {
    pub jobs: Vec<ProcessingJob>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListProcessingJobsInput {
    pub recording_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProcessingJobActionInput {
    pub job_id: String,
}

struct CatalogModel {
    id: &'static str,
    label: &'static str,
}

const LOCAL_WHISPER_CATALOG: [CatalogModel; 6] = [
    CatalogModel {
        id: "tiny",
        label: "Whisper tiny",
    },
    CatalogModel {
        id: "base",
        label: "Whisper base",
    },
    CatalogModel {
        id: "small",
        label: "Whisper small",
    },
    CatalogModel {
        id: "medium",
        label: "Whisper medium",
    },
    CatalogModel {
        id: "large-v3-turbo",
        label: "Whisper large v3 turbo",
    },
    CatalogModel {
        id: "large-v3",
        label: "Whisper large v3",
    },
];

#[tauri::command(rename = "listAiModels")]
pub fn list_ai_models(storage: State<'_, AppStorage>) -> Result<AiModelAssetList, AiError> {
    storage
        .with_connection(storage::list_ai_model_assets)
        .map_err(storage_error)
        .map(models_from_stored)
}

#[tauri::command(rename = "downloadAiModel")]
pub fn download_ai_model(
    input: DownloadAiModelInput,
    storage: State<'_, AppStorage>,
) -> Result<Notification, AiError> {
    ensure_catalog_model(&input.model_id)?;
    storage
        .with_connection(|connection| {
            let timestamp_ms = now_ms();
            let catalog = catalog_model(&input.model_id).ok_or(rusqlite::Error::InvalidQuery)?;
            storage::save_ai_model_asset(
                connection,
                &SaveAiModelAssetInput {
                    display_label: catalog.label.to_string(),
                    download_status: "failed".to_string(),
                    etag: None,
                    id: catalog.id.to_string(),
                    local_path: None,
                    provider: "local-whisper-cpp".to_string(),
                    self_test_status: "not-run".to_string(),
                    sha256: None,
                    size_bytes: None,
                    timestamp_ms,
                },
            )?;

            notifications::save_unavailable_notification(
                connection,
                "model-download",
                "Download unavailable",
                &format!(
                    "Model download is not implemented yet for {}.",
                    catalog.label
                ),
                &[model_download_retry_action(catalog.id)],
                timestamp_ms,
            )
        })
        .map_err(storage_error)
        .and_then(notification_from_row)
}

#[tauri::command(rename = "deleteAiModel")]
pub fn delete_ai_model(
    input: DeleteAiModelInput,
    storage: State<'_, AppStorage>,
) -> Result<AiModelAssetList, AiError> {
    storage
        .with_connection(|connection| {
            storage::delete_ai_model_asset(connection, &input.model_id)?;
            storage::list_ai_model_assets(connection)
        })
        .map_err(storage_error)
        .map(models_from_stored)
}

#[tauri::command(rename = "selfTestAiModel")]
pub fn self_test_ai_model(
    input: SelfTestAiModelInput,
    storage: State<'_, AppStorage>,
) -> Result<Notification, AiError> {
    ensure_catalog_model(&input.model_id)?;
    storage
        .with_connection(|connection| {
            let timestamp_ms = now_ms();
            let catalog = catalog_model(&input.model_id).ok_or(rusqlite::Error::InvalidQuery)?;
            let existing = storage::get_ai_model_asset(connection, catalog.id)?;
            let download_status = existing
                .as_ref()
                .map_or("missing".to_string(), |row| row.download_status.clone());
            storage::save_ai_model_asset(
                connection,
                &SaveAiModelAssetInput {
                    display_label: catalog.label.to_string(),
                    download_status,
                    etag: existing.as_ref().and_then(|row| row.etag.clone()),
                    id: catalog.id.to_string(),
                    local_path: existing.as_ref().and_then(|row| row.local_path.clone()),
                    provider: "local-whisper-cpp".to_string(),
                    self_test_status: "failed".to_string(),
                    sha256: existing.as_ref().and_then(|row| row.sha256.clone()),
                    size_bytes: existing.as_ref().and_then(|row| row.size_bytes),
                    timestamp_ms,
                },
            )?;

            notifications::save_unavailable_notification(
                connection,
                "model-self-test",
                "Smoke test unavailable",
                &format!(
                    "Model smoke test is not implemented yet for {}.",
                    catalog.label
                ),
                &[],
                timestamp_ms,
            )
        })
        .map_err(storage_error)
        .and_then(notification_from_row)
}

#[tauri::command(rename = "getAiProviderStatus")]
pub fn get_ai_provider_status(storage: State<'_, AppStorage>) -> Result<AiProviderStatus, AiError> {
    let app_settings = settings::load_app_settings(&storage).map_err(|error| AiError {
        code: error.code,
        message: error.message,
    })?;

    Ok(AiProviderStatus {
        local_transcription: "missing".to_string(),
        remote_transcription: if app_settings.ai.transcription.provider == "openai-compatible" {
            if app_settings.ai.transcription.remote_api_key_ref.is_some() {
                "configured".to_string()
            } else {
                "missing-key".to_string()
            }
        } else {
            "not-configured".to_string()
        },
        vision: if app_settings.ai.vision.provider == "disabled" {
            "disabled".to_string()
        } else if app_settings.ai.vision.api_key_ref.is_some() {
            "configured".to_string()
        } else {
            "missing-key".to_string()
        },
    })
}

#[tauri::command(rename = "saveAiProviderSecret")]
pub fn save_ai_provider_secret(
    input: SaveAiProviderSecretInput,
    storage: State<'_, AppStorage>,
) -> Result<SaveAiProviderSecretResult, AiError> {
    if input.role != "transcription" && input.role != "vision" {
        return Err(AiError {
            code: "ai_secret_role_invalid".to_string(),
            message: "AI secret role must be transcription or vision.".to_string(),
        });
    }
    if input.plaintext.trim().is_empty() {
        return Err(AiError {
            code: "ai_secret_empty".to_string(),
            message: "API key cannot be empty.".to_string(),
        });
    }

    let envelope =
        secret_crypto::encrypt_sensitive_setting_with_platform_key(input.plaintext.trim())
            .map_err(secret_error)?;
    let secret_ref = format!("ai.secret.{}.{}", input.role, Uuid::now_v7());
    storage
        .with_connection(|connection| {
            storage::save_app_setting(
                connection,
                &SaveAppSettingInput {
                    key: secret_ref.clone(),
                    value_json: envelope,
                    is_sensitive: true,
                    timestamp_ms: now_ms(),
                },
            )?;
            Ok(())
        })
        .map_err(storage_error)?;

    Ok(SaveAiProviderSecretResult { secret_ref })
}

#[tauri::command(rename = "deleteAiProviderSecret")]
pub fn delete_ai_provider_secret(
    input: DeleteAiProviderSecretInput,
    storage: State<'_, AppStorage>,
) -> Result<(), AiError> {
    if !input.secret_ref.starts_with("ai.secret.") {
        return Err(AiError {
            code: "ai_secret_ref_invalid".to_string(),
            message: "AI secret reference is invalid.".to_string(),
        });
    }

    storage
        .with_connection(|connection| storage::delete_app_setting(connection, &input.secret_ref))
        .map_err(storage_error)
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
    input: ProcessingJobActionInput,
    storage: State<'_, AppStorage>,
) -> Result<ProcessingJob, AiError> {
    update_processing_job(&storage, &input.job_id, "paused")
}

#[tauri::command(rename = "resumeProcessingJob")]
pub fn resume_processing_job(
    input: ProcessingJobActionInput,
    storage: State<'_, AppStorage>,
) -> Result<ProcessingJob, AiError> {
    update_processing_job(&storage, &input.job_id, "queued")
}

#[tauri::command(rename = "cancelProcessingJob")]
pub fn cancel_processing_job(
    input: ProcessingJobActionInput,
    storage: State<'_, AppStorage>,
) -> Result<ProcessingJob, AiError> {
    update_processing_job(&storage, &input.job_id, "cancelled")
}

#[tauri::command(rename = "retryProcessingJob")]
pub fn retry_processing_job(
    input: ProcessingJobActionInput,
    storage: State<'_, AppStorage>,
) -> Result<ProcessingJob, AiError> {
    update_processing_job(&storage, &input.job_id, "queued")
}

fn models_from_stored(stored: Vec<AiModelAssetRow>) -> AiModelAssetList {
    let models = LOCAL_WHISPER_CATALOG
        .iter()
        .map(|catalog| {
            stored.iter().find(|row| row.id == catalog.id).map_or_else(
                || catalog_asset(catalog),
                |row| model_asset_from_row(row.clone()),
            )
        })
        .collect();

    AiModelAssetList { models }
}

fn catalog_asset(catalog: &CatalogModel) -> AiModelAsset {
    AiModelAsset {
        created_at_ms: 0,
        download_status: "missing".to_string(),
        etag: None,
        id: catalog.id.to_string(),
        label: catalog.label.to_string(),
        local_path: None,
        provider: "local-whisper-cpp".to_string(),
        self_test_status: "not-run".to_string(),
        sha256: None,
        size_bytes: None,
        updated_at_ms: 0,
    }
}

fn model_asset_from_row(row: AiModelAssetRow) -> AiModelAsset {
    AiModelAsset {
        created_at_ms: row.created_at_ms,
        download_status: row.download_status,
        etag: row.etag,
        id: row.id,
        label: row.display_label,
        local_path: row.local_path,
        provider: row.provider,
        self_test_status: row.self_test_status,
        sha256: row.sha256,
        size_bytes: row.size_bytes,
        updated_at_ms: row.updated_at_ms,
    }
}

fn processing_job_list_from_rows(rows: Vec<ProcessingJobRow>) -> ProcessingJobList {
    ProcessingJobList {
        jobs: rows.into_iter().map(processing_job_from_row).collect(),
    }
}

fn processing_job_from_row(row: ProcessingJobRow) -> ProcessingJob {
    ProcessingJob {
        created_at_ms: row.created_at_ms,
        error_code: row.error,
        id: row.id,
        kind: row.kind,
        progress: match row.status.as_str() {
            "succeeded" => 1.0,
            "running" => 0.5,
            _ => 0.0,
        },
        recording_id: row.recording_id,
        status: row.status,
        updated_at_ms: row.updated_at_ms,
    }
}

fn update_processing_job(
    storage: &AppStorage,
    job_id: &str,
    status: &str,
) -> Result<ProcessingJob, AiError> {
    storage
        .with_connection(|connection| {
            storage::update_processing_job_status(connection, job_id, status, None, now_ms())
        })
        .map_err(storage_error)
        .map(processing_job_from_row)
}

fn notification_from_row(row: storage::NotificationRow) -> Result<Notification, AiError> {
    notifications::notification_from_row(row).map_err(|error| AiError {
        code: error.code,
        message: error.message,
    })
}

fn model_download_retry_action(model_id: &str) -> NotificationAction {
    NotificationAction {
        handler: "model-download-retry".to_string(),
        id: "retry".to_string(),
        label: "Retry".to_string(),
        payload: BTreeMap::from([("modelId".to_string(), model_id.to_string())]),
        style: "secondary".to_string(),
    }
}

fn ensure_catalog_model(model_id: &str) -> Result<(), AiError> {
    catalog_model(model_id)
        .map(|_model| ())
        .ok_or_else(|| AiError {
            code: "ai_model_unknown".to_string(),
            message: format!("AI model is unknown: {model_id}"),
        })
}

fn catalog_model(model_id: &str) -> Option<&'static CatalogModel> {
    LOCAL_WHISPER_CATALOG
        .iter()
        .find(|model| model.id == model_id)
}

fn storage_error(error: rusqlite::Error) -> AiError {
    AiError {
        code: "ai_storage_failed".to_string(),
        message: format!("AI storage failed: {error}"),
    }
}

fn secret_error(error: secret_crypto::SecretCryptoError) -> AiError {
    AiError {
        code: error.code,
        message: error.message,
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_models_default_to_missing() {
        let model = catalog_asset(&LOCAL_WHISPER_CATALOG[2]);

        assert_eq!(model.id, "small");
        assert_eq!(model.download_status, "missing");
        assert_eq!(model.self_test_status, "not-run");
    }

    #[test]
    fn retry_action_keeps_model_id_in_payload() {
        let action = model_download_retry_action("small");

        assert_eq!(action.handler, "model-download-retry");
        assert_eq!(action.payload.get("modelId"), Some(&"small".to_string()));
    }
}
