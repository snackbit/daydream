use std::{
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{mpsc, Mutex},
    thread::{self, JoinHandle},
    time::Duration,
};

use crate::whisper_backend;

use super::local_protocol::WhisperWorkerReady;
use super::remote::{
    check_cancelled, transcribe_via_rest, warm_local_via_rest, RestTranscriptionRequest,
};
use super::{
    transcription_error, whisper_error, worker_spawn_error, worker_unavailable_error,
    LocalTranscriptionInput, LocalTranscriptionWarmup, TranscriptionError, VerboseTranscription,
    LOCAL_WORKER_STARTUP_TIMEOUT, LOCAL_WORKER_WARMUP_TIMEOUT,
};

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

pub(super) struct WhisperCppTranscriptionRunner {
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

pub(super) struct WhisperProcessTranscriptionRunner {
    app_data_dir: PathBuf,
    state: Mutex<Option<WhisperWorkerProcess>>,
    worker_executable: PathBuf,
}

#[derive(Clone, Debug)]
struct WhisperWorkerEndpoint {
    token: String,
    transcription_url: String,
    warmup_url: String,
}

struct WhisperWorkerProcess {
    child: Child,
    endpoint: WhisperWorkerEndpoint,
}

impl WhisperCppTranscriptionRunner {
    pub(super) fn new() -> Self {
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

impl WhisperProcessTranscriptionRunner {
    pub(super) fn new(app_data_dir: PathBuf) -> Result<Self, TranscriptionError> {
        let worker_executable = std::env::current_exe().map_err(worker_spawn_error)?;

        Ok(Self {
            app_data_dir,
            state: Mutex::new(None),
            worker_executable,
        })
    }

    #[cfg(test)]
    pub(super) fn with_worker_executable(
        app_data_dir: PathBuf,
        worker_executable: PathBuf,
    ) -> Self {
        Self {
            app_data_dir,
            state: Mutex::new(None),
            worker_executable,
        }
    }

    fn endpoint(&self) -> Result<WhisperWorkerEndpoint, TranscriptionError> {
        let mut state = self.state.lock().expect("whisper worker process state");
        let needs_start = match state.as_mut() {
            Some(worker) => match worker.child.try_wait().map_err(worker_spawn_error)? {
                Some(_status) => true,
                None => false,
            },
            None => true,
        };
        if needs_start {
            *state = Some(self.spawn_worker()?);
        }

        state
            .as_ref()
            .map(|worker| worker.endpoint.clone())
            .ok_or_else(worker_unavailable_error)
    }

    fn spawn_worker(&self) -> Result<WhisperWorkerProcess, TranscriptionError> {
        let mut child = Command::new(&self.worker_executable)
            .arg("--daydream-whisper-worker")
            .arg(&self.app_data_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(worker_spawn_error)?;
        let stdout = child.stdout.take().ok_or_else(|| {
            transcription_error(
                "ai_transcription_local_worker_unavailable",
                "Local transcription worker stdout is unavailable.",
            )
        })?;
        let (ready_sender, ready_receiver) = mpsc::channel();
        let _reader = thread::Builder::new()
            .name("daydream-whisper-worker-ready".to_string())
            .spawn(move || {
                let mut line = String::new();
                let result = BufReader::new(stdout)
                    .read_line(&mut line)
                    .map(|read| (read, line));
                let _ = ready_sender.send(result);
            })
            .map_err(worker_spawn_error)?;

        let ready_line = match ready_receiver.recv_timeout(LOCAL_WORKER_STARTUP_TIMEOUT) {
            Ok(Ok((read, line))) if read > 0 => line,
            Ok(Ok((_read, _line))) => {
                let _ = child.kill();
                return Err(worker_unavailable_error());
            }
            Ok(Err(error)) => {
                let _ = child.kill();
                return Err(worker_spawn_error(error));
            }
            Err(_error) => {
                let _ = child.kill();
                return Err(transcription_error(
                    "ai_transcription_local_worker_timeout",
                    "Local transcription worker did not become ready.",
                ));
            }
        };
        let ready: WhisperWorkerReady =
            serde_json::from_str(ready_line.trim()).map_err(|_error| {
                let _ = child.kill();
                transcription_error(
                    "ai_transcription_local_worker_unavailable",
                    "Local transcription worker returned an invalid ready message.",
                )
            })?;

        Ok(WhisperWorkerProcess {
            child,
            endpoint: WhisperWorkerEndpoint {
                token: ready.token,
                transcription_url: ready.transcription_url,
                warmup_url: ready.warmup_url,
            },
        })
    }

    fn note_request_result<T>(
        &self,
        result: Result<T, TranscriptionError>,
    ) -> Result<T, TranscriptionError> {
        match result {
            Ok(value) => Ok(value),
            Err(error) => {
                let mut state = self.state.lock().expect("whisper worker process state");
                if let Some(worker) = state.as_mut() {
                    let mut status = worker.child.try_wait();
                    if matches!(status, Ok(None)) {
                        thread::sleep(Duration::from_millis(50));
                        status = worker.child.try_wait();
                    }
                    if let Ok(Some(_status)) = status {
                        *state = None;
                        return Err(transcription_error(
                            "ai_transcription_local_worker_crashed",
                            "Local transcription worker exited before returning a result.",
                        ));
                    }
                }
                Err(error)
            }
        }
    }
}

impl Drop for WhisperProcessTranscriptionRunner {
    fn drop(&mut self) {
        if let Ok(mut state) = self.state.lock() {
            if let Some(mut worker) = state.take() {
                let _ = worker.child.kill();
                let _ = worker.child.wait();
            }
        }
    }
}

impl LocalTranscriptionRunner for WhisperProcessTranscriptionRunner {
    fn warm_model(&self, input: LocalTranscriptionWarmup) -> Result<(), TranscriptionError> {
        let endpoint = self.endpoint()?;
        let result = tauri::async_runtime::block_on(warm_local_via_rest(
            &endpoint.warmup_url,
            &endpoint.token,
            &input,
            LOCAL_WORKER_WARMUP_TIMEOUT,
        ));

        self.note_request_result(result)
    }

    fn transcribe(
        &self,
        input: LocalTranscriptionInput,
    ) -> Result<VerboseTranscription, TranscriptionError> {
        let endpoint = self.endpoint()?;
        let result =
            tauri::async_runtime::block_on(transcribe_via_rest(RestTranscriptionRequest {
                audio_path: &input.audio_path,
                bearer_token: Some(&endpoint.token),
                cancel: Some(input.cancel),
                endpoint: &endpoint.transcription_url,
                language: &input.language,
                local_backend: Some(&input.local_backend),
                max_initial_timestamp_seconds: Some(input.max_initial_timestamp_seconds),
                model: &input.model_id,
                timeout: LOCAL_WORKER_WARMUP_TIMEOUT,
                word_timestamps: input.word_timestamps,
            }));

        self.note_request_result(result)
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
                            .transcribe_wav(
                                &input.audio_path,
                                &input.language,
                                input.max_initial_timestamp_seconds as f32,
                                input.word_timestamps,
                                input.cancel,
                            )
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
