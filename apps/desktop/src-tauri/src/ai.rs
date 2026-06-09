use std::{
    borrow::Cow,
    collections::{BTreeMap, HashSet, VecDeque},
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::Mutex,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use futures_util::StreamExt;
use reqwest::{
    header::{CONTENT_LENGTH, ETAG, IF_RANGE, RANGE},
    StatusCode,
};
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use tauri::{AppHandle, Emitter, Manager, State};
use uuid::Uuid;

use crate::{
    notifications::{self, Notification, NotificationAction},
    secret_crypto, settings,
    storage::{
        self, AiModelAssetRow, AppStorage, NotificationRow, ProcessingJobRow,
        SaveAiModelAssetInput, SaveAppSettingInput, SaveNotificationInput,
    },
    whisper_backend,
};

pub const AI_MODELS_CHANGED_EVENT: &str = "ai-models-changed";
const MODEL_PROGRESS_UPDATE_INTERVAL: Duration = Duration::from_millis(500);
const MODEL_PROGRESS_UPDATE_BYTES: i64 = 4 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiError {
    pub code: String,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiModelAsset {
    pub checksum: Option<String>,
    pub checksum_algorithm: Option<String>,
    pub created_at_ms: i64,
    pub downloaded_bytes: i64,
    pub download_status: String,
    pub etag: Option<String>,
    pub error_code: Option<String>,
    pub id: String,
    pub label: String,
    pub local_path: Option<String>,
    pub provider: String,
    pub self_test_status: String,
    pub size_bytes: Option<i64>,
    pub source_url: Option<String>,
    pub total_bytes: Option<i64>,
    pub updated_at_ms: i64,
    pub verification_status: String,
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
    pub notification_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CancelAiModelDownloadInput {
    pub model_id: String,
    pub notification_id: Option<String>,
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
    pub local_transcription: AiLocalTranscriptionStatus,
    pub remote_transcription: String,
    pub vision: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiLocalTranscriptionStatus {
    pub available_backends: Vec<AiBackendStatus>,
    pub checksum_status: String,
    pub local_model_id: String,
    pub model_download_status: String,
    pub model_path: Option<String>,
    pub selected_backend: String,
    pub self_test_status: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiBackendStatus {
    pub available: bool,
    pub id: String,
    pub reason: Option<String>,
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

pub struct AiModelManager {
    inner: Mutex<AiModelManagerState>,
}

#[derive(Default)]
struct AiModelManagerState {
    active: Option<String>,
    cancelled: HashSet<String>,
    queued: VecDeque<String>,
}

#[derive(Debug, PartialEq, Eq)]
enum DownloadRequestState {
    AlreadyRunning,
    Queued,
    Started,
}

struct CatalogModel {
    id: &'static str,
    label: &'static str,
    sha1: &'static str,
    size_bytes: i64,
}

const LOCAL_WHISPER_CATALOG: [CatalogModel; 6] = [
    CatalogModel {
        id: "tiny",
        label: "Whisper tiny",
        sha1: "bd577a113a864445d4c299885e0cb97d4ba92b5f",
        size_bytes: 75 * 1024 * 1024,
    },
    CatalogModel {
        id: "base",
        label: "Whisper base",
        sha1: "465707469ff3a37a2b9b8d8f89f2f99de7299dac",
        size_bytes: 142 * 1024 * 1024,
    },
    CatalogModel {
        id: "small",
        label: "Whisper small",
        sha1: "55356645c2b361a969dfd0ef2c5a50d530afd8d5",
        size_bytes: 466 * 1024 * 1024,
    },
    CatalogModel {
        id: "medium",
        label: "Whisper medium",
        sha1: "fd9727b6e1217c2f614f9b698455c4ffd82463b4",
        size_bytes: 1_500 * 1024 * 1024,
    },
    CatalogModel {
        id: "large-v3-turbo",
        label: "Whisper large v3 turbo",
        sha1: "4af2b29d7ec73d781377bfd1758ca957a807e941",
        size_bytes: 1_500 * 1024 * 1024,
    },
    CatalogModel {
        id: "large-v3",
        label: "Whisper large v3",
        sha1: "ad82bf6a9043ceed055076d0fd39f5f186ff8062",
        size_bytes: 2_900 * 1024 * 1024,
    },
];

impl AiModelManager {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(AiModelManagerState::default()),
        }
    }

    fn request_download(&self, model_id: &str) -> DownloadRequestState {
        let mut inner = self.inner.lock().expect("model manager lock");
        inner.cancelled.remove(model_id);
        if inner.active.as_deref() == Some(model_id) {
            return DownloadRequestState::AlreadyRunning;
        }
        if inner.queued.iter().any(|queued| queued == model_id) {
            return DownloadRequestState::Queued;
        }
        if inner.active.is_none() {
            inner.active = Some(model_id.to_string());
            DownloadRequestState::Started
        } else {
            inner.queued.push_back(model_id.to_string());
            DownloadRequestState::Queued
        }
    }

    fn cancel_download(&self, model_id: &str) -> bool {
        let mut inner = self.inner.lock().expect("model manager lock");
        if inner.active.as_deref() == Some(model_id) {
            inner.cancelled.insert(model_id.to_string());
            return true;
        }
        let before = inner.queued.len();
        inner.queued.retain(|queued| queued != model_id);
        before != inner.queued.len()
    }

    fn is_cancelled(&self, model_id: &str) -> bool {
        self.inner
            .lock()
            .expect("model manager lock")
            .cancelled
            .contains(model_id)
    }

    fn finish_download(&self, model_id: &str) -> Option<String> {
        let mut inner = self.inner.lock().expect("model manager lock");
        inner.cancelled.remove(model_id);
        if inner.active.as_deref() == Some(model_id) {
            inner.active = None;
        }
        let next = inner.queued.pop_front();
        inner.active = next.clone();
        next
    }

    fn is_active_or_queued(&self, model_id: &str) -> bool {
        let inner = self.inner.lock().expect("model manager lock");
        inner.active.as_deref() == Some(model_id)
            || inner.queued.iter().any(|queued| queued == model_id)
    }
}

#[tauri::command(rename = "listAiModels")]
pub fn list_ai_models(storage: State<'_, AppStorage>) -> Result<AiModelAssetList, AiError> {
    model_list(&storage)
}

#[tauri::command(rename = "downloadAiModel")]
pub fn download_ai_model(
    app: AppHandle,
    input: DownloadAiModelInput,
    manager: State<'_, AiModelManager>,
    storage: State<'_, AppStorage>,
) -> Result<Notification, AiError> {
    let catalog = ensure_catalog_model(&input.model_id)?;
    let existing_model_path = model_path(&app, catalog)?;
    if existing_model_path.is_file() {
        let notification = storage
            .with_connection(|connection| {
                let timestamp_ms = now_ms();
                save_catalog_model(
                    connection,
                    catalog,
                    ModelAssetPatch {
                        download_status: "available".into(),
                        downloaded_bytes: catalog.size_bytes,
                        error_code: None,
                        local_path: Some(existing_model_path.to_string_lossy().into_owned().into()),
                        total_bytes: Some(catalog.size_bytes),
                        verification_status: "passed".into(),
                        ..ModelAssetPatch::from_existing(connection, catalog)?
                    },
                    timestamp_ms,
                )?;
                save_model_download_notification(
                    connection,
                    catalog.id,
                    input.notification_id.as_deref(),
                    "succeeded",
                    "Model already downloaded",
                    &format!("{} is already available locally.", catalog.label),
                    &[],
                    timestamp_ms,
                )
            })
            .map_err(storage_error)?;
        emit_model_list(&app);

        return notification_from_row(notification);
    }

    let state = manager.request_download(catalog.id);
    let notification = storage
        .with_connection(|connection| {
            let timestamp_ms = now_ms();
            let status = if state == DownloadRequestState::Queued {
                "queued"
            } else {
                "downloading"
            };
            let patch = if state == DownloadRequestState::AlreadyRunning {
                ModelAssetPatch {
                    download_status: "downloading".into(),
                    ..ModelAssetPatch::from_existing(connection, catalog)?
                }
            } else {
                ModelAssetPatch {
                    download_status: status.into(),
                    downloaded_bytes: 0,
                    error_code: None,
                    etag: None,
                    local_path: None,
                    self_test_status: "not-run".into(),
                    total_bytes: Some(catalog.size_bytes),
                    verification_status: "pending".into(),
                }
            };
            save_catalog_model(connection, catalog, patch, timestamp_ms)?;
            save_model_download_notification(
                connection,
                catalog.id,
                input.notification_id.as_deref(),
                status_to_notification_status(status),
                if state == DownloadRequestState::Queued {
                    "Model download queued"
                } else if state == DownloadRequestState::AlreadyRunning {
                    "Model download already running"
                } else {
                    "Model download started"
                },
                &format!("{} will download from Hugging Face.", catalog.label),
                &[model_download_cancel_action(catalog.id)],
                timestamp_ms,
            )
        })
        .map_err(storage_error)?;
    emit_model_list(&app);

    if state == DownloadRequestState::Started {
        spawn_model_download(app, catalog.id.to_string());
    }

    notification_from_row(notification)
}

#[tauri::command(rename = "cancelAiModelDownload")]
pub fn cancel_ai_model_download(
    app: AppHandle,
    input: CancelAiModelDownloadInput,
    manager: State<'_, AiModelManager>,
    storage: State<'_, AppStorage>,
) -> Result<Notification, AiError> {
    let catalog = ensure_catalog_model(&input.model_id)?;
    let cancelled = manager.cancel_download(catalog.id);
    let notification = storage
        .with_connection(|connection| {
            let timestamp_ms = now_ms();
            save_cancel_download_result(
                connection,
                catalog,
                input.notification_id.as_deref(),
                cancelled,
                timestamp_ms,
            )
        })
        .map_err(storage_error)?;
    if cancelled {
        emit_model_list(&app);
    }

    notification_from_row(notification)
}

#[tauri::command(rename = "deleteAiModel")]
pub fn delete_ai_model(
    app: AppHandle,
    input: DeleteAiModelInput,
    manager: State<'_, AiModelManager>,
    storage: State<'_, AppStorage>,
) -> Result<AiModelAssetList, AiError> {
    let catalog = ensure_catalog_model(&input.model_id)?;
    if manager.is_active_or_queued(catalog.id) {
        return Err(AiError {
            code: "ai_model_download_busy".to_string(),
            message: "Cancel the model download before deleting it.".to_string(),
        });
    }

    let path = model_path(&app, catalog)?;
    let partial_path = partial_model_path(&app, catalog)?;
    delete_file_if_exists(&path)?;
    delete_file_if_exists(&partial_path)?;

    storage
        .with_connection(|connection| {
            save_catalog_model(connection, catalog, ModelAssetPatch::missing(), now_ms())?;
            storage::list_ai_model_assets(connection)
        })
        .map_err(storage_error)
        .map(models_from_stored)
        .inspect(|_models| emit_model_list(&app))
}

#[tauri::command(rename = "selfTestAiModel")]
pub async fn self_test_ai_model(
    app: AppHandle,
    input: SelfTestAiModelInput,
    storage: State<'_, AppStorage>,
) -> Result<Notification, AiError> {
    let catalog = ensure_catalog_model(&input.model_id)?;
    let path = model_path(&app, catalog)?;
    if !path.is_file() {
        return Err(AiError {
            code: "ai_model_missing".to_string(),
            message: "Download the model before running its load test.".to_string(),
        });
    }

    storage
        .with_connection(|connection| {
            save_catalog_model(
                connection,
                catalog,
                ModelAssetPatch {
                    self_test_status: "running".into(),
                    ..ModelAssetPatch::from_existing(connection, catalog)?
                },
                now_ms(),
            )
            .map(|_row| ())
        })
        .map_err(storage_error)?;
    emit_model_list(&app);

    let path_for_load = path.clone();
    let load_result = tauri::async_runtime::spawn_blocking(move || {
        whisper_backend::load_and_unload_model(&path_for_load)
    })
    .await
    .map_err(|error| AiError {
        code: "ai_model_self_test_failed".to_string(),
        message: format!("Model load test task failed: {error}"),
    })?;

    let notification = storage
        .with_connection(|connection| {
            let timestamp_ms = now_ms();
            match load_result {
                Ok(()) => {
                    save_catalog_model(
                        connection,
                        catalog,
                        ModelAssetPatch {
                            self_test_status: "passed".into(),
                            ..ModelAssetPatch::from_existing(connection, catalog)?
                        },
                        timestamp_ms,
                    )?;
                    save_model_notification(
                        connection,
                        "succeeded",
                        "model-self-test",
                        "Model load test passed",
                        &format!("{} loaded successfully on CPU.", catalog.label),
                        &[],
                        timestamp_ms,
                    )
                }
                Err(error) => {
                    save_catalog_model(
                        connection,
                        catalog,
                        ModelAssetPatch {
                            error_code: Some(error.code.as_str().into()),
                            self_test_status: "failed".into(),
                            ..ModelAssetPatch::from_existing(connection, catalog)?
                        },
                        timestamp_ms,
                    )?;
                    save_model_notification(
                        connection,
                        "failed",
                        "model-self-test",
                        "Model load test failed",
                        &format!("{} could not be loaded: {}.", catalog.label, error.code),
                        &[],
                        timestamp_ms,
                    )
                }
            }
        })
        .map_err(storage_error)?;
    emit_model_list(&app);

    notification_from_row(notification)
}

#[tauri::command(rename = "getAiProviderStatus")]
pub fn get_ai_provider_status(storage: State<'_, AppStorage>) -> Result<AiProviderStatus, AiError> {
    let app_settings = settings::load_app_settings(&storage).map_err(|error| AiError {
        code: error.code,
        message: error.message,
    })?;
    let models = model_list(&storage)?;
    let selected = models
        .models
        .iter()
        .find(|model| model.id == app_settings.ai.transcription.local_model_id)
        .or_else(|| models.models.iter().find(|model| model.id == "small"));
    let backend_status = whisper_backend::backend_status();

    Ok(AiProviderStatus {
        local_transcription: AiLocalTranscriptionStatus {
            available_backends: vec![
                AiBackendStatus {
                    available: true,
                    id: "cpu".to_string(),
                    reason: None,
                },
                AiBackendStatus {
                    available: backend_status.vulkan_compiled,
                    id: "vulkan".to_string(),
                    reason: (!backend_status.vulkan_compiled)
                        .then(|| "Vulkan backend is not compiled in this build.".to_string()),
                },
                AiBackendStatus {
                    available: backend_status.cuda_compiled,
                    id: "cuda".to_string(),
                    reason: (!backend_status.cuda_compiled)
                        .then(|| "CUDA backend is not compiled in this build.".to_string()),
                },
            ],
            checksum_status: selected
                .map(|model| model.verification_status.clone())
                .unwrap_or_else(|| "unknown".to_string()),
            local_model_id: app_settings.ai.transcription.local_model_id.clone(),
            model_download_status: selected
                .map(|model| model.download_status.clone())
                .unwrap_or_else(|| "missing".to_string()),
            model_path: selected.and_then(|model| model.local_path.clone()),
            selected_backend: app_settings.ai.transcription.local_backend.clone(),
            self_test_status: selected
                .map(|model| model.self_test_status.clone())
                .unwrap_or_else(|| "not-run".to_string()),
        },
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

fn spawn_model_download(app: AppHandle, model_id: String) {
    tauri::async_runtime::spawn(async move {
        let result = run_model_download(&app, &model_id).await;
        if let Err(error) = result {
            let storage = app.state::<AppStorage>();
            if let Some(catalog) = catalog_model(&model_id) {
                let _ = storage.with_connection(|connection| {
                    let timestamp_ms = now_ms();
                    save_catalog_model(
                        connection,
                        catalog,
                        ModelAssetPatch {
                            download_status: "failed".into(),
                            error_code: Some(error.code.as_str().into()),
                            verification_status: "failed".into(),
                            ..ModelAssetPatch::from_existing(connection, catalog)?
                        },
                        timestamp_ms,
                    )?;
                    save_model_download_notification(
                        connection,
                        catalog.id,
                        None,
                        "failed",
                        "Model download failed",
                        &format!("{} download failed: {}.", catalog.label, error.code),
                        &[model_download_retry_action(catalog.id)],
                        timestamp_ms,
                    )
                    .map(|_row| ())
                });
                emit_model_list(&app);
            }
        }

        let manager = app.state::<AiModelManager>();
        if let Some(next_model_id) = manager.finish_download(&model_id) {
            spawn_model_download(app.clone(), next_model_id);
        }
    });
}

async fn run_model_download(app: &AppHandle, model_id: &str) -> Result<(), AiError> {
    let catalog = ensure_catalog_model(model_id)?;
    let storage = app.state::<AppStorage>();
    let manager = app.state::<AiModelManager>();
    let final_path = model_path(app, catalog)?;
    let partial_path = partial_model_path(app, catalog)?;
    if let Some(parent) = final_path.parent() {
        fs::create_dir_all(parent).map_err(file_error)?;
    }

    let existing = storage
        .with_connection(|connection| storage::get_ai_model_asset(connection, catalog.id))
        .map_err(storage_error)?;
    let mut partial_len = match prepare_partial_download(catalog, &partial_path)? {
        PartialDownloadResume::Complete => {
            finalize_model_download(app, catalog, &partial_path, &final_path, &manager).await?;
            return Ok(());
        }
        PartialDownloadResume::Resume(length) => length,
    };
    let client = reqwest::Client::new();
    let mut retried_without_range = false;

    loop {
        let mut request = client.get(model_source_url(catalog));
        if partial_len > 0 {
            request = request.header(RANGE, format!("bytes={partial_len}-"));
            if let Some(etag) = existing.as_ref().and_then(|row| row.etag.clone()) {
                request = request.header(IF_RANGE, etag);
            }
        }
        let response = request.send().await.map_err(download_error)?;
        let status = response.status();
        if status == StatusCode::RANGE_NOT_SATISFIABLE && partial_len > 0 {
            if finalize_complete_partial_if_valid(
                app,
                catalog,
                &partial_path,
                &final_path,
                &manager,
            )
            .await?
            {
                return Ok(());
            }
            if retried_without_range {
                return Err(http_error(status));
            }
            partial_len = 0;
            retried_without_range = true;
            continue;
        }
        if !status.is_success() {
            return Err(http_error(status));
        }
        let append = status == StatusCode::PARTIAL_CONTENT && partial_len > 0;
        if !append {
            partial_len = 0;
        }
        let etag = response
            .headers()
            .get(ETAG)
            .and_then(|value| value.to_str().ok())
            .map(ToOwned::to_owned);
        let response_len = response
            .headers()
            .get(CONTENT_LENGTH)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<i64>().ok());
        let total_bytes = response_len.map(|length| length + partial_len as i64);

        storage
            .with_connection(|connection| {
                save_catalog_model(
                    connection,
                    catalog,
                    ModelAssetPatch {
                        download_status: "downloading".into(),
                        downloaded_bytes: partial_len as i64,
                        etag: etag.as_deref().map(Into::into),
                        total_bytes: total_bytes.or(Some(catalog.size_bytes)),
                        verification_status: "pending".into(),
                        ..ModelAssetPatch::from_existing(connection, catalog)?
                    },
                    now_ms(),
                )
                .map(|_row| ())
            })
            .map_err(storage_error)?;
        emit_model_list(app);

        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .append(append)
            .truncate(!append)
            .open(&partial_path)
            .map_err(file_error)?;
        let mut downloaded = partial_len as i64;
        let mut last_progress_update = Instant::now();
        let mut last_progress_bytes = downloaded;
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            if manager.is_cancelled(catalog.id) {
                save_cancelled_download(app, catalog, downloaded)?;
                return Ok(());
            }
            let chunk = chunk.map_err(download_error)?;
            file.write_all(&chunk).map_err(file_error)?;
            downloaded += chunk.len() as i64;
            if downloaded - last_progress_bytes >= MODEL_PROGRESS_UPDATE_BYTES
                || last_progress_update.elapsed() >= MODEL_PROGRESS_UPDATE_INTERVAL
            {
                save_download_progress(app, catalog, downloaded, total_bytes)?;
                last_progress_update = Instant::now();
                last_progress_bytes = downloaded;
            }
        }
        if downloaded != last_progress_bytes {
            save_download_progress(app, catalog, downloaded, total_bytes)?;
        }
        file.flush().map_err(file_error)?;
        drop(file);
        break;
    }

    finalize_model_download(app, catalog, &partial_path, &final_path, &manager).await
}

#[derive(Debug, PartialEq, Eq)]
enum PartialDownloadResume {
    Complete,
    Resume(u64),
}

fn prepare_partial_download(
    catalog: &CatalogModel,
    partial_path: &Path,
) -> Result<PartialDownloadResume, AiError> {
    let partial_len = fs::metadata(partial_path)
        .map(|metadata| metadata.len())
        .unwrap_or(0);
    if partial_len == 0 {
        return Ok(PartialDownloadResume::Resume(0));
    }

    if partial_len >= catalog.size_bytes as u64 {
        if sha1_file(partial_path)? == catalog.sha1 {
            return Ok(PartialDownloadResume::Complete);
        }
        delete_file_if_exists(partial_path)?;
        return Ok(PartialDownloadResume::Resume(0));
    }

    Ok(PartialDownloadResume::Resume(partial_len))
}

async fn finalize_complete_partial_if_valid(
    app: &AppHandle,
    catalog: &CatalogModel,
    partial_path: &Path,
    final_path: &Path,
    manager: &AiModelManager,
) -> Result<bool, AiError> {
    if !partial_path.is_file() {
        return Ok(false);
    }
    if sha1_file(partial_path)? != catalog.sha1 {
        delete_file_if_exists(partial_path)?;
        return Ok(false);
    }
    finalize_model_download(app, catalog, partial_path, final_path, manager).await?;
    Ok(true)
}

async fn finalize_model_download(
    app: &AppHandle,
    catalog: &CatalogModel,
    partial_path: &Path,
    final_path: &Path,
    manager: &AiModelManager,
) -> Result<(), AiError> {
    let storage = app.state::<AppStorage>();
    if manager.is_cancelled(catalog.id) {
        save_cancelled_download(app, catalog, downloaded_bytes(partial_path, final_path))?;
        return Ok(());
    }
    let actual_sha1 = sha1_file(partial_path)?;
    if actual_sha1 != catalog.sha1 {
        return Err(AiError {
            code: "ai_model_checksum_failed".to_string(),
            message: format!("Model checksum verification failed for {}.", catalog.label),
        });
    }
    if manager.is_cancelled(catalog.id) {
        save_cancelled_download(app, catalog, downloaded_bytes(partial_path, final_path))?;
        return Ok(());
    }
    fs::rename(partial_path, final_path).map_err(file_error)?;
    if manager.is_cancelled(catalog.id) {
        save_cancelled_finalized_download(app, catalog, partial_path, final_path)?;
        return Ok(());
    }
    let local_path = final_path.to_string_lossy().to_string();
    let load_path = final_path.to_path_buf();
    let self_test_error = match tauri::async_runtime::spawn_blocking(move || {
        whisper_backend::load_and_unload_model(&load_path)
    })
    .await
    {
        Ok(Ok(())) => None,
        Ok(Err(error)) => Some(error.code),
        Err(_error) => Some("ai_model_self_test_failed".to_string()),
    };
    if manager.is_cancelled(catalog.id) {
        save_cancelled_finalized_download(app, catalog, partial_path, final_path)?;
        return Ok(());
    }
    storage
        .with_connection(|connection| {
            let timestamp_ms = now_ms();
            let self_test_status = if self_test_error.is_some() {
                "failed"
            } else {
                "passed"
            };
            let notification_status = if self_test_error.is_some() {
                "warning"
            } else {
                "succeeded"
            };
            let notification_title = if self_test_error.is_some() {
                "Model download complete; load test failed"
            } else {
                "Model download complete"
            };
            let notification_message = if let Some(error_code) = self_test_error.as_deref() {
                format!(
                    "{} downloaded and verified, but load test failed: {}.",
                    catalog.label, error_code
                )
            } else {
                format!("{} downloaded, verified, and load-tested.", catalog.label)
            };
            save_catalog_model(
                connection,
                catalog,
                ModelAssetPatch {
                    download_status: "available".into(),
                    downloaded_bytes: catalog.size_bytes,
                    error_code: self_test_error.as_deref().map(Into::into),
                    local_path: Some(local_path.as_str().into()),
                    self_test_status: self_test_status.into(),
                    total_bytes: Some(catalog.size_bytes),
                    verification_status: "passed".into(),
                    ..ModelAssetPatch::from_existing(connection, catalog)?
                },
                timestamp_ms,
            )?;
            save_model_download_notification(
                connection,
                catalog.id,
                None,
                notification_status,
                notification_title,
                &notification_message,
                &[],
                timestamp_ms,
            )
            .map(|_row| ())
        })
        .map_err(storage_error)?;
    emit_model_list(app);

    Ok(())
}

fn save_cancelled_finalized_download(
    app: &AppHandle,
    catalog: &CatalogModel,
    partial_path: &Path,
    final_path: &Path,
) -> Result<(), AiError> {
    rollback_finalized_model_file(partial_path, final_path)?;
    save_cancelled_download(app, catalog, downloaded_bytes(partial_path, final_path))
}

fn rollback_finalized_model_file(partial_path: &Path, final_path: &Path) -> Result<(), AiError> {
    if !final_path.exists() {
        return Ok(());
    }
    if partial_path.exists() {
        delete_file_if_exists(final_path)?;
        return Ok(());
    }
    fs::rename(final_path, partial_path).map_err(file_error)
}

fn downloaded_bytes(partial_path: &Path, final_path: &Path) -> i64 {
    fs::metadata(partial_path)
        .or_else(|_error| fs::metadata(final_path))
        .map(|metadata| metadata.len() as i64)
        .unwrap_or(0)
}

fn save_download_progress(
    app: &AppHandle,
    catalog: &CatalogModel,
    downloaded: i64,
    total_bytes: Option<i64>,
) -> Result<(), AiError> {
    let storage = app.state::<AppStorage>();
    storage
        .with_connection(|connection| {
            save_catalog_model(
                connection,
                catalog,
                ModelAssetPatch {
                    download_status: "downloading".into(),
                    downloaded_bytes: downloaded,
                    total_bytes: total_bytes.or(Some(catalog.size_bytes)),
                    verification_status: "pending".into(),
                    ..ModelAssetPatch::from_existing(connection, catalog)?
                },
                now_ms(),
            )
            .map(|_row| ())
        })
        .map_err(storage_error)?;
    emit_model_list(app);

    Ok(())
}

fn save_cancelled_download(
    app: &AppHandle,
    catalog: &CatalogModel,
    downloaded: i64,
) -> Result<(), AiError> {
    let storage = app.state::<AppStorage>();
    storage
        .with_connection(|connection| {
            let timestamp_ms = now_ms();
            save_catalog_model(
                connection,
                catalog,
                ModelAssetPatch {
                    download_status: "cancelled".into(),
                    downloaded_bytes: downloaded,
                    error_code: Some("ai_model_download_cancelled".into()),
                    verification_status: "unknown".into(),
                    ..ModelAssetPatch::from_existing(connection, catalog)?
                },
                timestamp_ms,
            )?;
            save_model_download_notification(
                connection,
                catalog.id,
                None,
                "cancelled",
                "Model download cancelled",
                &format!("{} download was cancelled.", catalog.label),
                &[model_download_retry_action(catalog.id)],
                timestamp_ms,
            )
            .map(|_row| ())
        })
        .map_err(storage_error)?;
    emit_model_list(app);

    Ok(())
}

fn save_cancel_download_result(
    connection: &rusqlite::Connection,
    catalog: &CatalogModel,
    preferred_notification_id: Option<&str>,
    cancelled: bool,
    timestamp_ms: i64,
) -> rusqlite::Result<NotificationRow> {
    if cancelled {
        save_catalog_model(
            connection,
            catalog,
            ModelAssetPatch {
                download_status: "cancelled".into(),
                error_code: Some("ai_model_download_cancelled".into()),
                verification_status: "unknown".into(),
                ..ModelAssetPatch::from_existing(connection, catalog)?
            },
            timestamp_ms,
        )?;
        save_model_download_notification(
            connection,
            catalog.id,
            preferred_notification_id,
            "cancelled",
            "Model download cancelled",
            &format!("{} download was cancelled.", catalog.label),
            &[model_download_retry_action(catalog.id)],
            timestamp_ms,
        )
    } else {
        save_model_download_notification(
            connection,
            catalog.id,
            preferred_notification_id,
            "warning",
            "Model download not running",
            &format!(
                "{} has no active or queued download to cancel.",
                catalog.label
            ),
            &[model_download_retry_action(catalog.id)],
            timestamp_ms,
        )
    }
}

#[derive(Clone)]
struct ModelAssetPatch<'a> {
    download_status: Cow<'a, str>,
    downloaded_bytes: i64,
    error_code: Option<Cow<'a, str>>,
    etag: Option<Cow<'a, str>>,
    local_path: Option<Cow<'a, str>>,
    self_test_status: Cow<'a, str>,
    total_bytes: Option<i64>,
    verification_status: Cow<'a, str>,
}

impl<'a> ModelAssetPatch<'a> {
    fn missing() -> Self {
        Self {
            download_status: "missing".into(),
            downloaded_bytes: 0,
            error_code: None,
            etag: None,
            local_path: None,
            self_test_status: "not-run".into(),
            total_bytes: None,
            verification_status: "unknown".into(),
        }
    }

    fn from_existing(
        connection: &rusqlite::Connection,
        catalog: &CatalogModel,
    ) -> rusqlite::Result<Self> {
        Ok(storage::get_ai_model_asset(connection, catalog.id)?
            .as_ref()
            .map_or_else(Self::missing, |row| Self {
                download_status: Cow::Owned(row.download_status.clone()),
                downloaded_bytes: row.downloaded_bytes,
                error_code: row.error_code.clone().map(Cow::Owned),
                etag: row.etag.clone().map(Cow::Owned),
                local_path: row.local_path.clone().map(Cow::Owned),
                self_test_status: Cow::Owned(row.self_test_status.clone()),
                total_bytes: row.total_bytes,
                verification_status: Cow::Owned(row.verification_status.clone()),
            }))
    }
}

fn save_catalog_model(
    connection: &rusqlite::Connection,
    catalog: &CatalogModel,
    patch: ModelAssetPatch<'_>,
    timestamp_ms: i64,
) -> rusqlite::Result<AiModelAssetRow> {
    storage::save_ai_model_asset(
        connection,
        &SaveAiModelAssetInput {
            checksum: Some(catalog.sha1.to_string()),
            checksum_algorithm: Some("sha1".to_string()),
            display_label: catalog.label.to_string(),
            downloaded_bytes: patch.downloaded_bytes,
            download_status: patch.download_status.into_owned(),
            etag: patch.etag.map(Cow::into_owned),
            error_code: patch.error_code.map(Cow::into_owned),
            id: catalog.id.to_string(),
            local_path: patch.local_path.map(Cow::into_owned),
            provider: "local-whisper-cpp".to_string(),
            self_test_status: patch.self_test_status.into_owned(),
            size_bytes: Some(catalog.size_bytes),
            source_url: Some(model_source_url(catalog)),
            timestamp_ms,
            total_bytes: patch.total_bytes,
            verification_status: patch.verification_status.into_owned(),
        },
    )
}

fn model_list(storage: &AppStorage) -> Result<AiModelAssetList, AiError> {
    storage
        .with_connection(storage::list_ai_model_assets)
        .map_err(storage_error)
        .map(models_from_stored)
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
        checksum: Some(catalog.sha1.to_string()),
        checksum_algorithm: Some("sha1".to_string()),
        created_at_ms: 0,
        downloaded_bytes: 0,
        download_status: "missing".to_string(),
        etag: None,
        error_code: None,
        id: catalog.id.to_string(),
        label: catalog.label.to_string(),
        local_path: None,
        provider: "local-whisper-cpp".to_string(),
        self_test_status: "not-run".to_string(),
        size_bytes: Some(catalog.size_bytes),
        source_url: Some(model_source_url(catalog)),
        total_bytes: Some(catalog.size_bytes),
        updated_at_ms: 0,
        verification_status: "unknown".to_string(),
    }
}

fn model_asset_from_row(row: AiModelAssetRow) -> AiModelAsset {
    AiModelAsset {
        checksum: row.checksum,
        checksum_algorithm: row.checksum_algorithm,
        created_at_ms: row.created_at_ms,
        downloaded_bytes: row.downloaded_bytes,
        download_status: row.download_status,
        etag: row.etag,
        error_code: row.error_code,
        id: row.id,
        label: row.display_label,
        local_path: row.local_path,
        provider: row.provider,
        self_test_status: row.self_test_status,
        size_bytes: row.size_bytes,
        source_url: row.source_url,
        total_bytes: row.total_bytes,
        updated_at_ms: row.updated_at_ms,
        verification_status: row.verification_status,
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

fn save_model_notification(
    connection: &rusqlite::Connection,
    status: &str,
    topic: &str,
    title: &str,
    message: &str,
    actions: &[NotificationAction],
    timestamp_ms: i64,
) -> rusqlite::Result<NotificationRow> {
    let actions_json =
        serde_json::to_string(actions).map_err(|_error| rusqlite::Error::InvalidQuery)?;
    storage::save_notification(
        connection,
        &SaveNotificationInput {
            actions_json,
            id: Uuid::now_v7().to_string(),
            message: message.to_string(),
            source: "ai".to_string(),
            status: status.to_string(),
            timestamp_ms,
            title: title.to_string(),
            topic: topic.to_string(),
        },
    )
}

fn save_model_download_notification(
    connection: &rusqlite::Connection,
    model_id: &str,
    preferred_notification_id: Option<&str>,
    status: &str,
    title: &str,
    message: &str,
    actions: &[NotificationAction],
    timestamp_ms: i64,
) -> rusqlite::Result<NotificationRow> {
    let actions_json =
        serde_json::to_string(actions).map_err(|_error| rusqlite::Error::InvalidQuery)?;
    let notification_id =
        find_model_download_notification_id(connection, model_id, preferred_notification_id)?
            .unwrap_or_else(|| Uuid::now_v7().to_string());

    storage::save_unseen_notification(
        connection,
        &SaveNotificationInput {
            actions_json,
            id: notification_id,
            message: message.to_string(),
            source: "ai".to_string(),
            status: status.to_string(),
            timestamp_ms,
            title: title.to_string(),
            topic: "model-download".to_string(),
        },
    )
}

fn find_model_download_notification_id(
    connection: &rusqlite::Connection,
    model_id: &str,
    preferred_notification_id: Option<&str>,
) -> rusqlite::Result<Option<String>> {
    if let Some(notification_id) = preferred_notification_id {
        if let Some(row) = storage::get_notification(connection, notification_id)? {
            if row.source == "ai" && row.topic == "model-download" {
                return Ok(Some(row.id));
            }
        }
    }

    let mut fallback = None;
    for row in storage::list_notifications(connection)? {
        if row.source != "ai" || row.topic != "model-download" {
            continue;
        }

        let actions = serde_json::from_str::<Vec<NotificationAction>>(&row.actions_json)
            .map_err(|_error| rusqlite::Error::InvalidQuery)?;
        let has_model = actions.iter().any(|action| {
            action
                .payload
                .get("modelId")
                .is_some_and(|candidate| candidate == model_id)
        });

        if !has_model {
            continue;
        }

        if row.status == "running" {
            return Ok(Some(row.id));
        }

        fallback.get_or_insert(row.id);
    }

    Ok(fallback)
}

fn notification_from_row(row: NotificationRow) -> Result<Notification, AiError> {
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

fn model_download_cancel_action(model_id: &str) -> NotificationAction {
    NotificationAction {
        handler: "model-download-cancel".to_string(),
        id: "cancel".to_string(),
        label: "Cancel".to_string(),
        payload: BTreeMap::from([("modelId".to_string(), model_id.to_string())]),
        style: "destructive".to_string(),
    }
}

fn status_to_notification_status(status: &str) -> &'static str {
    match status {
        "queued" => "info",
        "downloading" => "running",
        "cancelled" => "cancelled",
        "failed" => "failed",
        "available" => "succeeded",
        _ => "info",
    }
}

fn ensure_catalog_model(model_id: &str) -> Result<&'static CatalogModel, AiError> {
    catalog_model(model_id).ok_or_else(|| AiError {
        code: "ai_model_unknown".to_string(),
        message: format!("AI model is unknown: {model_id}"),
    })
}

fn catalog_model(model_id: &str) -> Option<&'static CatalogModel> {
    LOCAL_WHISPER_CATALOG
        .iter()
        .find(|model| model.id == model_id)
}

fn model_source_url(catalog: &CatalogModel) -> String {
    format!(
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-{}.bin",
        catalog.id
    )
}

fn model_cache_dir(app: &AppHandle) -> Result<PathBuf, AiError> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|error| AiError {
            code: "ai_model_cache_unavailable".to_string(),
            message: format!("Model cache directory is unavailable: {error}"),
        })?
        .join("models")
        .join("whisper.cpp"))
}

fn model_path(app: &AppHandle, catalog: &CatalogModel) -> Result<PathBuf, AiError> {
    Ok(model_cache_dir(app)?.join(format!("ggml-{}.bin", catalog.id)))
}

fn partial_model_path(app: &AppHandle, catalog: &CatalogModel) -> Result<PathBuf, AiError> {
    Ok(model_cache_dir(app)?.join(format!("ggml-{}.bin.part", catalog.id)))
}

fn delete_file_if_exists(path: &Path) -> Result<(), AiError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(file_error(error)),
    }
}

fn sha1_file(path: &Path) -> Result<String, AiError> {
    let mut file = File::open(path).map_err(file_error)?;
    let mut hasher = Sha1::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(file_error)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    Ok(hex::encode(hasher.finalize()))
}

fn emit_model_list(app: &AppHandle) {
    let storage = app.state::<AppStorage>();
    if let Ok(models) = model_list(&storage) {
        let _ = app.emit(AI_MODELS_CHANGED_EVENT, models);
    }
}

fn storage_error(error: rusqlite::Error) -> AiError {
    AiError {
        code: "ai_storage_failed".to_string(),
        message: format!("AI storage failed: {error}"),
    }
}

fn file_error(error: std::io::Error) -> AiError {
    AiError {
        code: "ai_file_failed".to_string(),
        message: format!("AI file operation failed: {error}"),
    }
}

fn download_error(error: reqwest::Error) -> AiError {
    AiError {
        code: "ai_model_download_failed".to_string(),
        message: if let Some(status) = error.status() {
            format!("Model download failed with HTTP status {status}.")
        } else {
            "Model download failed before a response completed.".to_string()
        },
    }
}

fn http_error(status: StatusCode) -> AiError {
    AiError {
        code: "ai_model_download_http_failed".to_string(),
        message: format!("Model download failed with HTTP status {status}."),
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
    use std::path::PathBuf;

    #[test]
    fn catalog_models_default_to_missing_with_pinned_sha1() {
        let model = catalog_asset(&LOCAL_WHISPER_CATALOG[2]);

        assert_eq!(model.id, "small");
        assert_eq!(model.checksum_algorithm.as_deref(), Some("sha1"));
        assert_eq!(
            model.checksum.as_deref(),
            Some("55356645c2b361a969dfd0ef2c5a50d530afd8d5")
        );
        assert_eq!(model.download_status, "missing");
        assert_eq!(model.self_test_status, "not-run");
    }

    #[test]
    fn manager_queues_one_download_while_another_is_active() {
        let manager = AiModelManager::new();

        assert_eq!(
            manager.request_download("tiny"),
            DownloadRequestState::Started
        );
        assert_eq!(
            manager.request_download("small"),
            DownloadRequestState::Queued
        );
        assert_eq!(manager.finish_download("tiny"), Some("small".to_string()));
    }

    #[test]
    fn manager_clears_cancelled_state_when_retrying_active_download() {
        let manager = AiModelManager::new();

        assert_eq!(
            manager.request_download("tiny"),
            DownloadRequestState::Started
        );
        assert!(manager.cancel_download("tiny"));
        assert!(manager.is_cancelled("tiny"));

        assert_eq!(
            manager.request_download("tiny"),
            DownloadRequestState::AlreadyRunning
        );
        assert!(!manager.is_cancelled("tiny"));
    }

    #[test]
    fn stale_cancel_does_not_overwrite_available_model() {
        let storage = storage::test_storage(PathBuf::new());
        let catalog = &LOCAL_WHISPER_CATALOG[2];

        storage
            .with_connection(|connection| {
                save_catalog_model(
                    connection,
                    catalog,
                    ModelAssetPatch {
                        download_status: "available".into(),
                        downloaded_bytes: catalog.size_bytes,
                        local_path: Some("/tmp/ggml-small.bin".into()),
                        self_test_status: "passed".into(),
                        total_bytes: Some(catalog.size_bytes),
                        verification_status: "passed".into(),
                        ..ModelAssetPatch::missing()
                    },
                    10,
                )?;

                let notification =
                    save_cancel_download_result(connection, catalog, None, false, 20)?;

                assert_eq!(notification.status, "warning");
                assert_eq!(notification.title, "Model download not running");

                let model = storage::get_ai_model_asset(connection, catalog.id)?
                    .expect("model asset remains saved");
                assert_eq!(model.download_status, "available");
                assert_eq!(model.local_path.as_deref(), Some("/tmp/ggml-small.bin"));
                assert_eq!(model.verification_status, "passed");

                Ok(())
            })
            .expect("save stale cancel");
    }

    #[test]
    fn complete_partial_file_can_be_recovered_before_ranged_download() {
        let path = test_temp_path("complete-partial").join("model.part");
        fs::create_dir_all(path.parent().expect("partial parent")).expect("create temp dir");
        let bytes = b"complete model bytes";
        fs::write(&path, bytes).expect("write partial");
        let sha1 = Box::leak(test_sha1(bytes).into_boxed_str());
        let catalog = CatalogModel {
            id: "test",
            label: "Test model",
            sha1,
            size_bytes: bytes.len() as i64,
        };

        assert_eq!(
            prepare_partial_download(&catalog, &path).expect("prepare partial"),
            PartialDownloadResume::Complete
        );

        delete_file_if_exists(&path).expect("delete partial");
    }

    #[test]
    fn corrupt_complete_partial_file_restarts_from_zero() {
        let path = test_temp_path("corrupt-partial").join("model.part");
        fs::create_dir_all(path.parent().expect("partial parent")).expect("create temp dir");
        fs::write(&path, b"corrupt").expect("write partial");
        let sha1 = Box::leak(test_sha1(b"expected").into_boxed_str());
        let catalog = CatalogModel {
            id: "test",
            label: "Test model",
            sha1,
            size_bytes: 1,
        };

        assert_eq!(
            prepare_partial_download(&catalog, &path).expect("prepare partial"),
            PartialDownloadResume::Resume(0)
        );
        assert!(!path.exists());
    }

    #[test]
    fn cancelled_finalized_download_moves_final_file_back_to_partial() {
        let dir = test_temp_path("rollback-finalized");
        fs::create_dir_all(&dir).expect("create temp dir");
        let partial_path = dir.join("model.part");
        let final_path = dir.join("model.bin");
        fs::write(&final_path, b"complete model bytes").expect("write final");

        rollback_finalized_model_file(&partial_path, &final_path).expect("rollback final file");

        assert!(!final_path.exists());
        assert_eq!(
            fs::read(&partial_path).expect("read restored partial"),
            b"complete model bytes"
        );
        delete_file_if_exists(&partial_path).expect("delete partial");
    }

    #[test]
    fn cancelled_finalized_download_deletes_final_file_when_partial_exists() {
        let dir = test_temp_path("rollback-finalized-with-partial");
        fs::create_dir_all(&dir).expect("create temp dir");
        let partial_path = dir.join("model.part");
        let final_path = dir.join("model.bin");
        fs::write(&partial_path, b"existing partial").expect("write partial");
        fs::write(&final_path, b"complete model bytes").expect("write final");

        rollback_finalized_model_file(&partial_path, &final_path).expect("rollback final file");

        assert!(!final_path.exists());
        assert_eq!(
            fs::read(&partial_path).expect("read existing partial"),
            b"existing partial"
        );
        delete_file_if_exists(&partial_path).expect("delete partial");
    }

    #[test]
    fn retry_and_cancel_actions_keep_model_id_in_payload() {
        let retry = model_download_retry_action("small");
        let cancel = model_download_cancel_action("small");

        assert_eq!(retry.handler, "model-download-retry");
        assert_eq!(retry.payload.get("modelId"), Some(&"small".to_string()));
        assert_eq!(cancel.handler, "model-download-cancel");
        assert_eq!(cancel.payload.get("modelId"), Some(&"small".to_string()));
    }

    #[test]
    fn model_download_notification_update_prefers_clicked_notification() {
        let storage = storage::test_storage(PathBuf::new());

        storage
            .with_connection(|connection| {
                let started = save_model_notification(
                    connection,
                    "running",
                    "model-download",
                    "Model download started",
                    "Whisper small will download from Hugging Face.",
                    &[model_download_cancel_action("small")],
                    10,
                )?;
                let cancelled = save_model_notification(
                    connection,
                    "cancelled",
                    "model-download",
                    "Model download cancelled",
                    "Whisper small download was cancelled.",
                    &[model_download_retry_action("small")],
                    20,
                )?;
                storage::dismiss_notification(connection, &started.id, 25)?;

                let updated = save_model_download_notification(
                    connection,
                    "small",
                    Some(&started.id),
                    "cancelled",
                    "Model download cancelled",
                    "Whisper small download was cancelled.",
                    &[model_download_retry_action("small")],
                    30,
                )?;

                assert_eq!(updated.id, started.id);
                assert_eq!(updated.dismissed_at_ms, None);
                assert_eq!(updated.created_at_ms, 10);
                assert_eq!(updated.updated_at_ms, 30);

                let rows = storage::list_notifications(connection)?;
                assert_eq!(rows.len(), 2);
                let untouched = rows
                    .into_iter()
                    .find(|row| row.id == cancelled.id)
                    .expect("cancelled duplicate remains untouched");
                assert_eq!(untouched.updated_at_ms, 20);

                Ok(())
            })
            .expect("save notifications");
    }

    fn test_sha1(bytes: &[u8]) -> String {
        let mut hasher = Sha1::new();
        hasher.update(bytes);
        hex::encode(hasher.finalize())
    }

    fn test_temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("daydream-ai-{name}-{}", Uuid::now_v7()))
    }
}
