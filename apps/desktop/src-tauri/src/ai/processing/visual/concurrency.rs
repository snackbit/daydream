use super::super::super::*;
use tokio::sync::Notify;

struct VisualFramePoolLimiter {
    active: Mutex<usize>,
    notify: Notify,
}

pub(super) struct VisualFramePoolPermit {
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
            if let Ok(mut active) = self.active.lock() {
                if *active < max_active {
                    *active += 1;
                    return VisualFramePoolPermit {
                        limiter: self.clone(),
                    };
                }
            }
            self.notify.notified().await;
        }
    }
}

static OCR_LIMITER: OnceLock<Arc<VisualFramePoolLimiter>> = OnceLock::new();
static VLM_LIMITER: OnceLock<Arc<VisualFramePoolLimiter>> = OnceLock::new();

pub(super) async fn acquire_ocr_permit(max_active: usize) -> VisualFramePoolPermit {
    OCR_LIMITER
        .get_or_init(|| Arc::new(VisualFramePoolLimiter::new()))
        .clone()
        .acquire(max_active)
        .await
}

pub(super) async fn acquire_vlm_permit(max_active: usize) -> VisualFramePoolPermit {
    VLM_LIMITER
        .get_or_init(|| Arc::new(VisualFramePoolLimiter::new()))
        .clone()
        .acquire(max_active)
        .await
}
