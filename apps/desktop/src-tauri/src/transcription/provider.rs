use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use tauri::{AppHandle, Manager};

use crate::{
    secret_crypto, settings,
    storage::{self, AppStorage},
};

use super::local_requests::transcribe_local_via_rest;
use super::local_server::{LocalTranscriptionEndpoint, LocalTranscriptionServer};
use super::local_worker::{
    LocalTranscriptionRunner, WhisperCppTranscriptionRunner, WhisperProcessTranscriptionRunner,
};
use super::remote::{transcribe_remote, RemoteTranscriptionRequest, RestTranscriptionRequest};
use super::{
    local_whisper_model_path, transcription_error, LocalTranscriptionWarmup, TranscriptionError,
    TranscriptionRequest, VerboseTranscription,
};

#[derive(Clone)]
pub struct TranscriptionProviderManager {
    inner: Arc<Mutex<TranscriptionProviderState>>,
    local_runner: Arc<dyn LocalTranscriptionRunner>,
}

#[derive(Default)]
struct TranscriptionProviderState {
    local_server: Option<LocalTranscriptionServer>,
}

impl TranscriptionProviderManager {
    pub fn new(app_data_dir: PathBuf) -> Result<Self, TranscriptionError> {
        Ok(Self::with_local_runner(Arc::new(
            WhisperProcessTranscriptionRunner::new(app_data_dir)?,
        )))
    }

    pub fn with_local_runner(local_runner: Arc<dyn LocalTranscriptionRunner>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(TranscriptionProviderState::default())),
            local_runner,
        }
    }

    pub async fn transcribe(
        &self,
        app: &AppHandle,
        storage: &AppStorage,
        request: TranscriptionRequest,
    ) -> Result<VerboseTranscription, TranscriptionError> {
        let app_settings =
            settings::load_app_settings(storage).map_err(|error| TranscriptionError {
                code: error.code,
                message: error.message,
            })?;
        match selected_transcription_provider(&app_settings)? {
            TranscriptionProviderKind::OpenAiCompatible => {
                let api_key = remote_transcription_api_key(
                    storage,
                    app_settings.ai.transcription.remote_api_key_ref.as_deref(),
                )?;
                let base_url = app_settings
                    .ai
                    .transcription
                    .remote_base_url
                    .as_deref()
                    .ok_or_else(|| {
                        transcription_error(
                            "ai_transcription_remote_unconfigured",
                            "Remote transcription endpoint is not configured.",
                        )
                    })?;
                transcribe_remote(RemoteTranscriptionRequest {
                    base_url,
                    api_key: &api_key,
                    audio_path: &request.audio_path,
                    cancel: Some(request.cancel),
                    language: &app_settings.ai.transcription.language,
                    model: &request.model,
                    timeout: request.timeout,
                    word_timestamps: request.word_timestamps,
                })
                .await
            }
            TranscriptionProviderKind::LocalWhisperCpp => {
                self.transcribe_local(app, request, &app_settings.ai.transcription.local_backend)
                    .await
            }
        }
    }

    pub async fn transcribe_local(
        &self,
        app: &AppHandle,
        request: TranscriptionRequest,
        local_backend: &str,
    ) -> Result<VerboseTranscription, TranscriptionError> {
        let endpoint = self.local_endpoint(app)?;
        transcribe_local_via_rest(
            RestTranscriptionRequest {
                local_request_id: None,
                local_scratch_owner: None,
                audio_path: &request.audio_path,
                bearer_token: Some(&endpoint.token),
                cancel: Some(request.cancel),
                endpoint: &endpoint.url,
                language: &request.language,
                local_backend: Some(local_backend),
                max_initial_timestamp_seconds: Some(request.max_initial_timestamp_seconds),
                model: &request.model,
                timeout: request.timeout,
                word_timestamps: request.word_timestamps,
            },
            request.processing_lease,
        )
        .await
    }

    pub fn warm_selected_local_model(
        &self,
        app: &AppHandle,
        storage: &AppStorage,
    ) -> Result<Option<LocalTranscriptionWarmup>, TranscriptionError> {
        let app_settings =
            settings::load_app_settings(storage).map_err(|error| TranscriptionError {
                code: error.code,
                message: error.message,
            })?;
        if selected_transcription_provider(&app_settings)?
            != TranscriptionProviderKind::LocalWhisperCpp
        {
            return Ok(None);
        }

        let app_data_dir = app.path().app_data_dir().map_err(|error| {
            transcription_error(
                "ai_transcription_app_data_unavailable",
                &format!("App data directory is unavailable: {error}"),
            )
        })?;
        let model_id = app_settings.ai.transcription.local_model_id;
        let model = storage
            .with_connection(|connection| storage::get_ai_model_asset(connection, &model_id))
            .map_err(storage_error)?
            .ok_or_else(|| {
                transcription_error(
                    "ai_transcription_local_model_unavailable",
                    "Selected local transcription model has not been downloaded.",
                )
            })?;
        if !is_local_model_ready_for_warmup(&model) {
            return Err(transcription_error(
                "ai_transcription_local_model_unavailable",
                "Selected local transcription model is not ready to load.",
            ));
        }
        let model_path = local_whisper_model_path(&app_data_dir, &model_id);
        if !model_path.is_file() {
            return Err(transcription_error(
                "ai_transcription_local_model_unavailable",
                "Selected local transcription model is not available.",
            ));
        }

        let warmup = LocalTranscriptionWarmup {
            local_backend: app_settings.ai.transcription.local_backend,
            model_id,
            model_path,
        };
        self.local_runner.warm_model(warmup.clone())?;

        Ok(Some(warmup))
    }

    pub fn warm_local_model(
        &self,
        input: LocalTranscriptionWarmup,
    ) -> Result<(), TranscriptionError> {
        self.local_runner.warm_model(input)
    }

    fn local_endpoint(
        &self,
        app: &AppHandle,
    ) -> Result<LocalTranscriptionEndpoint, TranscriptionError> {
        let mut state = self.inner.lock().expect("transcription provider lock");
        if state.local_server.is_none() {
            let app_data_dir = app.path().app_data_dir().map_err(|error| {
                transcription_error(
                    "ai_transcription_app_data_unavailable",
                    &format!("App data directory is unavailable: {error}"),
                )
            })?;
            state.local_server = Some(LocalTranscriptionServer::start(
                app_data_dir,
                self.local_runner.clone(),
            )?);
        }

        state
            .local_server
            .as_ref()
            .map(LocalTranscriptionServer::endpoint)
            .ok_or_else(|| {
                transcription_error(
                    "ai_transcription_local_server_unavailable",
                    "Local transcription endpoint is unavailable.",
                )
            })
    }
}

pub(super) fn is_local_model_ready_for_warmup(model: &storage::AiModelAssetRow) -> bool {
    model.download_status == "available" && model.verification_status == "passed"
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TranscriptionProviderKind {
    LocalWhisperCpp,
    OpenAiCompatible,
}

pub(super) fn selected_transcription_provider(
    app_settings: &settings::AppSettings,
) -> Result<TranscriptionProviderKind, TranscriptionError> {
    match app_settings.ai.transcription.provider.as_str() {
        "local-whisper-cpp" => Ok(TranscriptionProviderKind::LocalWhisperCpp),
        "openai-compatible" => Ok(TranscriptionProviderKind::OpenAiCompatible),
        _ => Err(transcription_error(
            "ai_transcription_provider_invalid",
            "Transcription provider is not supported.",
        )),
    }
}

impl Default for TranscriptionProviderManager {
    fn default() -> Self {
        Self::with_local_runner(Arc::new(WhisperCppTranscriptionRunner::new()))
    }
}

fn remote_transcription_api_key(
    storage: &AppStorage,
    secret_ref: Option<&str>,
) -> Result<String, TranscriptionError> {
    let secret_ref = secret_ref.ok_or_else(|| {
        transcription_error(
            "ai_transcription_remote_unconfigured",
            "Remote transcription API key is not configured.",
        )
    })?;
    let envelope = storage
        .with_connection(|connection| storage::get_app_setting(connection, secret_ref))
        .map_err(storage_error)?
        .ok_or_else(|| {
            transcription_error(
                "ai_transcription_remote_unconfigured",
                "Remote transcription API key is not configured.",
            )
        })?
        .value_json;
    secret_crypto::decrypt_sensitive_setting_with_platform_key(&envelope).map_err(|error| {
        TranscriptionError {
            code: error.code,
            message: error.message,
        }
    })
}

fn storage_error(error: rusqlite::Error) -> TranscriptionError {
    transcription_error(
        "ai_transcription_storage_failed",
        &format!("Transcription settings could not be loaded: {error}"),
    )
}
