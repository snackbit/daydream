//! Exact duplicate frames share a recognition/commit lease. Distinct pixels can
//! run concurrently; a waiter observes the first frame's committed cache entry.
use super::super::super::*;
use std::collections::HashMap;
use std::sync::{OnceLock, Weak};
use tokio::sync::{Mutex as AsyncMutex, OwnedMutexGuard};

pub(in crate::ai) async fn acquire(
    recording: &str,
    source: &str,
    pixels: &str,
    configuration: &str,
    cancel: Arc<AtomicBool>,
) -> Result<OwnedMutexGuard<()>, AiError> {
    type Key = (String, String, String, String);
    static LEASES: OnceLock<Mutex<HashMap<Key, Weak<AsyncMutex<()>>>>> = OnceLock::new();
    let lock = {
        let mut leases = LEASES
            .get_or_init(|| Mutex::new(HashMap::new()))
            .lock()
            .map_err(|_| {
                ai_error(
                    "ai_processing_worker_state_failed",
                    "Recognition state is unavailable.",
                )
            })?;
        leases.retain(|_, lock| lock.strong_count() > 0);
        let key = (
            recording.into(),
            source.into(),
            pixels.into(),
            configuration.into(),
        );
        if let Some(lock) = leases.get(&key).and_then(Weak::upgrade) {
            lock
        } else {
            let lock = Arc::new(AsyncMutex::new(()));
            leases.insert(key, Arc::downgrade(&lock));
            lock
        }
    };
    let future = lock.lock_owned();
    tokio::pin!(future);
    loop {
        check_processing_cancel(&cancel)?;
        if let Ok(guard) = tokio::time::timeout(Duration::from_millis(50), &mut future).await {
            return Ok(guard);
        }
    }
}
