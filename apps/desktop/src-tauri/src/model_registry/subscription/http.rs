//! Bounded OAuth/account HTTP responses with allowlisted error metadata.
use super::diagnostics;
use serde_json::Value;
use std::time::Duration;

pub(crate) fn http() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|error| diagnostics::transport_error(&error).into())
}

fn oauth_error(value: &Value) -> &'static str {
    match value["error"].as_str() {
        Some("invalid_grant") => "invalid_grant",
        Some("invalid_client") => "invalid_client",
        Some("invalid_request") => "invalid_request",
        Some("invalid_scope") => "invalid_scope",
        Some("access_denied") => "access_denied",
        Some("unauthorized_client") => "unauthorized_client",
        Some("unsupported_grant_type") => "unsupported_grant_type",
        Some("server_error") => "server_error",
        Some("temporarily_unavailable") => "temporarily_unavailable",
        _ => "unclassified",
    }
}

pub(crate) async fn bounded_json(mut response: reqwest::Response) -> Result<Value, String> {
    let mut http_diagnostics = crate::provider_http::HttpDiagnostics::new("chatgpt_account_http");
    let result = async {
    let status = response.status();
    http_diagnostics.response(status.as_u16(), response.headers().get("x-request-id").and_then(|v| v.to_str().ok()));
    tracing::info!(http_status = status.as_u16(), "chatgpt_http_response");
    let failure = match status.as_u16() {
        401 | 403 => "model_subscription_reconnect",
        429 => "model_subscription_usage_limit",
        _ => "model_subscription_request_failed",
    };
    let mut bytes = Vec::new();
    http_diagnostics.stage("body_read");
    let limit = if status.is_success() {
        1024 * 1024
    } else {
        65536
    };
    while let Some(chunk) = response.chunk().await.map_err(|error| {
        let code = diagnostics::transport_error(&error);
        if status.is_success() {
            code
        } else {
            failure
        }
    })? {
        if bytes.len() + chunk.len() > limit {
            tracing::warn!("chatgpt_http_response_limit");
            return Err(if status.is_success() {
                "model_response_limit"
            } else {
                failure
            }
            .into());
        }
        bytes.extend_from_slice(&chunk);
        http_diagnostics.bytes(bytes.len());
    }
    let value = serde_json::from_slice::<Value>(&bytes);
    if !status.is_success() {
        tracing::warn!(
            http_status = status.as_u16(),
            oauth_error = value.as_ref().map(oauth_error).unwrap_or("invalid_json"),
            "chatgpt_http_request_failed"
        );
        return Err(failure.into());
    }
    http_diagnostics.stage("http_json");
    value.map_err(|error| {
        tracing::warn!(category = ?error.classify(), line = error.line(), column = error.column(), "chatgpt_json_invalid");
        "model_response_invalid".to_owned()
    })
    }.await;
    http_diagnostics.finish(result.as_ref().err().map(String::as_str));
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn error_metadata_never_echoes_provider_values() {
        assert_eq!(
            oauth_error(
                &serde_json::json!({"error":"invalid_grant","error_description":"synthetic-private"})
            ),
            "invalid_grant"
        );
        assert_eq!(
            oauth_error(&serde_json::json!({"error":"synthetic-private"})),
            "unclassified"
        );
    }
}
