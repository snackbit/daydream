use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};

use super::super::{errors::capture_adapter_failed_error, types::RecorderError};
use super::CapturedFrame;

#[derive(Debug)]
pub(crate) struct FrameSender {
    inner: SyncSender<CapturedFrame>,
}

#[derive(Debug)]
pub(crate) struct FrameReceiver {
    inner: Receiver<CapturedFrame>,
}

impl FrameSender {
    pub(crate) fn send(&self, frame: CapturedFrame) -> Result<(), RecorderError> {
        frame.validate()?;
        self.inner.send(frame).map_err(|_error| {
            capture_adapter_failed_error("Capture frame receiver is no longer available.")
        })
    }

    pub(crate) fn try_send(&self, frame: CapturedFrame) -> Result<(), FrameTrySendError> {
        frame.validate().map_err(FrameTrySendError::InvalidFrame)?;
        self.inner.try_send(frame).map_err(|error| match error {
            TrySendError::Full(frame) => FrameTrySendError::Full(frame),
            TrySendError::Disconnected(_frame) => FrameTrySendError::Disconnected,
        })
    }
}

#[derive(Debug)]
pub(crate) enum FrameTrySendError {
    Disconnected,
    Full(CapturedFrame),
    InvalidFrame(RecorderError),
}

impl Clone for FrameSender {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl FrameReceiver {
    #[cfg(test)]
    pub(crate) fn recv(&self) -> Result<CapturedFrame, mpsc::RecvError> {
        self.inner.recv()
    }

    pub(crate) fn recv_timeout(
        &self,
        timeout: std::time::Duration,
    ) -> Result<CapturedFrame, mpsc::RecvTimeoutError> {
        self.inner.recv_timeout(timeout)
    }
}

pub(crate) fn bounded_frame_channel(capacity: usize) -> (FrameSender, FrameReceiver) {
    let (sender, receiver) = mpsc::sync_channel(capacity);

    (
        FrameSender { inner: sender },
        FrameReceiver { inner: receiver },
    )
}
