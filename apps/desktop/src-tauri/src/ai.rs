use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet, HashSet, VecDeque},
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    process::Command,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
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
    ffmpeg,
    notifications::Notification,
    secret_crypto, settings,
    storage::{
        self, ActivityIntervalRow, AiModelAssetRow, AppStorage, CaptureEventRow,
        CreateActivityIntervalInput, CreateProcessingChunkInput, CreateProcessingJobInput,
        CreateTranscriptSegmentInput, CreateTranscriptWordInput, NotificationRow,
        ProcessingChunkRow, ProcessingJobRow, RecordingSegmentRow, RecordingTrackRow, RedactionRow,
        SaveAiModelAssetInput, SaveAppSettingInput, TranscriptSegmentRow, TranscriptWordRow,
        UpsertRecordingTrackInput,
    },
    transcription::{
        self, LocalTranscriptionWarmup, TranscriptionProviderManager, TranscriptionRequest,
    },
    whisper_backend,
};

pub const AI_MODELS_CHANGED_EVENT: &str = "ai-models-changed";
pub const AI_PROCESSING_CHANGED_EVENT: &str = "ai-processing-changed";
mod notification_helpers;
use notification_helpers::{
    model_download_cancel_action, model_download_retry_action, notification_from_row,
    processing_failed_actions, processing_paused_actions, processing_retry_actions,
    processing_running_actions, save_job_progress_notification, save_job_running_notification,
    save_model_download_notification, save_model_notification, save_processing_notification,
    status_to_notification_status, ModelDownloadNotificationInput, ProcessingNotificationInput,
};

const MODEL_PROGRESS_UPDATE_INTERVAL: Duration = Duration::from_millis(500);
const MODEL_PROGRESS_UPDATE_BYTES: i64 = 4 * 1024 * 1024;
const AI_PROCESSING_JOB_KIND: &str = "ai-processing";
const MAX_TRANSCRIPTION_CHUNK_DURATION_MS: i64 = 15 * 60 * 1_000;
const PROCESSING_WORKER_INTERVAL: Duration = Duration::from_millis(500);
const TRANSCRIPTION_TIMEOUT: Duration = Duration::from_secs(30 * 60);
const FRAME_DELTA_SAMPLE_INTERVAL_MS: i64 = 1_000;
const FRAME_DELTA_WIDTH: usize = 64;
const FRAME_DELTA_HEIGHT: usize = 36;
const FRAME_DELTA_THRESHOLD: f64 = 0.005;
const TRANSCRIPTION_RESULT_SKIPPED_SILENCE: &str = "skipped-silence";

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
    pub available_languages: Vec<AiTranscriptionLanguageStatus>,
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AiTranscriptionLanguageStatus {
    pub id: String,
    pub label: String,
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProcessingChunk {
    pub attempt_count: i64,
    pub created_at_ms: i64,
    pub end_offset_ms: Option<i64>,
    pub error_code: Option<String>,
    pub id: String,
    pub job_id: String,
    pub progress: f64,
    pub recording_id: String,
    pub result_code: Option<String>,
    pub segment_id: Option<String>,
    pub source_kind: String,
    pub start_offset_ms: Option<i64>,
    pub status: String,
    pub track_id: Option<String>,
    pub updated_at_ms: i64,
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StartRecordingAiProcessingInput {
    pub recording_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GetRecordingTranscriptInput {
    pub recording_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecordingAiResultInput {
    pub recording_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecordingAiProcessingState {
    pub chunks: Vec<ProcessingChunk>,
    pub job: Option<ProcessingJob>,
    pub recording_id: String,
    pub transcript: RecordingTranscript,
    pub transcription_tracks: Vec<RecordingTranscriptionTrackState>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecordingTranscriptionTrackState {
    pub chunks: Vec<RecordingTranscriptionChunkState>,
    pub description: String,
    pub label: String,
    pub library_track_id: Option<String>,
    pub source_role: String,
    pub status: String,
    pub status_label: String,
    pub text_segment_count: usize,
    pub track_id: Option<String>,
    pub track_key: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecordingTranscriptionChunkState {
    pub attempt_count: i64,
    pub end_offset_ms: Option<i64>,
    pub error_code: Option<String>,
    pub id: String,
    pub progress: f64,
    pub result: String,
    pub result_code: Option<String>,
    pub segment_id: Option<String>,
    pub start_offset_ms: Option<i64>,
    pub status: String,
    pub text: Option<String>,
    pub transcript_segments: Vec<RecordingTranscriptSegment>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecordingTranscript {
    pub recording_id: String,
    pub tracks: Vec<RecordingTranscriptTrack>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecordingTranscriptTrack {
    pub source_role: String,
    pub track_id: String,
    pub segments: Vec<RecordingTranscriptSegment>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecordingTranscriptSegment {
    pub confidence: Option<f64>,
    pub end_offset_ms: i64,
    pub id: String,
    pub model: Option<String>,
    pub provider: String,
    pub segment_id: String,
    pub start_offset_ms: i64,
    pub text: String,
    pub words: Vec<RecordingTranscriptWord>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecordingTranscriptWord {
    pub end_offset_ms: i64,
    pub id: String,
    pub probability: Option<f64>,
    pub start_offset_ms: i64,
    pub text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecordingActivity {
    pub intervals: Vec<ActivityInterval>,
    pub recording_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ActivityInterval {
    pub end_offset_ms: i64,
    pub id: String,
    pub job_id: String,
    pub recording_id: String,
    pub score: Option<f64>,
    pub source_kind: String,
    pub start_offset_ms: i64,
    pub state: String,
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

pub fn recover_model_downloads(app: AppHandle) {
    if let Err(error) = recover_model_downloads_inner(&app) {
        tracing::warn!(
            error_code = %error.code,
            "AI model download recovery failed."
        );
    }
}

fn recover_model_downloads_inner(app: &AppHandle) -> Result<(), AiError> {
    let storage = app.state::<AppStorage>();
    let manager = app.state::<AiModelManager>();
    let mut recoverable = storage
        .with_connection(storage::list_ai_model_assets)
        .map_err(storage_error)?
        .into_iter()
        .filter_map(|row| {
            if !matches!(row.download_status.as_str(), "downloading" | "queued") {
                return None;
            }
            catalog_model(&row.id).map(|catalog| (catalog, row))
        })
        .collect::<Vec<_>>();

    recoverable
        .sort_by_key(|(_catalog, row)| (row.download_status != "downloading", row.updated_at_ms));

    for (catalog, row) in recoverable {
        recover_model_download(app, &storage, &manager, catalog, &row)?;
    }

    Ok(())
}

fn recover_model_download(
    app: &AppHandle,
    storage: &AppStorage,
    manager: &AiModelManager,
    catalog: &'static CatalogModel,
    row: &AiModelAssetRow,
) -> Result<(), AiError> {
    let final_path = model_path(app, catalog)?;
    let partial_path = partial_model_path(app, catalog)?;
    match recover_model_files(catalog, &partial_path, &final_path)? {
        RecoveredModelFiles::VerifiedFinal => {
            let local_path = final_path.to_string_lossy().to_string();
            storage
                .with_connection(|connection| {
                    let timestamp_ms = now_ms();
                    save_catalog_model(
                        connection,
                        catalog,
                        ModelAssetPatch {
                            download_status: "available".into(),
                            downloaded_bytes: catalog.size_bytes,
                            error_code: None,
                            local_path: Some(local_path.as_str().into()),
                            total_bytes: Some(catalog.size_bytes),
                            verification_status: "passed".into(),
                            ..ModelAssetPatch::from_existing(connection, catalog)?
                        },
                        timestamp_ms,
                    )?;
                    save_model_download_notification(
                        connection,
                        ModelDownloadNotificationInput {
                            actions: &[],
                            message: &format!(
                                "{} was recovered from a verified local file.",
                                catalog.label
                            ),
                            model_id: catalog.id,
                            preferred_notification_id: None,
                            status: "succeeded",
                            timestamp_ms,
                            title: "Model download recovered",
                        },
                    )
                    .map(|_row| ())
                })
                .map_err(storage_error)?;
            emit_model_list(app);
        }
        RecoveredModelFiles::Partial(downloaded_bytes) => {
            resume_recovered_model_download(app, storage, manager, catalog, downloaded_bytes)?;
        }
        RecoveredModelFiles::None => {
            if row.download_status == "queued" && row.downloaded_bytes == 0 {
                resume_recovered_model_download(app, storage, manager, catalog, 0)?;
            } else {
                storage
                    .with_connection(|connection| {
                        let timestamp_ms = now_ms();
                        save_catalog_model(
                            connection,
                            catalog,
                            ModelAssetPatch {
                                download_status: "failed".into(),
                                downloaded_bytes: 0,
                                error_code: Some("ai_model_download_partial_missing".into()),
                                local_path: None,
                                verification_status: "failed".into(),
                                ..ModelAssetPatch::from_existing(connection, catalog)?
                            },
                            timestamp_ms,
                        )?;
                        save_model_download_notification(
                            connection,
                            ModelDownloadNotificationInput {
                                actions: &[model_download_retry_action(catalog.id)],
                                message: &format!(
                                    "{} download could not resume because its partial file is missing.",
                                    catalog.label
                                ),
                                model_id: catalog.id,
                                preferred_notification_id: None,
                                status: "failed",
                                timestamp_ms,
                                title: "Model download needs retry",
                            },
                        )
                        .map(|_row| ())
                    })
                    .map_err(storage_error)?;
                emit_model_list(app);
            }
        }
    }

    Ok(())
}

fn resume_recovered_model_download(
    app: &AppHandle,
    storage: &AppStorage,
    manager: &AiModelManager,
    catalog: &'static CatalogModel,
    downloaded_bytes: i64,
) -> Result<(), AiError> {
    let state = manager.request_download(catalog.id);
    let status = if state == DownloadRequestState::Queued {
        "queued"
    } else {
        "downloading"
    };
    storage
        .with_connection(|connection| {
            let timestamp_ms = now_ms();
            save_catalog_model(
                connection,
                catalog,
                ModelAssetPatch {
                    download_status: status.into(),
                    downloaded_bytes,
                    error_code: None,
                    local_path: None,
                    total_bytes: Some(catalog.size_bytes),
                    verification_status: "pending".into(),
                    ..ModelAssetPatch::from_existing(connection, catalog)?
                },
                timestamp_ms,
            )?;
            save_model_download_notification(
                connection,
                ModelDownloadNotificationInput {
                    actions: &[model_download_cancel_action(catalog.id)],
                    message: &format!(
                        "{} download will resume from local partial data.",
                        catalog.label
                    ),
                    model_id: catalog.id,
                    preferred_notification_id: None,
                    status: status_to_notification_status(status),
                    timestamp_ms,
                    title: if state == DownloadRequestState::Queued {
                        "Model download queued"
                    } else {
                        "Model download resumed"
                    },
                },
            )
            .map(|_row| ())
        })
        .map_err(storage_error)?;
    emit_model_list(app);

    if state == DownloadRequestState::Started {
        spawn_model_download(app.clone(), catalog.id.to_string());
    }

    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
enum RecoveredModelFiles {
    None,
    Partial(i64),
    VerifiedFinal,
}

fn recover_model_files(
    catalog: &CatalogModel,
    partial_path: &Path,
    final_path: &Path,
) -> Result<RecoveredModelFiles, AiError> {
    if final_path.is_file() {
        if is_verified_model_file(final_path, catalog)? {
            return Ok(RecoveredModelFiles::VerifiedFinal);
        }
        delete_file_if_exists(final_path)?;
    }

    match prepare_partial_download(catalog, partial_path)? {
        PartialDownloadResume::Complete => Ok(RecoveredModelFiles::Partial(catalog.size_bytes)),
        PartialDownloadResume::Resume(length) if length > 0 => {
            Ok(RecoveredModelFiles::Partial(length as i64))
        }
        PartialDownloadResume::Resume(_length) => Ok(RecoveredModelFiles::None),
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
    if existing_model_path.is_file() && is_verified_model_file(&existing_model_path, catalog)? {
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
                    ModelDownloadNotificationInput {
                        actions: &[],
                        message: &format!("{} is already available locally.", catalog.label),
                        model_id: catalog.id,
                        preferred_notification_id: input.notification_id.as_deref(),
                        status: "succeeded",
                        timestamp_ms,
                        title: "Model already downloaded",
                    },
                )
            })
            .map_err(storage_error)?;
        emit_model_list(&app);

        return notification_from_row(notification);
    }
    if existing_model_path.is_file() {
        delete_file_if_exists(&existing_model_path)?;
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
                ModelDownloadNotificationInput {
                    actions: &[model_download_cancel_action(catalog.id)],
                    message: &format!("{} will download from Hugging Face.", catalog.label),
                    model_id: catalog.id,
                    preferred_notification_id: input.notification_id.as_deref(),
                    status: status_to_notification_status(status),
                    timestamp_ms,
                    title: if state == DownloadRequestState::Queued {
                        "Model download queued"
                    } else if state == DownloadRequestState::AlreadyRunning {
                        "Model download already running"
                    } else {
                        "Model download started"
                    },
                },
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
    let active_or_queued_cancelled = manager.cancel_download(catalog.id);
    let persisted_cancelled = if active_or_queued_cancelled {
        false
    } else {
        storage
            .with_connection(|connection| storage::get_ai_model_asset(connection, catalog.id))
            .map_err(storage_error)?
            .is_some_and(|model| matches!(model.download_status.as_str(), "downloading" | "queued"))
    };
    let cancelled = active_or_queued_cancelled || persisted_cancelled;
    let downloaded = if cancelled {
        let partial_path = partial_model_path(&app, catalog)?;
        let final_path = model_path(&app, catalog)?;
        Some(downloaded_bytes(&partial_path, &final_path))
    } else {
        None
    };
    let notification = storage
        .with_connection(|connection| {
            let timestamp_ms = now_ms();
            save_cancel_download_result(
                connection,
                catalog,
                input.notification_id.as_deref(),
                cancelled,
                downloaded,
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
    provider: State<'_, TranscriptionProviderManager>,
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

    let app_settings = settings::load_app_settings(&storage).map_err(settings_error)?;
    let warmup = LocalTranscriptionWarmup {
        local_backend: app_settings.ai.transcription.local_backend,
        model_id: catalog.id.to_string(),
        model_path: path.clone(),
    };
    let provider = provider.inner().clone();
    let load_result = tauri::async_runtime::spawn_blocking(move || {
        provider
            .warm_local_model(warmup)
            .map_err(|error| whisper_backend::WhisperBackendError {
                code: error.code,
                message: error.message,
            })
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
            available_languages: whisper_backend::supported_languages()
                .into_iter()
                .map(|language| AiTranscriptionLanguageStatus {
                    id: language.id,
                    label: language.label,
                })
                .collect(),
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

pub fn start_processing_worker(app: AppHandle) {
    let storage = app.state::<AppStorage>();
    if let Err(error) = recover_stale_processing(&storage) {
        tracing::warn!(error_code = %error.code, "AI processing recovery failed.");
    }
    tauri::async_runtime::spawn(async move {
        loop {
            if let Err(error) = run_processing_worker_tick(&app).await {
                tracing::warn!(error_code = %error.code, "AI processing worker tick failed.");
            }
            tokio::time::sleep(PROCESSING_WORKER_INTERVAL).await;
        }
    });
}

fn recover_stale_processing(storage: &AppStorage) -> Result<(), AiError> {
    storage
        .with_connection(|connection| {
            let timestamp_ms = now_ms();
            let jobs = storage::list_processing_jobs_by_status(
                connection,
                AI_PROCESSING_JOB_KIND,
                &["running"],
                None,
            )?;
            for job in jobs {
                storage::update_processing_job_status(
                    connection,
                    &job.id,
                    "queued",
                    None,
                    timestamp_ms,
                )?;
                storage::requeue_processing_chunks(
                    connection,
                    &job.id,
                    &["running"],
                    timestamp_ms,
                )?;
            }
            Ok(())
        })
        .map_err(storage_error)
}

async fn run_processing_worker_tick(app: &AppHandle) -> Result<(), AiError> {
    let storage = app.state::<AppStorage>();
    let app_settings = settings::load_app_settings(&storage).map_err(settings_error)?;
    let running = storage
        .with_connection(|connection| {
            storage::list_processing_jobs_by_status(
                connection,
                AI_PROCESSING_JOB_KIND,
                &["running"],
                None,
            )
        })
        .map_err(storage_error)?;
    let available = app_settings
        .ai
        .processing
        .max_parallel_jobs
        .saturating_sub(running.len() as u32) as usize;
    if available == 0 {
        tracing::debug!(
            running_job_count = running.len(),
            max_parallel_jobs = app_settings.ai.processing.max_parallel_jobs,
            "ai_processing_worker_capacity_full"
        );
        return Ok(());
    }
    let queued = storage
        .with_connection(|connection| {
            storage::list_processing_jobs_by_status(
                connection,
                AI_PROCESSING_JOB_KIND,
                &["queued"],
                Some(available),
            )
        })
        .map_err(storage_error)?;
    tracing::debug!(
        running_job_count = running.len(),
        queued_job_count = queued.len(),
        available_job_slots = available,
        "ai_processing_worker_tick"
    );

    for job in queued {
        let claimed = storage
            .with_connection(|connection| {
                let current = storage::get_processing_job(connection, &job.id)?
                    .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
                if current.status != "queued" {
                    return Ok(None);
                }
                storage::update_processing_job_status(
                    connection,
                    &job.id,
                    "running",
                    None,
                    now_ms(),
                )
                .map(Some)
            })
            .map_err(storage_error)?;
        if let Some(claimed) = claimed {
            tracing::info!(
                recording_id = %claimed.recording_id,
                job_id = %claimed.id,
                "ai_processing_job_claimed"
            );
            emit_processing_state(&app, &claimed.recording_id);
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(error) = process_recording_ai_job(app.clone(), claimed.id.clone()).await
                {
                    let storage = app.state::<AppStorage>();
                    let _ = storage.with_connection(|connection| {
                        storage::update_processing_job_status(
                            connection,
                            &claimed.id,
                            "failed",
                            Some(&error.code),
                            now_ms(),
                        )?;
                        save_processing_notification(
                            connection,
                            ProcessingNotificationInput {
                                actions: &processing_failed_actions(&claimed.id),
                                job_id: Some(&claimed.id),
                                message: &format!("AI processing failed: {}.", error.code),
                                recording_id: &claimed.recording_id,
                                status: "failed",
                                timestamp_ms: now_ms(),
                                title: "AI processing failed",
                            },
                        )
                        .map(|_row| ())
                    });
                    emit_processing_state(&app, &claimed.recording_id);
                }
            });
        }
    }

    Ok(())
}

pub(crate) fn enqueue_recording_ai_processing<R: tauri::Runtime>(
    app: &AppHandle<R>,
    storage: &AppStorage,
    recording_id: &str,
) -> Result<ProcessingJobRow, AiError> {
    tracing::info!(recording_id = %recording_id, "ai_processing_start_requested");
    let app_settings = settings::load_app_settings(storage).map_err(settings_error)?;
    let recording = storage
        .with_connection(|connection| storage::get_recording(connection, recording_id))
        .map_err(storage_error)?;
    if recording.status != "completed" {
        tracing::warn!(
            recording_id = %recording_id,
            recording_status = %recording.status,
            "ai_processing_start_blocked_recording_not_completed"
        );
        return Err(save_readiness_failure(
            storage,
            recording_id,
            "ai_processing_recording_not_completed",
            "Recording must be completed before AI processing starts.",
        ));
    }
    let segments = storage
        .with_connection(|connection| storage::list_recording_segments(connection, recording_id))
        .map_err(storage_error)?;
    if storage
        .with_connection(|connection| storage::recording_has_redactions(connection, recording_id))
        .map_err(storage_error)?
    {
        tracing::warn!(
            recording_id = %recording_id,
            "ai_processing_start_blocked_redactions"
        );
        return Err(save_readiness_failure(
            storage,
            recording_id,
            "ai_processing_redactions_unsupported",
            "AI processing is blocked for redacted recordings until redaction-aware chunk splitting is implemented.",
        ));
    }
    let audio_plans = audio_chunk_plans(&segments);
    let activity_plans = activity_chunk_plans(&segments, &app_settings);
    let audio_chunk_count = audio_plans.len();
    let activity_chunk_count = activity_plans.len();
    tracing::info!(
        recording_id = %recording_id,
        segment_count = segments.len(),
        audio_chunk_count,
        activity_chunk_count,
        transcription_provider = %app_settings.ai.transcription.provider,
        transcription_model = %app_settings.ai.transcription.local_model_id,
        transcription_backend = %app_settings.ai.transcription.local_backend,
        word_timestamps = app_settings.ai.transcription.word_timestamps_enabled,
        "ai_processing_plan_built"
    );
    if !audio_plans.is_empty() {
        if let Err(error) = check_transcription_readiness(app, storage, &app_settings) {
            tracing::warn!(
                recording_id = %recording_id,
                error_code = %error.code,
                "ai_processing_start_blocked_transcription_readiness"
            );
            return Err(save_readiness_failure(
                storage,
                recording_id,
                &error.code,
                &error.message,
            ));
        }
    }

    if audio_plans.is_empty() && activity_plans.is_empty() {
        tracing::warn!(
            recording_id = %recording_id,
            "ai_processing_start_blocked_no_enabled_sources"
        );
        return Err(save_readiness_failure(
            storage,
            recording_id,
            "ai_processing_no_enabled_sources",
            "Recording has no enabled audio or activity sources to process.",
        ));
    }

    let job = storage
        .with_connection(|connection| {
            if let Some(existing) = storage::get_latest_processing_job_for_recording(
                connection,
                recording_id,
                AI_PROCESSING_JOB_KIND,
            )? {
                tracing::info!(
                    recording_id = %recording_id,
                    job_id = %existing.id,
                    job_status = %existing.status,
                    "ai_processing_existing_job_found"
                );
                match existing.status.as_str() {
                    "queued" | "running" | "paused" | "succeeded" => return Ok(existing),
                    "failed" | "cancelled" => {
                        return storage::retry_processing_job_chunks(
                            connection,
                            &existing.id,
                            now_ms(),
                        );
                    }
                    _ => {}
                }
            }

            let timestamp_ms = now_ms();
            let job_id = Uuid::now_v7().to_string();
            let job = storage::create_processing_job(
                connection,
                &CreateProcessingJobInput {
                    created_at_ms: timestamp_ms,
                    error: None,
                    id: job_id.clone(),
                    kind: AI_PROCESSING_JOB_KIND.to_string(),
                    recording_id: recording_id.to_string(),
                    status: "queued".to_string(),
                    updated_at_ms: timestamp_ms,
                },
            )?;
            let desktop_track = ensure_audio_track(
                connection,
                recording_id,
                "desktop-audio",
                "Desktop audio",
                timestamp_ms,
            )?;
            let microphone_track = ensure_audio_track(
                connection,
                recording_id,
                "microphone",
                "Microphone",
                timestamp_ms,
            )?;
            for plan in audio_plans {
                let track_id = match plan.source_role.as_str() {
                    "desktop-audio" => desktop_track.id.clone(),
                    "microphone" => microphone_track.id.clone(),
                    _ => return Err(rusqlite::Error::InvalidQuery),
                };
                storage::create_processing_chunk(
                    connection,
                    &CreateProcessingChunkInput {
                        attempt_count: 0,
                        created_at_ms: timestamp_ms,
                        end_offset_ms: Some(plan.end_offset_ms),
                        error_code: None,
                        id: Uuid::now_v7().to_string(),
                        job_id: job_id.clone(),
                        progress: 0.0,
                        recording_id: recording_id.to_string(),
                        result_code: None,
                        segment_id: Some(plan.segment_id),
                        source_kind: "audio".to_string(),
                        start_offset_ms: Some(plan.start_offset_ms),
                        status: "queued".to_string(),
                        track_id: Some(track_id),
                        updated_at_ms: timestamp_ms,
                    },
                )?;
            }
            for plan in activity_plans {
                storage::create_processing_chunk(
                    connection,
                    &CreateProcessingChunkInput {
                        attempt_count: 0,
                        created_at_ms: timestamp_ms,
                        end_offset_ms: Some(plan.end_offset_ms),
                        error_code: None,
                        id: Uuid::now_v7().to_string(),
                        job_id: job_id.clone(),
                        progress: 0.0,
                        recording_id: recording_id.to_string(),
                        result_code: None,
                        segment_id: Some(plan.segment_id),
                        source_kind: "activity".to_string(),
                        start_offset_ms: Some(plan.start_offset_ms),
                        status: "queued".to_string(),
                        track_id: None,
                        updated_at_ms: timestamp_ms,
                    },
                )?;
            }
            save_processing_notification(
                connection,
                ProcessingNotificationInput {
                    actions: &processing_running_actions(&job_id),
                    job_id: Some(&job_id),
                    message: "AI processing is queued for this recording.",
                    recording_id,
                    status: "running",
                    timestamp_ms,
                    title: "AI processing queued",
                },
            )?;
            tracing::info!(
                recording_id = %recording_id,
                job_id = %job_id,
                audio_chunk_count,
                activity_chunk_count,
                "ai_processing_job_created"
            );
            Ok(job)
        })
        .map_err(storage_error)?;
    emit_processing_state(app, recording_id);
    Ok(job)
}

#[derive(Clone, Debug)]
struct AudioChunkPlan {
    end_offset_ms: i64,
    segment_id: String,
    source_role: String,
    start_offset_ms: i64,
}

#[derive(Clone, Debug)]
struct ActivityChunkPlan {
    end_offset_ms: i64,
    segment_id: String,
    start_offset_ms: i64,
}

fn audio_chunk_plans(segments: &[RecordingSegmentRow]) -> Vec<AudioChunkPlan> {
    let mut playback_offset_ms = 0;
    let mut plans = Vec::new();
    for (index, segment) in segments.iter().enumerate() {
        let duration_ms = segment.duration_ms.unwrap_or_else(|| {
            segment
                .ended_at_ms
                .map(|ended_at_ms| ended_at_ms.saturating_sub(segment.started_at_ms))
                .unwrap_or(0)
        });
        let next_started_at_ms = segments.get(index + 1).map(|next| next.started_at_ms);
        let pause_after_ms = segment
            .ended_at_ms
            .zip(next_started_at_ms)
            .map(|(ended_at_ms, next_started_at_ms)| next_started_at_ms.saturating_sub(ended_at_ms))
            .unwrap_or(0);
        if segment.state == "finalized" && Path::new(&segment.file_path).is_file() {
            if segment.desktop_audio_stream_index.is_some() {
                append_audio_slice_plans(
                    &mut plans,
                    segment,
                    "desktop-audio",
                    playback_offset_ms,
                    duration_ms.max(0),
                );
            }
            if segment.microphone_audio_stream_index.is_some() {
                append_audio_slice_plans(
                    &mut plans,
                    segment,
                    "microphone",
                    playback_offset_ms,
                    duration_ms.max(0),
                );
            }
            playback_offset_ms += duration_ms.max(0) + pause_after_ms.max(0);
        }
    }

    plans
}

fn activity_chunk_plans(
    segments: &[RecordingSegmentRow],
    app_settings: &settings::AppSettings,
) -> Vec<ActivityChunkPlan> {
    if !app_settings.ai.activity.frame_deltas.enabled
        && !app_settings.ai.activity.keyboard.enabled
        && !app_settings.ai.activity.mouse.enabled
    {
        return Vec::new();
    }

    let mut playback_offset_ms = 0;
    let mut plans = Vec::new();
    for (index, segment) in segments.iter().enumerate() {
        let duration_ms = segment.duration_ms.unwrap_or_else(|| {
            segment
                .ended_at_ms
                .map(|ended_at_ms| ended_at_ms.saturating_sub(segment.started_at_ms))
                .unwrap_or(0)
        });
        let next_started_at_ms = segments.get(index + 1).map(|next| next.started_at_ms);
        let pause_after_ms = segment
            .ended_at_ms
            .zip(next_started_at_ms)
            .map(|(ended_at_ms, next_started_at_ms)| next_started_at_ms.saturating_sub(ended_at_ms))
            .unwrap_or(0);
        if segment.state == "finalized" && Path::new(&segment.file_path).is_file() {
            plans.push(ActivityChunkPlan {
                end_offset_ms: playback_offset_ms + duration_ms.max(0),
                segment_id: segment.id.clone(),
                start_offset_ms: playback_offset_ms,
            });
            playback_offset_ms += duration_ms.max(0) + pause_after_ms.max(0);
        }
    }

    plans
}

fn append_audio_slice_plans(
    plans: &mut Vec<AudioChunkPlan>,
    segment: &RecordingSegmentRow,
    source_role: &str,
    playback_offset_ms: i64,
    duration_ms: i64,
) {
    let mut slice_start_ms = 0;
    while slice_start_ms < duration_ms {
        let slice_end_ms = (slice_start_ms + MAX_TRANSCRIPTION_CHUNK_DURATION_MS).min(duration_ms);
        plans.push(AudioChunkPlan {
            end_offset_ms: playback_offset_ms + slice_end_ms,
            segment_id: segment.id.clone(),
            source_role: source_role.to_string(),
            start_offset_ms: playback_offset_ms + slice_start_ms,
        });
        slice_start_ms = slice_end_ms;
    }
}

fn playback_offset_for_segment(segments: &[RecordingSegmentRow], segment_id: &str) -> Option<i64> {
    let mut playback_offset_ms = 0;
    for (index, segment) in segments.iter().enumerate() {
        let duration_ms = segment.duration_ms.unwrap_or_else(|| {
            segment
                .ended_at_ms
                .map(|ended_at_ms| ended_at_ms.saturating_sub(segment.started_at_ms))
                .unwrap_or(0)
        });
        let next_started_at_ms = segments.get(index + 1).map(|next| next.started_at_ms);
        let pause_after_ms = segment
            .ended_at_ms
            .zip(next_started_at_ms)
            .map(|(ended_at_ms, next_started_at_ms)| next_started_at_ms.saturating_sub(ended_at_ms))
            .unwrap_or(0);

        if segment.id == segment_id {
            return Some(playback_offset_ms);
        }

        if segment.state == "finalized" && Path::new(&segment.file_path).is_file() {
            playback_offset_ms += duration_ms.max(0) + pause_after_ms.max(0);
        }
    }

    None
}

fn ensure_audio_track(
    connection: &rusqlite::Connection,
    recording_id: &str,
    kind: &str,
    label: &str,
    timestamp_ms: i64,
) -> rusqlite::Result<RecordingTrackRow> {
    if let Some(track) = storage::get_recording_track_by_kind(connection, recording_id, kind)? {
        return Ok(track);
    }
    storage::upsert_recording_track(
        connection,
        &UpsertRecordingTrackInput {
            codec: None,
            created_at_ms: timestamp_ms,
            id: Uuid::now_v7().to_string(),
            kind: kind.to_string(),
            label: label.to_string(),
            path: None,
            recording_id: recording_id.to_string(),
            source_id: None,
        },
    )
}

async fn process_recording_ai_job(app: AppHandle, job_id: String) -> Result<(), AiError> {
    let storage = app.state::<AppStorage>();
    let job = storage
        .with_connection(|connection| storage::get_processing_job(connection, &job_id))
        .map_err(storage_error)?
        .ok_or_else(|| ai_error("ai_processing_job_missing", "Processing job is missing."))?;
    tracing::info!(
        job_id = %job.id,
        recording_id = %job.recording_id,
        "AI processing job started."
    );
    save_job_running_notification(&storage, &job)?;

    loop {
        let current = storage
            .with_connection(|connection| storage::get_processing_job(connection, &job_id))
            .map_err(storage_error)?
            .ok_or_else(|| ai_error("ai_processing_job_missing", "Processing job is missing."))?;
        match current.status.as_str() {
            "paused" => {
                storage
                    .with_connection(|connection| {
                        storage::requeue_processing_chunks(
                            connection,
                            &job_id,
                            &["running"],
                            now_ms(),
                        )?;
                        save_processing_notification(
                            connection,
                            ProcessingNotificationInput {
                                actions: &processing_paused_actions(&job_id),
                                job_id: Some(&job_id),
                                message: "Audio transcription is paused.",
                                recording_id: &current.recording_id,
                                status: "info",
                                timestamp_ms: now_ms(),
                                title: "AI processing paused",
                            },
                        )
                        .map(|_row| ())
                    })
                    .map_err(storage_error)?;
                emit_processing_state(&app, &current.recording_id);
                return Ok(());
            }
            "cancelled" => {
                storage
                    .with_connection(|connection| {
                        storage::cancel_processing_chunks(
                            connection,
                            &job_id,
                            &["queued", "running"],
                            now_ms(),
                        )?;
                        save_processing_notification(
                            connection,
                            ProcessingNotificationInput {
                                actions: &processing_retry_actions(&job_id),
                                job_id: Some(&job_id),
                                message: "Audio transcription was cancelled.",
                                recording_id: &current.recording_id,
                                status: "cancelled",
                                timestamp_ms: now_ms(),
                                title: "AI processing cancelled",
                            },
                        )
                        .map(|_row| ())
                    })
                    .map_err(storage_error)?;
                emit_processing_state(&app, &current.recording_id);
                return Ok(());
            }
            "running" => {}
            _ => return Ok(()),
        }

        let next_chunk = storage
            .with_connection(|connection| {
                storage::claim_next_queued_processing_chunk(connection, &job_id, 0.05, now_ms())
            })
            .map_err(storage_error)?;
        let Some(chunk) = next_chunk else {
            finish_processing_job(&app, &storage, &job_id)?;
            return Ok(());
        };
        tracing::info!(
            recording_id = %chunk.recording_id,
            job_id = %job_id,
            chunk_id = %chunk.id,
            source_kind = %chunk.source_kind,
            track_id = chunk.track_id.as_deref().unwrap_or(""),
            segment_id = chunk.segment_id.as_deref().unwrap_or(""),
            start_offset_ms = ?chunk.start_offset_ms,
            end_offset_ms = ?chunk.end_offset_ms,
            attempt_count = chunk.attempt_count,
            "ai_processing_chunk_claimed"
        );

        match process_processing_chunk(&app, &job_id, chunk).await {
            Ok(()) => {}
            Err(error) if error.code == "ai_processing_paused" => return Ok(()),
            Err(error) if error.code == "ai_processing_cancelled" => return Ok(()),
            Err(error) => {
                fail_processing_job(&app, &storage, &job_id, &error.code)?;
                return Err(error);
            }
        }
    }
}

async fn process_processing_chunk(
    app: &AppHandle,
    job_id: &str,
    chunk: ProcessingChunkRow,
) -> Result<(), AiError> {
    match chunk.source_kind.as_str() {
        "audio" => process_audio_chunk(app, job_id, chunk).await,
        "activity" => process_activity_chunk(app, job_id, chunk).await,
        _ => Err(ai_error(
            "ai_processing_chunk_source_unsupported",
            "Processing chunk source is not supported.",
        )),
    }
}

async fn process_audio_chunk(
    app: &AppHandle,
    job_id: &str,
    chunk: ProcessingChunkRow,
) -> Result<(), AiError> {
    let storage = app.state::<AppStorage>();
    tracing::info!(
        recording_id = %chunk.recording_id,
        job_id = %job_id,
        chunk_id = %chunk.id,
        track_id = chunk.track_id.as_deref().unwrap_or(""),
        segment_id = chunk.segment_id.as_deref().unwrap_or(""),
        start_offset_ms = ?chunk.start_offset_ms,
        end_offset_ms = ?chunk.end_offset_ms,
        attempt_count = chunk.attempt_count,
        "ai_processing_audio_chunk_started"
    );
    emit_processing_state(app, &chunk.recording_id);
    let cancel = Arc::new(AtomicBool::new(false));
    let monitor = spawn_job_cancel_monitor(app.clone(), job_id.to_string(), cancel.clone());
    let result = process_audio_chunk_inner(app, &chunk, cancel.clone()).await;
    monitor.abort();

    match result {
        Ok(outcome) => {
            storage
                .with_connection(|connection| {
                    storage::update_processing_chunk_status(
                        connection,
                        &chunk.id,
                        "succeeded",
                        1.0,
                        None,
                        now_ms(),
                    )?;
                    if let Some(result_code) = outcome.result_code.as_deref() {
                        storage::update_processing_chunk_result_code(
                            connection,
                            &chunk.id,
                            Some(result_code),
                            now_ms(),
                        )?;
                    }
                    Ok(())
                })
                .map_err(storage_error)?;
            save_job_progress_notification(&storage, job_id)?;
            tracing::info!(
                recording_id = %chunk.recording_id,
                job_id = %job_id,
                chunk_id = %chunk.id,
                track_id = chunk.track_id.as_deref().unwrap_or(""),
                "ai_processing_audio_chunk_succeeded"
            );
            emit_processing_state(app, &chunk.recording_id);
            Ok(())
        }
        Err(error) if error.code == "ai_transcription_cancelled" => {
            let job = storage
                .with_connection(|connection| storage::get_processing_job(connection, job_id))
                .map_err(storage_error)?
                .ok_or_else(|| {
                    ai_error("ai_processing_job_missing", "Processing job is missing.")
                })?;
            match job.status.as_str() {
                "paused" => {
                    storage
                        .with_connection(|connection| {
                            storage::update_processing_chunk_status(
                                connection,
                                &chunk.id,
                                "queued",
                                0.0,
                                None,
                                now_ms(),
                            )
                            .map(|_row| ())
                        })
                        .map_err(storage_error)?;
                    tracing::info!(
                        recording_id = %chunk.recording_id,
                        job_id = %job_id,
                        chunk_id = %chunk.id,
                        track_id = chunk.track_id.as_deref().unwrap_or(""),
                        "ai_processing_audio_chunk_requeued_paused"
                    );
                    emit_processing_state(app, &chunk.recording_id);
                    Err(ai_error(
                        "ai_processing_paused",
                        "AI processing was paused.",
                    ))
                }
                "cancelled" => {
                    storage
                        .with_connection(|connection| {
                            storage::update_processing_chunk_status(
                                connection,
                                &chunk.id,
                                "cancelled",
                                0.0,
                                Some("ai_processing_cancelled"),
                                now_ms(),
                            )
                            .map(|_row| ())
                        })
                        .map_err(storage_error)?;
                    tracing::info!(
                        recording_id = %chunk.recording_id,
                        job_id = %job_id,
                        chunk_id = %chunk.id,
                        track_id = chunk.track_id.as_deref().unwrap_or(""),
                        "ai_processing_audio_chunk_cancelled"
                    );
                    emit_processing_state(app, &chunk.recording_id);
                    Err(ai_error(
                        "ai_processing_cancelled",
                        "AI processing was cancelled.",
                    ))
                }
                _ => {
                    storage
                        .with_connection(|connection| {
                            storage::update_processing_chunk_status(
                                connection,
                                &chunk.id,
                                "failed",
                                0.0,
                                Some(&error.code),
                                now_ms(),
                            )
                            .map(|_row| ())
                        })
                        .map_err(storage_error)?;
                    tracing::warn!(
                        recording_id = %chunk.recording_id,
                        job_id = %job_id,
                        chunk_id = %chunk.id,
                        track_id = chunk.track_id.as_deref().unwrap_or(""),
                        error_code = %error.code,
                        job_status = %job.status,
                        "ai_processing_audio_chunk_failed_after_cancel_signal"
                    );
                    emit_processing_state(app, &chunk.recording_id);
                    Err(error)
                }
            }
        }
        Err(error) => {
            storage
                .with_connection(|connection| {
                    storage::update_processing_chunk_status(
                        connection,
                        &chunk.id,
                        "failed",
                        0.0,
                        Some(&error.code),
                        now_ms(),
                    )
                    .map(|_row| ())
                })
                .map_err(storage_error)?;
            tracing::warn!(
                recording_id = %chunk.recording_id,
                job_id = %job_id,
                chunk_id = %chunk.id,
                track_id = chunk.track_id.as_deref().unwrap_or(""),
                error_code = %error.code,
                "ai_processing_audio_chunk_failed"
            );
            emit_processing_state(app, &chunk.recording_id);
            Err(error)
        }
    }
}

async fn process_activity_chunk(
    app: &AppHandle,
    job_id: &str,
    chunk: ProcessingChunkRow,
) -> Result<(), AiError> {
    let storage = app.state::<AppStorage>();
    tracing::info!(
        recording_id = %chunk.recording_id,
        job_id = %job_id,
        chunk_id = %chunk.id,
        segment_id = chunk.segment_id.as_deref().unwrap_or(""),
        start_offset_ms = ?chunk.start_offset_ms,
        end_offset_ms = ?chunk.end_offset_ms,
        attempt_count = chunk.attempt_count,
        "ai_processing_activity_chunk_started"
    );
    emit_processing_state(app, &chunk.recording_id);
    let cancel = Arc::new(AtomicBool::new(false));
    let monitor = spawn_job_cancel_monitor(app.clone(), job_id.to_string(), cancel.clone());
    let result = process_activity_chunk_inner(app, &chunk, cancel.clone()).await;
    monitor.abort();

    match result {
        Ok(()) => {
            storage
                .with_connection(|connection| {
                    storage::update_processing_chunk_status(
                        connection,
                        &chunk.id,
                        "succeeded",
                        1.0,
                        None,
                        now_ms(),
                    )
                    .map(|_row| ())
                })
                .map_err(storage_error)?;
            save_job_progress_notification(&storage, job_id)?;
            tracing::info!(
                recording_id = %chunk.recording_id,
                job_id = %job_id,
                chunk_id = %chunk.id,
                "ai_processing_activity_chunk_succeeded"
            );
            emit_processing_state(app, &chunk.recording_id);
            Ok(())
        }
        Err(error) if error.code == "ai_processing_cancelled" => {
            let job = storage
                .with_connection(|connection| storage::get_processing_job(connection, job_id))
                .map_err(storage_error)?
                .ok_or_else(|| {
                    ai_error("ai_processing_job_missing", "Processing job is missing.")
                })?;
            match job.status.as_str() {
                "paused" => {
                    storage
                        .with_connection(|connection| {
                            storage::update_processing_chunk_status(
                                connection,
                                &chunk.id,
                                "queued",
                                0.0,
                                None,
                                now_ms(),
                            )
                            .map(|_row| ())
                        })
                        .map_err(storage_error)?;
                    tracing::info!(
                        recording_id = %chunk.recording_id,
                        job_id = %job_id,
                        chunk_id = %chunk.id,
                        "ai_processing_activity_chunk_requeued_paused"
                    );
                    emit_processing_state(app, &chunk.recording_id);
                    Err(ai_error(
                        "ai_processing_paused",
                        "AI processing was paused.",
                    ))
                }
                "cancelled" => {
                    storage
                        .with_connection(|connection| {
                            storage::update_processing_chunk_status(
                                connection,
                                &chunk.id,
                                "cancelled",
                                0.0,
                                Some("ai_processing_cancelled"),
                                now_ms(),
                            )
                            .map(|_row| ())
                        })
                        .map_err(storage_error)?;
                    tracing::info!(
                        recording_id = %chunk.recording_id,
                        job_id = %job_id,
                        chunk_id = %chunk.id,
                        "ai_processing_activity_chunk_cancelled"
                    );
                    emit_processing_state(app, &chunk.recording_id);
                    Err(error)
                }
                _ => Err(error),
            }
        }
        Err(error) => {
            storage
                .with_connection(|connection| {
                    storage::update_processing_chunk_status(
                        connection,
                        &chunk.id,
                        "failed",
                        0.0,
                        Some(&error.code),
                        now_ms(),
                    )
                    .map(|_row| ())
                })
                .map_err(storage_error)?;
            tracing::warn!(
                recording_id = %chunk.recording_id,
                job_id = %job_id,
                chunk_id = %chunk.id,
                error_code = %error.code,
                "ai_processing_activity_chunk_failed"
            );
            emit_processing_state(app, &chunk.recording_id);
            Err(error)
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AudioChunkProcessingOutcome {
    result_code: Option<String>,
}

async fn process_audio_chunk_inner(
    app: &AppHandle,
    chunk: &ProcessingChunkRow,
    cancel: Arc<AtomicBool>,
) -> Result<AudioChunkProcessingOutcome, AiError> {
    let storage = app.state::<AppStorage>();
    let app_settings = settings::load_app_settings(&storage).map_err(settings_error)?;
    let provider_name = app_settings.ai.transcription.provider.clone();
    check_atomic_cancel(&cancel)?;
    let (segment, track, segment_playback_offset_ms) = storage
        .with_connection(|connection| {
            let segment_id = chunk
                .segment_id
                .as_deref()
                .ok_or(rusqlite::Error::InvalidQuery)?;
            let segments = storage::list_recording_segments(connection, &chunk.recording_id)?;
            let segment_playback_offset_ms = playback_offset_for_segment(&segments, segment_id)
                .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
            let segment = segments
                .into_iter()
                .find(|candidate| candidate.id == segment_id)
                .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
            let track_id = chunk
                .track_id
                .as_deref()
                .ok_or(rusqlite::Error::InvalidQuery)?;
            let track = storage::get_recording_track(connection, track_id)?
                .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
            Ok((segment, track, segment_playback_offset_ms))
        })
        .map_err(storage_error)?;
    let stream_index = match track.kind.as_str() {
        "desktop-audio" => segment.desktop_audio_stream_index,
        "microphone" => segment.microphone_audio_stream_index,
        _ => None,
    }
    .ok_or_else(|| {
        ai_error(
            "ai_processing_audio_stream_missing",
            "Audio stream is missing.",
        )
    })?;
    let chunk_start_offset_ms = chunk.start_offset_ms.ok_or_else(|| {
        ai_error(
            "ai_processing_chunk_offset_missing",
            "Processing chunk start offset is missing.",
        )
    })?;
    let chunk_end_offset_ms = chunk.end_offset_ms.ok_or_else(|| {
        ai_error(
            "ai_processing_chunk_offset_missing",
            "Processing chunk end offset is missing.",
        )
    })?;
    let slice_start_ms = chunk_start_offset_ms
        .checked_sub(segment_playback_offset_ms)
        .ok_or_else(|| {
            ai_error(
                "ai_processing_chunk_offset_invalid",
                "Processing chunk offset is outside its segment.",
            )
        })?;
    let slice_duration_ms = chunk_end_offset_ms
        .checked_sub(chunk_start_offset_ms)
        .filter(|duration| *duration > 0)
        .ok_or_else(|| {
            ai_error(
                "ai_processing_chunk_duration_invalid",
                "Processing chunk duration is invalid.",
            )
        })?;
    let temp_audio = TempAudioFile::new(processing_temp_audio_path(app, &chunk.job_id, &chunk.id)?);
    tracing::info!(
        recording_id = %chunk.recording_id,
        job_id = %chunk.job_id,
        chunk_id = %chunk.id,
        track_id = %track.id,
        source_role = %track.kind,
        stream_index,
        slice_start_ms,
        slice_duration_ms,
        "ai_processing_audio_extract_started"
    );
    extract_audio_stream(
        &segment.file_path,
        stream_index,
        slice_start_ms,
        slice_duration_ms,
        temp_audio.path(),
        &cancel,
    )?;
    let temp_audio_bytes = fs::metadata(temp_audio.path())
        .ok()
        .map(|metadata| metadata.len())
        .unwrap_or_default();
    tracing::info!(
        recording_id = %chunk.recording_id,
        job_id = %chunk.job_id,
        chunk_id = %chunk.id,
        track_id = %track.id,
        source_role = %track.kind,
        stream_index,
        slice_duration_ms,
        temp_audio_bytes,
        "ai_processing_audio_extract_completed"
    );
    let model = selected_transcription_model(&app_settings)?;
    let provider = app.state::<TranscriptionProviderManager>();
    let transcription_ranges = if app_settings.ai.transcription.silence_gate_enabled {
        whisper_backend::detect_speech_ranges(
            temp_audio.path(),
            whisper_backend::SilenceGateConfig {
                merge_gap_ms: i64::from(app_settings.ai.transcription.merge_gap_ms),
                min_speech_ms: i64::from(app_settings.ai.transcription.min_speech_ms),
                silence_threshold_db: app_settings.ai.transcription.silence_threshold_db,
            },
        )
        .map_err(whisper_backend_error)?
    } else {
        vec![whisper_backend::SpeechRange {
            end_ms: slice_duration_ms,
            start_ms: 0,
        }]
    }
    .into_iter()
    .filter(|range| range.end_ms > range.start_ms)
    .collect::<Vec<_>>();

    storage
        .with_connection(|connection| {
            storage::delete_transcript_output_for_chunks(
                connection,
                std::slice::from_ref(&chunk.id),
            )
        })
        .map_err(storage_error)?;

    if transcription_ranges.is_empty() {
        tracing::info!(
            recording_id = %chunk.recording_id,
            job_id = %chunk.job_id,
            chunk_id = %chunk.id,
            track_id = %track.id,
            source_role = %track.kind,
            silence_threshold_db = app_settings.ai.transcription.silence_threshold_db,
            min_speech_ms = app_settings.ai.transcription.min_speech_ms,
            "ai_processing_audio_chunk_skipped_silence"
        );
        return Ok(AudioChunkProcessingOutcome {
            result_code: Some(TRANSCRIPTION_RESULT_SKIPPED_SILENCE.to_string()),
        });
    }

    tracing::info!(
        chunk_id = %chunk.id,
        job_id = %chunk.job_id,
        model = %model,
        provider = %provider_name,
        backend = %app_settings.ai.transcription.local_backend,
        speech_range_count = transcription_ranges.len(),
        "AI transcription started."
    );
    let mut pending_transcriptions = Vec::with_capacity(transcription_ranges.len());
    for (range_index, range) in transcription_ranges.iter().enumerate() {
        check_atomic_cancel(&cancel)?;
        let mut range_audio = None;
        let audio_path = if range.start_ms == 0 && range.end_ms == slice_duration_ms {
            temp_audio.path().to_path_buf()
        } else {
            let range_duration_ms = range.end_ms.checked_sub(range.start_ms).ok_or_else(|| {
                ai_error(
                    "ai_processing_audio_range_invalid",
                    "Detected transcription range is invalid.",
                )
            })?;
            let range_path =
                processing_temp_audio_range_path(app, &chunk.job_id, &chunk.id, range_index)?;
            let temp_range_audio = TempAudioFile::new(range_path);
            extract_audio_stream(
                &segment.file_path,
                stream_index,
                slice_start_ms + range.start_ms,
                range_duration_ms,
                temp_range_audio.path(),
                &cancel,
            )?;
            let path = temp_range_audio.path().to_path_buf();
            range_audio = Some(temp_range_audio);
            path
        };
        let transcription = provider
            .transcribe(
                app,
                &storage,
                TranscriptionRequest {
                    audio_path,
                    cancel: cancel.clone(),
                    language: app_settings.ai.transcription.language.clone(),
                    max_initial_timestamp_seconds: app_settings
                        .ai
                        .transcription
                        .max_initial_timestamp_seconds,
                    model: model.clone(),
                    timeout: TRANSCRIPTION_TIMEOUT,
                    word_timestamps: app_settings.ai.transcription.word_timestamps_enabled,
                },
            )
            .await
            .map_err(transcription_error)?;
        drop(range_audio);
        tracing::info!(
            chunk_id = %chunk.id,
            job_id = %chunk.job_id,
            speech_range_index = range_index,
            speech_range_start_ms = range.start_ms,
            speech_range_end_ms = range.end_ms,
            segment_count = transcription.segments.len(),
            word_count = transcription.words.len(),
            text_char_count = transcription
                .segments
                .iter()
                .map(|segment| segment.text.trim().chars().count())
                .sum::<usize>(),
            "AI transcription range completed."
        );
        check_atomic_cancel(&cancel)?;
        pending_transcriptions.push(PendingTranscription {
            base_offset_ms: chunk_start_offset_ms + range.start_ms,
            transcription,
        });
    }
    check_atomic_cancel(&cancel)?;
    persist_transcriptions(
        app,
        chunk,
        &track,
        &provider_name,
        &model,
        pending_transcriptions,
    )?;

    Ok(AudioChunkProcessingOutcome { result_code: None })
}

async fn process_activity_chunk_inner(
    app: &AppHandle,
    chunk: &ProcessingChunkRow,
    cancel: Arc<AtomicBool>,
) -> Result<(), AiError> {
    let storage = app.state::<AppStorage>();
    let app_settings = settings::load_app_settings(&storage).map_err(settings_error)?;
    let chunk_start = chunk.start_offset_ms.ok_or_else(|| {
        ai_error(
            "ai_processing_chunk_offset_missing",
            "Processing chunk start offset is missing.",
        )
    })?;
    let chunk_end = chunk.end_offset_ms.ok_or_else(|| {
        ai_error(
            "ai_processing_chunk_offset_missing",
            "Processing chunk end offset is missing.",
        )
    })?;
    if chunk_end <= chunk_start {
        return Err(ai_error(
            "ai_processing_chunk_duration_invalid",
            "Processing chunk duration is invalid.",
        ));
    }

    check_processing_cancel(&cancel)?;
    let (segment, segment_playback_offset_ms, events) = storage
        .with_connection(|connection| {
            let segment_id = chunk
                .segment_id
                .as_deref()
                .ok_or(rusqlite::Error::InvalidQuery)?;
            let segments = storage::list_recording_segments(connection, &chunk.recording_id)?;
            let segment_playback_offset_ms = playback_offset_for_segment(&segments, segment_id)
                .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
            let segment = segments
                .into_iter()
                .find(|candidate| candidate.id == segment_id)
                .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
            let events = storage::list_capture_events(
                connection,
                &chunk.recording_id,
                &["keyboard-activity", "mouse-activity", "capture-state"],
                chunk_start,
                chunk_end,
            )?;
            Ok((segment, segment_playback_offset_ms, events))
        })
        .map_err(storage_error)?;

    let mut intervals = Vec::new();
    if app_settings.ai.activity.keyboard.enabled {
        intervals.extend(activity_intervals_from_events(
            "keyboard",
            "keyboard-activity",
            &events,
            chunk_start,
            chunk_end,
            app_settings.ai.activity.keyboard.window_ms.into(),
        ));
    }
    if app_settings.ai.activity.mouse.enabled {
        intervals.extend(activity_intervals_from_events(
            "mouse",
            "mouse-activity",
            &events,
            chunk_start,
            chunk_end,
            app_settings.ai.activity.mouse.window_ms.into(),
        ));
    }
    check_processing_cancel(&cancel)?;
    if app_settings.ai.activity.frame_deltas.enabled {
        intervals.extend(frame_delta_intervals(
            &segment,
            segment_playback_offset_ms,
            chunk_start,
            chunk_end,
            &cancel,
        )?);
    }
    if !intervals.is_empty() {
        intervals.extend(merge_activity_intervals(&intervals, chunk_start, chunk_end));
    }

    storage
        .with_connection(|connection| {
            storage::delete_activity_output_for_chunks(
                connection,
                std::slice::from_ref(&chunk.id),
            )?;
            for interval in intervals {
                storage::create_activity_interval(
                    connection,
                    &CreateActivityIntervalInput {
                        chunk_id: Some(chunk.id.clone()),
                        end_offset_ms: interval.end_offset_ms,
                        id: Uuid::now_v7().to_string(),
                        job_id: chunk.job_id.clone(),
                        recording_id: chunk.recording_id.clone(),
                        regions_json: interval.regions_json,
                        score: interval.score,
                        source_kind: interval.source_kind,
                        start_offset_ms: interval.start_offset_ms,
                        state: interval.state,
                    },
                )?;
            }
            Ok(())
        })
        .map_err(storage_error)?;

    Ok(())
}

#[derive(Clone, Debug, PartialEq)]
struct PlannedActivityInterval {
    end_offset_ms: i64,
    regions_json: Option<String>,
    score: Option<f64>,
    source_kind: String,
    start_offset_ms: i64,
    state: String,
}

fn activity_intervals_from_events(
    source_kind: &str,
    event_kind: &str,
    events: &[CaptureEventRow],
    chunk_start: i64,
    chunk_end: i64,
    window_ms: i64,
) -> Vec<PlannedActivityInterval> {
    if input_activity_unavailable(events) {
        return vec![PlannedActivityInterval {
            end_offset_ms: chunk_end,
            regions_json: None,
            score: None,
            source_kind: source_kind.to_string(),
            start_offset_ms: chunk_start,
            state: "unknown".to_string(),
        }];
    }
    let mut windows = events
        .iter()
        .filter(|event| event.kind == event_kind)
        .map(|event| (event.offset_ms, event.offset_ms.saturating_add(window_ms)))
        .collect::<Vec<_>>();
    windows.sort_unstable();
    intervals_from_active_windows(source_kind, chunk_start, chunk_end, windows)
}

fn intervals_from_active_windows(
    source_kind: &str,
    chunk_start: i64,
    chunk_end: i64,
    windows: Vec<(i64, i64)>,
) -> Vec<PlannedActivityInterval> {
    let mut intervals = Vec::new();
    let mut cursor = chunk_start;
    for (start, end) in merge_windows(windows, chunk_start, chunk_end) {
        if cursor < start {
            intervals.push(planned_interval(
                source_kind,
                "idle",
                cursor,
                start,
                Some(0.0),
                None,
            ));
        }
        intervals.push(planned_interval(
            source_kind,
            "active",
            start,
            end,
            Some(1.0),
            None,
        ));
        cursor = end;
    }
    if cursor < chunk_end {
        intervals.push(planned_interval(
            source_kind,
            "idle",
            cursor,
            chunk_end,
            Some(0.0),
            None,
        ));
    }
    if intervals.is_empty() {
        intervals.push(planned_interval(
            source_kind,
            "idle",
            chunk_start,
            chunk_end,
            Some(0.0),
            None,
        ));
    }
    intervals
}

fn merge_windows(windows: Vec<(i64, i64)>, chunk_start: i64, chunk_end: i64) -> Vec<(i64, i64)> {
    let mut merged = Vec::<(i64, i64)>::new();
    for (start, end) in windows {
        let start = start.max(chunk_start);
        let end = end.min(chunk_end);
        if end <= start {
            continue;
        }
        if let Some(last) = merged.last_mut() {
            if start <= last.1 {
                last.1 = last.1.max(end);
                continue;
            }
        }
        merged.push((start, end));
    }
    merged
}

fn frame_delta_intervals(
    segment: &RecordingSegmentRow,
    segment_playback_offset_ms: i64,
    chunk_start: i64,
    chunk_end: i64,
    cancel: &AtomicBool,
) -> Result<Vec<PlannedActivityInterval>, AiError> {
    let video_stream_index = segment.video_stream_index.ok_or_else(|| {
        ai_error(
            "ai_processing_video_stream_missing",
            "Video stream is missing.",
        )
    })?;
    let slice_start_ms = chunk_start
        .checked_sub(segment_playback_offset_ms)
        .ok_or_else(|| {
            ai_error(
                "ai_processing_chunk_offset_invalid",
                "Processing chunk offset is outside its segment.",
            )
        })?;
    let slice_duration_ms = chunk_end.saturating_sub(chunk_start);
    let frame_bytes = FRAME_DELTA_WIDTH * FRAME_DELTA_HEIGHT;
    let output = Command::new(ffmpeg::binary_path())
        .args([
            "-hide_banner",
            "-v",
            "error",
            "-ss",
            &format_seconds(slice_start_ms),
            "-t",
            &format_seconds(slice_duration_ms),
            "-i",
            &segment.file_path,
            "-map",
            &format!("0:{video_stream_index}"),
            "-vf",
            &format!(
                "fps=1,scale={}:{}:flags=fast_bilinear,format=gray",
                FRAME_DELTA_WIDTH, FRAME_DELTA_HEIGHT
            ),
            "-f",
            "rawvideo",
            "-",
        ])
        .output()
        .map_err(file_error)?;
    check_processing_cancel(cancel)?;
    if !output.status.success() {
        return Err(ai_error(
            "ai_processing_frame_delta_extract_failed",
            "Frame delta extraction failed.",
        ));
    }

    let frames = output
        .stdout
        .chunks_exact(frame_bytes)
        .map(|chunk| chunk.to_vec())
        .collect::<Vec<_>>();
    Ok(frame_intervals_from_frames(
        &frames,
        chunk_start,
        chunk_end,
        frame_bytes,
    ))
}

fn frame_intervals_from_frames(
    frames: &[Vec<u8>],
    chunk_start: i64,
    chunk_end: i64,
    frame_bytes: usize,
) -> Vec<PlannedActivityInterval> {
    if frames.is_empty() {
        return vec![planned_interval(
            "frame-delta",
            "unknown",
            chunk_start,
            chunk_end,
            None,
            None,
        )];
    }

    let mut intervals = Vec::new();
    for (index, frame) in frames.iter().enumerate() {
        let start = chunk_start + (index as i64 * FRAME_DELTA_SAMPLE_INTERVAL_MS);
        let end = (start + FRAME_DELTA_SAMPLE_INTERVAL_MS).min(chunk_end);
        if end <= start {
            continue;
        }
        let score = index
            .checked_sub(1)
            .and_then(|previous| frames.get(previous))
            .map(|previous| frame_delta_score(previous, frame, frame_bytes))
            .unwrap_or(0.0);
        let active = score >= FRAME_DELTA_THRESHOLD;
        intervals.push(planned_interval(
            "frame-delta",
            if active { "active" } else { "idle" },
            start,
            end,
            Some(score),
            if active {
                Some(r#"[{"x":0,"y":0,"width":1,"height":1}]"#.to_string())
            } else {
                None
            },
        ));
    }

    coalesce_activity_intervals(intervals)
}

fn frame_delta_score(previous: &[u8], current: &[u8], frame_bytes: usize) -> f64 {
    let changed = previous
        .iter()
        .zip(current.iter())
        .take(frame_bytes)
        .filter(|(left, right)| u8::abs_diff(**left, **right) > 12)
        .count();
    changed as f64 / frame_bytes as f64
}

fn merge_activity_intervals(
    source_intervals: &[PlannedActivityInterval],
    chunk_start: i64,
    chunk_end: i64,
) -> Vec<PlannedActivityInterval> {
    let mut boundaries = vec![chunk_start, chunk_end];
    for interval in source_intervals {
        if interval.source_kind == "merged" {
            continue;
        }
        boundaries.push(interval.start_offset_ms.max(chunk_start).min(chunk_end));
        boundaries.push(interval.end_offset_ms.max(chunk_start).min(chunk_end));
    }
    boundaries.sort_unstable();
    boundaries.dedup();

    let mut merged = Vec::new();
    for pair in boundaries.windows(2) {
        let start = pair[0];
        let end = pair[1];
        if end <= start {
            continue;
        }
        let midpoint = start + ((end - start) / 2);
        let states = source_intervals
            .iter()
            .filter(|interval| interval.source_kind != "merged")
            .filter(|interval| {
                interval.start_offset_ms <= midpoint && interval.end_offset_ms > midpoint
            })
            .map(|interval| interval.state.as_str())
            .collect::<Vec<_>>();
        if states.is_empty() {
            continue;
        }
        let state = if states.contains(&"active") {
            "active"
        } else if states.iter().all(|state| *state == "unknown") {
            "unknown"
        } else {
            "idle"
        };
        merged.push(planned_interval("merged", state, start, end, None, None));
    }

    coalesce_activity_intervals(merged)
}

fn coalesce_activity_intervals(
    intervals: Vec<PlannedActivityInterval>,
) -> Vec<PlannedActivityInterval> {
    let mut coalesced = Vec::<PlannedActivityInterval>::new();
    for interval in intervals {
        if let Some(last) = coalesced.last_mut() {
            if last.source_kind == interval.source_kind
                && last.state == interval.state
                && last.regions_json == interval.regions_json
            {
                last.end_offset_ms = interval.end_offset_ms;
                last.score = match (last.score, interval.score) {
                    (Some(left), Some(right)) => Some(left.max(right)),
                    (left, right) => left.or(right),
                };
                continue;
            }
        }
        coalesced.push(interval);
    }
    coalesced
}

fn planned_interval(
    source_kind: &str,
    state: &str,
    start_offset_ms: i64,
    end_offset_ms: i64,
    score: Option<f64>,
    regions_json: Option<String>,
) -> PlannedActivityInterval {
    PlannedActivityInterval {
        end_offset_ms,
        regions_json,
        score,
        source_kind: source_kind.to_string(),
        start_offset_ms,
        state: state.to_string(),
    }
}

fn input_activity_unavailable(events: &[CaptureEventRow]) -> bool {
    events
        .iter()
        .filter(|event| event.kind == "capture-state")
        .any(|event| {
            event
                .payload_json
                .contains("input-device-access-unavailable")
        })
}

fn spawn_job_cancel_monitor(
    app: AppHandle,
    job_id: String,
    cancel: Arc<AtomicBool>,
) -> tauri::async_runtime::JoinHandle<()> {
    tauri::async_runtime::spawn(async move {
        loop {
            if cancel.load(Ordering::SeqCst) {
                return;
            }
            let storage = app.state::<AppStorage>();
            let status = storage
                .with_connection(|connection| storage::get_processing_job(connection, &job_id))
                .ok()
                .flatten()
                .map(|job| job.status);
            if matches!(status.as_deref(), Some("paused" | "cancelled")) || status.is_none() {
                cancel.store(true, Ordering::SeqCst);
                return;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
}

fn processing_temp_audio_path(
    app: &AppHandle,
    job_id: &str,
    chunk_id: &str,
) -> Result<PathBuf, AiError> {
    let directory = processing_temp_audio_directory(app, job_id)?;
    Ok(directory.join(format!("{chunk_id}.wav")))
}

fn processing_temp_audio_range_path(
    app: &AppHandle,
    job_id: &str,
    chunk_id: &str,
    range_index: usize,
) -> Result<PathBuf, AiError> {
    let directory = processing_temp_audio_directory(app, job_id)?;
    Ok(directory.join(format!("{chunk_id}-{range_index}.wav")))
}

fn processing_temp_audio_directory(app: &AppHandle, job_id: &str) -> Result<PathBuf, AiError> {
    let directory = app
        .path()
        .app_data_dir()
        .map_err(|error| {
            ai_error(
                "ai_processing_app_data_unavailable",
                &format!("App data directory is unavailable: {error}"),
            )
        })?
        .join("processing")
        .join("transcription")
        .join(job_id);
    fs::create_dir_all(&directory).map_err(file_error)?;
    Ok(directory)
}

struct TempAudioFile {
    path: PathBuf,
}

impl TempAudioFile {
    fn new(path: PathBuf) -> Self {
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempAudioFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn extract_audio_stream(
    input_path: &str,
    stream_index: i64,
    slice_start_ms: i64,
    slice_duration_ms: i64,
    output_path: &Path,
    cancel: &AtomicBool,
) -> Result<(), AiError> {
    check_atomic_cancel(cancel)?;
    let _ = fs::remove_file(output_path);
    let output = Command::new(ffmpeg::binary_path())
        .args(ffmpeg_audio_extract_args(
            input_path,
            stream_index,
            slice_start_ms,
            slice_duration_ms,
        ))
        .arg(output_path)
        .output()
        .map_err(file_error)?;
    check_atomic_cancel(cancel)?;
    if !output.status.success() {
        return Err(ai_error(
            "ai_processing_audio_extract_failed",
            "Audio extraction failed.",
        ));
    }

    Ok(())
}

fn ffmpeg_audio_extract_args(
    input_path: &str,
    stream_index: i64,
    slice_start_ms: i64,
    slice_duration_ms: i64,
) -> Vec<String> {
    vec![
        "-hide_banner".to_string(),
        "-v".to_string(),
        "error".to_string(),
        "-y".to_string(),
        "-i".to_string(),
        input_path.to_string(),
        "-ss".to_string(),
        format_seconds(slice_start_ms),
        "-t".to_string(),
        format_seconds(slice_duration_ms),
        "-map".to_string(),
        format!("0:{stream_index}"),
        "-vn".to_string(),
        "-ac".to_string(),
        "1".to_string(),
        "-ar".to_string(),
        "16000".to_string(),
        "-c:a".to_string(),
        "pcm_s16le".to_string(),
        "-f".to_string(),
        "wav".to_string(),
    ]
}

struct PendingTranscription {
    base_offset_ms: i64,
    transcription: transcription::VerboseTranscription,
}

#[derive(Debug, Default)]
struct PersistedTranscriptionStats {
    segment_count: usize,
    text_char_count: usize,
    word_count: usize,
}

fn persist_transcriptions(
    app: &AppHandle,
    chunk: &ProcessingChunkRow,
    track: &RecordingTrackRow,
    provider: &str,
    model: &str,
    transcriptions: Vec<PendingTranscription>,
) -> Result<(), AiError> {
    if transcriptions
        .iter()
        .all(|pending| pending.transcription.segments.is_empty())
    {
        tracing::info!(
            recording_id = %chunk.recording_id,
            job_id = %chunk.job_id,
            chunk_id = %chunk.id,
            track_id = chunk.track_id.as_deref().unwrap_or(""),
            source_role = %track.kind,
            "ai_processing_transcript_persist_skipped_empty"
        );
        return Ok(());
    }
    let segment_id = chunk.segment_id.clone().ok_or_else(|| {
        ai_error(
            "ai_processing_segment_missing",
            "Processing segment is missing.",
        )
    })?;
    let storage = app.state::<AppStorage>();
    let stats = storage
        .with_connection_mut(|connection| {
            persist_transcriptions_for_chunk(
                connection,
                chunk,
                track,
                provider,
                model,
                &segment_id,
                transcriptions,
            )
        })
        .map_err(storage_error)?;
    if stats.segment_count == 0 {
        tracing::info!(
            recording_id = %chunk.recording_id,
            job_id = %chunk.job_id,
            chunk_id = %chunk.id,
            track_id = chunk.track_id.as_deref().unwrap_or(""),
            source_role = %track.kind,
            "ai_processing_transcript_persist_skipped_empty"
        );
        return Ok(());
    }
    tracing::info!(
        recording_id = %chunk.recording_id,
        job_id = %chunk.job_id,
        chunk_id = %chunk.id,
        track_id = %track.id,
        source_role = %track.kind,
        segment_count = stats.segment_count,
        word_count = stats.word_count,
        text_char_count = stats.text_char_count,
        "ai_processing_transcript_persisted"
    );
    Ok(())
}

fn persist_transcriptions_for_chunk(
    connection: &mut rusqlite::Connection,
    chunk: &ProcessingChunkRow,
    track: &RecordingTrackRow,
    provider: &str,
    model: &str,
    segment_id: &str,
    transcriptions: Vec<PendingTranscription>,
) -> rusqlite::Result<PersistedTranscriptionStats> {
    let transaction = connection.transaction()?;
    let mut stats = PersistedTranscriptionStats::default();

    for pending in transcriptions {
        for segment in pending.transcription.segments {
            let text = segment.text.trim().to_string();
            if text.is_empty() {
                continue;
            }
            let transcript_segment_id = Uuid::now_v7().to_string();
            let segment_words =
                transcription_words_for_segment(&segment, &pending.transcription.words);
            let (start_offset_ms, end_offset_ms) =
                transcript_segment_offsets_ms(pending.base_offset_ms, &segment, &segment_words);
            storage::create_transcript_segment(
                &transaction,
                &CreateTranscriptSegmentInput {
                    chunk_id: chunk.id.clone(),
                    confidence: None,
                    created_at_ms: now_ms(),
                    end_offset_ms,
                    id: transcript_segment_id.clone(),
                    job_id: chunk.job_id.clone(),
                    model: Some(model.to_string()),
                    provider: provider.to_string(),
                    recording_id: chunk.recording_id.clone(),
                    segment_id: segment_id.to_string(),
                    source_role: track.kind.clone(),
                    start_offset_ms,
                    text: text.clone(),
                    track_id: track.id.clone(),
                },
            )?;
            stats.segment_count += 1;
            stats.text_char_count += text.chars().count();
            for word in segment_words {
                storage::create_transcript_word(
                    &transaction,
                    &CreateTranscriptWordInput {
                        end_offset_ms: pending.base_offset_ms + seconds_to_ms(word.end),
                        id: Uuid::now_v7().to_string(),
                        probability: word.probability,
                        start_offset_ms: pending.base_offset_ms + seconds_to_ms(word.start),
                        text: word.word.trim().to_string(),
                        transcript_segment_id: transcript_segment_id.clone(),
                    },
                )?;
                stats.word_count += 1;
            }
        }
    }

    transaction.commit()?;
    Ok(stats)
}

fn transcription_words_for_segment<'a>(
    segment: &transcription::VerboseTranscriptionSegment,
    words: &'a [transcription::VerboseTranscriptionWord],
) -> Vec<&'a transcription::VerboseTranscriptionWord> {
    words
        .iter()
        .filter(|word| {
            word.start >= segment.start
                && word.end <= segment.end
                && word.end > word.start
                && word.start.is_finite()
                && word.end.is_finite()
                && !word.word.trim().is_empty()
        })
        .collect()
}

fn transcript_segment_offsets_ms(
    base_offset_ms: i64,
    segment: &transcription::VerboseTranscriptionSegment,
    words: &[&transcription::VerboseTranscriptionWord],
) -> (i64, i64) {
    let start_seconds = words
        .iter()
        .map(|word| word.start)
        .reduce(f64::min)
        .unwrap_or(segment.start);
    let end_seconds = words
        .iter()
        .map(|word| word.end)
        .reduce(f64::max)
        .unwrap_or(segment.end);
    let start_offset_ms = base_offset_ms + seconds_to_ms(start_seconds);
    let end_offset_ms = base_offset_ms + seconds_to_ms(end_seconds);

    (start_offset_ms, end_offset_ms.max(start_offset_ms))
}

fn finish_processing_job(
    app: &AppHandle,
    storage: &AppStorage,
    job_id: &str,
) -> Result<(), AiError> {
    let mut recording_id = None;
    storage
        .with_connection(|connection| {
            let chunks = storage::list_processing_chunks(connection, job_id)?;
            let chunk_status_counts = processing_chunk_status_counts(&chunks);
            let ProcessingCompletion::Terminal { error, status } =
                summarize_processing_completion(&chunks)
            else {
                tracing::info!(
                    job_id = %job_id,
                    chunk_status_counts = %chunk_status_counts,
                    "ai_processing_job_completion_deferred_active_chunks"
                );
                return Ok(());
            };
            let job =
                storage::update_processing_job_status(connection, job_id, status, error, now_ms())?;
            tracing::info!(
                recording_id = %job.recording_id,
                job_id = %job_id,
                job_status = %status,
                error_code = error.unwrap_or(""),
                chunk_status_counts = %chunk_status_counts,
                "ai_processing_job_completion_decided"
            );
            recording_id = Some(job.recording_id.clone());
            let (notification_status, title, message, actions) = match status {
                "succeeded" => (
                    "succeeded",
                    "AI processing complete",
                    "Audio transcription completed for this recording.",
                    Vec::new(),
                ),
                "cancelled" => (
                    "cancelled",
                    "AI processing cancelled",
                    "Audio transcription was cancelled.",
                    processing_retry_actions(job_id),
                ),
                _ => (
                    "failed",
                    "AI processing failed",
                    "Audio transcription failed.",
                    processing_failed_actions(job_id),
                ),
            };
            save_processing_notification(
                connection,
                ProcessingNotificationInput {
                    actions: &actions,
                    job_id: Some(job_id),
                    message,
                    recording_id: &job.recording_id,
                    status: notification_status,
                    timestamp_ms: now_ms(),
                    title,
                },
            )
            .map(|_row| ())
        })
        .map_err(storage_error)?;
    if let Some(recording_id) = recording_id {
        emit_processing_state(app, &recording_id);
    }
    Ok(())
}

enum ProcessingCompletion<'a> {
    Active,
    Terminal {
        error: Option<&'a str>,
        status: &'static str,
    },
}

fn summarize_processing_completion(chunks: &[ProcessingChunkRow]) -> ProcessingCompletion<'_> {
    if chunks
        .iter()
        .any(|chunk| matches!(chunk.status.as_str(), "queued" | "running"))
    {
        return ProcessingCompletion::Active;
    }

    if let Some(chunk) = chunks.iter().find(|chunk| chunk.status == "failed") {
        return ProcessingCompletion::Terminal {
            error: chunk.error_code.as_deref(),
            status: "failed",
        };
    }

    if let Some(chunk) = chunks.iter().find(|chunk| chunk.status == "cancelled") {
        return ProcessingCompletion::Terminal {
            error: chunk.error_code.as_deref(),
            status: "cancelled",
        };
    }

    ProcessingCompletion::Terminal {
        error: None,
        status: "succeeded",
    }
}

fn fail_processing_job(
    app: &AppHandle,
    storage: &AppStorage,
    job_id: &str,
    error_code: &str,
) -> Result<(), AiError> {
    let mut recording_id = None;
    storage
        .with_connection(|connection| {
            let job = storage::get_processing_job(connection, job_id)?
                .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
            recording_id = Some(job.recording_id.clone());
            storage::cancel_processing_chunks(
                connection,
                job_id,
                &["queued", "running"],
                now_ms(),
            )?;
            storage::update_processing_job_status(
                connection,
                job_id,
                "failed",
                Some(error_code),
                now_ms(),
            )?;
            save_processing_notification(
                connection,
                ProcessingNotificationInput {
                    actions: &processing_failed_actions(job_id),
                    job_id: Some(job_id),
                    message: &format!("Audio transcription failed: {error_code}."),
                    recording_id: &job.recording_id,
                    status: "failed",
                    timestamp_ms: now_ms(),
                    title: "AI processing failed",
                },
            )
            .map(|_row| ())
        })
        .map_err(storage_error)?;
    if let Some(recording_id) = recording_id {
        emit_processing_state(app, &recording_id);
    }
    Ok(())
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
                        ModelDownloadNotificationInput {
                            actions: &[model_download_retry_action(catalog.id)],
                            message: &format!("{} download failed: {}.", catalog.label, error.code),
                            model_id: catalog.id,
                            preferred_notification_id: None,
                            status: "failed",
                            timestamp_ms,
                            title: "Model download failed",
                        },
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
    let app_settings = settings::load_app_settings(&storage).map_err(settings_error)?;
    let warmup = LocalTranscriptionWarmup {
        local_backend: app_settings.ai.transcription.local_backend,
        model_id: catalog.id.to_string(),
        model_path: final_path.to_path_buf(),
    };
    let provider = app.state::<TranscriptionProviderManager>().inner().clone();
    let self_test_error =
        match tauri::async_runtime::spawn_blocking(move || provider.warm_local_model(warmup)).await
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
                ModelDownloadNotificationInput {
                    actions: &[],
                    message: &notification_message,
                    model_id: catalog.id,
                    preferred_notification_id: None,
                    status: notification_status,
                    timestamp_ms,
                    title: notification_title,
                },
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
                ModelDownloadNotificationInput {
                    actions: &[model_download_retry_action(catalog.id)],
                    message: &format!("{} download was cancelled.", catalog.label),
                    model_id: catalog.id,
                    preferred_notification_id: None,
                    status: "cancelled",
                    timestamp_ms,
                    title: "Model download cancelled",
                },
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
    downloaded: Option<i64>,
    timestamp_ms: i64,
) -> rusqlite::Result<NotificationRow> {
    if cancelled {
        let existing = ModelAssetPatch::from_existing(connection, catalog)?;
        save_catalog_model(
            connection,
            catalog,
            ModelAssetPatch {
                download_status: "cancelled".into(),
                downloaded_bytes: downloaded.unwrap_or(existing.downloaded_bytes),
                error_code: Some("ai_model_download_cancelled".into()),
                verification_status: "unknown".into(),
                ..existing
            },
            timestamp_ms,
        )?;
        save_model_download_notification(
            connection,
            ModelDownloadNotificationInput {
                actions: &[model_download_retry_action(catalog.id)],
                message: &format!("{} download was cancelled.", catalog.label),
                model_id: catalog.id,
                preferred_notification_id,
                status: "cancelled",
                timestamp_ms,
                title: "Model download cancelled",
            },
        )
    } else {
        save_model_download_notification(
            connection,
            ModelDownloadNotificationInput {
                actions: &[model_download_retry_action(catalog.id)],
                message: &format!(
                    "{} has no active or queued download to cancel.",
                    catalog.label
                ),
                model_id: catalog.id,
                preferred_notification_id,
                status: "warning",
                timestamp_ms,
                title: "Model download not running",
            },
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

fn transcript_for_recording(
    storage: &AppStorage,
    recording_id: &str,
) -> Result<RecordingTranscript, AiError> {
    let segments = visible_transcript_segments_for_recording(storage, recording_id)?;
    let mut words_by_segment = BTreeMap::new();
    for segment in &segments {
        let words = storage
            .with_connection(|connection| storage::list_transcript_words(connection, &segment.id))
            .map_err(storage_error)?;
        words_by_segment.insert(segment.id.clone(), words);
    }
    Ok(transcript_from_rows(
        recording_id,
        segments,
        words_by_segment,
    ))
}

pub(crate) fn visible_transcript_segments_for_recording(
    storage: &AppStorage,
    recording_id: &str,
) -> Result<Vec<TranscriptSegmentRow>, AiError> {
    let (segments, redactions) = storage
        .with_connection(|connection| {
            Ok((
                storage::list_transcript_segments(connection, recording_id)?,
                storage::list_redactions(connection, recording_id)?,
            ))
        })
        .map_err(storage_error)?;

    Ok(filter_visible_transcript_segments(segments, &redactions))
}

fn filter_visible_transcript_segments(
    segments: Vec<TranscriptSegmentRow>,
    redactions: &[RedactionRow],
) -> Vec<TranscriptSegmentRow> {
    if redactions.is_empty() {
        return segments;
    }

    segments
        .into_iter()
        .filter(|segment| {
            !redactions.iter().any(|redaction| {
                storage::redaction_applies_to_track_range(
                    redaction,
                    &segment.track_id,
                    segment.start_offset_ms,
                    segment.end_offset_ms,
                )
            })
        })
        .collect()
}

pub(crate) fn processing_state_for_recording(
    storage: &AppStorage,
    recording_id: &str,
) -> Result<RecordingAiProcessingState, AiError> {
    let job_row = storage
        .with_connection(|connection| {
            storage::get_latest_processing_job_for_recording(
                connection,
                recording_id,
                AI_PROCESSING_JOB_KIND,
            )
        })
        .map_err(storage_error)?;
    let chunk_rows = if let Some(job) = &job_row {
        storage
            .with_connection(|connection| storage::list_processing_chunks(connection, &job.id))
            .map_err(storage_error)?
    } else {
        Vec::new()
    };
    let transcript = transcript_for_recording(storage, recording_id)?;
    let recording_segments = storage
        .with_connection(|connection| storage::list_recording_segments(connection, recording_id))
        .map_err(storage_error)?;
    let recording_tracks = storage
        .with_connection(|connection| storage::list_recording_tracks(connection, recording_id))
        .map_err(storage_error)?;
    let chunks = chunk_rows
        .iter()
        .cloned()
        .map(processing_chunk_from_row)
        .collect::<Vec<_>>();
    let transcription_tracks = transcription_tracks_for_recording(
        recording_id,
        &recording_segments,
        &recording_tracks,
        &chunks,
        &transcript,
    );

    Ok(RecordingAiProcessingState {
        chunks,
        job: job_row.map(|job| processing_job_from_row_with_storage(storage, job)),
        recording_id: recording_id.to_string(),
        transcript,
        transcription_tracks,
    })
}

fn transcription_tracks_for_recording(
    _recording_id: &str,
    recording_segments: &[RecordingSegmentRow],
    recording_tracks: &[RecordingTrackRow],
    chunks: &[ProcessingChunk],
    transcript: &RecordingTranscript,
) -> Vec<RecordingTranscriptionTrackState> {
    let track_roles_by_id = recording_tracks
        .iter()
        .map(|track| (track.id.clone(), track.kind.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut source_roles = audio_source_roles_from_segments(recording_segments);
    for role in chunks.iter().filter_map(|chunk| {
        chunk
            .track_id
            .as_ref()
            .and_then(|track_id| track_roles_by_id.get(track_id))
            .cloned()
    }) {
        source_roles.insert(role);
    }
    for role in transcript
        .tracks
        .iter()
        .map(|track| track.source_role.clone())
    {
        source_roles.insert(role);
    }

    source_roles
        .into_iter()
        .filter_map(|source_role| {
            if !matches!(source_role.as_str(), "desktop-audio" | "microphone") {
                return None;
            }
            let track_id = recording_tracks
                .iter()
                .find(|track| track.kind == source_role)
                .map(|track| track.id.clone());
            let library_track_id = library_audio_track_id(recording_segments, &source_role);
            let transcript_segments = transcript
                .tracks
                .iter()
                .filter(|track| track.source_role == source_role)
                .flat_map(|track| track.segments.clone())
                .collect::<Vec<_>>();
            let track_chunks = chunks
                .iter()
                .filter(|chunk| {
                    chunk.source_kind == "audio"
                        && chunk.track_id.as_ref().is_some_and(|chunk_track_id| {
                            track_roles_by_id
                                .get(chunk_track_id)
                                .is_some_and(|role| role == &source_role)
                        })
                })
                .map(|chunk| transcription_chunk_state(chunk, &transcript_segments))
                .collect::<Vec<_>>();
            let (status, status_label) =
                transcription_track_status(&track_chunks, !library_track_id.is_some());

            Some(RecordingTranscriptionTrackState {
                chunks: track_chunks,
                description: transcription_track_description(
                    &source_role,
                    library_track_id.as_deref(),
                ),
                label: transcription_track_label(&source_role).to_string(),
                library_track_id,
                source_role: source_role.clone(),
                status,
                status_label,
                text_segment_count: transcript_segments.len(),
                track_id,
                track_key: source_role,
            })
        })
        .collect()
}

fn audio_source_roles_from_segments(segments: &[RecordingSegmentRow]) -> BTreeSet<String> {
    let mut roles = BTreeSet::new();
    for segment in segments {
        if segment.desktop_audio_stream_index.is_some() {
            roles.insert("desktop-audio".to_string());
        }
        if segment.microphone_audio_stream_index.is_some() {
            roles.insert("microphone".to_string());
        }
    }
    roles
}

fn library_audio_track_id(segments: &[RecordingSegmentRow], source_role: &str) -> Option<String> {
    let stream_index = segments.iter().find_map(|segment| match source_role {
        "desktop-audio" => segment.desktop_audio_stream_index,
        "microphone" => segment.microphone_audio_stream_index,
        _ => None,
    })?;
    Some(format!("{source_role}-{stream_index}"))
}

fn transcription_chunk_state(
    chunk: &ProcessingChunk,
    transcript_segments: &[RecordingTranscriptSegment],
) -> RecordingTranscriptionChunkState {
    let chunk_start_ms = chunk.start_offset_ms.unwrap_or(0);
    let chunk_end_ms = chunk.end_offset_ms.unwrap_or(chunk_start_ms);
    let chunk_segments = transcript_segments
        .iter()
        .filter(|segment| {
            segment.start_offset_ms < chunk_end_ms && segment.end_offset_ms > chunk_start_ms
        })
        .cloned()
        .collect::<Vec<_>>();
    let text = chunk_segments
        .iter()
        .map(|segment| segment.text.trim())
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    let result = match chunk.status.as_str() {
        "queued" => "queued",
        "running" => "running",
        "failed" => "failed",
        "cancelled" => "cancelled",
        "succeeded" if chunk.result_code.as_deref() == Some("skipped-silence") => "skipped-silence",
        "succeeded" if text.is_empty() => "no-speech",
        "succeeded" => "text",
        _ => "unknown",
    }
    .to_string();

    RecordingTranscriptionChunkState {
        attempt_count: chunk.attempt_count,
        end_offset_ms: chunk.end_offset_ms,
        error_code: chunk.error_code.clone(),
        id: chunk.id.clone(),
        progress: chunk.progress,
        result,
        result_code: chunk.result_code.clone(),
        segment_id: chunk.segment_id.clone(),
        start_offset_ms: chunk.start_offset_ms,
        status: chunk.status.clone(),
        text: (!text.is_empty()).then_some(text),
        transcript_segments: chunk_segments,
    }
}

fn transcription_track_status(
    chunks: &[RecordingTranscriptionChunkState],
    missing_audio_source: bool,
) -> (String, String) {
    if missing_audio_source {
        return ("no-audio".to_string(), "No audio source".to_string());
    }
    if chunks.is_empty() {
        return ("ready".to_string(), "Ready to process".to_string());
    }
    if chunks
        .iter()
        .any(|chunk| matches!(chunk.result.as_str(), "queued" | "running"))
    {
        return (
            "in-progress".to_string(),
            "Transcription in progress".to_string(),
        );
    }
    if chunks.iter().any(|chunk| chunk.result == "failed") {
        return ("failed".to_string(), "Transcription failed".to_string());
    }
    if chunks.iter().any(|chunk| chunk.result == "cancelled") {
        return (
            "cancelled".to_string(),
            "Transcription cancelled".to_string(),
        );
    }
    if chunks.iter().any(|chunk| chunk.result == "text") {
        return ("complete".to_string(), "Transcription complete".to_string());
    }
    ("no-speech".to_string(), "No speech detected".to_string())
}

fn transcription_track_label(source_role: &str) -> &str {
    match source_role {
        "desktop-audio" => "Desktop audio transcription",
        "microphone" => "Microphone transcription",
        _ => "Audio transcription",
    }
}

fn transcription_track_description(source_role: &str, library_track_id: Option<&str>) -> String {
    match library_track_id {
        Some(track_id) => format!("{source_role} timeline track {track_id}"),
        None => format!("{source_role} has no library audio track"),
    }
}

fn transcript_from_rows(
    recording_id: &str,
    segments: Vec<TranscriptSegmentRow>,
    words_by_segment: BTreeMap<String, Vec<TranscriptWordRow>>,
) -> RecordingTranscript {
    let mut tracks: BTreeMap<(String, String), Vec<RecordingTranscriptSegment>> = BTreeMap::new();
    for segment in segments {
        tracks
            .entry((segment.track_id.clone(), segment.source_role.clone()))
            .or_default()
            .push(RecordingTranscriptSegment {
                confidence: segment.confidence,
                end_offset_ms: segment.end_offset_ms,
                id: segment.id.clone(),
                model: segment.model,
                provider: segment.provider,
                segment_id: segment.segment_id,
                start_offset_ms: segment.start_offset_ms,
                text: segment.text,
                words: words_by_segment
                    .get(&segment.id)
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .map(transcript_word_from_row)
                    .collect(),
            });
    }

    RecordingTranscript {
        recording_id: recording_id.to_string(),
        tracks: tracks
            .into_iter()
            .map(
                |((track_id, source_role), segments)| RecordingTranscriptTrack {
                    source_role,
                    track_id,
                    segments,
                },
            )
            .collect(),
    }
}

fn processing_chunk_from_row(row: ProcessingChunkRow) -> ProcessingChunk {
    ProcessingChunk {
        attempt_count: row.attempt_count,
        created_at_ms: row.created_at_ms,
        end_offset_ms: row.end_offset_ms,
        error_code: row.error_code,
        id: row.id,
        job_id: row.job_id,
        progress: row.progress,
        recording_id: row.recording_id,
        result_code: row.result_code,
        segment_id: row.segment_id,
        source_kind: row.source_kind,
        start_offset_ms: row.start_offset_ms,
        status: row.status,
        track_id: row.track_id,
        updated_at_ms: row.updated_at_ms,
    }
}

fn transcript_word_from_row(row: TranscriptWordRow) -> RecordingTranscriptWord {
    RecordingTranscriptWord {
        end_offset_ms: row.end_offset_ms,
        id: row.id,
        probability: row.probability,
        start_offset_ms: row.start_offset_ms,
        text: row.text,
    }
}

fn activity_for_recording(
    storage: &AppStorage,
    recording_id: &str,
) -> Result<RecordingActivity, AiError> {
    let intervals = storage
        .with_connection(|connection| storage::list_activity_intervals(connection, recording_id))
        .map_err(storage_error)?;
    Ok(RecordingActivity {
        intervals: intervals
            .into_iter()
            .map(activity_interval_from_row)
            .collect(),
        recording_id: recording_id.to_string(),
    })
}

fn activity_interval_from_row(row: ActivityIntervalRow) -> ActivityInterval {
    ActivityInterval {
        end_offset_ms: row.end_offset_ms,
        id: row.id,
        job_id: row.job_id,
        recording_id: row.recording_id,
        score: row.score,
        source_kind: row.source_kind,
        start_offset_ms: row.start_offset_ms,
        state: row.state,
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

fn processing_job_from_row_with_storage(
    storage: &AppStorage,
    row: ProcessingJobRow,
) -> ProcessingJob {
    let progress = storage
        .with_connection(|connection| storage::processing_job_chunk_progress(connection, &row.id))
        .ok()
        .map(|(total, succeeded, average)| {
            if total == 0 {
                match row.status.as_str() {
                    "succeeded" => 1.0,
                    "running" => 0.5,
                    _ => 0.0,
                }
            } else if succeeded == total {
                1.0
            } else {
                average
            }
        })
        .unwrap_or(0.0);
    ProcessingJob {
        progress,
        ..processing_job_from_row(row)
    }
}

fn update_processing_job_for_action(
    storage: &AppStorage,
    job_id: &str,
    status: &str,
) -> Result<ProcessingJob, AiError> {
    if status == "queued" {
        ensure_no_running_processing_chunks(
            storage,
            job_id,
            "ai_processing_resume_pending",
            "AI processing cannot resume until the active chunk has paused.",
        )?;
    }

    storage
        .with_connection(|connection| {
            match status {
                "paused" => {}
                "cancelled" => {
                    storage::cancel_processing_chunks(connection, job_id, &["queued"], now_ms())?;
                }
                "queued" => {
                    storage::requeue_processing_chunks(
                        connection,
                        job_id,
                        &["cancelled"],
                        now_ms(),
                    )?;
                }
                _ => {}
            }
            storage::update_processing_job_status(connection, job_id, status, None, now_ms())
        })
        .map_err(storage_error)
        .map(|row| processing_job_from_row_with_storage(storage, row))
}

fn retry_processing_job_for_action(
    storage: &AppStorage,
    job_id: &str,
) -> Result<ProcessingJob, AiError> {
    ensure_no_running_processing_chunks(
        storage,
        job_id,
        "ai_processing_retry_pending",
        "AI processing cannot retry until active chunks have stopped.",
    )?;

    storage
        .with_connection(|connection| {
            storage::retry_processing_job_chunks(connection, job_id, now_ms())
        })
        .map_err(storage_error)
        .map(|row| processing_job_from_row_with_storage(storage, row))
}

fn ensure_no_running_processing_chunks(
    storage: &AppStorage,
    job_id: &str,
    code: &str,
    message: &str,
) -> Result<(), AiError> {
    let running = storage
        .with_connection(|connection| {
            storage::list_processing_chunks_by_status(connection, job_id, &["running"], Some(1))
        })
        .map_err(storage_error)?;
    if running.is_empty() {
        Ok(())
    } else {
        Err(ai_error(code, message))
    }
}

fn check_transcription_readiness<R: tauri::Runtime>(
    app: &AppHandle<R>,
    storage: &AppStorage,
    app_settings: &settings::AppSettings,
) -> Result<(), AiError> {
    match app_settings.ai.transcription.provider.as_str() {
        "local-whisper-cpp" => {
            let model_id = &app_settings.ai.transcription.local_model_id;
            let model = storage
                .with_connection(|connection| storage::get_ai_model_asset(connection, model_id))
                .map_err(storage_error)?
                .ok_or_else(|| {
                    ai_error(
                        "ai_processing_local_model_unavailable",
                        "Selected local transcription model has not been downloaded.",
                    )
                })?;
            let Some(local_path) = model.local_path.as_deref() else {
                return Err(ai_error(
                    "ai_processing_local_model_unavailable",
                    "Selected local transcription model has no local file.",
                ));
            };
            if model.download_status != "available" || !Path::new(local_path).is_file() {
                return Err(ai_error(
                    "ai_processing_local_model_unavailable",
                    "Selected local transcription model file is not available.",
                ));
            }
            let _ = app.path().app_data_dir().map_err(|error| {
                ai_error(
                    "ai_processing_app_data_unavailable",
                    &format!("App data directory is unavailable: {error}"),
                )
            })?;
            Ok(())
        }
        "openai-compatible" => {
            if app_settings
                .ai
                .transcription
                .remote_base_url
                .as_deref()
                .is_none_or(str::is_empty)
            {
                return Err(ai_error(
                    "ai_processing_remote_base_url_required",
                    "Remote transcription base URL is required.",
                ));
            }
            if app_settings
                .ai
                .transcription
                .remote_api_key_ref
                .as_deref()
                .is_none_or(str::is_empty)
            {
                return Err(ai_error(
                    "ai_processing_remote_api_key_required",
                    "Remote transcription API key is required.",
                ));
            }
            if app_settings
                .ai
                .transcription
                .remote_model
                .as_deref()
                .is_none_or(str::is_empty)
            {
                return Err(ai_error(
                    "ai_processing_remote_model_required",
                    "Remote transcription model is required.",
                ));
            }
            Ok(())
        }
        _ => Err(ai_error(
            "ai_processing_transcription_provider_invalid",
            "Transcription provider is not supported.",
        )),
    }
}

fn selected_transcription_model(app_settings: &settings::AppSettings) -> Result<String, AiError> {
    match app_settings.ai.transcription.provider.as_str() {
        "local-whisper-cpp" => Ok(app_settings.ai.transcription.local_model_id.clone()),
        "openai-compatible" => app_settings
            .ai
            .transcription
            .remote_model
            .clone()
            .filter(|model| !model.trim().is_empty())
            .ok_or_else(|| {
                ai_error(
                    "ai_processing_remote_model_required",
                    "Remote transcription model is required.",
                )
            }),
        _ => Err(ai_error(
            "ai_processing_transcription_provider_invalid",
            "Transcription provider is not supported.",
        )),
    }
}

fn save_readiness_failure(
    storage: &AppStorage,
    recording_id: &str,
    code: &str,
    message: &str,
) -> AiError {
    let notification_message = format!("{message} Recording: {recording_id}.");
    let _ = storage.with_connection(|connection| {
        save_processing_notification(
            connection,
            ProcessingNotificationInput {
                actions: &[],
                job_id: None,
                message: &notification_message,
                recording_id,
                status: "warning",
                timestamp_ms: now_ms(),
                title: "AI processing unavailable",
            },
        )
        .map(|_row| ())
    });
    ai_error(code, message)
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

fn is_verified_model_file(path: &Path, catalog: &CatalogModel) -> Result<bool, AiError> {
    Ok(path.is_file() && sha1_file(path)? == catalog.sha1)
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

fn emit_processing_state<R: tauri::Runtime>(app: &AppHandle<R>, recording_id: &str) {
    let storage = app.state::<AppStorage>();
    if let Ok(state) = processing_state_for_recording(&storage, recording_id) {
        tracing::info!(
            recording_id = %recording_id,
            job_id = state.job.as_ref().map(|job| job.id.as_str()).unwrap_or(""),
            job_status = state.job.as_ref().map(|job| job.status.as_str()).unwrap_or("none"),
            chunk_status_counts = %processing_state_chunk_status_counts(&state.chunks),
            transcript_track_count = state.transcript.tracks.len(),
            "ai_processing_state_emitted"
        );
        let _ = app.emit(AI_PROCESSING_CHANGED_EVENT, state);
    }
}

fn processing_chunk_status_counts(chunks: &[ProcessingChunkRow]) -> String {
    let mut counts = BTreeMap::<String, usize>::new();
    for chunk in chunks {
        *counts.entry(chunk.status.clone()).or_default() += 1;
    }
    serde_json::to_string(&counts).unwrap_or_else(|_error| "{}".to_string())
}

fn processing_state_chunk_status_counts(chunks: &[ProcessingChunk]) -> String {
    let mut counts = BTreeMap::<String, usize>::new();
    for chunk in chunks {
        *counts.entry(chunk.status.clone()).or_default() += 1;
    }
    serde_json::to_string(&counts).unwrap_or_else(|_error| "{}".to_string())
}

fn storage_error(error: rusqlite::Error) -> AiError {
    AiError {
        code: "ai_storage_failed".to_string(),
        message: format!("AI storage failed: {error}"),
    }
}

fn settings_error(error: settings::AppSettingsError) -> AiError {
    AiError {
        code: error.code,
        message: error.message,
    }
}

fn transcription_error(error: transcription::TranscriptionError) -> AiError {
    AiError {
        code: error.code,
        message: error.message,
    }
}

fn whisper_backend_error(error: whisper_backend::WhisperBackendError) -> AiError {
    AiError {
        code: error.code,
        message: error.message,
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

fn ai_error(code: &str, message: &str) -> AiError {
    AiError {
        code: code.to_string(),
        message: message.to_string(),
    }
}

fn check_atomic_cancel(cancel: &AtomicBool) -> Result<(), AiError> {
    if cancel.load(Ordering::SeqCst) {
        return Err(ai_error(
            "ai_transcription_cancelled",
            "Transcription request was cancelled.",
        ));
    }
    Ok(())
}

fn check_processing_cancel(cancel: &AtomicBool) -> Result<(), AiError> {
    if cancel.load(Ordering::SeqCst) {
        return Err(ai_error(
            "ai_processing_cancelled",
            "AI processing was cancelled.",
        ));
    }
    Ok(())
}

fn seconds_to_ms(seconds: f64) -> i64 {
    (seconds * 1000.0).round().max(0.0) as i64
}

fn format_seconds(milliseconds: i64) -> String {
    format!("{:.3}", milliseconds.max(0) as f64 / 1000.0)
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
    use std::path::{Path, PathBuf};

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
    fn transcript_segment_offsets_use_word_bounds_when_available() {
        let segment = transcription::VerboseTranscriptionSegment {
            end: 5.0,
            id: 0,
            start: 0.0,
            text: " test test ".to_string(),
        };
        let words = vec![
            transcription::VerboseTranscriptionWord {
                end: 4.1,
                probability: Some(0.9),
                start: 3.8,
                word: "test".to_string(),
            },
            transcription::VerboseTranscriptionWord {
                end: 4.8,
                probability: Some(0.8),
                start: 4.2,
                word: "test".to_string(),
            },
        ];
        let segment_words = transcription_words_for_segment(&segment, &words);

        assert_eq!(
            transcript_segment_offsets_ms(1_000, &segment, &segment_words),
            (4_800, 5_800)
        );
    }

    #[test]
    fn transcript_segment_offsets_fall_back_to_segment_without_words() {
        let segment = transcription::VerboseTranscriptionSegment {
            end: 2.5,
            id: 0,
            start: 0.5,
            text: "fallback".to_string(),
        };

        assert_eq!(
            transcript_segment_offsets_ms(1_000, &segment, &[]),
            (1_500, 3_500)
        );
    }

    #[test]
    fn transcription_words_for_segment_ignores_invalid_or_empty_words() {
        let segment = transcription::VerboseTranscriptionSegment {
            end: 5.0,
            id: 0,
            start: 0.0,
            text: "ok".to_string(),
        };
        let words = vec![
            transcription::VerboseTranscriptionWord {
                end: 1.2,
                probability: None,
                start: 1.0,
                word: "ok".to_string(),
            },
            transcription::VerboseTranscriptionWord {
                end: 2.0,
                probability: None,
                start: 2.0,
                word: "zero".to_string(),
            },
            transcription::VerboseTranscriptionWord {
                end: 3.5,
                probability: None,
                start: 3.0,
                word: " ".to_string(),
            },
            transcription::VerboseTranscriptionWord {
                end: 6.0,
                probability: None,
                start: 5.5,
                word: "outside".to_string(),
            },
        ];

        let segment_words = transcription_words_for_segment(&segment, &words);

        assert_eq!(segment_words.len(), 1);
        assert_eq!(segment_words[0].word, "ok");
    }

    #[test]
    fn persist_transcriptions_for_chunk_commits_all_completed_ranges() {
        let storage = storage::test_storage(PathBuf::new());

        storage
            .with_connection_mut(|connection| {
                let (chunk, track) = test_transcription_chunk(connection);
                let stats = persist_transcriptions_for_chunk(
                    connection,
                    &chunk,
                    &track,
                    "local-whisper-cpp",
                    "small",
                    chunk.segment_id.as_deref().expect("chunk segment id"),
                    vec![
                        PendingTranscription {
                            base_offset_ms: 1_000,
                            transcription: test_verbose_transcription(
                                0.5,
                                1.0,
                                " first ",
                                vec![test_transcription_word(0.6, 0.8, "first", Some(0.8))],
                            ),
                        },
                        PendingTranscription {
                            base_offset_ms: 5_000,
                            transcription: test_verbose_transcription(
                                0.25,
                                0.75,
                                "second",
                                vec![test_transcription_word(0.3, 0.7, "second", Some(0.9))],
                            ),
                        },
                    ],
                )
                .expect("persist completed ranges");

                let segments = storage::list_transcript_segments(connection, &chunk.recording_id)?;
                let words = segments
                    .iter()
                    .map(|segment| storage::list_transcript_words(connection, &segment.id))
                    .collect::<rusqlite::Result<Vec<_>>>()?
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>();

                assert_eq!(stats.segment_count, 2);
                assert_eq!(stats.word_count, 2);
                assert_eq!(segments.len(), 2);
                assert_eq!(segments[0].text, "first");
                assert_eq!(segments[0].start_offset_ms, 1_600);
                assert_eq!(segments[0].end_offset_ms, 1_800);
                assert_eq!(segments[1].text, "second");
                assert_eq!(segments[1].start_offset_ms, 5_300);
                assert_eq!(segments[1].end_offset_ms, 5_700);
                assert_eq!(words.len(), 2);

                Ok(())
            })
            .expect("persist transcriptions");
    }

    #[test]
    fn transcript_queries_hide_redacted_track_overlaps_without_deleting_rows() {
        let storage = storage::test_storage(PathBuf::new());
        let (recording_id, redacted_segment_id) = storage
            .with_connection_mut(|connection| {
                let (chunk, track) = test_transcription_chunk(connection);
                let other_track_id = Uuid::now_v7().to_string();
                storage::upsert_recording_track(
                    connection,
                    &UpsertRecordingTrackInput {
                        codec: Some("pcm_s16le".to_string()),
                        created_at_ms: 1,
                        id: other_track_id.clone(),
                        kind: "desktop-audio".to_string(),
                        label: "Desktop audio".to_string(),
                        path: None,
                        recording_id: chunk.recording_id.clone(),
                        source_id: None,
                    },
                )?;
                storage::create_redaction(
                    connection,
                    &storage::CreateRedactionInput {
                        created_at_ms: 20,
                        end_offset_ms: 2_000,
                        id: Uuid::now_v7().to_string(),
                        reason: None,
                        recording_id: chunk.recording_id.clone(),
                        start_offset_ms: 1_000,
                        track_ids_json: serde_json::to_string(&vec![track.id.clone()])
                            .expect("serialize track ids"),
                    },
                )?;

                let redacted_segment = storage::create_transcript_segment(
                    connection,
                    &CreateTranscriptSegmentInput {
                        chunk_id: chunk.id.clone(),
                        confidence: Some(0.9),
                        created_at_ms: 30,
                        end_offset_ms: 1_500,
                        id: Uuid::now_v7().to_string(),
                        job_id: chunk.job_id.clone(),
                        model: Some("small".to_string()),
                        provider: "local-whisper-cpp".to_string(),
                        recording_id: chunk.recording_id.clone(),
                        segment_id: chunk.segment_id.clone().expect("segment id"),
                        source_role: "microphone".to_string(),
                        start_offset_ms: 500,
                        text: "hidden words".to_string(),
                        track_id: track.id.clone(),
                    },
                )?;
                storage::create_transcript_word(
                    connection,
                    &CreateTranscriptWordInput {
                        end_offset_ms: 800,
                        id: Uuid::now_v7().to_string(),
                        probability: Some(0.8),
                        start_offset_ms: 700,
                        text: "hidden".to_string(),
                        transcript_segment_id: redacted_segment.id.clone(),
                    },
                )?;
                storage::create_transcript_segment(
                    connection,
                    &CreateTranscriptSegmentInput {
                        chunk_id: chunk.id.clone(),
                        confidence: Some(0.8),
                        created_at_ms: 31,
                        end_offset_ms: 1_500,
                        id: Uuid::now_v7().to_string(),
                        job_id: chunk.job_id.clone(),
                        model: Some("small".to_string()),
                        provider: "local-whisper-cpp".to_string(),
                        recording_id: chunk.recording_id.clone(),
                        segment_id: chunk.segment_id.clone().expect("segment id"),
                        source_role: "desktop-audio".to_string(),
                        start_offset_ms: 500,
                        text: "visible other track".to_string(),
                        track_id: other_track_id,
                    },
                )?;
                storage::create_transcript_segment(
                    connection,
                    &CreateTranscriptSegmentInput {
                        chunk_id: chunk.id.clone(),
                        confidence: Some(0.7),
                        created_at_ms: 32,
                        end_offset_ms: 3_000,
                        id: Uuid::now_v7().to_string(),
                        job_id: chunk.job_id,
                        model: Some("small".to_string()),
                        provider: "local-whisper-cpp".to_string(),
                        recording_id: chunk.recording_id.clone(),
                        segment_id: chunk.segment_id.expect("segment id"),
                        source_role: "microphone".to_string(),
                        start_offset_ms: 2_000,
                        text: "visible after redaction".to_string(),
                        track_id: track.id,
                    },
                )?;

                Ok((chunk.recording_id, redacted_segment.id))
            })
            .expect("seed transcript and redaction");

        let visible_segments = visible_transcript_segments_for_recording(&storage, &recording_id)
            .expect("visible transcript segments");
        let transcript = transcript_for_recording(&storage, &recording_id).expect("transcript");
        let stored_segments = storage
            .with_connection(|connection| {
                storage::list_transcript_segments(connection, &recording_id)
            })
            .expect("stored transcript segments");

        assert_eq!(stored_segments.len(), 3);
        assert!(stored_segments
            .iter()
            .any(|segment| segment.id == redacted_segment_id));
        let visible_text = visible_segments
            .iter()
            .map(|segment| segment.text.as_str())
            .collect::<Vec<_>>();
        assert_eq!(visible_text.len(), 2);
        assert!(visible_text.contains(&"visible other track"));
        assert!(visible_text.contains(&"visible after redaction"));

        let transcript_text = transcript
            .tracks
            .iter()
            .flat_map(|track| track.segments.iter())
            .map(|segment| segment.text.as_str())
            .collect::<Vec<_>>();
        assert_eq!(transcript_text.len(), 2);
        assert!(transcript_text.contains(&"visible other track"));
        assert!(transcript_text.contains(&"visible after redaction"));
        assert!(transcript
            .tracks
            .iter()
            .flat_map(|track| track.segments.iter())
            .flat_map(|segment| segment.words.iter())
            .all(|word| word.text != "hidden"));
    }

    #[test]
    fn persist_transcriptions_for_chunk_rolls_back_failed_range_batch() {
        let storage = storage::test_storage(PathBuf::new());

        storage
            .with_connection_mut(|connection| {
                let (chunk, track) = test_transcription_chunk(connection);
                let error = persist_transcriptions_for_chunk(
                    connection,
                    &chunk,
                    &track,
                    "local-whisper-cpp",
                    "small",
                    chunk.segment_id.as_deref().expect("chunk segment id"),
                    vec![
                        PendingTranscription {
                            base_offset_ms: 1_000,
                            transcription: test_verbose_transcription(
                                0.0,
                                1.0,
                                "would otherwise persist",
                                vec![],
                            ),
                        },
                        PendingTranscription {
                            base_offset_ms: 2_000,
                            transcription: test_verbose_transcription(
                                0.0,
                                1.0,
                                "invalid word probability",
                                vec![test_transcription_word(0.1, 0.2, "bad", Some(2.0))],
                            ),
                        },
                    ],
                )
                .expect_err("invalid word probability should fail");

                assert!(matches!(error, rusqlite::Error::SqliteFailure(_, Some(_))));
                let segment_count: i64 = connection.query_row(
                    "SELECT COUNT(*) FROM transcript_segments WHERE chunk_id = ?1",
                    [&chunk.id],
                    |row| row.get(0),
                )?;
                let word_count: i64 = connection.query_row(
                    r#"
                    SELECT COUNT(*)
                    FROM transcript_words
                    WHERE transcript_segment_id IN (
                        SELECT id FROM transcript_segments WHERE chunk_id = ?1
                    )
                    "#,
                    [&chunk.id],
                    |row| row.get(0),
                )?;

                assert_eq!(segment_count, 0);
                assert_eq!(word_count, 0);

                Ok(())
            })
            .expect("rollback checked");
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
    fn processing_completion_stays_active_until_chunks_are_terminal() {
        for active_status in ["queued", "running"] {
            let chunks = vec![test_processing_chunk(active_status)];

            assert!(matches!(
                summarize_processing_completion(&chunks),
                ProcessingCompletion::Active
            ));
        }
    }

    #[test]
    fn processing_completion_succeeds_only_after_all_chunks_succeed() {
        let chunks = vec![
            test_processing_chunk("succeeded"),
            test_processing_chunk("succeeded"),
        ];

        assert!(matches!(
            summarize_processing_completion(&chunks),
            ProcessingCompletion::Terminal {
                error: None,
                status: "succeeded"
            }
        ));
    }

    #[test]
    fn transcription_chunk_state_preserves_skipped_silence_result() {
        let mut chunk = test_processing_chunk("succeeded");
        chunk.result_code = Some(TRANSCRIPTION_RESULT_SKIPPED_SILENCE.to_string());

        let state = transcription_chunk_state(&processing_chunk_from_row(chunk), &[]);

        assert_eq!(state.result, TRANSCRIPTION_RESULT_SKIPPED_SILENCE);
        assert_eq!(
            state.result_code.as_deref(),
            Some(TRANSCRIPTION_RESULT_SKIPPED_SILENCE)
        );
        assert_eq!(state.text, None);
    }

    #[test]
    fn activity_events_expand_to_active_window_without_private_payload() {
        let events = vec![CaptureEventRow {
            id: "event-1".to_string(),
            kind: "keyboard-activity".to_string(),
            occurred_at_ms: 1_000,
            offset_ms: 1_000,
            payload_json: r#"{"source":"evdev","bucketMs":500}"#.to_string(),
            recording_id: "recording-1".to_string(),
        }];

        let intervals = activity_intervals_from_events(
            "keyboard",
            "keyboard-activity",
            &events,
            0,
            5_000,
            2_000,
        );

        assert_eq!(
            intervals
                .iter()
                .map(|interval| (
                    interval.state.as_str(),
                    interval.start_offset_ms,
                    interval.end_offset_ms
                ))
                .collect::<Vec<_>>(),
            vec![
                ("idle", 0, 1_000),
                ("active", 1_000, 3_000),
                ("idle", 3_000, 5_000)
            ]
        );
        assert!(!events[0].payload_json.contains("KEY_"));
        assert!(!events[0].payload_json.contains("BTN_"));
    }

    #[test]
    fn unavailable_input_source_produces_unknown_interval() {
        let events = vec![CaptureEventRow {
            id: "event-1".to_string(),
            kind: "capture-state".to_string(),
            occurred_at_ms: 0,
            offset_ms: 0,
            payload_json: r#"{"reason":"input-device-access-unavailable"}"#.to_string(),
            recording_id: "recording-1".to_string(),
        }];

        let intervals =
            activity_intervals_from_events("mouse", "mouse-activity", &events, 0, 5_000, 1_000);

        assert_eq!(intervals.len(), 1);
        assert_eq!(intervals[0].state, "unknown");
        assert_eq!(intervals[0].start_offset_ms, 0);
        assert_eq!(intervals[0].end_offset_ms, 5_000);
    }

    #[test]
    fn frame_delta_scores_changed_pixels() {
        let previous = vec![0; 100];
        let mut current = vec![0; 100];
        current.iter_mut().take(10).for_each(|pixel| *pixel = 255);

        assert_eq!(frame_delta_score(&previous, &current, 100), 0.1);
    }

    #[test]
    fn merged_activity_prefers_active_then_idle_over_unknown() {
        let intervals = vec![
            planned_interval("keyboard", "unknown", 0, 5_000, None, None),
            planned_interval("frame-delta", "idle", 0, 2_000, Some(0.0), None),
            planned_interval("frame-delta", "active", 2_000, 3_000, Some(1.0), None),
            planned_interval("frame-delta", "idle", 3_000, 5_000, Some(0.0), None),
        ];

        let merged = merge_activity_intervals(&intervals, 0, 5_000);

        assert_eq!(
            merged
                .iter()
                .map(|interval| (
                    interval.state.as_str(),
                    interval.start_offset_ms,
                    interval.end_offset_ms
                ))
                .collect::<Vec<_>>(),
            vec![
                ("idle", 0, 2_000),
                ("active", 2_000, 3_000),
                ("idle", 3_000, 5_000)
            ]
        );
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
                    save_cancel_download_result(connection, catalog, None, false, None, 20)?;

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
    fn cancel_persisted_download_records_partial_bytes() {
        let storage = storage::test_storage(PathBuf::new());
        let catalog = &LOCAL_WHISPER_CATALOG[2];

        storage
            .with_connection(|connection| {
                save_catalog_model(
                    connection,
                    catalog,
                    ModelAssetPatch {
                        download_status: "downloading".into(),
                        downloaded_bytes: 10,
                        total_bytes: Some(catalog.size_bytes),
                        verification_status: "pending".into(),
                        ..ModelAssetPatch::missing()
                    },
                    10,
                )?;

                let notification =
                    save_cancel_download_result(connection, catalog, None, true, Some(42), 20)?;

                assert_eq!(notification.status, "cancelled");
                assert_eq!(notification.title, "Model download cancelled");

                let model = storage::get_ai_model_asset(connection, catalog.id)?
                    .expect("model asset remains saved");
                assert_eq!(model.download_status, "cancelled");
                assert_eq!(model.downloaded_bytes, 42);
                assert_eq!(
                    model.error_code.as_deref(),
                    Some("ai_model_download_cancelled")
                );

                Ok(())
            })
            .expect("cancel persisted download");
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
    fn recovery_uses_verified_final_model_file() {
        let dir = test_temp_path("recovery-final");
        fs::create_dir_all(&dir).expect("create temp dir");
        let final_path = dir.join("model.bin");
        let partial_path = dir.join("model.part");
        let bytes = b"complete model bytes";
        fs::write(&final_path, bytes).expect("write final");
        let sha1 = Box::leak(test_sha1(bytes).into_boxed_str());
        let catalog = CatalogModel {
            id: "test",
            label: "Test model",
            sha1,
            size_bytes: bytes.len() as i64,
        };

        assert_eq!(
            recover_model_files(&catalog, &partial_path, &final_path).expect("recover files"),
            RecoveredModelFiles::VerifiedFinal
        );

        delete_file_if_exists(&final_path).expect("delete final");
    }

    #[test]
    fn recovery_deletes_corrupt_final_and_resumes_partial() {
        let dir = test_temp_path("recovery-corrupt-final");
        fs::create_dir_all(&dir).expect("create temp dir");
        let final_path = dir.join("model.bin");
        let partial_path = dir.join("model.part");
        fs::write(&final_path, b"corrupt").expect("write final");
        fs::write(&partial_path, b"partial").expect("write partial");
        let sha1 = Box::leak(test_sha1(b"expected").into_boxed_str());
        let catalog = CatalogModel {
            id: "test",
            label: "Test model",
            sha1,
            size_bytes: 100,
        };

        assert_eq!(
            recover_model_files(&catalog, &partial_path, &final_path).expect("recover files"),
            RecoveredModelFiles::Partial(7)
        );
        assert!(!final_path.exists());

        delete_file_if_exists(&partial_path).expect("delete partial");
    }

    #[test]
    fn recovery_reports_no_files_when_partial_is_missing() {
        let dir = test_temp_path("recovery-missing");
        fs::create_dir_all(&dir).expect("create temp dir");
        let final_path = dir.join("model.bin");
        let partial_path = dir.join("model.part");
        let sha1 = Box::leak(test_sha1(b"expected").into_boxed_str());
        let catalog = CatalogModel {
            id: "test",
            label: "Test model",
            sha1,
            size_bytes: 100,
        };

        assert_eq!(
            recover_model_files(&catalog, &partial_path, &final_path).expect("recover files"),
            RecoveredModelFiles::None
        );
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
    fn audio_chunk_plans_split_long_segments_by_source_and_preserve_pause_gap() {
        let dir = test_temp_path("chunk-plans");
        fs::create_dir_all(&dir).expect("create temp dir");
        let segment_1_path = dir.join("segment-1.mkv");
        let segment_2_path = dir.join("segment-2.mkv");
        fs::write(&segment_1_path, b"segment 1").expect("write segment 1");
        fs::write(&segment_2_path, b"segment 2").expect("write segment 2");
        let segment_1_duration_ms = (31 * 60 * 1_000) as i64;
        let segment_2_duration_ms = 5 * 60 * 1_000;
        let pause_after_segment_1_ms = 2 * 60 * 1_000;
        let segment_1 = test_segment(TestSegmentInput {
            desktop_audio_stream_index: Some(1),
            duration_ms: segment_1_duration_ms,
            id: "segment-1",
            microphone_audio_stream_index: Some(2),
            path: &segment_1_path,
            segment_index: 1,
            started_at_ms: 1_000,
            state: "finalized",
        });
        let segment_2 = test_segment(TestSegmentInput {
            desktop_audio_stream_index: Some(1),
            duration_ms: segment_2_duration_ms,
            id: "segment-2",
            microphone_audio_stream_index: None,
            path: &segment_2_path,
            segment_index: 2,
            started_at_ms: segment_1.ended_at_ms.expect("segment 1 ended")
                + pause_after_segment_1_ms,
            state: "finalized",
        });
        let failed_segment = test_segment(TestSegmentInput {
            desktop_audio_stream_index: Some(1),
            duration_ms: segment_2_duration_ms,
            id: "failed-segment",
            microphone_audio_stream_index: None,
            path: &segment_2_path,
            segment_index: 3,
            started_at_ms: segment_2.ended_at_ms.expect("segment 2 ended"),
            state: "failed",
        });

        let plans = audio_chunk_plans(&[segment_1.clone(), segment_2.clone(), failed_segment]);
        let desktop_segment_1 = plans
            .iter()
            .filter(|plan| plan.segment_id == "segment-1" && plan.source_role == "desktop-audio")
            .collect::<Vec<_>>();
        let microphone_segment_1 = plans
            .iter()
            .filter(|plan| plan.segment_id == "segment-1" && plan.source_role == "microphone")
            .collect::<Vec<_>>();
        let desktop_segment_2 = plans
            .iter()
            .filter(|plan| plan.segment_id == "segment-2" && plan.source_role == "desktop-audio")
            .collect::<Vec<_>>();

        assert_eq!(desktop_segment_1.len(), 3);
        assert_eq!(microphone_segment_1.len(), 3);
        assert_eq!(desktop_segment_1[0].start_offset_ms, 0);
        assert_eq!(desktop_segment_1[0].end_offset_ms, 15 * 60 * 1_000);
        assert_eq!(desktop_segment_1[2].start_offset_ms, 30 * 60 * 1_000);
        assert_eq!(desktop_segment_1[2].end_offset_ms, 31 * 60 * 1_000);
        assert_eq!(desktop_segment_2.len(), 1);
        assert_eq!(
            desktop_segment_2[0].start_offset_ms,
            segment_1_duration_ms + pause_after_segment_1_ms
        );
        assert_eq!(
            plans
                .iter()
                .filter(|plan| plan.segment_id == "failed-segment")
                .count(),
            0
        );
        assert_eq!(
            playback_offset_for_segment(&[segment_1, segment_2], "segment-2"),
            Some(segment_1_duration_ms + pause_after_segment_1_ms)
        );
    }

    #[test]
    fn ffmpeg_audio_extract_args_include_slice_seek_and_duration() {
        let args = ffmpeg_audio_extract_args("/recording/segment.mkv", 2, 900_000, 123_456);

        assert_eq!(
            args,
            vec![
                "-hide_banner",
                "-v",
                "error",
                "-y",
                "-i",
                "/recording/segment.mkv",
                "-ss",
                "900.000",
                "-t",
                "123.456",
                "-map",
                "0:2",
                "-vn",
                "-ac",
                "1",
                "-ar",
                "16000",
                "-c:a",
                "pcm_s16le",
                "-f",
                "wav",
            ]
        );
    }

    #[test]
    fn temp_audio_file_removes_wav_on_drop() {
        let path = test_temp_path("temp-audio-cleanup").join("chunk.wav");
        fs::create_dir_all(path.parent().expect("temp parent")).expect("create temp dir");
        fs::write(&path, b"private audio").expect("write temp audio");

        {
            let _temp_audio = TempAudioFile::new(path.clone());
            assert!(path.exists());
        }

        assert!(!path.exists());
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
                    ModelDownloadNotificationInput {
                        actions: &[model_download_retry_action("small")],
                        message: "Whisper small download was cancelled.",
                        model_id: "small",
                        preferred_notification_id: Some(&started.id),
                        status: "cancelled",
                        timestamp_ms: 30,
                        title: "Model download cancelled",
                    },
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

    fn test_transcription_chunk(
        connection: &rusqlite::Connection,
    ) -> (ProcessingChunkRow, RecordingTrackRow) {
        let recording_id = Uuid::now_v7().to_string();
        let segment_id = Uuid::now_v7().to_string();
        let track_id = Uuid::now_v7().to_string();
        let job_id = Uuid::now_v7().to_string();
        let chunk_id = Uuid::now_v7().to_string();

        storage::create_recording(
            connection,
            &storage::CreateRecordingInput {
                active_scene_id: None,
                created_at_ms: 1,
                default_fps: 1.0,
                id: recording_id.clone(),
                notes: None,
                output_root: None,
                quality_label: "balanced-ocr".to_string(),
                storage_dir: "/tmp/daydream-test".to_string(),
            },
        )
        .expect("create recording");
        storage::create_recording_segment(
            connection,
            &storage::CreateRecordingSegmentInput {
                file_path: "/tmp/daydream-test/segments/0001.mkv".to_string(),
                id: segment_id.clone(),
                recording_id: recording_id.clone(),
                segment_index: 1,
                started_at_ms: 1,
            },
        )
        .expect("create segment");
        let track = storage::upsert_recording_track(
            connection,
            &UpsertRecordingTrackInput {
                codec: Some("pcm_s16le".to_string()),
                created_at_ms: 1,
                id: track_id.clone(),
                kind: "microphone".to_string(),
                label: "Microphone".to_string(),
                path: None,
                recording_id: recording_id.clone(),
                source_id: None,
            },
        )
        .expect("create track");
        storage::create_processing_job(
            connection,
            &CreateProcessingJobInput {
                created_at_ms: 1,
                error: None,
                id: job_id.clone(),
                kind: AI_PROCESSING_JOB_KIND.to_string(),
                recording_id: recording_id.clone(),
                status: "running".to_string(),
                updated_at_ms: 1,
            },
        )
        .expect("create job");
        let chunk = storage::create_processing_chunk(
            connection,
            &CreateProcessingChunkInput {
                attempt_count: 1,
                created_at_ms: 1,
                end_offset_ms: Some(10_000),
                error_code: None,
                id: chunk_id,
                job_id,
                progress: 0.0,
                recording_id,
                result_code: None,
                segment_id: Some(segment_id),
                source_kind: "audio".to_string(),
                start_offset_ms: Some(0),
                status: "running".to_string(),
                track_id: Some(track_id),
                updated_at_ms: 1,
            },
        )
        .expect("create chunk");

        (chunk, track)
    }

    fn test_verbose_transcription(
        start: f64,
        end: f64,
        text: &str,
        words: Vec<transcription::VerboseTranscriptionWord>,
    ) -> transcription::VerboseTranscription {
        transcription::VerboseTranscription {
            duration: Some(end),
            language: Some("en".to_string()),
            segments: vec![transcription::VerboseTranscriptionSegment {
                end,
                id: 0,
                start,
                text: text.to_string(),
            }],
            task: Some("transcribe".to_string()),
            text: text.to_string(),
            words,
        }
    }

    fn test_transcription_word(
        start: f64,
        end: f64,
        word: &str,
        probability: Option<f64>,
    ) -> transcription::VerboseTranscriptionWord {
        transcription::VerboseTranscriptionWord {
            end,
            probability,
            start,
            word: word.to_string(),
        }
    }

    fn test_temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("daydream-ai-{name}-{}", Uuid::now_v7()))
    }

    fn test_processing_chunk(status: &str) -> ProcessingChunkRow {
        ProcessingChunkRow {
            attempt_count: 0,
            created_at_ms: 1,
            end_offset_ms: Some(1_000),
            error_code: None,
            id: Uuid::now_v7().to_string(),
            job_id: Uuid::now_v7().to_string(),
            progress: if status == "succeeded" { 1.0 } else { 0.0 },
            recording_id: Uuid::now_v7().to_string(),
            result_code: None,
            segment_id: None,
            source_kind: "audio".to_string(),
            start_offset_ms: Some(0),
            status: status.to_string(),
            track_id: None,
            updated_at_ms: 1,
        }
    }

    struct TestSegmentInput<'a> {
        desktop_audio_stream_index: Option<i64>,
        duration_ms: i64,
        id: &'a str,
        microphone_audio_stream_index: Option<i64>,
        path: &'a Path,
        segment_index: i64,
        started_at_ms: i64,
        state: &'a str,
    }

    fn test_segment(input: TestSegmentInput<'_>) -> RecordingSegmentRow {
        RecordingSegmentRow {
            desktop_audio_stream_index: input.desktop_audio_stream_index,
            duration_ms: Some(input.duration_ms),
            ended_at_ms: Some(input.started_at_ms + input.duration_ms),
            error_code: None,
            ffmpeg_exit_code: Some(0),
            file_path: input.path.to_string_lossy().to_string(),
            id: input.id.to_string(),
            microphone_audio_stream_index: input.microphone_audio_stream_index,
            probe_json: None,
            recording_id: "recording-1".to_string(),
            segment_index: input.segment_index,
            started_at_ms: input.started_at_ms,
            state: input.state.to_string(),
            video_stream_index: Some(0),
        }
    }
}
