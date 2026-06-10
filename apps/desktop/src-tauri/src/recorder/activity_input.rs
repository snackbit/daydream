use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

use serde_json::json;
use tauri::{AppHandle, Manager, Runtime};
use tracing::warn;
use uuid::Uuid;

use crate::{
    settings::AiActivitySettings,
    storage::{self, AppStorage, CreateCaptureEventInput},
};

use super::manager::now_ms;

const ACTIVITY_BUCKET_MS: i64 = 500;

#[derive(Debug)]
pub(crate) struct ActivityMonitorHandle {
    stop: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
}

impl ActivityMonitorHandle {
    pub(crate) fn stop(mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

pub(crate) fn start_activity_monitor(
    app: AppHandle<impl Runtime>,
    recording_id: String,
    recording_started_at_ms: u64,
    settings: &AiActivitySettings,
) -> Option<ActivityMonitorHandle> {
    if !settings.keyboard.enabled && !settings.mouse.enabled {
        return None;
    }

    let stop = Arc::new(AtomicBool::new(false));
    let worker_stop = stop.clone();
    let keyboard_enabled = settings.keyboard.enabled;
    let mouse_enabled = settings.mouse.enabled;
    let worker = thread::Builder::new()
        .name("daydream-activity-input".to_string())
        .spawn(move || {
            run_activity_monitor(
                app,
                recording_id,
                recording_started_at_ms,
                keyboard_enabled,
                mouse_enabled,
                worker_stop,
            );
        });

    match worker {
        Ok(worker) => Some(ActivityMonitorHandle {
            stop,
            worker: Some(worker),
        }),
        Err(error) => {
            warn!(
                error = %error,
                "Input activity monitor thread could not start."
            );
            None
        }
    }
}

#[cfg(target_os = "linux")]
fn run_activity_monitor(
    app: AppHandle<impl Runtime>,
    recording_id: String,
    recording_started_at_ms: u64,
    keyboard_enabled: bool,
    mouse_enabled: bool,
    stop: Arc<AtomicBool>,
) {
    let devices = linux_devices();
    if devices.is_empty() {
        save_input_unavailable(&app, &recording_id, recording_started_at_ms);
        return;
    }

    let mut last_bucket_by_kind = HashMap::<&'static str, i64>::new();
    let storage = app.state::<AppStorage>();
    let payload = json!({
        "source": "evdev",
        "bucketMs": ACTIVITY_BUCKET_MS,
    })
    .to_string();
    let mut devices = devices;

    while !stop.load(Ordering::SeqCst) {
        let mut saw_events = false;
        for device in &mut devices {
            match device.fetch_events() {
                Ok(events) => {
                    for event in events {
                        saw_events = true;
                        let Some(kind) = classify_evdev_event(
                            &event.destructure(),
                            keyboard_enabled,
                            mouse_enabled,
                        ) else {
                            continue;
                        };
                        let timestamp_ms = now_ms() as i64;
                        let offset_ms = timestamp_ms
                            .saturating_sub(recording_started_at_ms as i64)
                            .max(0);
                        let bucket = offset_ms / ACTIVITY_BUCKET_MS;
                        if last_bucket_by_kind.get(kind) == Some(&bucket) {
                            continue;
                        }
                        last_bucket_by_kind.insert(kind, bucket);
                        let _ = storage.with_connection(|connection| {
                            storage::create_capture_event(
                                connection,
                                &CreateCaptureEventInput {
                                    id: Uuid::now_v7().to_string(),
                                    kind: kind.to_string(),
                                    occurred_at_ms: timestamp_ms,
                                    offset_ms,
                                    payload_json: payload.clone(),
                                    recording_id: recording_id.clone(),
                                },
                            )
                        });
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(error) => {
                    warn!(
                        error = %error,
                        "Input activity monitor device read failed."
                    );
                }
            }
        }

        if !saw_events {
            thread::sleep(Duration::from_millis(50));
        }
    }
}

#[cfg(not(target_os = "linux"))]
fn run_activity_monitor(
    app: AppHandle<impl Runtime>,
    recording_id: String,
    recording_started_at_ms: u64,
    _keyboard_enabled: bool,
    _mouse_enabled: bool,
    _stop: Arc<AtomicBool>,
) {
    save_input_unavailable(&app, &recording_id, recording_started_at_ms);
}

#[cfg(target_os = "linux")]
fn linux_devices() -> Vec<evdev::Device> {
    evdev::enumerate()
        .filter_map(|(_path, device)| {
            let keyboard_like = device.supported_keys().is_some_and(|keys| {
                keys.contains(evdev::KeyCode::KEY_A) || keys.contains(evdev::KeyCode::KEY_ENTER)
            });
            let mouse_like = device.supported_relative_axes().is_some()
                || device.supported_absolute_axes().is_some()
                || device.supported_keys().is_some_and(|keys| {
                    keys.contains(evdev::KeyCode::BTN_LEFT)
                        || keys.contains(evdev::KeyCode::BTN_RIGHT)
                });
            if !keyboard_like && !mouse_like {
                return None;
            }
            if let Err(error) = device.set_nonblocking(true) {
                warn!(
                    error = %error,
                    "Input activity monitor could not set nonblocking mode."
                );
                return None;
            }
            Some(device)
        })
        .collect()
}

#[cfg(target_os = "linux")]
fn classify_evdev_event(
    summary: &evdev::EventSummary,
    keyboard_enabled: bool,
    mouse_enabled: bool,
) -> Option<&'static str> {
    match summary {
        evdev::EventSummary::Key(_event, code, value) if *value != 0 => {
            let label = format!("{code:?}");
            if mouse_enabled && label.starts_with("BTN_") {
                Some("mouse-activity")
            } else if keyboard_enabled {
                Some("keyboard-activity")
            } else {
                None
            }
        }
        evdev::EventSummary::RelativeAxis(_, _, value)
        | evdev::EventSummary::AbsoluteAxis(_, _, value)
            if mouse_enabled && *value != 0 =>
        {
            Some("mouse-activity")
        }
        _ => None,
    }
}

fn save_input_unavailable(
    app: &AppHandle<impl Runtime>,
    recording_id: &str,
    recording_started_at_ms: u64,
) {
    let storage = app.state::<AppStorage>();
    let timestamp_ms = now_ms() as i64;
    let offset_ms = timestamp_ms
        .saturating_sub(recording_started_at_ms as i64)
        .max(0);
    let payload = json!({
        "source": "evdev",
        "status": "unavailable",
        "reason": "input-device-access-unavailable",
    })
    .to_string();
    let _ = storage.with_connection(|connection| {
        storage::create_capture_event(
            connection,
            &CreateCaptureEventInput {
                id: Uuid::now_v7().to_string(),
                kind: "capture-state".to_string(),
                occurred_at_ms: timestamp_ms,
                offset_ms,
                payload_json: payload,
                recording_id: recording_id.to_string(),
            },
        )
    });
}

#[cfg(test)]
mod tests {
    #[test]
    fn placeholder_keeps_module_testable() {
        assert_eq!(super::ACTIVITY_BUCKET_MS, 500);
    }
}
