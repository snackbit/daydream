use super::super::*;

pub fn start_processing_worker(app: AppHandle) {
    let storage = app.state::<AppStorage>();
    if let Err(error) = recover_stale_processing(&storage) {
        tracing::warn!(error_code = %error.code, "AI processing recovery failed.");
    }
    if storage.with_connection(|connection| connection.execute("INSERT OR IGNORE INTO waveform_admission(recording_id) SELECT id FROM recordings", []).map(|_| ())).is_err() {
        tracing::warn!(error_code="waveform_admission_failed", "Waveform recovery could not be queued.");
    }
    tauri::async_runtime::spawn(async move {
        loop {
            let Ok(_activity) = crate::maintenance::runtime::admit(&app, "processing-worker")
            else {
                tokio::time::sleep(PROCESSING_WORKER_INTERVAL).await;
                continue;
            };
            let admission_app = app.clone();
            match tauri::async_runtime::spawn_blocking(move || {
                crate::library::waveforms::jobs::admit_batch(&admission_app.state::<AppStorage>())
            })
            .await
            {
                Ok(Ok(())) => {}
                _ => tracing::warn!(
                    error_code = "waveform_admission_failed",
                    "Waveform admission could not finish."
                ),
            }
            if let Err(error) = run_processing_worker_tick(&app).await {
                tracing::warn!(error_code = %error.code, "AI processing worker tick failed.");
            }
            drop(_activity);
            tokio::time::sleep(PROCESSING_WORKER_INTERVAL).await;
        }
    });
}
