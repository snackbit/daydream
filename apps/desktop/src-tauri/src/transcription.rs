#![allow(dead_code)]

use std::{
    collections::BTreeMap,
    fs,
    future::Future,
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use reqwest::multipart::{Form, Part};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use uuid::Uuid;

use crate::{
    secret_crypto, settings,
    storage::{self, AppStorage},
    whisper_backend,
};

const TRANSCRIPTION_PATH: &str = "/v1/audio/transcriptions";
const MAX_TRANSCRIPTION_UPLOAD_BYTES: u64 = 128 * 1024 * 1024;
const HTTP_HEADER_LIMIT_BYTES: usize = 32 * 1024;
const LOCAL_BACKEND_FORM_FIELD: &str = "daydream_local_backend";

#[derive(Clone)]
pub struct TranscriptionProviderManager {
    inner: Arc<Mutex<TranscriptionProviderState>>,
    local_runner: Arc<dyn LocalTranscriptionRunner>,
}

#[derive(Default)]
struct TranscriptionProviderState {
    local_server: Option<LocalTranscriptionServer>,
}

#[derive(Clone, Debug)]
pub struct TranscriptionRequest {
    pub audio_path: PathBuf,
    pub cancel: Arc<AtomicBool>,
    pub model: String,
    pub timeout: Duration,
    pub word_timestamps: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VerboseTranscription {
    pub duration: Option<f64>,
    pub language: Option<String>,
    pub segments: Vec<VerboseTranscriptionSegment>,
    pub task: Option<String>,
    pub text: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub words: Vec<VerboseTranscriptionWord>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VerboseTranscriptionSegment {
    pub end: f64,
    pub id: i64,
    pub start: f64,
    pub text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VerboseTranscriptionWord {
    pub end: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub probability: Option<f64>,
    pub start: f64,
    pub word: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptionError {
    pub code: String,
    pub message: String,
}

#[derive(Clone, Debug)]
pub struct LocalTranscriptionInput {
    pub audio_path: PathBuf,
    pub cancel: Arc<AtomicBool>,
    pub local_backend: String,
    pub model_id: String,
    pub model_path: PathBuf,
    pub word_timestamps: bool,
}

#[derive(Clone, Debug)]
pub struct LocalTranscriptionWarmup {
    pub local_backend: String,
    pub model_id: String,
    pub model_path: PathBuf,
}

pub trait LocalTranscriptionRunner: Send + Sync + 'static {
    fn warm_model(&self, input: LocalTranscriptionWarmup) -> Result<(), TranscriptionError> {
        let _input = input;
        Ok(())
    }

    fn transcribe(
        &self,
        input: LocalTranscriptionInput,
    ) -> Result<VerboseTranscription, TranscriptionError>;
}

struct WhisperCppTranscriptionRunner {
    sender: Mutex<mpsc::Sender<WhisperWorkerCommand>>,
    thread: Mutex<Option<JoinHandle<()>>>,
}

enum WhisperWorkerCommand {
    Stop,
    Transcribe {
        input: LocalTranscriptionInput,
        response: mpsc::Sender<Result<VerboseTranscription, TranscriptionError>>,
    },
    Warm {
        input: LocalTranscriptionWarmup,
        response: mpsc::Sender<Result<(), TranscriptionError>>,
    },
}

struct CachedWhisperModel {
    backend: whisper_backend::WhisperExecutionBackend,
    model: whisper_backend::LoadedWhisperModel,
    model_id: String,
    model_path: PathBuf,
}

impl WhisperCppTranscriptionRunner {
    fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        let thread = thread::Builder::new()
            .name("daydream-whisper-cache".to_string())
            .spawn(move || run_whisper_worker(receiver))
            .expect("spawn whisper cache worker");

        Self {
            sender: Mutex::new(sender),
            thread: Mutex::new(Some(thread)),
        }
    }

    fn send_command<T>(
        &self,
        build: impl FnOnce(mpsc::Sender<Result<T, TranscriptionError>>) -> WhisperWorkerCommand,
    ) -> Result<T, TranscriptionError> {
        let (response, receiver) = mpsc::channel();
        self.sender
            .lock()
            .expect("whisper worker sender")
            .send(build(response))
            .map_err(|_error| {
                transcription_error(
                    "ai_transcription_local_worker_unavailable",
                    "Local transcription worker is unavailable.",
                )
            })?;
        receiver.recv().map_err(|_error| {
            transcription_error(
                "ai_transcription_local_worker_unavailable",
                "Local transcription worker stopped before returning a result.",
            )
        })?
    }
}

impl Default for WhisperCppTranscriptionRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for WhisperCppTranscriptionRunner {
    fn drop(&mut self) {
        if let Ok(sender) = self.sender.lock() {
            let _ = sender.send(WhisperWorkerCommand::Stop);
        }
        if let Ok(mut thread) = self.thread.lock() {
            if let Some(thread) = thread.take() {
                let _ = thread.join();
            }
        }
    }
}

impl LocalTranscriptionRunner for WhisperCppTranscriptionRunner {
    fn warm_model(&self, input: LocalTranscriptionWarmup) -> Result<(), TranscriptionError> {
        self.send_command(|response| WhisperWorkerCommand::Warm { input, response })
    }

    fn transcribe(
        &self,
        input: LocalTranscriptionInput,
    ) -> Result<VerboseTranscription, TranscriptionError> {
        self.send_command(|response| WhisperWorkerCommand::Transcribe { input, response })
    }
}

fn run_whisper_worker(receiver: mpsc::Receiver<WhisperWorkerCommand>) {
    let mut cached_model: Option<CachedWhisperModel> = None;

    while let Ok(command) = receiver.recv() {
        match command {
            WhisperWorkerCommand::Stop => break,
            WhisperWorkerCommand::Warm { input, response } => {
                let result = ensure_cached_model(
                    &mut cached_model,
                    &input.model_id,
                    &input.model_path,
                    &input.local_backend,
                )
                .map(|_model| ());
                let _ = response.send(result);
            }
            WhisperWorkerCommand::Transcribe { input, response } => {
                let result = check_cancelled(Some(input.cancel.as_ref())).and_then(|()| {
                    ensure_cached_model(
                        &mut cached_model,
                        &input.model_id,
                        &input.model_path,
                        &input.local_backend,
                    )
                    .and_then(|model| {
                        model
                            .model
                            .transcribe_wav(&input.audio_path, input.word_timestamps, input.cancel)
                            .map_err(whisper_error)
                    })
                });
                let _ = response.send(result);
            }
        }
    }
}

fn ensure_cached_model<'a>(
    cached_model: &'a mut Option<CachedWhisperModel>,
    model_id: &str,
    model_path: &Path,
    local_backend: &str,
) -> Result<&'a mut CachedWhisperModel, TranscriptionError> {
    let backend = whisper_backend::WhisperExecutionBackend::from_setting(local_backend)
        .map_err(whisper_error)?;
    let needs_load = cached_model.as_ref().is_none_or(|cached| {
        cached.model_id != model_id || cached.model_path != model_path || cached.backend != backend
    });

    if needs_load {
        let model = whisper_backend::LoadedWhisperModel::load(model_path, backend)
            .map_err(whisper_error)?;
        *cached_model = Some(CachedWhisperModel {
            backend,
            model,
            model_id: model_id.to_string(),
            model_path: model_path.to_path_buf(),
        });
    }

    cached_model.as_mut().ok_or_else(|| {
        transcription_error(
            "ai_transcription_local_model_unavailable",
            "Local transcription model is not available.",
        )
    })
}

impl TranscriptionProviderManager {
    pub fn new() -> Self {
        Self::with_local_runner(Arc::new(WhisperCppTranscriptionRunner::new()))
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
                transcribe_remote(
                    base_url,
                    &api_key,
                    &request.audio_path,
                    &request.model,
                    request.word_timestamps,
                    request.timeout,
                    Some(request.cancel),
                )
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
        transcribe_via_rest(
            &endpoint.url,
            Some(&endpoint.token),
            &request.audio_path,
            &request.model,
            request.word_timestamps,
            Some(local_backend),
            request.timeout,
            Some(request.cancel),
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TranscriptionProviderKind {
    LocalWhisperCpp,
    OpenAiCompatible,
}

fn selected_transcription_provider(
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
        Self::new()
    }
}

#[derive(Clone, Debug)]
pub struct LocalTranscriptionEndpoint {
    pub address: SocketAddr,
    pub token: String,
    pub url: String,
}

#[derive(Debug)]
struct LocalTranscriptionServer {
    address: SocketAddr,
    shutdown: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    token: String,
}

impl LocalTranscriptionServer {
    fn start(
        app_data_dir: PathBuf,
        runner: Arc<dyn LocalTranscriptionRunner>,
    ) -> Result<Self, TranscriptionError> {
        let listener = TcpListener::bind(("127.0.0.1", 0)).map_err(local_bind_error)?;
        let address = listener.local_addr().map_err(local_bind_error)?;
        listener.set_nonblocking(true).map_err(local_bind_error)?;
        let shutdown = Arc::new(AtomicBool::new(false));
        let thread_shutdown = shutdown.clone();
        let token = Uuid::now_v7().to_string();
        let thread_token = token.clone();
        let thread = thread::Builder::new()
            .name("daydream-local-transcription".to_string())
            .spawn(move || {
                while !thread_shutdown.load(Ordering::SeqCst) {
                    match listener.accept() {
                        Ok((stream, _peer)) => {
                            let context = LocalTranscriptionContext {
                                app_data_dir: app_data_dir.clone(),
                                max_upload_bytes: MAX_TRANSCRIPTION_UPLOAD_BYTES,
                                runner: runner.clone(),
                                token: thread_token.clone(),
                            };
                            handle_local_transcription_stream(stream, context);
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            thread::sleep(Duration::from_millis(20));
                        }
                        Err(_error) => break,
                    }
                }
            })
            .map_err(local_bind_error)?;

        Ok(Self {
            address,
            shutdown,
            thread: Some(thread),
            token,
        })
    }

    fn endpoint(&self) -> LocalTranscriptionEndpoint {
        LocalTranscriptionEndpoint {
            address: self.address,
            token: self.token.clone(),
            url: format!("http://{}{}", self.address, TRANSCRIPTION_PATH),
        }
    }
}

impl Drop for LocalTranscriptionServer {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(self.address);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

#[derive(Clone)]
struct LocalTranscriptionContext {
    app_data_dir: PathBuf,
    max_upload_bytes: u64,
    runner: Arc<dyn LocalTranscriptionRunner>,
    token: String,
}

fn handle_local_transcription_stream(mut stream: TcpStream, context: LocalTranscriptionContext) {
    let result = read_http_request(&mut stream, context.max_upload_bytes).and_then(|request| {
        let disconnect_stream = stream.try_clone().map_err(network_io_error)?;
        handle_local_transcription_request(request, context, disconnect_stream)
    });
    let response = match result {
        Ok(transcription) => HttpResponse::json(200, &transcription),
        Err(error) => error_response(&error),
    };
    let _ = write_http_response(&mut stream, response);
}

fn handle_local_transcription_request(
    request: HttpRequest,
    context: LocalTranscriptionContext,
    disconnect_stream: TcpStream,
) -> Result<VerboseTranscription, TranscriptionError> {
    if request.method != "POST" || request.path != TRANSCRIPTION_PATH {
        return Err(transcription_error(
            "ai_transcription_route_not_found",
            "Local transcription route is not available.",
        ));
    }
    let authorization = request.headers.get("authorization").ok_or_else(|| {
        transcription_error(
            "ai_transcription_token_required",
            "Local transcription requires a bearer token.",
        )
    })?;
    if authorization != &format!("Bearer {}", context.token) {
        return Err(transcription_error(
            "ai_transcription_token_invalid",
            "Local transcription bearer token is invalid.",
        ));
    }
    let content_type = request.headers.get("content-type").ok_or_else(|| {
        transcription_error(
            "ai_transcription_request_invalid",
            "Transcription request content type is missing.",
        )
    })?;
    let boundary = multipart_boundary(content_type)?;
    let multipart = parse_multipart_form(&request.body, &boundary)?;
    let model_id = multipart.text("model")?;
    let response_format = multipart.text("response_format")?;
    let local_backend = multipart
        .optional_text(LOCAL_BACKEND_FORM_FIELD)
        .unwrap_or(settings::DEFAULT_AI_TRANSCRIPTION_BACKEND);
    if response_format != "verbose_json" {
        return Err(transcription_error(
            "ai_transcription_response_format_invalid",
            "Local transcription requires verbose_json response format.",
        ));
    }
    let word_timestamps = multipart
        .texts("timestamp_granularities[]")
        .iter()
        .any(|value| *value == "word");
    let file = multipart.file("file")?;
    let model_path = local_whisper_model_path(&context.app_data_dir, model_id);
    if !model_path.is_file() {
        return Err(transcription_error(
            "ai_transcription_local_model_unavailable",
            "Local transcription model is not available.",
        ));
    }

    let cancel = Arc::new(AtomicBool::new(false));
    let audio_path = temp_audio_path();
    fs::write(&audio_path, &file.data).map_err(file_error)?;
    let _disconnect_monitor =
        LocalTranscriptionCancelMonitor::start(disconnect_stream, cancel.clone());
    let result = context.runner.transcribe(LocalTranscriptionInput {
        audio_path: audio_path.clone(),
        cancel,
        local_backend: local_backend.to_string(),
        model_id: model_id.to_string(),
        model_path,
        word_timestamps,
    });
    let _ = fs::remove_file(audio_path);

    result
}

struct LocalTranscriptionCancelMonitor {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl LocalTranscriptionCancelMonitor {
    fn start(mut stream: TcpStream, cancel: Arc<AtomicBool>) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let thread = stream
            .set_read_timeout(Some(Duration::from_millis(50)))
            .ok()
            .and_then(|()| {
                let thread_stop = stop.clone();
                thread::Builder::new()
                    .name("daydream-local-transcription-cancel".to_string())
                    .spawn(move || {
                        let mut buffer = [0_u8; 1];
                        while !thread_stop.load(Ordering::SeqCst) {
                            match stream.read(&mut buffer) {
                                Ok(0) => {
                                    cancel.store(true, Ordering::SeqCst);
                                    break;
                                }
                                Ok(_read) => {}
                                Err(error)
                                    if matches!(
                                        error.kind(),
                                        std::io::ErrorKind::WouldBlock
                                            | std::io::ErrorKind::TimedOut
                                            | std::io::ErrorKind::Interrupted
                                    ) => {}
                                Err(_error) => {
                                    cancel.store(true, Ordering::SeqCst);
                                    break;
                                }
                            }
                        }
                    })
                    .ok()
            });

        Self { stop, thread }
    }
}

impl Drop for LocalTranscriptionCancelMonitor {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

pub async fn transcribe_remote(
    base_url: &str,
    api_key: &str,
    audio_path: &Path,
    model: &str,
    word_timestamps: bool,
    timeout: Duration,
    cancel: Option<Arc<AtomicBool>>,
) -> Result<VerboseTranscription, TranscriptionError> {
    let endpoint = transcription_endpoint_url(base_url)?;
    transcribe_via_rest(
        &endpoint,
        Some(api_key),
        audio_path,
        model,
        word_timestamps,
        None,
        timeout,
        cancel,
    )
    .await
}

async fn transcribe_via_rest(
    endpoint: &str,
    bearer_token: Option<&str>,
    audio_path: &Path,
    model: &str,
    word_timestamps: bool,
    local_backend: Option<&str>,
    timeout: Duration,
    cancel: Option<Arc<AtomicBool>>,
) -> Result<VerboseTranscription, TranscriptionError> {
    check_cancelled(cancel.as_deref())?;
    let bytes = fs::read(audio_path).map_err(file_error)?;
    check_cancelled(cancel.as_deref())?;
    if bytes.len() as u64 > MAX_TRANSCRIPTION_UPLOAD_BYTES {
        return Err(transcription_error(
            "ai_transcription_input_too_large",
            "Transcription input exceeds the upload limit.",
        ));
    }
    let mut form = Form::new()
        .part(
            "file",
            Part::bytes(bytes)
                .file_name("audio.wav")
                .mime_str("audio/wav")
                .map_err(request_build_error)?,
        )
        .text("model", model.to_string())
        .text("response_format", "verbose_json")
        .text("timestamp_granularities[]", "segment");
    if word_timestamps {
        form = form.text("timestamp_granularities[]", "word");
    }
    if let Some(local_backend) = local_backend {
        form = form.text(LOCAL_BACKEND_FORM_FIELD, local_backend.to_string());
    }
    let client = reqwest::Client::builder()
        .timeout(timeout)
        .build()
        .map_err(request_build_error)?;
    let mut request = client.post(endpoint).multipart(form);
    if let Some(token) = bearer_token {
        request = request.bearer_auth(token);
    }
    let response = await_cancellable(request.send(), cancel.clone())
        .await?
        .map_err(network_error)?;
    let status = response.status();
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        return Err(transcription_error(
            "ai_transcription_remote_auth_failed",
            &format!("Transcription provider rejected authentication with status {status}."),
        ));
    }
    if !status.is_success() {
        let body = await_cancellable(response.text(), cancel.clone())
            .await?
            .map_err(network_error)?;
        if let Some(error) = parse_typed_transcription_error(&body) {
            return Err(error);
        }
        return Err(transcription_error(
            "ai_transcription_provider_failed",
            &format!("Transcription provider failed with status {status}."),
        ));
    }
    let body = await_cancellable(response.text(), cancel)
        .await?
        .map_err(network_error)?;
    parse_verbose_transcription(&body)
}

async fn await_cancellable<F, T>(
    future: F,
    cancel: Option<Arc<AtomicBool>>,
) -> Result<T, TranscriptionError>
where
    F: Future<Output = T>,
{
    let Some(cancel) = cancel else {
        return Ok(future.await);
    };
    tokio::pin!(future);
    loop {
        check_cancelled(Some(cancel.as_ref()))?;
        match tokio::time::timeout(Duration::from_millis(50), &mut future).await {
            Ok(output) => return Ok(output),
            Err(_elapsed) => {}
        }
    }
}

fn check_cancelled(cancel: Option<&AtomicBool>) -> Result<(), TranscriptionError> {
    if cancel.is_some_and(|cancel| cancel.load(Ordering::SeqCst)) {
        return Err(transcription_error(
            "ai_transcription_cancelled",
            "Transcription request was cancelled.",
        ));
    }
    Ok(())
}

pub fn parse_verbose_transcription(body: &str) -> Result<VerboseTranscription, TranscriptionError> {
    let transcription: VerboseTranscription = serde_json::from_str(body).map_err(|_error| {
        transcription_error(
            "ai_transcription_response_invalid",
            "Transcription provider returned invalid verbose JSON.",
        )
    })?;
    if transcription.segments.is_empty() {
        if transcription.text.is_empty() && transcription.words.is_empty() {
            return Ok(transcription);
        }
        return Err(missing_timestamp_error());
    }
    for segment in &transcription.segments {
        validate_time_range(segment.start, segment.end)?;
    }
    for word in &transcription.words {
        validate_time_range(word.start, word.end)?;
    }

    Ok(transcription)
}

fn parse_typed_transcription_error(body: &str) -> Option<TranscriptionError> {
    let value: serde_json::Value = serde_json::from_str(body).ok()?;
    let object = value.as_object()?;
    if object.len() != 2 {
        return None;
    }
    let code = object.get("code")?.as_str()?;
    let message = object.get("message")?.as_str()?;
    Some(transcription_error(code, message))
}

fn missing_timestamp_error() -> TranscriptionError {
    transcription_error(
        "ai_transcription_response_invalid",
        "Transcription response is missing segment timestamps.",
    )
}

fn validate_time_range(start: f64, end: f64) -> Result<(), TranscriptionError> {
    if start.is_finite() && end.is_finite() && end >= start {
        return Ok(());
    }
    Err(transcription_error(
        "ai_transcription_response_invalid",
        "Transcription response contains an invalid timestamp range.",
    ))
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

fn transcription_endpoint_url(base_url: &str) -> Result<String, TranscriptionError> {
    let trimmed = base_url.trim().trim_end_matches('/');
    if !(trimmed.starts_with("http://") || trimmed.starts_with("https://")) {
        return Err(transcription_error(
            "ai_transcription_remote_unconfigured",
            "Remote transcription endpoint must be an HTTP URL.",
        ));
    }
    if trimmed.ends_with(TRANSCRIPTION_PATH) {
        Ok(trimmed.to_string())
    } else if trimmed.ends_with("/v1") {
        Ok(format!("{trimmed}/audio/transcriptions"))
    } else {
        Ok(format!("{trimmed}{TRANSCRIPTION_PATH}"))
    }
}

fn local_whisper_model_path(app_data_dir: &Path, model_id: &str) -> PathBuf {
    app_data_dir
        .join("models")
        .join("whisper.cpp")
        .join(format!("ggml-{model_id}.bin"))
}

fn temp_audio_path() -> PathBuf {
    std::env::temp_dir().join(format!("daydream-transcription-{}.wav", Uuid::now_v7()))
}

#[derive(Debug)]
struct HttpRequest {
    body: Vec<u8>,
    headers: BTreeMap<String, String>,
    method: String,
    path: String,
}

#[derive(Debug)]
struct HttpResponse {
    body: Vec<u8>,
    content_type: &'static str,
    status: u16,
}

impl HttpResponse {
    fn json(status: u16, value: &impl Serialize) -> Self {
        Self {
            body: serde_json::to_vec(value).unwrap_or_else(|_error| b"{}".to_vec()),
            content_type: "application/json",
            status,
        }
    }
}

fn error_response(error: &TranscriptionError) -> HttpResponse {
    let status = match error.code.as_str() {
        "ai_transcription_token_required" | "ai_transcription_token_invalid" => 401,
        "ai_transcription_input_too_large" => 413,
        "ai_transcription_route_not_found" => 404,
        _ => 400,
    };
    HttpResponse::json(status, error)
}

fn read_http_request(
    stream: &mut TcpStream,
    max_body_bytes: u64,
) -> Result<HttpRequest, TranscriptionError> {
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .map_err(network_io_error)?;
    let mut buffer = Vec::new();
    let mut temp = [0_u8; 4096];
    let header_end;
    loop {
        let read = stream.read(&mut temp).map_err(network_io_error)?;
        if read == 0 {
            return Err(transcription_error(
                "ai_transcription_request_invalid",
                "HTTP request ended before headers were complete.",
            ));
        }
        buffer.extend_from_slice(&temp[..read]);
        if let Some(index) = find_bytes(&buffer, b"\r\n\r\n") {
            header_end = index + 4;
            break;
        }
        if buffer.len() > HTTP_HEADER_LIMIT_BYTES {
            return Err(transcription_error(
                "ai_transcription_request_invalid",
                "HTTP request headers are too large.",
            ));
        }
    }
    let header_text = std::str::from_utf8(&buffer[..header_end]).map_err(|_error| {
        transcription_error(
            "ai_transcription_request_invalid",
            "HTTP request headers are not valid UTF-8.",
        )
    })?;
    let mut lines = header_text.split("\r\n");
    let request_line = lines.next().ok_or_else(|| {
        transcription_error(
            "ai_transcription_request_invalid",
            "HTTP request line is missing.",
        )
    })?;
    let mut request_parts = request_line.split_whitespace();
    let method = request_parts.next().unwrap_or_default().to_string();
    let path = request_parts.next().unwrap_or_default().to_string();
    let mut headers = BTreeMap::new();
    for line in lines.filter(|line| !line.is_empty()) {
        if let Some((name, value)) = line.split_once(':') {
            headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_string());
        }
    }
    let content_length = headers
        .get("content-length")
        .and_then(|value| value.parse::<u64>().ok())
        .ok_or_else(|| {
            transcription_error(
                "ai_transcription_request_invalid",
                "HTTP request content length is missing.",
            )
        })?;
    if content_length > max_body_bytes {
        return Err(transcription_error(
            "ai_transcription_input_too_large",
            "Transcription input exceeds the upload limit.",
        ));
    }
    let mut body = buffer[header_end..].to_vec();
    while body.len() < content_length as usize {
        let read = stream.read(&mut temp).map_err(network_io_error)?;
        if read == 0 {
            break;
        }
        body.extend_from_slice(&temp[..read]);
    }
    body.truncate(content_length as usize);

    Ok(HttpRequest {
        body,
        headers,
        method,
        path,
    })
}

fn write_http_response(
    stream: &mut TcpStream,
    response: HttpResponse,
) -> Result<(), TranscriptionError> {
    let status_text = match response.status {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        413 => "Payload Too Large",
        _ => "Internal Server Error",
    };
    write!(
        stream,
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        response.status,
        status_text,
        response.content_type,
        response.body.len()
    )
    .map_err(network_io_error)?;
    stream.write_all(&response.body).map_err(network_io_error)
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct MultipartForm {
    fields: Vec<MultipartField>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct MultipartField {
    data: Vec<u8>,
    filename: Option<String>,
    name: String,
}

impl MultipartForm {
    fn file(&self, name: &str) -> Result<&MultipartField, TranscriptionError> {
        self.fields
            .iter()
            .find(|field| field.name == name && field.filename.is_some())
            .ok_or_else(|| {
                transcription_error(
                    "ai_transcription_request_invalid",
                    "Transcription request is missing the audio file.",
                )
            })
    }

    fn text(&self, name: &str) -> Result<&str, TranscriptionError> {
        self.texts(name).into_iter().next().ok_or_else(|| {
            transcription_error(
                "ai_transcription_request_invalid",
                "Transcription request is missing a required form field.",
            )
        })
    }

    fn optional_text(&self, name: &str) -> Option<&str> {
        self.texts(name).into_iter().next()
    }

    fn texts(&self, name: &str) -> Vec<&str> {
        self.fields
            .iter()
            .filter(|field| field.name == name && field.filename.is_none())
            .filter_map(|field| std::str::from_utf8(&field.data).ok())
            .collect()
    }
}

fn multipart_boundary(content_type: &str) -> Result<String, TranscriptionError> {
    for part in content_type.split(';') {
        let trimmed = part.trim();
        if let Some(boundary) = trimmed.strip_prefix("boundary=") {
            return Ok(boundary.trim_matches('"').to_string());
        }
    }
    Err(transcription_error(
        "ai_transcription_request_invalid",
        "Multipart boundary is missing.",
    ))
}

fn parse_multipart_form(body: &[u8], boundary: &str) -> Result<MultipartForm, TranscriptionError> {
    let marker = format!("--{boundary}");
    let marker = marker.as_bytes();
    let mut fields = Vec::new();
    for raw_part in split_bytes(body, marker) {
        let mut part = raw_part;
        if let Some(stripped) = part.strip_prefix(b"\r\n") {
            part = stripped;
        }
        if part.is_empty() {
            continue;
        }
        if part.starts_with(b"--") {
            continue;
        }
        let header_end = find_bytes(part, b"\r\n\r\n").ok_or_else(|| {
            transcription_error(
                "ai_transcription_request_invalid",
                "Multipart part headers are invalid.",
            )
        })?;
        let header_text = std::str::from_utf8(&part[..header_end]).map_err(|_error| {
            transcription_error(
                "ai_transcription_request_invalid",
                "Multipart part headers are invalid.",
            )
        })?;
        let data = strip_one_trailing_crlf(&part[header_end + 4..]).to_vec();
        let disposition = header_text
            .split("\r\n")
            .find_map(|line| {
                line.split_once(':').and_then(|(name, value)| {
                    name.eq_ignore_ascii_case("content-disposition")
                        .then(|| value.trim())
                })
            })
            .ok_or_else(|| {
                transcription_error(
                    "ai_transcription_request_invalid",
                    "Multipart part disposition is missing.",
                )
            })?;
        let parameters = content_disposition_parameters(disposition);
        let name = parameters.get("name").cloned().ok_or_else(|| {
            transcription_error(
                "ai_transcription_request_invalid",
                "Multipart field name is missing.",
            )
        })?;
        fields.push(MultipartField {
            data,
            filename: parameters.get("filename").cloned(),
            name,
        });
    }

    Ok(MultipartForm { fields })
}

fn strip_one_trailing_crlf(input: &[u8]) -> &[u8] {
    input.strip_suffix(b"\r\n").unwrap_or(input)
}

fn content_disposition_parameters(value: &str) -> BTreeMap<String, String> {
    let mut parameters = BTreeMap::new();
    for part in value.split(';').skip(1) {
        if let Some((name, raw_value)) = part.trim().split_once('=') {
            parameters.insert(
                name.trim().to_ascii_lowercase(),
                raw_value.trim().trim_matches('"').to_string(),
            );
        }
    }
    parameters
}

fn split_bytes<'a>(input: &'a [u8], needle: &[u8]) -> Vec<&'a [u8]> {
    let mut parts = Vec::new();
    let mut start = 0;
    while let Some(relative) = find_bytes(&input[start..], needle) {
        parts.push(&input[start..start + relative]);
        start += relative + needle.len();
    }
    parts.push(&input[start..]);
    parts
}

fn trim_crlf(mut input: &[u8]) -> &[u8] {
    while input.starts_with(b"\r\n") {
        input = &input[2..];
    }
    while input.ends_with(b"\r\n") {
        input = &input[..input.len() - 2];
    }
    input
}

fn find_bytes(input: &[u8], needle: &[u8]) -> Option<usize> {
    input
        .windows(needle.len())
        .position(|window| window == needle)
}

fn transcription_error(code: &str, message: &str) -> TranscriptionError {
    TranscriptionError {
        code: code.to_string(),
        message: message.to_string(),
    }
}

fn file_error(error: std::io::Error) -> TranscriptionError {
    transcription_error(
        "ai_transcription_file_failed",
        &format!("Transcription file operation failed: {error}"),
    )
}

fn local_bind_error(error: std::io::Error) -> TranscriptionError {
    transcription_error(
        "ai_transcription_local_server_unavailable",
        &format!("Local transcription endpoint could not start: {error}"),
    )
}

fn network_io_error(error: std::io::Error) -> TranscriptionError {
    transcription_error(
        "ai_transcription_network_failed",
        &format!("Transcription network operation failed: {error}"),
    )
}

fn network_error(error: reqwest::Error) -> TranscriptionError {
    if error.is_timeout() {
        return transcription_error(
            "ai_transcription_timeout",
            "Transcription provider request timed out.",
        );
    }
    transcription_error(
        "ai_transcription_provider_failed",
        "Transcription provider request failed.",
    )
}

fn request_build_error(error: impl std::fmt::Display) -> TranscriptionError {
    transcription_error(
        "ai_transcription_request_invalid",
        &format!("Transcription request could not be built: {error}"),
    )
}

fn storage_error(error: rusqlite::Error) -> TranscriptionError {
    transcription_error(
        "ai_transcription_storage_failed",
        &format!("Transcription settings could not be loaded: {error}"),
    )
}

fn whisper_error(error: whisper_backend::WhisperBackendError) -> TranscriptionError {
    TranscriptionError {
        code: error.code,
        message: error.message,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex as StdMutex;

    #[derive(Default)]
    struct MockRunner {
        calls: StdMutex<Vec<LocalTranscriptionInput>>,
    }

    impl LocalTranscriptionRunner for MockRunner {
        fn transcribe(
            &self,
            input: LocalTranscriptionInput,
        ) -> Result<VerboseTranscription, TranscriptionError> {
            self.calls.lock().expect("mock calls").push(input);
            Ok(VerboseTranscription {
                duration: Some(1.0),
                language: Some("en".to_string()),
                segments: vec![VerboseTranscriptionSegment {
                    end: 1.0,
                    id: 0,
                    start: 0.0,
                    text: "hello".to_string(),
                }],
                task: Some("transcribe".to_string()),
                text: "hello".to_string(),
                words: vec![VerboseTranscriptionWord {
                    end: 0.5,
                    probability: Some(0.9),
                    start: 0.0,
                    word: "hello".to_string(),
                }],
            })
        }
    }

    #[derive(Default)]
    struct CancellationRunner {
        observed_cancel: AtomicBool,
        started: AtomicBool,
    }

    impl LocalTranscriptionRunner for CancellationRunner {
        fn transcribe(
            &self,
            input: LocalTranscriptionInput,
        ) -> Result<VerboseTranscription, TranscriptionError> {
            self.started.store(true, Ordering::SeqCst);
            for _attempt in 0..200 {
                if input.cancel.load(Ordering::SeqCst) {
                    self.observed_cancel.store(true, Ordering::SeqCst);
                    return Err(transcription_error(
                        "ai_transcription_cancelled",
                        "Transcription was cancelled.",
                    ));
                }
                thread::sleep(Duration::from_millis(10));
            }
            Err(transcription_error(
                "ai_transcription_cancel_timeout",
                "Test runner did not observe cancellation.",
            ))
        }
    }

    #[test]
    fn local_server_binds_to_loopback_and_requires_token() {
        tauri::async_runtime::block_on(async {
            let temp_dir = temp_test_dir();
            create_model_file(&temp_dir, "tiny");
            let runner = Arc::new(MockRunner::default());
            let server =
                LocalTranscriptionServer::start(temp_dir.clone(), runner).expect("start server");
            let endpoint = server.endpoint();

            assert!(endpoint.address.ip().is_loopback());
            let audio_path = temp_dir.join("audio.wav");
            fs::write(&audio_path, b"audio").expect("audio file");
            let no_token = transcribe_via_rest(
                &endpoint.url,
                None,
                &audio_path,
                "tiny",
                true,
                None,
                Duration::from_secs(5),
                None,
            )
            .await
            .expect_err("missing token should fail");
            assert_eq!(no_token.code, "ai_transcription_remote_auth_failed");

            let wrong_token = transcribe_via_rest(
                &endpoint.url,
                Some("wrong"),
                &audio_path,
                "tiny",
                true,
                None,
                Duration::from_secs(5),
                None,
            )
            .await
            .expect_err("wrong token should fail");
            assert_eq!(wrong_token.code, "ai_transcription_remote_auth_failed");

            drop(server);
            let _ = fs::remove_dir_all(temp_dir);
        });
    }

    #[test]
    fn local_server_returns_verbose_json_for_valid_token() {
        tauri::async_runtime::block_on(async {
            let temp_dir = temp_test_dir();
            create_model_file(&temp_dir, "tiny");
            let runner = Arc::new(MockRunner::default());
            let server =
                LocalTranscriptionServer::start(temp_dir.clone(), runner.clone()).expect("server");
            let audio_path = temp_dir.join("audio.wav");
            fs::write(&audio_path, b"audio").expect("audio file");

            let response = transcribe_via_rest(
                &server.endpoint().url,
                Some(&server.endpoint().token),
                &audio_path,
                "tiny",
                true,
                Some("vulkan"),
                Duration::from_secs(5),
                None,
            )
            .await
            .expect("valid local response");

            assert_eq!(response.text, "hello");
            assert_eq!(response.segments[0].start, 0.0);
            assert_eq!(response.words[0].word, "hello");
            let calls = runner.calls.lock().expect("calls");
            assert_eq!(calls.len(), 1);
            assert_eq!(calls[0].model_id, "tiny");
            assert_eq!(calls[0].local_backend, "vulkan");
            assert!(calls[0].word_timestamps);
            assert!(calls[0].model_path.is_file());

            drop(server);
            let _ = fs::remove_dir_all(temp_dir);
        });
    }

    #[test]
    fn local_server_preserves_missing_model_error_code() {
        tauri::async_runtime::block_on(async {
            let temp_dir = temp_test_dir();
            let runner = Arc::new(MockRunner::default());
            let server = LocalTranscriptionServer::start(temp_dir.clone(), runner).expect("server");
            let audio_path = temp_dir.join("audio.wav");
            fs::write(&audio_path, b"audio").expect("audio file");

            let error = transcribe_via_rest(
                &server.endpoint().url,
                Some(&server.endpoint().token),
                &audio_path,
                "tiny",
                true,
                None,
                Duration::from_secs(5),
                None,
            )
            .await
            .expect_err("missing model");

            assert_eq!(error.code, "ai_transcription_local_model_unavailable");

            drop(server);
            let _ = fs::remove_dir_all(temp_dir);
        });
    }

    #[test]
    fn local_server_cancel_flag_follows_client_disconnect() {
        tauri::async_runtime::block_on(async {
            let temp_dir = temp_test_dir();
            create_model_file(&temp_dir, "tiny");
            let runner = Arc::new(CancellationRunner::default());
            let server =
                LocalTranscriptionServer::start(temp_dir.clone(), runner.clone()).expect("server");
            let endpoint = server.endpoint();
            let audio_path = temp_dir.join("audio.wav");
            fs::write(&audio_path, b"audio").expect("audio file");
            let client_cancel = Arc::new(AtomicBool::new(false));
            let request_cancel = client_cancel.clone();
            let url = endpoint.url.clone();
            let token = endpoint.token.clone();
            let request_audio_path = audio_path.clone();

            let request = tauri::async_runtime::spawn(async move {
                transcribe_via_rest(
                    &url,
                    Some(&token),
                    &request_audio_path,
                    "tiny",
                    false,
                    None,
                    Duration::from_secs(10),
                    Some(request_cancel),
                )
                .await
            });

            wait_for_atomic(&runner.started, "runner start").await;
            client_cancel.store(true, Ordering::SeqCst);
            let error = request
                .await
                .expect("join request")
                .expect_err("client cancellation");

            assert_eq!(error.code, "ai_transcription_cancelled");
            wait_for_atomic(&runner.observed_cancel, "runner cancellation").await;

            drop(server);
            let _ = fs::remove_dir_all(temp_dir);
        });
    }

    #[test]
    fn whisper_runner_honors_pre_cancelled_transcribe_without_loading_model() {
        let temp_dir = temp_test_dir();
        let runner = WhisperCppTranscriptionRunner::new();
        let cancel = Arc::new(AtomicBool::new(true));

        let error = runner
            .transcribe(LocalTranscriptionInput {
                audio_path: temp_dir.join("missing.wav"),
                cancel,
                local_backend: "cpu".to_string(),
                model_id: "tiny".to_string(),
                model_path: temp_dir.join("missing-model.bin"),
                word_timestamps: false,
            })
            .expect_err("pre-cancelled local transcription");

        assert_eq!(error.code, "ai_transcription_cancelled");
        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn verbose_json_parser_allows_silent_chunks_and_requires_timestamps_for_text() {
        let parsed = parse_verbose_transcription(
            r#"{
              "text":"hello",
              "segments":[{"id":0,"start":0.0,"end":1.0,"text":"hello"}],
              "words":[{"word":"hello","start":0.0,"end":0.5}]
            }"#,
        )
        .expect("parse verbose response");
        assert_eq!(parsed.segments.len(), 1);
        assert_eq!(parsed.words.len(), 1);

        let silent =
            parse_verbose_transcription(r#"{"text":"","segments":[],"words":[]}"#).expect("silent");
        assert_eq!(silent.text, "");
        assert!(silent.segments.is_empty());
        assert!(silent.words.is_empty());

        let error =
            parse_verbose_transcription(r#"{"text":"hello","segments":[]}"#).expect_err("invalid");
        assert_eq!(error.code, "ai_transcription_response_invalid");
    }

    #[test]
    fn provider_selection_switches_between_local_and_remote() {
        let local = settings::default_settings();
        assert_eq!(
            selected_transcription_provider(&local).expect("local provider"),
            TranscriptionProviderKind::LocalWhisperCpp
        );

        let mut remote = settings::default_settings();
        remote.ai.transcription.provider = "openai-compatible".to_string();
        assert_eq!(
            selected_transcription_provider(&remote).expect("remote provider"),
            TranscriptionProviderKind::OpenAiCompatible
        );

        remote.ai.transcription.provider = "unsupported".to_string();
        let error = selected_transcription_provider(&remote).expect_err("invalid provider");
        assert_eq!(error.code, "ai_transcription_provider_invalid");
    }

    #[test]
    fn remote_provider_sends_multipart_and_bearer_auth() {
        tauri::async_runtime::block_on(async {
            let server = OneShotServer::start_json(
                200,
                "OK",
                r#"{"text":"ok","segments":[{"id":0,"start":0.0,"end":1.0,"text":"ok"}],"words":[]}"#,
            );
            let temp_dir = temp_test_dir();
            let audio_path = temp_dir.join("audio.wav");
            fs::write(&audio_path, b"audio").expect("audio");

            let response = transcribe_remote(
                &format!("http://{}/v1", server.address),
                "sk-test",
                &audio_path,
                "tiny",
                true,
                Duration::from_secs(5),
                None,
            )
            .await
            .expect("remote response");

            assert_eq!(response.text, "ok");
            let request = server.join();
            assert!(request.starts_with("POST /v1/audio/transcriptions HTTP/1.1"));
            assert!(request.contains("authorization: Bearer sk-test"));
            assert!(request.contains("name=\"model\""));
            assert!(request.contains("tiny"));
            assert!(request.contains("name=\"timestamp_granularities[]\""));
            assert!(request.contains("word"));
            assert!(request.contains("segment"));

            let _ = fs::remove_dir_all(temp_dir);
        });
    }

    #[test]
    fn remote_auth_failure_returns_safe_error() {
        tauri::async_runtime::block_on(async {
            let server = OneShotServer::start_text(401, "Unauthorized", "private body");
            let temp_dir = temp_test_dir();
            let audio_path = temp_dir.join("audio.wav");
            fs::write(&audio_path, b"audio").expect("audio");

            let error = transcribe_remote(
                &format!("http://{}/v1", server.address),
                "sk-secret",
                &audio_path,
                "tiny",
                false,
                Duration::from_secs(5),
                None,
            )
            .await
            .expect_err("auth failure");

            assert_eq!(error.code, "ai_transcription_remote_auth_failed");
            assert!(!error.message.contains("private body"));
            assert!(!error.message.contains("sk-secret"));
            let _ = server.join();
            let _ = fs::remove_dir_all(temp_dir);
        });
    }

    #[test]
    fn remote_typed_error_body_is_preserved_without_private_fallback_body() {
        tauri::async_runtime::block_on(async {
            let server = OneShotServer::start_json(
                400,
                "Bad Request",
                r#"{"code":"ai_transcription_vendor_typed","message":"Typed provider error."}"#,
            );
            let temp_dir = temp_test_dir();
            let audio_path = temp_dir.join("audio.wav");
            fs::write(&audio_path, b"audio").expect("audio");

            let error = transcribe_remote(
                &format!("http://{}/v1", server.address),
                "sk-secret",
                &audio_path,
                "tiny",
                false,
                Duration::from_secs(5),
                None,
            )
            .await
            .expect_err("typed provider error");

            assert_eq!(error.code, "ai_transcription_vendor_typed");
            assert_eq!(error.message, "Typed provider error.");
            let _ = server.join();
            let _ = fs::remove_dir_all(temp_dir);
        });
    }

    #[test]
    fn remote_arbitrary_error_body_stays_redacted() {
        tauri::async_runtime::block_on(async {
            let server = OneShotServer::start_text(400, "Bad Request", "private transcript");
            let temp_dir = temp_test_dir();
            let audio_path = temp_dir.join("audio.wav");
            fs::write(&audio_path, b"audio").expect("audio");

            let error = transcribe_remote(
                &format!("http://{}/v1", server.address),
                "sk-secret",
                &audio_path,
                "tiny",
                false,
                Duration::from_secs(5),
                None,
            )
            .await
            .expect_err("generic provider error");

            assert_eq!(error.code, "ai_transcription_provider_failed");
            assert!(!error.message.contains("private transcript"));
            assert!(!error.message.contains("sk-secret"));
            let _ = server.join();
            let _ = fs::remove_dir_all(temp_dir);
        });
    }

    #[test]
    fn rest_adapter_honors_pre_cancelled_request_without_uploading() {
        tauri::async_runtime::block_on(async {
            let temp_dir = temp_test_dir();
            let audio_path = temp_dir.join("audio.wav");
            fs::write(&audio_path, b"private audio").expect("audio");
            let cancel = Arc::new(AtomicBool::new(true));

            let error = transcribe_via_rest(
                "http://127.0.0.1:9/v1/audio/transcriptions",
                Some("sk-secret"),
                &audio_path,
                "tiny",
                false,
                None,
                Duration::from_secs(5),
                Some(cancel),
            )
            .await
            .expect_err("cancelled request");

            assert_eq!(error.code, "ai_transcription_cancelled");
            assert!(!error.message.contains("private audio"));
            assert!(!error.message.contains("sk-secret"));
            let _ = fs::remove_dir_all(temp_dir);
        });
    }

    #[test]
    fn multipart_parser_preserves_file_payload_trailing_crlf_bytes() {
        let body = b"--boundary\r\nContent-Disposition: form-data; name=\"file\"; filename=\"audio.wav\"\r\nContent-Type: audio/wav\r\n\r\nRIFF\r\n\r\n--boundary--\r\n";

        let form = parse_multipart_form(body, "boundary").expect("multipart");
        let file = form.file("file").expect("file field");

        assert_eq!(file.data, b"RIFF\r\n");
    }

    async fn wait_for_atomic(flag: &AtomicBool, label: &str) {
        for _attempt in 0..100 {
            if flag.load(Ordering::SeqCst) {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("timed out waiting for {label}");
    }

    fn temp_test_dir() -> PathBuf {
        let path = std::env::temp_dir().join(format!("daydream-test-{}", Uuid::now_v7()));
        fs::create_dir_all(&path).expect("temp test dir");
        path
    }

    fn create_model_file(app_data_dir: &Path, model_id: &str) {
        let path = local_whisper_model_path(app_data_dir, model_id);
        fs::create_dir_all(path.parent().expect("model parent")).expect("model dir");
        fs::write(path, b"model").expect("model file");
    }

    struct OneShotServer {
        address: SocketAddr,
        handle: JoinHandle<String>,
    }

    impl OneShotServer {
        fn start_json(status: u16, status_text: &'static str, body: &'static str) -> Self {
            Self::start_response(status, status_text, "application/json", body)
        }

        fn start_text(status: u16, status_text: &'static str, body: &'static str) -> Self {
            Self::start_response(status, status_text, "text/plain", body)
        }

        fn start_response(
            status: u16,
            status_text: &'static str,
            content_type: &'static str,
            body: &'static str,
        ) -> Self {
            let response = format!(
                "HTTP/1.1 {status} {status_text}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            Self::start(response)
        }

        fn start(response: String) -> Self {
            let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind one-shot");
            let address = listener.local_addr().expect("address");
            let handle = thread::spawn(move || {
                let (mut stream, _peer) = listener.accept().expect("accept");
                let mut buffer = Vec::new();
                let mut temp = [0_u8; 4096];
                loop {
                    let read = stream.read(&mut temp).expect("read request");
                    if read == 0 {
                        break;
                    }
                    buffer.extend_from_slice(&temp[..read]);
                    if let Some(header_end) = find_bytes(&buffer, b"\r\n\r\n") {
                        let header_text =
                            std::str::from_utf8(&buffer[..header_end]).expect("headers utf8");
                        let content_length = header_text
                            .lines()
                            .find_map(|line| {
                                line.split_once(':').and_then(|(name, value)| {
                                    name.eq_ignore_ascii_case("content-length")
                                        .then(|| value.trim().parse::<usize>().ok())
                                        .flatten()
                                })
                            })
                            .unwrap_or(0);
                        if buffer.len() >= header_end + 4 + content_length {
                            break;
                        }
                    }
                }
                stream
                    .write_all(response.as_bytes())
                    .expect("write response");
                String::from_utf8_lossy(&buffer).to_string()
            });

            Self { address, handle }
        }

        fn join(self) -> String {
            self.handle.join().expect("one-shot join")
        }
    }
}
