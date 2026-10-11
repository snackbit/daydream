//! Operation metadata only; SDK errors and provider payloads must never be formatted.
use std::{future::Future, time::Instant};
use tracing::Instrument;

pub(super) fn stage(stage: &'static str) {
    tracing::Span::current().record("stage", stage);
}
pub(super) fn failure(
    stage_name: &'static str,
    reason: &'static str,
    code: &'static str,
) -> String {
    stage(stage_name);
    tracing::warn!(
        stage = stage_name,
        reason,
        error_code = code,
        "model_response_rejected"
    );
    code.into()
}
pub(super) fn json_failure(stage_name: &'static str, error: &serde_json::Error) -> String {
    stage(stage_name);
    tracing::warn!(stage = stage_name, category = ?error.classify(), line = error.line(), column = error.column(), "model_json_invalid");
    "model_response_invalid".into()
}
pub(super) fn provider_failure(value: &serde_json::Value) {
    let error = value
        .get("error")
        .or_else(|| value.pointer("/response/error"))
        .unwrap_or(value);
    let code = super::provider_error::known(value)
        .map(|(code, _)| code)
        .unwrap_or("unclassified");
    let parameter = match error["param"].as_str() {
        Some(
            parameter @ ("model"
            | "input"
            | "instructions"
            | "stream"
            | "store"
            | "temperature"
            | "max_tokens"
            | "max_completion_tokens"
            | "max_output_tokens"
            | "text"
            | "text.format"
            | "text.format.type"
            | "response_format"
            | "tools"
            | "tool_choice"
            | "image_url"),
        ) => Some(parameter),
        _ => None,
    };
    let shape = if value.get("detail").is_some() {
        "detail"
    } else if error.is_object() {
        "error_object"
    } else {
        "unrecognized"
    };
    tracing::warn!(
        provider_error_code = code,
        provider_parameter = parameter,
        response_shape = shape,
        "model_provider_rejected"
    );
}
pub(super) fn operation<T>(
    operation: &'static str,
    future: impl Future<Output = Result<T, String>>,
) -> impl Future<Output = Result<T, String>> {
    // SDK futures are large; keep nested diagnostic polls off the worker stack.
    let future = Box::pin(future);
    async move {
        let span = tracing::info_span!("model_operation", operation, operation_id = %uuid::Uuid::new_v4(), stage = tracing::field::Empty);
        async move {
        let started = Instant::now();
        let result = future.await;
        match &result {
            Ok(_) => tracing::debug!(elapsed_ms = started.elapsed().as_millis() as u64, "model_operation_succeeded"),
            Err(code) if code == "model_cancelled" => tracing::info!(elapsed_ms = started.elapsed().as_millis() as u64, "model_operation_cancelled"),
            Err(code) => tracing::warn!(error_code = %code, elapsed_ms = started.elapsed().as_millis() as u64, "model_operation_failed"),
        }
        result
    }.instrument(span).await
    }
}

pub(crate) use crate::telemetry::failures::provider_request_id;
