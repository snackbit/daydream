use super::super::*;

pub struct AiModelManager {
    inner: Mutex<AiModelManagerState>,
    changed: tokio::sync::Notify,
    command: Mutex<()>,
}

pub(in crate::ai) struct ModelUseLease<'a> {
    manager: &'a AiModelManager,
    model_id: String,
}
impl Drop for ModelUseLease<'_> {
    fn drop(&mut self) {
        self.manager
            .inner
            .lock()
            .expect("model manager lock")
            .testing
            .remove(&self.model_id);
        self.manager.changed.notify_waiters();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sqlite_notification_failure_rolls_back_asset_and_does_not_reserve_slot() {
        let connection = rusqlite::Connection::open_in_memory().unwrap();
        connection.execute_batch("CREATE TABLE assets(id TEXT PRIMARY KEY); CREATE TABLE notices(id TEXT PRIMARY KEY CHECK(id <> 'fail'));").unwrap();
        let manager = AiModelManager::new();
        let result = manager.admit("e5", |_| {
            let transaction = connection.unchecked_transaction()?;
            transaction.execute("INSERT INTO assets VALUES ('e5')", [])?;
            transaction.execute("INSERT INTO notices VALUES ('fail')", [])?;
            transaction.commit()
        });
        assert!(result.is_err());
        assert_eq!(
            connection
                .query_row("SELECT count(*) FROM assets", [], |row| row
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
        assert!(!manager.is_active_or_queued("e5"));
        assert_eq!(
            manager.request_download("bge"),
            DownloadRequestState::Started
        );
        assert_eq!(manager.request_download("e5"), DownloadRequestState::Queued);
        manager.pause_download("e5");
        assert_eq!(
            manager.request_download("tiny"),
            DownloadRequestState::Queued
        );
        assert_eq!(manager.request_download("e5"), DownloadRequestState::Queued);
        assert_eq!(manager.finish_download("bge"), Some("tiny".into()));
        assert_eq!(manager.finish_download("bge"), None);
        assert_eq!(manager.finish_download("tiny"), Some("e5".into()));
    }
}

#[derive(Default)]
pub(in crate::ai) struct AiModelManagerState {
    active: Option<String>,
    cancelled: HashSet<String>,
    queued: VecDeque<String>,
    paused: HashSet<String>,
    testing: HashSet<String>,
    telemetry: std::collections::HashMap<String, (AiModelTransfer, Instant)>,
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
            changed: tokio::sync::Notify::new(),
            command: Mutex::new(()),
        }
    }

    pub(in crate::ai) fn admit<T, E>(
        &self,
        model_id: &str,
        persist: impl FnOnce(&DownloadRequestState) -> Result<T, E>,
    ) -> Result<(DownloadRequestState, T), E> {
        let mut inner = self.inner.lock().expect("model manager lock");
        let state = if inner.active.as_deref() == Some(model_id) {
            DownloadRequestState::AlreadyRunning
        } else if inner.queued.iter().any(|id| id == model_id) {
            DownloadRequestState::Queued
        } else if inner.active.is_none() {
            DownloadRequestState::Started
        } else {
            DownloadRequestState::Queued
        };
        let result = persist(&state)?;
        if state != DownloadRequestState::AlreadyRunning {
            inner.cancelled.remove(model_id);
            inner.paused.remove(model_id);
        }
        if state == DownloadRequestState::Started {
            inner.active = Some(model_id.to_string());
        } else if state == DownloadRequestState::Queued
            && !inner.queued.iter().any(|id| id == model_id)
        {
            inner.queued.push_back(model_id.to_string());
        }
        Ok((state, result))
    }

    pub(in crate::ai) fn command_guard(&self) -> std::sync::MutexGuard<'_, ()> {
        self.command.lock().expect("model command lock")
    }

    #[cfg(test)]
    pub(in crate::ai) fn request_download(&self, model_id: &str) -> DownloadRequestState {
        self.admit(model_id, |_| Ok::<_, ()>(())).unwrap().0
    }

    pub(in crate::ai) fn cancel_download(&self, model_id: &str) -> bool {
        let mut inner = self.inner.lock().expect("model manager lock");
        if inner.active.as_deref() == Some(model_id) {
            inner.cancelled.insert(model_id.to_string());
            inner.paused.remove(model_id);
            self.changed.notify_waiters();
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
        if inner.active.as_deref() != Some(model_id) {
            return None;
        }
        inner.cancelled.remove(model_id);
        if inner.active.as_deref() == Some(model_id) {
            inner.active = None;
        }
        let next = inner.queued.pop_front();
        inner.active = next.clone();
        self.changed.notify_waiters();
        next
    }

    pub(in crate::ai) fn abort_worker_admission(&self, model_id: &str) {
        let mut inner = self.inner.lock().expect("model manager lock");
        if inner.active.as_deref() == Some(model_id) {
            inner.active = None;
        }
        inner.queued.clear();
        self.changed.notify_waiters();
    }

    pub(in crate::ai) fn is_active_or_queued(&self, model_id: &str) -> bool {
        let inner = self.inner.lock().expect("model manager lock");
        inner.active.as_deref() == Some(model_id)
            || inner.queued.iter().any(|queued| queued == model_id)
            || inner.testing.contains(model_id)
    }

    pub(in crate::ai) fn use_model(&self, model_id: &str) -> Result<ModelUseLease<'_>, AiError> {
        let _command = self.command_guard();
        let mut inner = self.inner.lock().expect("model manager lock");
        if inner.active.as_deref() == Some(model_id)
            || inner.queued.iter().any(|id| id == model_id)
            || inner.testing.contains(model_id)
        {
            return Err(AiError {
                code: "ai_model_download_busy".into(),
                message: "Wait for the current model action to finish.".into(),
            });
        }
        inner.testing.insert(model_id.into());
        Ok(ModelUseLease {
            manager: self,
            model_id: model_id.into(),
        })
    }

    pub(in crate::ai) fn pause_download(&self, model_id: &str) {
        let mut inner = self.inner.lock().expect("model manager lock");
        inner.paused.insert(model_id.into());
        inner.queued.retain(|id| id != model_id);
        self.changed.notify_waiters();
    }

    pub(in crate::ai) fn is_paused(&self, model_id: &str) -> bool {
        self.inner
            .lock()
            .expect("model manager lock")
            .paused
            .contains(model_id)
    }

    pub(in crate::ai) async fn stopped(&self, model_id: &str) {
        loop {
            let notified = self.changed.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            if self.is_cancelled(model_id) || self.is_paused(model_id) {
                return;
            }
            notified.await;
        }
    }

    pub(in crate::ai) async fn wait_for_model(&self, model_id: &str) {
        loop {
            let notified = self.changed.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            if !self.is_active_or_queued(model_id) {
                return;
            }
            notified.await;
        }
    }

    pub async fn cancel_and_wait_for_maintenance(&self) {
        {
            let mut inner = self.inner.lock().expect("model manager lock");
            if let Some(id) = inner.active.clone() {
                inner.cancelled.insert(id);
            }
            inner.queued.clear();
        }
        self.changed.notify_waiters();
        loop {
            let notified = self.changed.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            if self
                .inner
                .lock()
                .expect("model manager lock")
                .active
                .is_none()
            {
                return;
            }
            notified.await;
        }
    }

    pub(in crate::ai) fn set_telemetry(
        &self,
        model_id: &str,
        phase: &str,
        speed: Option<i64>,
        eta: Option<i64>,
    ) {
        let mut inner = self.inner.lock().expect("model manager lock");
        let attempt = inner
            .telemetry
            .get(model_id)
            .map(|(telemetry, _)| telemetry.attempt)
            .unwrap_or(1);
        inner.telemetry.insert(
            model_id.into(),
            (
                AiModelTransfer {
                    attempt,
                    phase: phase.into(),
                    bytes_per_second: speed,
                    eta_ms: eta,
                },
                Instant::now(),
            ),
        );
    }

    pub(in crate::ai) fn decorate(&self, models: &mut AiModelAssetList) {
        let inner = self.inner.lock().expect("model manager lock");
        for model in &mut models.models {
            if let Some((telemetry, updated)) = inner.telemetry.get(&model.id) {
                let mut next = telemetry.clone();
                if updated.elapsed() > Duration::from_secs(3) {
                    next.bytes_per_second = None;
                    next.eta_ms = None;
                    if next.phase == "transferring" {
                        next.phase = "stalled".into();
                    }
                }
                model.transfer = Some(next);
            }
        }
    }

    pub(in crate::ai) fn begin_attempt(&self, model_id: &str, attempt: i64) {
        self.inner
            .lock()
            .expect("model manager lock")
            .telemetry
            .insert(
                model_id.into(),
                (
                    AiModelTransfer {
                        attempt,
                        phase: "connecting".into(),
                        bytes_per_second: None,
                        eta_ms: None,
                    },
                    Instant::now(),
                ),
            );
    }
}
