use std::{
    fs,
    future::Future,
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use reqwest::multipart::{Form, Part};

use crate::settings;

use super::local_protocol::LocalWarmupRequest;
use super::{
    file_error, network_error, request_build_error, transcription_endpoint_url,
    transcription_error, LocalTranscriptionWarmup, TranscriptionError, VerboseTranscription,
    LOCAL_BACKEND_FORM_FIELD, MAX_TRANSCRIPTION_UPLOAD_BYTES,
};

pub struct RemoteTranscriptionRequest<'a> {
    pub api_key: &'a str,
    pub audio_path: &'a Path,
    pub base_url: &'a str,
    pub cancel: Option<Arc<AtomicBool>>,
    pub language: &'a str,
    pub model: &'a str,
    pub timeout: Duration,
    pub word_timestamps: bool,
}

pub async fn transcribe_remote(
    request: RemoteTranscriptionRequest<'_>,
) -> Result<VerboseTranscription, TranscriptionError> {
    let endpoint = transcription_endpoint_url(request.base_url)?;
    transcribe_via_rest(RestTranscriptionRequest {
        local_request_id: None,
        local_scratch_owner: None,
        audio_path: request.audio_path,
        bearer_token: Some(request.api_key),
        cancel: request.cancel,
        endpoint: &endpoint,
        language: request.language,
        local_backend: None,
        max_initial_timestamp_seconds: None,
        model: request.model,
        timeout: request.timeout,
        word_timestamps: request.word_timestamps,
    })
    .await
}

pub(super) async fn warm_local_via_rest(
    endpoint: &str,
    bearer_token: &str,
    input: &LocalTranscriptionWarmup,
    timeout: Duration,
) -> Result<(), TranscriptionError> {
    let mut http_diagnostics = crate::provider_http::HttpDiagnostics::new("transcription_warmup");
    let result: Result<_, TranscriptionError> = async {
        let request = LocalWarmupRequest {
            local_backend: input.local_backend.clone(),
            model: input.model_id.clone(),
        };
        let body = serde_json::to_vec(&request).map_err(request_build_error)?;
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .map_err(request_build_error)?;
        http_diagnostics.stage("http_request");
        let response = client
            .post(endpoint)
            .bearer_auth(bearer_token)
            .header("content-type", "application/json")
            .body(body)
            .send()
            .await
            .map_err(network_error)?;
        let status = response.status();
        http_diagnostics.response(
            status.as_u16(),
            response
                .headers()
                .get("x-request-id")
                .and_then(|v| v.to_str().ok()),
        );
        if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
            return Err(transcription_error(
                "ai_transcription_remote_auth_failed",
                &format!("Transcription provider rejected authentication with status {status}."),
            ));
        }
        if !status.is_success() {
            let body = response.text().await.map_err(network_error)?;
            if let Some(error) = parse_typed_transcription_error(&body) {
                return Err(error);
            }
            return Err(transcription_error(
                &format!(
                    "ai_transcription_remote_{}",
                    crate::provider_http::status_code(status)
                ),
                &format!("Transcription provider failed with status {status}."),
            ));
        }

        Ok(())
    }
    .await;
    http_diagnostics.finish(result.as_ref().err().map(|error| error.code.as_str()));
    result
}

pub(super) struct RestTranscriptionRequest<'a> {
    pub(super) local_request_id: Option<&'a str>,
    pub(super) local_scratch_owner: Option<&'a super::local_scratch::LocalScratchOwner>,
    pub(super) audio_path: &'a Path,
    pub(super) bearer_token: Option<&'a str>,
    pub(super) cancel: Option<Arc<AtomicBool>>,
    pub(super) endpoint: &'a str,
    pub(super) language: &'a str,
    pub(super) local_backend: Option<&'a str>,
    pub(super) max_initial_timestamp_seconds: Option<f64>,
    pub(super) model: &'a str,
    pub(super) timeout: Duration,
    pub(super) word_timestamps: bool,
}

pub(super) async fn transcribe_via_rest(
    request: RestTranscriptionRequest<'_>,
) -> Result<VerboseTranscription, TranscriptionError> {
    let mut http_diagnostics = crate::provider_http::HttpDiagnostics::new("transcription");
    let result: Result<_, TranscriptionError> = async {
        let RestTranscriptionRequest {
            local_request_id,
            local_scratch_owner,
            audio_path,
            bearer_token,
            cancel,
            endpoint,
            language,
            local_backend,
            max_initial_timestamp_seconds,
            model,
            timeout,
            word_timestamps,
        } = request;
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
        if let Some(id) = local_request_id {
            form = form.text("daydream_request_id", id.to_string());
        }
        if let Some(owner) = local_scratch_owner {
            form = form.text("daydream_scratch_request_id", owner.request_id.clone());
            if let Some(job_id) = &owner.job_id {
                form = form.text("daydream_scratch_job_id", job_id.clone());
            }
        }
        if word_timestamps {
            form = form.text("timestamp_granularities[]", "word");
        }
        if language != settings::DEFAULT_AI_TRANSCRIPTION_LANGUAGE {
            form = form.text("language", language.to_string());
        }
        if let Some(local_backend) = local_backend {
            form = form.text(LOCAL_BACKEND_FORM_FIELD, local_backend.to_string());
        }
        if let Some(max_initial_timestamp_seconds) = max_initial_timestamp_seconds {
            form = form.text(
                "max_initial_timestamp_seconds",
                max_initial_timestamp_seconds.to_string(),
            );
        }
        let mut request = crate::provider_http::client()
            .post(endpoint)
            .timeout(timeout)
            .multipart(form);
        if let Some(token) = bearer_token {
            request = request.bearer_auth(token);
        }
        http_diagnostics.stage("http_request");
        let response = await_cancellable(request.send(), cancel.clone())
            .await?
            .map_err(network_error)?;
        let status = response.status();
        http_diagnostics.response(
            status.as_u16(),
            response
                .headers()
                .get("x-request-id")
                .and_then(|v| v.to_str().ok()),
        );
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
                &format!(
                    "ai_transcription_remote_{}",
                    crate::provider_http::status_code(status)
                ),
                &format!("Transcription provider failed with status {status}."),
            ));
        }
        let body = await_cancellable(response.text(), cancel)
            .await?
            .map_err(network_error)?;
        http_diagnostics.bytes(body.len());
        http_diagnostics.stage("output_validation");
        parse_verbose_transcription(&body)
    }
    .await;
    http_diagnostics.finish(result.as_ref().err().map(|error| error.code.as_str()));
    result
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

pub(super) fn check_cancelled(cancel: Option<&AtomicBool>) -> Result<(), TranscriptionError> {
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
    // Only our local worker protocol has typed errors. Never surface arbitrary
    // provider content as an error code or log message.
    if !matches!(
        code,
        "ai_transcription_local_worker_crashed"
            | "ai_transcription_local_worker_unavailable"
            | "ai_transcription_local_gpu_failed"
            | "ai_transcription_local_model_unavailable"
            | "ai_transcription_cancelled"
    ) {
        return None;
    }
    Some(transcription_error(
        code,
        "The local transcription worker could not complete this section.",
    ))
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
