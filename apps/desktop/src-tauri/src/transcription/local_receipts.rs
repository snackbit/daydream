use super::{transcription_error, TranscriptionError};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, AtomicU8, Ordering},
        Arc, Mutex, OnceLock,
    },
};
use tokio::sync::{Notify, OwnedRwLockReadGuard};

const PENDING: u8 = 0;
const RUNNING: u8 = 1;
const FINISHED: u8 = 2;
type ProcessingLease = Arc<OwnedRwLockReadGuard<()>>;
type Registry = Mutex<HashMap<String, Arc<Receipt>>>;
fn registry() -> &'static Registry {
    static REGISTRY: OnceLock<Registry> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

struct Receipt {
    cancel: Arc<AtomicBool>,
    finished: Notify,
    state: AtomicU8,
    // The already-owned lease is shared, never recursively acquired behind a writer.
    _lease: Option<ProcessingLease>,
    job_id: Option<String>,
}

pub(super) struct LocalRequestRegistration {
    id: String,
    receipt: Arc<Receipt>,
}
impl LocalRequestRegistration {
    pub(super) fn new(
        cancel: Arc<AtomicBool>,
        lease: Option<ProcessingLease>,
        job_id: Option<String>,
    ) -> Self {
        let id = uuid::Uuid::now_v7().to_string();
        let receipt = Arc::new(Receipt {
            cancel,
            finished: Notify::new(),
            state: AtomicU8::new(PENDING),
            _lease: lease,
            job_id,
        });
        registry()
            .lock()
            .expect("local receipt registry")
            .insert(id.clone(), receipt.clone());
        Self { id, receipt }
    }
    pub(super) fn id(&self) -> &str {
        &self.id
    }
    fn revoke(&self) {
        registry()
            .lock()
            .expect("local receipt registry")
            .remove(&self.id);
        if self
            .receipt
            .state
            .compare_exchange(PENDING, FINISHED, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            self.receipt.finished.notify_waiters();
        }
    }
    pub(super) async fn drain(&self, cancelled: bool) {
        if cancelled {
            self.receipt.cancel.store(true, Ordering::SeqCst);
        }
        self.revoke();
        loop {
            let notified = self.receipt.finished.notified();
            if self.receipt.state.load(Ordering::SeqCst) == FINISHED {
                return;
            }
            notified.await;
        }
    }
}
impl Drop for LocalRequestRegistration {
    fn drop(&mut self) {
        if self.receipt.state.load(Ordering::SeqCst) != FINISHED {
            self.receipt.cancel.store(true, Ordering::SeqCst);
        }
        self.revoke();
    }
}

pub(super) struct LocalHandlerReceipt {
    id: String,
    receipt: Arc<Receipt>,
}
impl LocalHandlerReceipt {
    pub(super) fn claim(id: &str) -> Result<Self, TranscriptionError> {
        let entries = registry().lock().expect("local receipt registry");
        let receipt = entries
            .get(id)
            .filter(|receipt| {
                receipt
                    .state
                    .compare_exchange(PENDING, RUNNING, Ordering::SeqCst, Ordering::SeqCst)
                    .is_ok()
            })
            .cloned()
            .ok_or_else(|| {
                transcription_error(
                    "ai_transcription_cancelled",
                    "Local request was revoked before its handler started.",
                )
            })?;
        Ok(Self {
            id: id.to_string(),
            receipt,
        })
    }
    pub(super) fn cancel(&self) -> Arc<AtomicBool> {
        self.receipt.cancel.clone()
    }
    pub(super) fn id(&self) -> &str {
        &self.id
    }
    pub(super) fn job_id(&self) -> Option<&str> {
        self.receipt.job_id.as_deref()
    }
}
impl Drop for LocalHandlerReceipt {
    fn drop(&mut self) {
        self.receipt.state.store(FINISHED, Ordering::SeqCst);
        self.receipt.finished.notify_waiters();
    }
}
