//! One SDK boundary. Never expose SDK errors: they can contain provider bodies.
use super::types::{Protocol, Route, TokenLimit};
use async_openai::{config::Config, error::OpenAIError, middleware::HttpRequestFactory, Client};
use futures_util::StreamExt;
use model_http::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use serde_json::{json, Value};
use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};
pub(crate) mod diagnostics;
mod provider_error;
mod response;
#[cfg(test)]
mod tests;

struct Configuration {
    base: String,
    key: secrecy::SecretString,
}
impl Config for Configuration {
    fn headers(&self) -> HeaderMap {
        use secrecy::ExposeSecret;
        let mut headers = HeaderMap::new();
        if !self.key.expose_secret().is_empty() {
            if let Ok(mut value) =
                HeaderValue::from_str(&format!("Bearer {}", self.key.expose_secret()))
            {
                value.set_sensitive(true);
                headers.insert(AUTHORIZATION, value);
            }
        }
        headers
    }
    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base.trim_end_matches('/'))
    }
    fn query(&self) -> Vec<(&str, &str)> {
        vec![]
    }
    fn api_base(&self) -> &str {
        &self.base
    }
    fn api_key(&self) -> &secrecy::SecretString {
        &self.key
    }
}

fn safe_error(code: &str) -> OpenAIError {
    OpenAIError::InvalidArgument(code.into())
}
fn status_code(status: u16) -> &'static str {
    match status {
        401 | 403 => "model_auth_failed",
        404 => "model_not_found",
        429 => "model_rate_limited",
        408 | 504 => "model_timeout",
        500..=599 => "model_server_failed",
        _ => "model_request_invalid",
    }
}
use provider_error::app_code as subscription_failure;
pub(crate) fn feature_error(code: &str, prefix: &str) -> String {
    if code == "model_cancelled" {
        return if prefix.starts_with("recall_") {
            "recall_cancelled"
        } else {
            "ai_processing_cancelled"
        }
        .into();
    }
    format!(
        "{prefix}_{}",
        code.strip_prefix("model_").unwrap_or("response_invalid")
    )
}
fn client(base: &str, key: Option<&str>, limit: usize) -> Result<Client<Configuration>, String> {
    if let Some(key) = key.filter(|key| !key.is_empty()) {
        HeaderValue::from_str(&format!("Bearer {key}")).map_err(|_| "model_auth_failed")?;
    }
    let http = model_http::Client::builder()
        .redirect(model_http::redirect::Policy::none())
        .timeout(Duration::from_secs(90))
        .build()
        .map_err(|_| "model_client_failed")?;
    let service = tower::service_fn(move |factory: HttpRequestFactory| {
        let http = http.clone();
        async move {
            let mut http_diagnostics = crate::provider_http::HttpDiagnostics::new("model_http");
            let result: Result<model_http::Response, OpenAIError> = async {
                let request = factory.build().await?;
                http_diagnostics.stage("http_request");
                let response = http.execute(request).await.map_err(|error| {
                    tracing::warn!(
                        timeout = error.is_timeout(),
                        connect = error.is_connect(),
                        "model_transport_failed"
                    );
                    safe_error(if error.is_timeout() {
                        "model_timeout"
                    } else {
                        "model_connection_failed"
                    })
                })?;
                let status = response.status();
                http_diagnostics.response(
                    status.as_u16(),
                    response
                        .headers()
                        .get("x-request-id")
                        .and_then(|v| v.to_str().ok()),
                );
                diagnostics::stage("http_response");
                let provider_request_id = diagnostics::provider_request_id(
                    response
                        .headers()
                        .get("x-request-id")
                        .and_then(|v| v.to_str().ok()),
                )
                .map(str::to_owned);
                // Reject before the SDK's error-body logging or automatic retry layer.
                let content_type = response
                    .headers()
                    .get("content-type")
                    .and_then(|value| value.to_str().ok())
                    .and_then(|value| value.split(';').next());
                let sse = content_type
                    .is_some_and(|value| value.trim().eq_ignore_ascii_case("text/event-stream"));
                let response_kind = if sse {
                    "sse"
                } else if content_type
                    .is_some_and(|value| value.trim().eq_ignore_ascii_case("application/json"))
                {
                    "json"
                } else {
                    "other"
                };
                tracing::info!(
                    http_status = status.as_u16(),
                    provider_request_id,
                    response_kind,
                    "model_http_response"
                );
                let mut bytes = Vec::new();
                http_diagnostics.stage("body_read");
                let mut stream = response.bytes_stream();
                while let Some(chunk) = stream.next().await {
                    let chunk = chunk.map_err(|_| {
                        safe_error(&diagnostics::failure(
                            "body_read",
                            "body_read_failed",
                            "model_response_failed",
                        ))
                    })?;
                    if bytes.len().saturating_add(chunk.len())
                        > if status.is_success() { limit } else { 65536 }
                    {
                        return Err(safe_error(&diagnostics::failure(
                            "body_read",
                            "body_limit",
                            if status.is_success() {
                                "model_response_limit"
                            } else {
                                status_code(status.as_u16())
                            },
                        )));
                    }
                    bytes.extend_from_slice(&chunk);
                    http_diagnostics.bytes(bytes.len());
                }
                if !status.is_success() {
                    let value = serde_json::from_slice::<Value>(&bytes).ok();
                    if let Some(value) = &value {
                        diagnostics::provider_failure(value);
                    }
                    let code = value
                        .as_ref()
                        .and_then(subscription_failure)
                        .unwrap_or_else(|| status_code(status.as_u16()));
                    tracing::warn!(
                        http_status = status.as_u16(),
                        error_code = code,
                        "model_http_request_failed"
                    );
                    return Err(safe_error(code));
                }
                if sse {
                    http_diagnostics.stage("sse_decode");
                    bytes = response::normalize_sse(bytes)
                        .await
                        .map_err(|code| safe_error(&code))?;
                } else {
                    http_diagnostics.stage("http_json");
                    serde_json::from_slice::<Value>(&bytes).map_err(|error| {
                        safe_error(&diagnostics::json_failure("http_json", &error))
                    })?;
                }
                Ok::<_, OpenAIError>(
                    http::Response::builder()
                        .status(status)
                        .header(
                            "content-type",
                            if sse {
                                "text/event-stream"
                            } else {
                                "application/json"
                            },
                        )
                        .body(bytes)
                        .map_err(|_| safe_error("model_response_invalid"))?
                        .into(),
                )
            }
            .await;
            http_diagnostics.finish(result.as_ref().err().map(|error| match error {
                OpenAIError::InvalidArgument(code) => code.as_str(),
                _ => "model_response_invalid",
            }));
            result
        }
    });
    Ok(Client::with_config(Configuration {
        base: base.into(),
        key: key.unwrap_or_default().to_owned().into(),
    })
    .with_http_service(service))
}
fn error_code(error: OpenAIError) -> String {
    match error {
        OpenAIError::InvalidArgument(code) if code.starts_with("model_") => code,
        _ => diagnostics::failure("sdk_decode", "sdk_error", "model_response_invalid"),
    }
}

pub(crate) async fn cancellable<T, F: std::future::Future<Output = Result<T, String>>>(
    future: F,
    cancelled: &AtomicBool,
) -> Result<T, String> {
    if cancelled.load(Ordering::Acquire) {
        return Err("model_cancelled".into());
    }
    tokio::pin!(future);
    loop {
        tokio::select! { biased;
            _=tokio::time::sleep(Duration::from_millis(25)) => if cancelled.load(Ordering::Acquire) { return Err("model_cancelled".into()); },
            result=&mut future => return result,
        }
    }
}

pub(crate) async fn embeddings(
    base: &str,
    key: Option<&str>,
    body: Value,
    cancelled: &AtomicBool,
) -> Result<Value, String> {
    diagnostics::operation("embeddings", async {
        let client = client(base, key, 4 * 1024 * 1024)?;
        cancellable(
            async {
                client
                    .embeddings()
                    .create_byot(body)
                    .await
                    .map_err(error_code)
            },
            cancelled,
        )
        .await
    })
    .await
}

pub(crate) async fn authenticated_completion(
    storage: Option<&crate::storage::AppStorage>,
    base: &str,
    key: Option<&str>,
    route: Option<&Route>,
    body: Value,
    limit: usize,
    cancelled: &AtomicBool,
) -> Result<Value, String> {
    diagnostics::operation("completion", async {
        diagnostics::stage("route_validation");
        if route.is_some_and(|route| route.subscription || route.subscription_id.is_some())
            && (base != "https://api.openai.com/v1"
                || route.is_none_or(|route| route.protocol != Protocol::Responses))
        {
            return Err("model_subscription_invalid".into());
        }
        if route.is_some_and(|route| route.subscription && route.subscription_id.is_none()) {
            return Err("model_subscription_reconnect".into());
        }
        if let Some(id) = route.and_then(|route| route.subscription_id.as_deref()) {
            let lease = cancellable(
                super::subscription::acquire(storage.ok_or("model_subscription_reconnect")?, id),
                cancelled,
            )
            .await?;
            return cancellable(
                completion(base, Some(&lease.key), route, body, limit, &lease.cancelled),
                cancelled,
            )
            .await;
        }
        completion(base, key, route, body, limit, cancelled).await
    })
    .await
}

/// Accept the established message contract; retain feature-specific output validation.
pub(crate) async fn completion(
    base: &str,
    key: Option<&str>,
    route: Option<&Route>,
    body: Value,
    limit: usize,
    cancelled: &AtomicBool,
) -> Result<Value, String> {
    diagnostics::stage("request_build");
    let responses = route.is_some_and(|route| route.protocol == Protocol::Responses);
    let subscription = route.is_some_and(|route| route.subscription_id.is_some());
    let client = client(
        base,
        key,
        if subscription {
            limit.saturating_mul(32).min(32 * 1024 * 1024)
        } else {
            limit
        },
    )?;
    let body = wire_body(body, route)?;
    cancellable(
        async {
            if !responses {
                return client.chat().create_byot(body).await.map_err(error_code);
            }
            let text = if subscription {
                let mut stream = client
                    .responses()
                    .create_stream_byot::<_, Value>(body)
                    .await
                    .map_err(error_code)?;
                let mut output = response::StreamOutput::default();
                while let Some(event) = stream.next().await {
                    let event = event.map_err(error_code)?;
                    output.accept(event, limit)?;
                }
                output.finish(limit)?
            } else {
                let response: Value = client
                    .responses()
                    .create_byot(body)
                    .await
                    .map_err(error_code)?;
                response::completed_text(&response, limit, false)?
            };
            Ok(response::envelope(text))
        },
        cancelled,
    )
    .await
}

pub(crate) fn wire_body(mut body: Value, route: Option<&Route>) -> Result<Value, String> {
    body.as_object_mut()
        .ok_or("model_request_invalid")?
        .remove("temperature");
    if !route.is_some_and(|route| route.protocol == Protocol::Responses) {
        if route.is_some_and(|route| route.token_limit == TokenLimit::Modern) {
            if let Some(limit) = body.as_object_mut().unwrap().remove("max_tokens") {
                body["max_completion_tokens"] = limit;
            }
        }
        return Ok(body);
    }
    let mut instructions = Vec::new();
    let mut input = Vec::new();
    for message in body["messages"].as_array().ok_or("model_request_invalid")? {
        if message["role"] == "system" {
            instructions.push(message["content"].as_str().ok_or("model_request_invalid")?);
            continue;
        }
        let content = if let Some(text) = message["content"].as_str() {
            vec![json!({"type":"input_text","text":text})]
        } else {
            message["content"]
                .as_array()
                .ok_or("model_request_invalid")?
                .iter()
                .map(|part| match part["type"].as_str() {
                    Some("text") => Ok(json!({"type":"input_text","text":part["text"]})),
                    Some("image_url") => {
                        Ok(json!({"type":"input_image","image_url":part["image_url"]["url"]}))
                    }
                    _ => Err("model_request_invalid".to_owned()),
                })
                .collect::<Result<Vec<_>, _>>()?
        };
        input.push(json!({"role":message["role"],"content":content}));
    }
    let mut output = json!({"model":body["model"],"input":input,"instructions":instructions.join("\n"),"store":false});
    if route.is_some_and(|route| route.subscription_id.is_some()) {
        output["stream"] = json!(true);
    } else {
        if let Some(limit) = body.get("max_tokens") {
            output["max_output_tokens"] = limit.clone();
        }
    }
    if body["response_format"]["type"] == "json_object" {
        output["text"] = json!({"format":{"type":"json_object"}});
    }
    Ok(output)
}
