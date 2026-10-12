use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, OnceLock,
    },
};
use tokio::sync::Notify;

pub(super) struct Control {
    pub cancel: Arc<AtomicBool>,
    done: AtomicBool,
    changed: Notify,
}
fn controls() -> &'static Mutex<HashMap<String, Arc<Control>>> {
    static CONTROLS: OnceLock<Mutex<HashMap<String, Arc<Control>>>> = OnceLock::new();
    CONTROLS.get_or_init(Mutex::default)
}
pub(super) async fn worker_slot(
    cancel: &AtomicBool,
) -> Result<tokio::sync::OwnedSemaphorePermit, String> {
    static WORKERS: OnceLock<Arc<tokio::sync::Semaphore>> = OnceLock::new();
    let semaphore = WORKERS
        .get_or_init(|| Arc::new(tokio::sync::Semaphore::new(1)))
        .clone();
    tokio::select! {
        permit=semaphore.acquire_owned()=>permit.map_err(|_|"Audio cleanup is unavailable.".into()),
        _=async{while !cancel.load(Ordering::Acquire){tokio::time::sleep(std::time::Duration::from_millis(50)).await;}}=>Err("Cleanup was cancelled.".into())
    }
}
impl Control {
    pub fn register(id: &str) -> Result<Arc<Self>, String> {
        let mut map = controls()
            .lock()
            .map_err(|_| "Cleanup runtime is unavailable.")?;
        if map.contains_key(id) {
            return Err("Cleanup is already running for this recording.".into());
        }
        let control = Arc::new(Self {
            cancel: Arc::new(AtomicBool::new(false)),
            done: AtomicBool::new(false),
            changed: Notify::new(),
        });
        map.insert(id.into(), control.clone());
        Ok(control)
    }
    pub fn finish(&self, id: &str) {
        self.done.store(true, Ordering::Release);
        if let Ok(mut map) = controls().lock() {
            map.remove(id);
        }
        self.changed.notify_waiters();
    }
    pub fn is_done(&self) -> bool {
        self.done.load(Ordering::Acquire)
    }
}
pub(super) async fn cancel(id: &str) -> Result<(), String> {
    let control = controls()
        .lock()
        .map_err(|_| "Cleanup runtime is unavailable.")?
        .get(id)
        .cloned();
    if let Some(control) = control {
        control.cancel.store(true, Ordering::Release);
        tokio::time::timeout(std::time::Duration::from_secs(15), async {
            loop {
                let changed = control.changed.notified();
                if control.is_done() {
                    break;
                }
                changed.await;
            }
        })
        .await
        .map_err(|_| "Cleanup is still stopping. Retry before changing this recording.")?;
    }
    Ok(())
}
pub(crate) fn request_cancel(id: &str) {
    if let Ok(map) = controls().lock() {
        if let Some(control) = map.get(id) {
            control.cancel.store(true, Ordering::Release);
        }
    }
}
