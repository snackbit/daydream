use std::collections::BTreeMap;
use std::sync::{
    mpsc::{self, Receiver, SyncSender, TrySendError},
    Arc, Condvar, Mutex,
};

use super::super::{errors::capture_adapter_failed_error, types::RecorderError};
use super::CapturedFrame;

#[derive(Debug)]
pub(crate) struct FrameSender {
    inner: Option<SyncSender<CapturedFrame>>,
    latest: Option<Arc<LatestFrames>>,
}

#[derive(Debug)]
pub(crate) struct FrameReceiver {
    inner: Option<Receiver<CapturedFrame>>,
    latest: Option<Arc<LatestFrames>>,
}

#[derive(Debug)]
struct LatestFrames {
    state: Mutex<LatestState>,
    ready: Condvar,
    max_sources: usize,
}
#[derive(Debug)]
struct LatestState {
    frames: BTreeMap<String, CapturedFrame>,
    senders: usize,
    receiver_alive: bool,
}

impl FrameSender {
    pub(crate) fn send(&self, frame: CapturedFrame) -> Result<(), RecorderError> {
        frame.validate()?;
        if let Some(latest) = &self.latest {
            return store_latest(latest, frame);
        }
        self.inner
            .as_ref()
            .expect("bounded sender")
            .send(frame)
            .map_err(|_error| {
                capture_adapter_failed_error("Capture frame receiver is no longer available.")
            })
    }

    pub(crate) fn try_send(&self, frame: CapturedFrame) -> Result<(), FrameTrySendError> {
        frame.validate().map_err(FrameTrySendError::InvalidFrame)?;
        if let Some(latest) = &self.latest {
            let Ok(mut state) = latest.state.try_lock() else {
                return Err(FrameTrySendError::Full(frame));
            };
            if !state.receiver_alive {
                return Err(FrameTrySendError::Disconnected);
            }
            if !state.frames.contains_key(&frame.source_id)
                && state.frames.len() >= latest.max_sources
            {
                return Err(FrameTrySendError::Full(frame));
            }
            state.frames.insert(frame.source_id.clone(), frame);
            latest.ready.notify_one();
            return Ok(());
        }
        self.inner
            .as_ref()
            .expect("bounded sender")
            .try_send(frame)
            .map_err(|error| match error {
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
            latest: self.latest.as_ref().map(|latest| {
                latest.state.lock().expect("frame channel").senders += 1;
                latest.clone()
            }),
        }
    }
}

impl FrameReceiver {
    #[cfg(test)]
    pub(crate) fn recv(&self) -> Result<CapturedFrame, mpsc::RecvError> {
        if let Some(inner) = &self.inner {
            return inner.recv();
        }
        loop {
            match self.recv_timeout(std::time::Duration::from_secs(60)) {
                Ok(frame) => return Ok(frame),
                Err(mpsc::RecvTimeoutError::Disconnected) => return Err(mpsc::RecvError),
                Err(mpsc::RecvTimeoutError::Timeout) => {}
            }
        }
    }

    pub(crate) fn recv_timeout(
        &self,
        timeout: std::time::Duration,
    ) -> Result<CapturedFrame, mpsc::RecvTimeoutError> {
        if let Some(inner) = &self.inner {
            return inner.recv_timeout(timeout);
        }
        let latest = self.latest.as_ref().expect("latest receiver");
        let deadline = std::time::Instant::now() + timeout;
        let mut state = latest
            .state
            .lock()
            .map_err(|_| mpsc::RecvTimeoutError::Disconnected)?;
        loop {
            if let Some((_, frame)) = state.frames.pop_first() {
                return Ok(frame);
            }
            if state.senders == 0 {
                return Err(mpsc::RecvTimeoutError::Disconnected);
            }
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            if remaining.is_zero() {
                return Err(mpsc::RecvTimeoutError::Timeout);
            }
            state = latest
                .ready
                .wait_timeout(state, remaining)
                .map_err(|_| mpsc::RecvTimeoutError::Disconnected)?
                .0;
        }
    }
}

pub(crate) fn bounded_frame_channel(capacity: usize) -> (FrameSender, FrameReceiver) {
    let (sender, receiver) = mpsc::sync_channel(capacity);

    (
        FrameSender {
            inner: Some(sender),
            latest: None,
        },
        FrameReceiver {
            inner: Some(receiver),
            latest: None,
        },
    )
}

pub(crate) fn latest_frame_channel(max_sources: usize) -> (FrameSender, FrameReceiver) {
    let latest = Arc::new(LatestFrames {
        state: Mutex::new(LatestState {
            frames: BTreeMap::new(),
            senders: 1,
            receiver_alive: true,
        }),
        ready: Condvar::new(),
        max_sources: max_sources.max(1),
    });
    (
        FrameSender {
            inner: None,
            latest: Some(latest.clone()),
        },
        FrameReceiver {
            inner: None,
            latest: Some(latest),
        },
    )
}
fn store_latest(latest: &LatestFrames, frame: CapturedFrame) -> Result<(), RecorderError> {
    let mut state = latest
        .state
        .lock()
        .map_err(|_| capture_adapter_failed_error("Frame channel unavailable."))?;
    if !state.receiver_alive
        || (!state.frames.contains_key(&frame.source_id)
            && state.frames.len() >= latest.max_sources)
    {
        return Err(capture_adapter_failed_error("Frame channel unavailable."));
    }
    state.frames.insert(frame.source_id.clone(), frame);
    latest.ready.notify_one();
    Ok(())
}
impl Drop for FrameSender {
    fn drop(&mut self) {
        if let Some(latest) = &self.latest {
            if let Ok(mut state) = latest.state.lock() {
                state.senders = state.senders.saturating_sub(1);
                latest.ready.notify_all();
            }
        }
    }
}
impl Drop for FrameReceiver {
    fn drop(&mut self) {
        if let Some(latest) = &self.latest {
            if let Ok(mut state) = latest.state.lock() {
                state.receiver_alive = false;
                state.frames.clear();
                latest.ready.notify_all();
            }
        }
    }
}
