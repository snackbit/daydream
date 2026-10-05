use std::{
    fs,
    io::{Read, Write},
    net::{SocketAddr, TcpListener},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex as StdMutex,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use uuid::Uuid;

use super::http::{find_bytes, parse_multipart_form};
use super::local_server::LocalTranscriptionServer;
use super::local_worker::{
    should_drop_cached_model_after_error, LocalTranscriptionRunner, WhisperCppTranscriptionRunner,
    WhisperProcessTranscriptionRunner,
};
use super::provider::{
    is_local_model_ready_for_warmup, selected_transcription_provider, TranscriptionProviderKind,
};
use super::remote::{
    parse_verbose_transcription, transcribe_remote, transcribe_via_rest, warm_local_via_rest,
    RemoteTranscriptionRequest, RestTranscriptionRequest,
};
use super::*;
mod local_drain;
use crate::whisper_backend::WhisperExecutionBackend;
use crate::{settings, storage};

#[derive(Default)]
struct MockRunner {
    calls: StdMutex<Vec<LocalTranscriptionInput>>,
    warm_calls: StdMutex<Vec<LocalTranscriptionWarmup>>,
}

impl LocalTranscriptionRunner for MockRunner {
    fn warm_model(&self, input: LocalTranscriptionWarmup) -> Result<(), TranscriptionError> {
        self.warm_calls.lock().expect("mock warm calls").push(input);
        Ok(())
    }

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
        let no_token = transcribe_via_rest(RestTranscriptionRequest {
            local_request_id: None,
            local_scratch_owner: None,
            audio_path: &audio_path,
            bearer_token: None,
            cancel: None,
            endpoint: &endpoint.url,
            language: "auto",
            local_backend: None,
            max_initial_timestamp_seconds: None,
            model: "tiny",
            timeout: Duration::from_secs(5),
            word_timestamps: true,
        })
        .await
        .expect_err("missing token should fail");
        assert_eq!(no_token.code, "ai_transcription_remote_auth_failed");

        let wrong_token = transcribe_via_rest(RestTranscriptionRequest {
            local_request_id: None,
            local_scratch_owner: None,
            audio_path: &audio_path,
            bearer_token: Some("wrong"),
            cancel: None,
            endpoint: &endpoint.url,
            language: "auto",
            local_backend: None,
            max_initial_timestamp_seconds: None,
            model: "tiny",
            timeout: Duration::from_secs(5),
            word_timestamps: true,
        })
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

        let endpoint = server.endpoint();
        let response = transcribe_via_rest(RestTranscriptionRequest {
            local_request_id: None,
            local_scratch_owner: None,
            audio_path: &audio_path,
            bearer_token: Some(&endpoint.token),
            cancel: None,
            endpoint: &endpoint.url,
            language: "auto",
            local_backend: Some("vulkan"),
            max_initial_timestamp_seconds: Some(3.0),
            model: "tiny",
            timeout: Duration::from_secs(5),
            word_timestamps: true,
        })
        .await
        .expect("valid local response");

        assert_eq!(response.text, "hello");
        assert_eq!(response.segments[0].start, 0.0);
        assert_eq!(response.words[0].word, "hello");
        let calls = runner.calls.lock().expect("calls");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].model_id, "tiny");
        assert_eq!(calls[0].language, "auto");
        assert_eq!(calls[0].local_backend, "vulkan");
        assert_eq!(calls[0].max_initial_timestamp_seconds, 3.0);
        assert!(calls[0].word_timestamps);
        assert!(calls[0].model_path.is_file());

        drop(server);
        let _ = fs::remove_dir_all(temp_dir);
    });
}

#[test]
fn local_server_warmup_route_uses_bearer_token() {
    tauri::async_runtime::block_on(async {
        let temp_dir = temp_test_dir();
        create_model_file(&temp_dir, "tiny");
        let runner = Arc::new(MockRunner::default());
        let server =
            LocalTranscriptionServer::start(temp_dir.clone(), runner.clone()).expect("server");
        let endpoint = server.endpoint();
        let warmup = LocalTranscriptionWarmup {
            local_backend: "cpu".to_string(),
            model_id: "tiny".to_string(),
            model_path: temp_dir.join("models/whisper.cpp/ggml-tiny.bin"),
        };

        let wrong_token = warm_local_via_rest(
            &endpoint.warmup_url,
            "wrong",
            &warmup,
            Duration::from_secs(5),
        )
        .await
        .expect_err("wrong token should fail");
        assert_eq!(wrong_token.code, "ai_transcription_remote_auth_failed");

        warm_local_via_rest(
            &endpoint.warmup_url,
            &endpoint.token,
            &warmup,
            Duration::from_secs(5),
        )
        .await
        .expect("warmup succeeds");
        let calls = runner.warm_calls.lock().expect("warm calls");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].model_id, "tiny");
        assert_eq!(calls[0].local_backend, "cpu");
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

        let endpoint = server.endpoint();
        let error = transcribe_via_rest(RestTranscriptionRequest {
            local_request_id: None,
            local_scratch_owner: None,
            audio_path: &audio_path,
            bearer_token: Some(&endpoint.token),
            cancel: None,
            endpoint: &endpoint.url,
            language: "auto",
            local_backend: None,
            max_initial_timestamp_seconds: None,
            model: "tiny",
            timeout: Duration::from_secs(5),
            word_timestamps: true,
        })
        .await
        .expect_err("missing model");

        assert_eq!(error.code, "ai_transcription_local_model_unavailable");

        drop(server);
        let _ = fs::remove_dir_all(temp_dir);
    });
}

#[test]
fn local_model_warmup_requires_available_verified_model() {
    let ready = test_model_asset_row("available", "passed");
    let downloading = test_model_asset_row("downloading", "pending");
    let unverified = test_model_asset_row("available", "pending");

    assert!(is_local_model_ready_for_warmup(&ready));
    assert!(!is_local_model_ready_for_warmup(&downloading));
    assert!(!is_local_model_ready_for_warmup(&unverified));
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
            transcribe_via_rest(RestTranscriptionRequest {
                local_request_id: None,
                local_scratch_owner: None,
                audio_path: &request_audio_path,
                bearer_token: Some(&token),
                cancel: Some(request_cancel),
                endpoint: &url,
                language: "auto",
                local_backend: None,
                max_initial_timestamp_seconds: None,
                model: "tiny",
                timeout: Duration::from_secs(10),
                word_timestamps: false,
            })
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
            language: "auto".to_string(),
            local_backend: "cpu".to_string(),
            max_initial_timestamp_seconds: 3.0,
            model_id: "tiny".to_string(),
            model_path: temp_dir.join("missing-model.bin"),
            word_timestamps: false,
        })
        .expect_err("pre-cancelled local transcription");

    assert_eq!(error.code, "ai_transcription_cancelled");
    let _ = fs::remove_dir_all(temp_dir);
}

#[test]
fn whisper_runner_drops_gpu_cache_after_provider_failure_only() {
    let provider_failed = transcription_error(
        "ai_transcription_provider_failed",
        "Whisper transcription failed.",
    );
    let cancelled = transcription_error("ai_transcription_cancelled", "Cancelled.");

    assert!(should_drop_cached_model_after_error(
        WhisperExecutionBackend::Gpu,
        &provider_failed
    ));
    assert!(!should_drop_cached_model_after_error(
        WhisperExecutionBackend::Cpu,
        &provider_failed
    ));
    assert!(!should_drop_cached_model_after_error(
        WhisperExecutionBackend::Gpu,
        &cancelled
    ));
}

#[cfg(unix)]
#[test]
fn process_runner_reports_worker_that_exits_before_ready() {
    let runner = WhisperProcessTranscriptionRunner::with_worker_executable(
        temp_test_dir(),
        PathBuf::from("/bin/false"),
    );

    let error = runner
        .warm_model(LocalTranscriptionWarmup {
            local_backend: "cpu".to_string(),
            model_id: "tiny".to_string(),
            model_path: PathBuf::from("/missing-model.bin"),
        })
        .expect_err("worker exits before ready");

    assert_eq!(error.code, "ai_transcription_local_worker_unavailable");
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

        let response = transcribe_remote(RemoteTranscriptionRequest {
            api_key: "sk-test",
            audio_path: &audio_path,
            base_url: &format!("http://{}/v1", server.address),
            cancel: None,
            language: "de",
            model: "tiny",
            timeout: Duration::from_secs(5),
            word_timestamps: true,
        })
        .await
        .expect("remote response");

        assert_eq!(response.text, "ok");
        let request = server.join();
        assert!(request.starts_with("POST /v1/audio/transcriptions HTTP/1.1"));
        assert!(request.contains("authorization: Bearer sk-test"));
        assert!(request.contains("name=\"model\""));
        assert!(request.contains("tiny"));
        assert!(request.contains("name=\"language\""));
        assert!(request.contains("de"));
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

        let error = transcribe_remote(RemoteTranscriptionRequest {
            api_key: "sk-secret",
            audio_path: &audio_path,
            base_url: &format!("http://{}/v1", server.address),
            cancel: None,
            language: "auto",
            model: "tiny",
            timeout: Duration::from_secs(5),
            word_timestamps: false,
        })
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

        let error = transcribe_remote(RemoteTranscriptionRequest {
            api_key: "sk-secret",
            audio_path: &audio_path,
            base_url: &format!("http://{}/v1", server.address),
            cancel: None,
            language: "auto",
            model: "tiny",
            timeout: Duration::from_secs(5),
            word_timestamps: false,
        })
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

        let error = transcribe_remote(RemoteTranscriptionRequest {
            api_key: "sk-secret",
            audio_path: &audio_path,
            base_url: &format!("http://{}/v1", server.address),
            cancel: None,
            language: "auto",
            model: "tiny",
            timeout: Duration::from_secs(5),
            word_timestamps: false,
        })
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

        let error = transcribe_via_rest(RestTranscriptionRequest {
            local_request_id: None,
            local_scratch_owner: None,
            audio_path: &audio_path,
            bearer_token: Some("sk-secret"),
            cancel: Some(cancel),
            endpoint: "http://127.0.0.1:9/v1/audio/transcriptions",
            language: "auto",
            local_backend: None,
            max_initial_timestamp_seconds: None,
            model: "tiny",
            timeout: Duration::from_secs(5),
            word_timestamps: false,
        })
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

fn test_model_asset_row(
    download_status: &str,
    verification_status: &str,
) -> storage::AiModelAssetRow {
    storage::AiModelAssetRow {
        checksum: None,
        checksum_algorithm: None,
        created_at_ms: 1,
        display_label: "Whisper tiny".to_string(),
        downloaded_bytes: 0,
        download_status: download_status.to_string(),
        etag: None,
        error_code: None,
        id: "tiny".to_string(),
        local_path: None,
        provider: "local-whisper-cpp".to_string(),
        self_test_status: "not-run".to_string(),
        size_bytes: Some(1),
        source_url: None,
        total_bytes: Some(1),
        updated_at_ms: 1,
        verification_status: verification_status.to_string(),
    }
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
