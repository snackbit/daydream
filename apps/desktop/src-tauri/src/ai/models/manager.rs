use super::super::*;

pub struct AiModelManager {
    inner: Mutex<AiModelManagerState>,
}

#[derive(Default)]
pub(in crate::ai) struct AiModelManagerState {
    active: Option<String>,
    cancelled: HashSet<String>,
    queued: VecDeque<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub(in crate::ai) enum DownloadRequestState {
    AlreadyRunning,
    Queued,
    Started,
}

impl AiModelManager {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(AiModelManagerState::default()),
        }
    }

    pub(in crate::ai) fn request_download(&self, model_id: &str) -> DownloadRequestState {
        let mut inner = self.inner.lock().expect("model manager lock");
        inner.cancelled.remove(model_id);
        if inner.active.as_deref() == Some(model_id) {
            return DownloadRequestState::AlreadyRunning;
        }
        if inner.queued.iter().any(|queued| queued == model_id) {
            return DownloadRequestState::Queued;
        }
        if inner.active.is_none() {
            inner.active = Some(model_id.to_string());
            DownloadRequestState::Started
        } else {
            inner.queued.push_back(model_id.to_string());
            DownloadRequestState::Queued
        }
    }

    pub(in crate::ai) fn cancel_download(&self, model_id: &str) -> bool {
        let mut inner = self.inner.lock().expect("model manager lock");
        if inner.active.as_deref() == Some(model_id) {
            inner.cancelled.insert(model_id.to_string());
            return true;
        }
        let before = inner.queued.len();
        inner.queued.retain(|queued| queued != model_id);
        before != inner.queued.len()
    }

    pub(in crate::ai) fn is_cancelled(&self, model_id: &str) -> bool {
        self.inner
            .lock()
            .expect("model manager lock")
            .cancelled
            .contains(model_id)
    }

    pub(in crate::ai) fn finish_download(&self, model_id: &str) -> Option<String> {
        let mut inner = self.inner.lock().expect("model manager lock");
        inner.cancelled.remove(model_id);
        if inner.active.as_deref() == Some(model_id) {
            inner.active = None;
        }
        let next = inner.queued.pop_front();
        inner.active = next.clone();
        next
    }

    pub(in crate::ai) fn is_active_or_queued(&self, model_id: &str) -> bool {
        let inner = self.inner.lock().expect("model manager lock");
        inner.active.as_deref() == Some(model_id)
            || inner.queued.iter().any(|queued| queued == model_id)
    }
}
