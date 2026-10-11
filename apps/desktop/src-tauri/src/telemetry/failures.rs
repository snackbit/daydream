//! Closed frontend metadata boundary. Never accept error messages or arbitrary context.
use serde::Deserialize;
use std::sync::OnceLock;

#[derive(Deserialize)]
struct Policy {
    operations: Vec<String>,
    #[serde(rename = "errorCodes")]
    error_codes: Vec<String>,
}
fn policy() -> &'static Policy {
    static POLICY: OnceLock<Policy> = OnceLock::new();
    POLICY.get_or_init(|| {
        serde_json::from_str(include_str!(
            "../../../../../packages/shared/src/diagnostics/failure-policy.json"
        ))
        .expect("bundled failure metadata policy")
    })
}
pub(crate) fn safe_error_code(code: &str) -> &str {
    if policy().error_codes.iter().any(|allowed| allowed == code) {
        code
    } else {
        "unclassified"
    }
}
pub(crate) fn provider_request_id(value: Option<&str>) -> Option<&str> {
    value.filter(|value| {
        uuid::Uuid::parse_str(value).is_ok()
            || value.strip_prefix("req_").is_some_and(|id| {
                id.len() <= 64 && !id.is_empty() && id.bytes().all(|c| c.is_ascii_hexdigit())
            })
    })
}
pub(super) fn install_panic_logging() {
    std::panic::set_hook(Box::new(|info| {
        let location = info.location();
        tracing::error!(
            source_file = location.and_then(|location| location.file().rsplit('/').next()),
            line = location.map(|location| location.line()),
            column = location.map(|location| location.column()),
            "native_panic"
        );
    }));
}
#[derive(Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Stage {
    Invoke,
    Decode,
    Subscribe,
    Callback,
    Execute,
    Render,
    UnhandledRejection,
    Resource,
}
#[derive(Deserialize, Debug)]
pub(crate) enum ErrorKind {
    Error,
    TypeError,
    RangeError,
    SyntaxError,
    ReferenceError,
    #[serde(rename = "native-code")]
    NativeCode,
    #[serde(rename = "other")]
    Other,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct FrontendFailureInput {
    request_id: String,
    operation: String,
    stage: Stage,
    error_code: Option<String>,
    error_kind: ErrorKind,
    elapsed_ms: Option<u64>,
}
fn validate(input: &FrontendFailureInput) -> Result<uuid::Uuid, &'static str> {
    let id = uuid::Uuid::parse_str(&input.request_id).map_err(|_| "diagnostic_metadata_invalid")?;
    if !policy().operations.contains(&input.operation)
        || input
            .error_code
            .as_ref()
            .is_some_and(|code| !policy().error_codes.contains(code))
        || input.elapsed_ms.is_some_and(|ms| ms > 86400000)
    {
        return Err("diagnostic_metadata_invalid");
    }
    Ok(id)
}
#[tauri::command]
pub(crate) fn log_frontend_failure(input: FrontendFailureInput) -> Result<(), String> {
    let request_id = validate(&input).map_err(|code| {
        tracing::warn!(error_code = code, "frontend_diagnostic_rejected");
        code.to_owned()
    })?;
    let cancelled = matches!(
        input.error_code.as_deref(),
        Some("model_cancelled" | "ai_processing_cancelled" | "recall_cancelled")
    );
    if cancelled {
        tracing::info!(%request_id, operation = %input.operation, stage = ?input.stage, error_code = input.error_code.as_deref(), elapsed_ms = input.elapsed_ms, "frontend_operation_cancelled");
    } else if matches!(input.stage, Stage::Render | Stage::UnhandledRejection) {
        tracing::error!(%request_id, operation = %input.operation, stage = ?input.stage, error_code = input.error_code.as_deref(), error_kind = ?input.error_kind, elapsed_ms = input.elapsed_ms, "frontend_operation_failed");
    } else {
        tracing::warn!(%request_id, operation = %input.operation, stage = ?input.stage, error_code = input.error_code.as_deref(), error_kind = ?input.error_kind, elapsed_ms = input.elapsed_ms, "frontend_operation_failed");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn frontend_failures_reach_the_native_log_without_untrusted_context() {
        let capture = crate::telemetry::test_support::LogCapture::default();
        let request = uuid::Uuid::new_v4();
        let input: FrontendFailureInput = serde_json::from_value(serde_json::json!({"requestId":request.to_string(),"operation":"saveAppSettings","stage":"invoke","errorCode":"model_auth_failed","errorKind":"native-code","elapsedMs":12})).unwrap();
        tracing::subscriber::with_default(capture.subscriber(), || {
            log_frontend_failure(input).unwrap();
            let input = serde_json::from_value(serde_json::json!({"requestId":request.to_string(),"operation":"synthetic-private-operation","stage":"invoke","errorCode":null,"errorKind":"other","elapsedMs":12})).unwrap();
            assert!(log_frontend_failure(input).is_err());
        });
        let logs = capture.text();
        assert!(logs.contains("frontend_operation_failed"));
        assert!(logs.contains("saveAppSettings"));
        assert!(logs.contains(&request.to_string()));
        assert!(logs.contains("model_auth_failed"));
        assert!(logs.contains("frontend_diagnostic_rejected"));
        assert!(!logs.contains("synthetic-private"));
    }
    #[test]
    fn frontend_metadata_rejects_arbitrary_content_and_unknown_fields() {
        let base = serde_json::json!({"requestId":uuid::Uuid::new_v4().to_string(),"operation":"saveAppSettings","stage":"invoke","errorCode":null,"errorKind":"other","elapsedMs":12});
        let input = serde_json::from_value(base.clone()).unwrap();
        assert!(validate(&input).is_ok());
        for (field, value) in [
            ("operation", serde_json::json!("synthetic_private_content")),
            ("errorCode", serde_json::json!("synthetic_private_content")),
            ("requestId", serde_json::json!("synthetic_private_content")),
            ("elapsedMs", serde_json::json!(86400001)),
        ] {
            let mut value_json = base.clone();
            value_json[field] = value;
            assert!(validate(&serde_json::from_value(value_json).unwrap()).is_err());
        }
        let mut value = base;
        value["message"] = serde_json::json!("synthetic-private-content");
        assert!(serde_json::from_value::<FrontendFailureInput>(value).is_err());
    }
}
