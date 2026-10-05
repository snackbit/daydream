use super::super::super::*;

pub(crate) fn cancel_job_if_inactive(
    storage: &AppStorage,
    job_id: &str,
    cancel: &AtomicBool,
) -> bool {
    let status = storage
        .with_connection(|connection| storage::get_processing_job(connection, job_id))
        .ok()
        .flatten()
        .map(|job| job.status);
    if status.as_deref() != Some("running") {
        cancel.store(true, Ordering::SeqCst);
        return true;
    }
    false
}

pub(super) fn spawn_job_cancel_monitor(
    app: AppHandle,
    job_id: String,
    cancel: Arc<AtomicBool>,
) -> tauri::async_runtime::JoinHandle<()> {
    tauri::async_runtime::spawn(async move {
        loop {
            if cancel.load(Ordering::SeqCst) {
                return;
            }
            let storage = app.state::<AppStorage>();
            if cancel_job_if_inactive(&storage, &job_id, &cancel) {
                return;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
}
