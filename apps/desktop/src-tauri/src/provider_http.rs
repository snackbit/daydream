use std::sync::OnceLock;
use std::time::Instant;

/// Owns one HTTP attempt's metadata. Never format a reqwest error or body here.
pub(crate) struct HttpDiagnostics {
    operation: &'static str,
    request_id: uuid::Uuid,
    started: Instant,
    stage: &'static str,
    http_status: Option<u16>,
    response_bytes: usize,
    finished: bool,
}
impl HttpDiagnostics {
    pub(crate) fn new(operation: &'static str) -> Self {
        Self {
            operation,
            request_id: uuid::Uuid::new_v4(),
            started: Instant::now(),
            stage: "request_build",
            http_status: None,
            response_bytes: 0,
            finished: false,
        }
    }
    pub(crate) fn stage(&mut self, stage: &'static str) {
        self.stage = stage;
    }
    pub(crate) fn bytes(&mut self, bytes: usize) {
        self.response_bytes = bytes;
    }
    pub(crate) fn response(&mut self, status: u16, request_id: Option<&str>) {
        self.http_status = Some(status);
        self.stage = "http_response";
        if !(200..300).contains(&status) {
            tracing::warn!(operation = self.operation, request_id = %self.request_id, http_status = status, provider_request_id = crate::telemetry::failures::provider_request_id(request_id), elapsed_ms = self.started.elapsed().as_millis() as u64, "http_response_failed");
        }
    }
    pub(crate) fn finish(&mut self, code: Option<&str>) {
        self.finished = true;
        let elapsed_ms = self.started.elapsed().as_millis() as u64;
        match code {
            Some(
                "model_cancelled"
                | "ai_processing_cancelled"
                | "recall_cancelled"
                | "ai_transcription_cancelled",
            ) => {
                tracing::info!(operation = self.operation, request_id = %self.request_id, stage = self.stage, elapsed_ms, "http_operation_cancelled")
            }
            Some(code) => {
                tracing::warn!(operation = self.operation, request_id = %self.request_id, stage = self.stage, http_status = self.http_status, response_bytes = self.response_bytes, error_code = crate::telemetry::failures::safe_error_code(code), elapsed_ms, "http_operation_failed")
            }
            None => {
                tracing::debug!(operation = self.operation, request_id = %self.request_id, http_status = self.http_status, response_bytes = self.response_bytes, elapsed_ms, "http_operation_succeeded")
            }
        }
    }
}
impl Drop for HttpDiagnostics {
    fn drop(&mut self) {
        if !self.finished {
            tracing::info!(operation = self.operation, request_id = %self.request_id, stage = self.stage, http_status = self.http_status, elapsed_ms = self.started.elapsed().as_millis() as u64, "http_operation_aborted");
        }
    }
}
pub(crate) fn client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(reqwest::Client::new)
}
pub(crate) fn status_code(status: reqwest::StatusCode) -> &'static str {
    match status.as_u16() {
        401 | 403 => "auth_failed",
        404 => "model_not_found",
        408 | 504 => "timeout",
        429 => "rate_limited",
        500..=599 => "server_failed",
        400 | 413 | 415 | 422 => "request_invalid",
        _ => "status_failed",
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn failed_http_attempts_record_status_stage_and_timing_without_external_text() {
        let capture = crate::telemetry::test_support::LogCapture::default();
        tracing::subscriber::with_default(capture.subscriber(), || {
            let mut attempt = HttpDiagnostics::new("transcription");
            attempt.response(503, Some("synthetic-private-header"));
            attempt.stage("body_read");
            attempt.bytes(42);
            attempt.finish(Some("synthetic-private-provider-code"));
        });
        let logs = capture.text();
        assert!(logs.contains("http_response_failed"));
        assert!(logs.contains("http_operation_failed"));
        assert!(logs.contains("body_read"));
        assert!(logs.contains("503"));
        assert!(logs.contains("response_bytes"));
        assert!(logs.contains("elapsed_ms"));
        assert!(logs.contains("unclassified"));
        assert!(!logs.contains("synthetic-private"));
    }
}
