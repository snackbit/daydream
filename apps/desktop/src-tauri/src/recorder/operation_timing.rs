//! Capture timings contain operation names and elapsed time, never capture content.
use std::time::Instant;

pub(crate) struct OperationTimer {
    operation: &'static str,
    started: Instant,
}

impl OperationTimer {
    pub(crate) fn start(operation: &'static str) -> Self {
        Self {
            operation,
            started: Instant::now(),
        }
    }
}

impl Drop for OperationTimer {
    fn drop(&mut self) {
        tracing::debug!(
            operation = self.operation,
            elapsed_ms = self.started.elapsed().as_millis() as u64,
            "capture_operation_duration"
        );
    }
}

pub(crate) async fn run_blocking<T: Send + 'static>(
    operation: &'static str,
    work: impl FnOnce() -> Result<T, super::types::RecorderError> + Send + 'static,
) -> Result<T, super::types::RecorderError> {
    tauri::async_runtime::spawn_blocking(move || {
        let _timing = OperationTimer::start(operation);
        work()
    })
    .await
    .map_err(|_| {
        super::errors::capture_adapter_failed_error(
            "Capture operation stopped unexpectedly. Try again.",
        )
    })?
}
