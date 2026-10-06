use super::super::super::*;
use tokio::sync::Notify;

struct VisualFramePoolLimiter {
    active: Mutex<usize>,
    notify: Notify,
}

pub(in crate::ai) struct VisualFramePoolPermit {
    limiter: Arc<VisualFramePoolLimiter>,
}

impl Drop for VisualFramePoolPermit {
    fn drop(&mut self) {
        if let Ok(mut active) = self.limiter.active.lock() {
            *active = active.saturating_sub(1);
        }
        self.limiter.notify.notify_waiters();
    }
}

impl VisualFramePoolLimiter {
    fn new() -> Self {
        Self {
            active: Mutex::new(0),
            notify: Notify::new(),
        }
    }

    async fn acquire(self: &Arc<Self>, max_active: usize) -> VisualFramePoolPermit {
        let max_active = max_active.max(1);
        loop {
            // notify_waiters only wakes futures that existed when capacity changed.
            let capacity_changed = self.notify.notified();
            if let Ok(mut active) = self.active.lock() {
                if *active < max_active {
                    *active += 1;
                    return VisualFramePoolPermit {
                        limiter: self.clone(),
                    };
                }
            }
            capacity_changed.await;
        }
    }
}

static OCR_LIMITER: OnceLock<Arc<VisualFramePoolLimiter>> = OnceLock::new();
static VLM_LIMITER: OnceLock<Arc<VisualFramePoolLimiter>> = OnceLock::new();

pub(in crate::ai) async fn acquire_ocr_permit(max_active: usize) -> VisualFramePoolPermit {
    OCR_LIMITER
        .get_or_init(|| Arc::new(VisualFramePoolLimiter::new()))
        .clone()
        .acquire(max_active)
        .await
}

pub(in crate::ai) async fn acquire_vlm_permit(max_active: usize) -> VisualFramePoolPermit {
    VLM_LIMITER
        .get_or_init(|| Arc::new(VisualFramePoolLimiter::new()))
        .clone()
        .acquire(max_active)
        .await
}

pub(in crate::ai) async fn acquire_ocr_cancellable(
    max_active: usize,
    cancel: Arc<AtomicBool>,
) -> Result<VisualFramePoolPermit, AiError> {
    await_permit(acquire_ocr_permit(max_active), cancel).await
}
pub(in crate::ai) async fn acquire_vlm_cancellable(
    max_active: usize,
    cancel: Arc<AtomicBool>,
) -> Result<VisualFramePoolPermit, AiError> {
    await_permit(acquire_vlm_permit(max_active), cancel).await
}
async fn await_permit(
    future: impl std::future::Future<Output = VisualFramePoolPermit>,
    cancel: Arc<AtomicBool>,
) -> Result<VisualFramePoolPermit, AiError> {
    tokio::pin!(future);
    loop {
        check_processing_cancel(&cancel)?;
        if let Ok(permit) = tokio::time::timeout(Duration::from_millis(50), &mut future).await {
            return Ok(permit);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::{future::join, poll};

    #[test]
    fn queued_vision_requests_observe_limits_and_release_recording_leases() {
        tauri::async_runtime::block_on(async {
            let pool = Arc::new(VisualFramePoolLimiter::new());
            let held = pool.acquire(1).await;
            let recording_id = Uuid::now_v7().to_string();
            let first_lease = crate::recording_privacy::processing_lease(&recording_id).await;
            let second_lease = crate::recording_privacy::processing_lease(&recording_id).await;
            let first_request = async {
                let permit = pool.acquire(1).await;
                assert_eq!(*pool.active.lock().unwrap(), 1);
                tokio::task::yield_now().await;
                drop(permit);
                drop(first_lease);
            };
            let second_request = async {
                let permit = pool.acquire(1).await;
                assert_eq!(*pool.active.lock().unwrap(), 1);
                tokio::task::yield_now().await;
                drop(permit);
                drop(second_lease);
            };
            tokio::pin!(first_request, second_request);
            assert!(poll!(first_request.as_mut()).is_pending());
            assert!(poll!(second_request.as_mut()).is_pending());
            let mutation = crate::recording_privacy::mutation_lease(&recording_id);
            tokio::pin!(mutation);
            assert!(poll!(mutation.as_mut()).is_pending());
            drop(held);
            tokio::time::timeout(Duration::from_secs(1), join(first_request, second_request))
                .await
                .expect("queued inference did not drain");
            let writer = tokio::time::timeout(Duration::from_secs(1), mutation)
                .await
                .expect("completed requests retained recording leases");
            assert_eq!(*pool.active.lock().unwrap(), 0);
            drop(writer);
        });
    }

    #[test]
    fn changed_capacity_does_not_strand_an_existing_waiter() {
        tauri::async_runtime::block_on(async {
            let pool = Arc::new(VisualFramePoolLimiter::new());
            let held = pool.acquire(1).await;
            let waiting = pool.acquire(1);
            tokio::pin!(waiting);
            assert!(poll!(waiting.as_mut()).is_pending());
            let additional = pool.acquire(2).await;
            assert_eq!(*pool.active.lock().unwrap(), 2);
            drop(additional);
            assert!(poll!(waiting.as_mut()).is_pending());
            drop(held);
            let admitted = tokio::time::timeout(Duration::from_secs(1), waiting)
                .await
                .expect("waiter missed released capacity");
            assert_eq!(*pool.active.lock().unwrap(), 1);
            drop(admitted);
            assert_eq!(*pool.active.lock().unwrap(), 0);
        });
    }
}
