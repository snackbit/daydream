use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use uuid::Uuid;

use super::{
    local_receipts::LocalHandlerReceipt,
    local_scratch::{LocalAudioCopy, LocalScratchOwner},
    remote::check_cancelled,
};
use crate::settings;

use super::http::{
    error_response, multipart_boundary, parse_multipart_form, read_http_request,
    write_http_response, HttpRequest, HttpResponse,
};
use super::local_protocol::{LocalWarmupRequest, WhisperWorkerReady};
use super::local_worker::{LocalTranscriptionRunner, WhisperCppTranscriptionRunner};
use super::{
    local_bind_error, local_whisper_model_path, network_io_error, request_build_error,
    transcription_error, LocalTranscriptionInput, LocalTranscriptionWarmup, TranscriptionError,
    VerboseTranscription, LOCAL_BACKEND_FORM_FIELD, LOCAL_WARMUP_PATH,
    MAX_TRANSCRIPTION_UPLOAD_BYTES, TRANSCRIPTION_PATH,
};

#[derive(Clone, Debug)]
pub(super) struct LocalTranscriptionEndpoint {
    #[cfg(test)]
    pub(super) address: SocketAddr,
    pub(super) token: String,
    pub(super) url: String,
    pub(super) warmup_url: String,
}

#[derive(Debug)]
pub(super) struct LocalTranscriptionServer {
    address: SocketAddr,
    shutdown: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    token: String,
}

impl LocalTranscriptionServer {
    pub(super) fn start(
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

    pub(super) fn endpoint(&self) -> LocalTranscriptionEndpoint {
        LocalTranscriptionEndpoint {
            #[cfg(test)]
            address: self.address,
            token: self.token.clone(),
            url: format!("http://{}{}", self.address, TRANSCRIPTION_PATH),
            warmup_url: format!("http://{}{}", self.address, LOCAL_WARMUP_PATH),
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

pub fn run_whisper_worker_process(app_data_dir: PathBuf) -> i32 {
    match run_whisper_worker_process_inner(app_data_dir) {
        Ok(()) => 0,
        Err(error) => {
            eprintln!(
                "daydream whisper worker failed: {}: {}",
                error.code, error.message
            );
            1
        }
    }
}

fn run_whisper_worker_process_inner(app_data_dir: PathBuf) -> Result<(), TranscriptionError> {
    let runner = Arc::new(WhisperCppTranscriptionRunner::new());
    let server = LocalTranscriptionServer::start(app_data_dir, runner)?;
    let endpoint = server.endpoint();
    let ready = WhisperWorkerReady {
        token: endpoint.token,
        transcription_url: endpoint.url,
        warmup_url: endpoint.warmup_url,
    };
    println!(
        "{}",
        serde_json::to_string(&ready).map_err(request_build_error)?
    );
    std::io::stdout().flush().map_err(network_io_error)?;

    let mut stdin = std::io::stdin();
    let mut buffer = [0_u8; 1024];
    loop {
        match stdin.read(&mut buffer) {
            Ok(0) => break,
            Ok(_read) => {}
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
            Err(error) => return Err(network_io_error(error)),
        }
    }

    drop(server);
    Ok(())
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
    if request.method != "POST"
        || (request.path != TRANSCRIPTION_PATH && request.path != LOCAL_WARMUP_PATH)
    {
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
    if request.path == LOCAL_WARMUP_PATH {
        handle_local_warmup_request(request, context)?;
        return Ok(VerboseTranscription {
            duration: None,
            language: None,
            language_confidence: None,
            segments: Vec::new(),
            task: None,
            text: String::new(),
            words: Vec::new(),
        });
    }
    let content_type = request.headers.get("content-type").ok_or_else(|| {
        transcription_error(
            "ai_transcription_request_invalid",
            "Transcription request content type is missing.",
        )
    })?;
    let boundary = multipart_boundary(content_type)?;
    let multipart = parse_multipart_form(&request.body, &boundary)?;
    let handler_receipt = multipart
        .optional_text("daydream_request_id")
        .map(LocalHandlerReceipt::claim)
        .transpose()?;
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
        .contains(&"word");
    let max_initial_timestamp_seconds = multipart
        .optional_text("max_initial_timestamp_seconds")
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(settings::DEFAULT_AI_TRANSCRIPTION_MAX_INITIAL_TIMESTAMP_SECONDS);
    let language = multipart
        .optional_text("language")
        .unwrap_or(settings::DEFAULT_AI_TRANSCRIPTION_LANGUAGE);
    let file = multipart.file("file")?;
    let model_path = local_whisper_model_path(&context.app_data_dir, model_id);
    if !model_path.is_file() {
        return Err(transcription_error(
            "ai_transcription_local_model_unavailable",
            "Local transcription model is not available.",
        ));
    }

    let cancel = handler_receipt
        .as_ref()
        .map(LocalHandlerReceipt::cancel)
        .unwrap_or_else(|| Arc::new(AtomicBool::new(false)));
    check_cancelled(Some(&cancel))?;
    let owner = if let Some(receipt) = &handler_receipt {
        LocalScratchOwner::validated(receipt.job_id(), receipt.id())?
    } else if let Some(id) = multipart.optional_text("daydream_scratch_request_id") {
        LocalScratchOwner::validated(multipart.optional_text("daydream_scratch_job_id"), id)?
    } else {
        LocalScratchOwner::validated(None, &Uuid::now_v7().to_string())?
    };
    let audio_copy = LocalAudioCopy::write(&context.app_data_dir, &owner, &file.data)?;
    let _disconnect_monitor =
        LocalTranscriptionCancelMonitor::start(disconnect_stream, cancel.clone());
    let result = context.runner.transcribe(LocalTranscriptionInput {
        audio_path: audio_copy.path().to_path_buf(),
        cancel,
        language: language.to_string(),
        local_backend: local_backend.to_string(),
        max_initial_timestamp_seconds,
        model_id: model_id.to_string(),
        model_path,
        word_timestamps,
    });
    drop(audio_copy);
    drop(handler_receipt);
    result
}

fn handle_local_warmup_request(
    request: HttpRequest,
    context: LocalTranscriptionContext,
) -> Result<(), TranscriptionError> {
    let warmup: LocalWarmupRequest = serde_json::from_slice(&request.body).map_err(|_error| {
        transcription_error(
            "ai_transcription_request_invalid",
            "Local transcription warmup request is invalid.",
        )
    })?;
    let model_path = local_whisper_model_path(&context.app_data_dir, &warmup.model);
    if !model_path.is_file() {
        return Err(transcription_error(
            "ai_transcription_local_model_unavailable",
            "Local transcription model is not available.",
        ));
    }

    context.runner.warm_model(LocalTranscriptionWarmup {
        local_backend: warmup.local_backend,
        model_id: warmup.model,
        model_path,
    })
}

struct LocalTranscriptionCancelMonitor {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl LocalTranscriptionCancelMonitor {
    pub(super) fn start(mut stream: TcpStream, cancel: Arc<AtomicBool>) -> Self {
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
