use super::super::super::*;

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
            let status = storage
                .with_connection(|connection| storage::get_processing_job(connection, &job_id))
                .ok()
                .flatten()
                .map(|job| job.status);
            if matches!(status.as_deref(), Some("paused" | "cancelled")) || status.is_none() {
                cancel.store(true, Ordering::SeqCst);
                return;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
}
