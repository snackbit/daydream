use super::state::{ProtectionDecision, ProtectionState};
use crate::recorder::{
    commands,
    errors::dependency_error,
    manager::{now_ms, RecorderManager},
    platform,
    types::RecorderError,
};
use crate::{settings::RecordWindowProtectionSettings, storage::AppStorage};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};
use tauri::{AppHandle, Manager, Runtime};

#[derive(Debug)]
pub(crate) struct WindowProtectionMonitor {
    stop: Arc<AtomicBool>,
    worker: thread::JoinHandle<()>,
}
impl WindowProtectionMonitor {
    #[cfg(test)]
    pub(crate) fn for_test(on_stop: impl FnOnce() + Send + 'static) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = stop.clone();
        let worker = thread::spawn(move || {
            while !worker_stop.load(Ordering::SeqCst) {
                thread::sleep(Duration::from_millis(1));
            }
            on_stop();
        });
        Self { stop, worker }
    }
    pub(crate) fn stop(self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = self.worker.join();
    }
}

pub(crate) fn start_window_protection_monitor<R: Runtime>(
    app: AppHandle<R>,
    recording_id: String,
    started_at_ms: u64,
    settings: RecordWindowProtectionSettings,
) -> Result<Option<WindowProtectionMonitor>, RecorderError> {
    if !settings.enabled || settings.rules.iter().all(|rule| !rule.enabled) {
        return Ok(None);
    }
    let stop = Arc::new(AtomicBool::new(false));
    let worker_stop = stop.clone();
    let worker = thread::Builder::new()
        .name("daydream-window-protection".into())
        .spawn(move || {
            if let Err(error) =
                run_monitor(&app, &recording_id, started_at_ms, &settings, &worker_stop)
            {
                commands::report_recording_failure(app, recording_id, error);
            }
        })
        .map_err(|_| {
            dependency_error(
                "recorder_window_protection_monitor_failed",
                "Protected-window monitoring could not start.",
            )
        })?;
    Ok(Some(WindowProtectionMonitor { stop, worker }))
}

fn run_monitor<R: Runtime>(
    app: &AppHandle<R>,
    recording_id: &str,
    started_at_ms: u64,
    settings: &RecordWindowProtectionSettings,
    stop: &AtomicBool,
) -> Result<(), RecorderError> {
    let storage = app.state::<AppStorage>();
    let manager = app.state::<RecorderManager>();
    let mut protection = ProtectionState::default();
    while !stop.load(Ordering::SeqCst) {
        let status = manager.status()?;
        if status.active_recording_id.as_deref() != Some(recording_id) {
            break;
        }
        let snapshot = platform::window_metadata_snapshot();
        if stop.load(Ordering::SeqCst) {
            break;
        }
        match protection.observe(
            &storage,
            recording_id,
            started_at_ms,
            settings,
            now_ms(),
            snapshot,
        )? {
            ProtectionDecision::None => (),
            ProtectionDecision::Pause => {
                commands::request_auto_pause(app, recording_id);
            }
            ProtectionDecision::Resume => commands::request_auto_resume(app, recording_id),
        }
        thread::sleep(Duration::from_millis(250));
    }
    Ok(())
}
