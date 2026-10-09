use super::super::*;

pub fn start_processing_worker(app: AppHandle) {
    let storage = app.state::<AppStorage>();
    if let Err(error) = recover_stale_processing(&storage) {
        tracing::warn!(error_code = %error.code, "AI processing recovery failed.");
    }
    tauri::async_runtime::spawn(async move {
        loop {
            let Ok(_activity) = crate::maintenance::runtime::admit(&app, "processing-worker")
            else {
                tokio::time::sleep(PROCESSING_WORKER_INTERVAL).await;
                continue;
            };
            if let Err(error) = run_processing_worker_tick(&app).await {
                tracing::warn!(error_code = %error.code, "AI processing worker tick failed.");
            }
            drop(_activity);
            tokio::time::sleep(PROCESSING_WORKER_INTERVAL).await;
        }
    });
}
