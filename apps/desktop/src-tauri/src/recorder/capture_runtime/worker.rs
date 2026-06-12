use std::thread;

use super::super::{errors::capture_cleanup_failed_error, types::RecorderError};

#[derive(Debug)]
pub(crate) struct RuntimeWorker {
    name: &'static str,
    handle: thread::JoinHandle<Result<(), RecorderError>>,
}

impl RuntimeWorker {
    pub(crate) fn spawn(
        name: &'static str,
        run: impl FnOnce() -> Result<(), RecorderError> + Send + 'static,
    ) -> Self {
        Self {
            name,
            handle: thread::spawn(run),
        }
    }

    pub(crate) fn join(self) -> Result<(), RecorderError> {
        match self.handle.join() {
            Ok(Ok(())) => Ok(()),
            Ok(Err(error)) => Err(error),
            Err(_error) => Err(capture_cleanup_failed_error(format!(
                "Capture worker {} panicked during cleanup.",
                self.name
            ))),
        }
    }
}
