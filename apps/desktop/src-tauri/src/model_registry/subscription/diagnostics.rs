//! Authentication metadata only. Never record URLs, bodies, identities or credentials.
use std::{future::Future, time::Instant};
use tracing::Instrument;

pub(crate) fn operation<T>(
    operation: &'static str,
    subscription_id: Option<&str>,
    request_id: Option<&str>,
    future: impl Future<Output = Result<T, String>>,
) -> impl Future<Output = Result<T, String>> {
    // Bound stack use when commands nest authentication and SDK operations.
    let future = Box::pin(future);
    // IPC identifiers are untrusted until validated; log only opaque UUIDs.
    let subscription_id = subscription_id.and_then(|id| uuid::Uuid::parse_str(id).ok());
    let request_id = request_id.and_then(|id| uuid::Uuid::parse_str(id).ok());
    async move {
        let span = tracing::info_span!("chatgpt_operation", operation,
        subscription_id = %subscription_id.map(|id| id.to_string()).unwrap_or_default(),
        request_id = %request_id.map(|id| id.to_string()).unwrap_or_default(),
        stage = tracing::field::Empty);
        async {
        let mut attempt = Attempt { started: Instant::now(), finished: false };
        tracing::info!("chatgpt_operation_started");
        let result = future.await;
        attempt.finished = true;
        // These are Daydream's stable error codes, never external error messages.
        match &result {
            Ok(_) => tracing::info!(elapsed_ms = attempt.started.elapsed().as_millis() as u64, "chatgpt_operation_succeeded"),
            Err(code) => tracing::warn!(error_code = %code, elapsed_ms = attempt.started.elapsed().as_millis() as u64, "chatgpt_operation_failed"),
        }
        result
    }.instrument(span).await
    }
}

struct Attempt {
    started: Instant,
    finished: bool,
}
impl Drop for Attempt {
    fn drop(&mut self) {
        if !self.finished {
            tracing::warn!(
                elapsed_ms = self.started.elapsed().as_millis() as u64,
                "chatgpt_operation_aborted"
            );
        }
    }
}

pub(super) fn stage(stage: &'static str) {
    tracing::Span::current().record("stage", stage);
    tracing::info!(stage, "chatgpt_operation_stage");
}

pub(super) fn transport_error(error: &reqwest::Error) -> &'static str {
    let category = if error.is_timeout() {
        "timeout"
    } else if error.is_connect() {
        "connect"
    } else if error.is_body() {
        "body"
    } else if error.is_decode() {
        "decode"
    } else {
        "request"
    };
    tracing::warn!(category, "chatgpt_transport_failed");
    if error.is_timeout() {
        "model_timeout"
    } else {
        "model_connection_failed"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::Write,
        sync::{Arc, Mutex},
    };
    use tracing::instrument::WithSubscriber;

    #[derive(Clone)]
    struct Sink(Arc<Mutex<Vec<u8>>>);
    impl Write for Sink {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn failure_trace_identifies_attempt_and_stage_without_private_arguments() {
        let bytes = Arc::new(Mutex::new(Vec::new()));
        let sink = Sink(bytes.clone());
        let subscriber = tracing_subscriber::fmt()
            .json()
            .without_time()
            .with_writer(move || sink.clone())
            .finish();
        let request = uuid::Uuid::new_v4().to_string();
        let result = operation(
            "sign_in",
            Some("synthetic-private-identity"),
            Some(&request),
            async {
                stage("registration_validate");
                Err::<(), String>("model_subscription_registration_missing".into())
            },
        )
        .with_subscriber(subscriber)
        .await;
        assert!(result.is_err());
        let logs = String::from_utf8(bytes.lock().unwrap().clone()).unwrap();
        assert!(logs.contains(&request));
        assert!(logs.contains("registration_validate"));
        assert!(logs.contains("model_subscription_registration_missing"));
        assert!(!logs.contains("synthetic-private-identity"));
    }

    #[tokio::test]
    async fn token_exchange_logs_status_and_allowlisted_error_without_provider_body() {
        let bytes = Arc::new(Mutex::new(Vec::new()));
        let sink = Sink(bytes.clone());
        let subscriber = tracing_subscriber::fmt()
            .json()
            .without_time()
            .with_writer(move || sink.clone())
            .finish();
        let response = reqwest::Response::from(::http::Response::builder().status(400)
            .body(r#"{"error":"invalid_grant","error_description":"synthetic-private-code","access_token":"synthetic-private-access","refresh_token":"synthetic-private-refresh"}"#).unwrap());
        let result = operation("sign_in", None, None, async {
            stage("token_exchange");
            super::super::bounded_json(response).await
        })
        .with_subscriber(subscriber)
        .await;
        assert_eq!(result.unwrap_err(), "model_subscription_request_failed");
        let logs = String::from_utf8(bytes.lock().unwrap().clone()).unwrap();
        assert!(logs.contains("token_exchange"));
        assert!(logs.contains("invalid_grant"));
        assert!(logs.contains("400"));
        assert!(!logs.contains("synthetic-private"));
    }
}
