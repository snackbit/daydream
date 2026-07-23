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
    settings::{AiActivitySettings, InputActivityStatusReason},
    storage::{self, AppStorage, CreateCaptureEventInput},
};

use super::{
    manager::now_ms,
    platform,
    types::{ProtectableWindow, ProtectableWindowProviderStatus, WindowMetadataSnapshot},
};

const ACTIVITY_BUCKET_MS: i64 = 500;
const WINDOW_ACTIVITY_POLL_MS: u64 = 500;

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
    if !settings.keyboard.enabled && !settings.mouse.enabled && !settings.windows.enabled {
        return None;
    }

    let stop = Arc::new(AtomicBool::new(false));
    let worker_stop = stop.clone();
    let keyboard_enabled = settings.keyboard.enabled;
    let mouse_enabled = settings.mouse.enabled;
    let windows_enabled = settings.windows.enabled;
    let worker = thread::Builder::new()
        .name("daydream-activity-input".to_string())
        .spawn(move || {
            run_activity_monitor(
                app,
                recording_id,
                recording_started_at_ms,
                keyboard_enabled,
                mouse_enabled,
                windows_enabled,
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
    windows_enabled: bool,
    stop: Arc<AtomicBool>,
) {
    let input_enabled = keyboard_enabled || mouse_enabled;
    let devices = if input_enabled {
        linux_devices()
    } else {
        Vec::new()
    };
    if input_enabled && devices.is_empty() {
        let status = crate::settings::input_activity::input_activity_status();
        save_input_unavailable(
            &app,
            &recording_id,
            recording_started_at_ms,
            input_activity_reason_code(&status.reason),
        );
        if !windows_enabled {
            return;
        }
    }

    let mut last_bucket_by_kind = HashMap::<&'static str, i64>::new();
    let mut window_activity = WindowActivityState::default();
    let mut next_window_poll_at_ms = 0;
    let storage = app.state::<AppStorage>();
    let payload = json!({
        "source": "evdev",
        "bucketMs": ACTIVITY_BUCKET_MS,
    })
    .to_string();
    let mut devices = devices;

    while !stop.load(Ordering::SeqCst) {
        if windows_enabled {
            let timestamp_ms = now_ms();
            if timestamp_ms >= next_window_poll_at_ms {
                next_window_poll_at_ms = timestamp_ms.saturating_add(WINDOW_ACTIVITY_POLL_MS);
                save_window_activity_changes(
                    &storage,
                    &recording_id,
                    recording_started_at_ms,
                    timestamp_ms,
                    &mut window_activity,
                    platform::window_metadata_snapshot(),
                );
            }
        }

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
    windows_enabled: bool,
    _stop: Arc<AtomicBool>,
) {
    save_input_unavailable(
        &app,
        &recording_id,
        recording_started_at_ms,
        "unsupported-platform",
    );
    if windows_enabled {
        let storage = app.state::<AppStorage>();
        let mut window_activity = WindowActivityState::default();
        save_window_activity_changes(
            &storage,
            &recording_id,
            recording_started_at_ms,
            now_ms(),
            &mut window_activity,
            platform::window_metadata_snapshot(),
        );
    }
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
    reason: &str,
) {
    let storage = app.state::<AppStorage>();
    let timestamp_ms = now_ms() as i64;
    let offset_ms = timestamp_ms
        .saturating_sub(recording_started_at_ms as i64)
        .max(0);
    let payload = json!({
        "source": "evdev",
        "status": "unavailable",
        "reason": reason,
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

fn input_activity_reason_code(reason: &InputActivityStatusReason) -> &'static str {
    match reason {
        InputActivityStatusReason::Available => "available",
        InputActivityStatusReason::DevInputMissing => "dev-input-missing",
        InputActivityStatusReason::NoSupportedDevices => "no-supported-devices",
        InputActivityStatusReason::PermissionDenied => "permission-denied",
        InputActivityStatusReason::Unknown => "unknown",
        InputActivityStatusReason::UnsupportedPlatform => "unsupported-platform",
    }
}

#[derive(Default)]
struct WindowActivityState {
    last_focus_signature: Option<String>,
    last_inventory_signature: Option<String>,
    unavailable_saved: bool,
}

struct PlannedCaptureEvent {
    kind: &'static str,
    occurred_at_ms: i64,
    offset_ms: i64,
    payload_json: String,
}

fn save_window_activity_changes(
    storage: &AppStorage,
    recording_id: &str,
    recording_started_at_ms: u64,
    timestamp_ms: u64,
    state: &mut WindowActivityState,
    snapshot: WindowMetadataSnapshot,
) {
    let events =
        plan_window_activity_events(state, recording_started_at_ms, timestamp_ms, snapshot);
    for event in events {
        let _ = storage.with_connection(|connection| {
            storage::create_capture_event(
                connection,
                &CreateCaptureEventInput {
                    id: Uuid::now_v7().to_string(),
                    kind: event.kind.to_string(),
                    occurred_at_ms: event.occurred_at_ms,
                    offset_ms: event.offset_ms,
                    payload_json: event.payload_json,
                    recording_id: recording_id.to_string(),
                },
            )
        });
    }
}

fn plan_window_activity_events(
    state: &mut WindowActivityState,
    recording_started_at_ms: u64,
    timestamp_ms: u64,
    snapshot: WindowMetadataSnapshot,
) -> Vec<PlannedCaptureEvent> {
    let occurred_at_ms = timestamp_ms as i64;
    let offset_ms = occurred_at_ms
        .saturating_sub(recording_started_at_ms as i64)
        .max(0);

    if snapshot.status != ProtectableWindowProviderStatus::Available {
        state.last_focus_signature = None;
        state.last_inventory_signature = None;
        if state.unavailable_saved {
            return Vec::new();
        }
        state.unavailable_saved = true;
        return vec![PlannedCaptureEvent {
            kind: "capture-state",
            occurred_at_ms,
            offset_ms,
            payload_json: json!({
                "source": "window-metadata",
                "status": "unavailable",
                "reason": provider_status_code(&snapshot.status),
                "provider": snapshot.provider,
            })
            .to_string(),
        }];
    }

    state.unavailable_saved = false;
    let mut events = Vec::new();
    let focus_signature = snapshot.focused_window.as_ref().map(window_signature);
    if focus_signature != state.last_focus_signature {
        state.last_focus_signature = focus_signature;
        if let Some(window) = &snapshot.focused_window {
            events.push(PlannedCaptureEvent {
                kind: "focused-window",
                occurred_at_ms,
                offset_ms,
                payload_json: focused_window_payload(snapshot.provider.as_deref(), window),
            });
        }
    }

    let inventory_signature = inventory_signature(&snapshot);
    if state.last_inventory_signature.as_deref() != Some(inventory_signature.as_str()) {
        events.push(PlannedCaptureEvent {
            kind: "open-windows",
            occurred_at_ms,
            offset_ms,
            payload_json: open_windows_payload(&snapshot),
        });
        state.last_inventory_signature = Some(inventory_signature);
    }

    events
}

fn focused_window_payload(provider: Option<&str>, window: &ProtectableWindow) -> String {
    let application_name = window
        .app_identity
        .as_deref()
        .or(window.process_name.as_deref())
        .and_then(non_empty_str);
    json!({
        "source": "window-metadata",
        "provider": provider,
        "windowId": non_empty_str(&window.id),
        "applicationName": application_name,
        "appIdentity": window.app_identity.as_deref().and_then(non_empty_str),
        "processName": window.process_name.as_deref().and_then(non_empty_str),
        "processId": window.process_id,
        "title": window.title.as_deref().and_then(non_empty_str),
    })
    .to_string()
}

fn open_windows_payload(snapshot: &WindowMetadataSnapshot) -> String {
    let mut windows = snapshot.windows.clone();
    windows.sort_by(|left, right| left.id.cmp(&right.id));
    let windows = windows
        .iter()
        .map(window_payload)
        .collect::<Vec<serde_json::Value>>();
    json!({
        "source": "window-metadata",
        "provider": snapshot.provider.as_deref(),
        "focusedWindowId": snapshot.focused_window.as_ref().map(|window| window.id.as_str()).and_then(non_empty_str),
        "windows": windows,
    })
    .to_string()
}

fn window_payload(window: &ProtectableWindow) -> serde_json::Value {
    json!({
        "id": non_empty_str(&window.id),
        "appIdentity": window.app_identity.as_deref().and_then(non_empty_str),
        "processName": window.process_name.as_deref().and_then(non_empty_str),
        "processId": window.process_id,
        "title": window.title.as_deref().and_then(non_empty_str),
    })
}

fn inventory_signature(snapshot: &WindowMetadataSnapshot) -> String {
    let mut windows = snapshot
        .windows
        .iter()
        .map(window_signature)
        .collect::<Vec<_>>();
    windows.sort();
    serde_json::to_string(&json!({
        "provider": snapshot.provider.as_deref(),
        "focusedWindowId": snapshot.focused_window.as_ref().map(|window| window.id.as_str()).and_then(non_empty_str),
        "windows": windows,
    }))
    .unwrap_or_default()
}

fn window_signature(window: &ProtectableWindow) -> String {
    serde_json::to_string(&window_payload(window)).unwrap_or_default()
}

fn provider_status_code(status: &ProtectableWindowProviderStatus) -> String {
    serde_json::to_value(status)
        .ok()
        .and_then(|value| value.as_str().map(ToString::to_string))
        .unwrap_or_else(|| format!("{status:?}"))
}

fn non_empty_str(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use crate::recorder::types::{
        ProtectableWindow, ProtectableWindowProviderStatus, WindowMetadataSnapshot,
    };

    #[test]
    fn placeholder_keeps_module_testable() {
        assert_eq!(super::ACTIVITY_BUCKET_MS, 500);
    }

    #[test]
    fn window_activity_events_are_written_only_on_change() {
        let mut state = super::WindowActivityState::default();
        let snapshot = available_snapshot(
            Some(window("window-1", "Editor", "Notes.md")),
            vec![window("window-1", "Editor", "Notes.md")],
        );

        let first = super::plan_window_activity_events(&mut state, 1_000, 1_500, snapshot.clone());
        assert_eq!(first.len(), 2);
        assert_eq!(first[0].kind, "focused-window");
        assert_eq!(first[1].kind, "open-windows");

        let duplicate = super::plan_window_activity_events(&mut state, 1_000, 2_000, snapshot);
        assert!(duplicate.is_empty());

        let changed = super::plan_window_activity_events(
            &mut state,
            1_000,
            2_500,
            available_snapshot(
                Some(window("window-2", "Browser", "Docs")),
                vec![
                    window("window-1", "Editor", "Notes.md"),
                    window("window-2", "Browser", "Docs"),
                ],
            ),
        );
        assert_eq!(
            changed.iter().map(|event| event.kind).collect::<Vec<_>>(),
            vec!["focused-window", "open-windows"]
        );
        let focus_payload: Value = serde_json::from_str(&changed[0].payload_json).unwrap();
        assert_eq!(focus_payload["applicationName"], "Browser");
        assert_eq!(focus_payload["title"], "Docs");
    }

    #[test]
    fn window_activity_unavailable_state_is_written_once_without_private_payload() {
        let mut state = super::WindowActivityState::default();
        let snapshot = WindowMetadataSnapshot {
            focused_window: None,
            provider: Some("linux-wayland".to_string()),
            status: ProtectableWindowProviderStatus::ProviderUnavailable,
            windows: Vec::new(),
        };

        let first = super::plan_window_activity_events(&mut state, 1_000, 1_500, snapshot.clone());
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].kind, "capture-state");
        let payload: Value = serde_json::from_str(&first[0].payload_json).unwrap();
        assert_eq!(payload["source"], "window-metadata");
        assert_eq!(payload["reason"], "provider-unavailable");
        assert!(payload.get("title").is_none());
        assert!(payload.get("windows").is_none());

        let duplicate = super::plan_window_activity_events(&mut state, 1_000, 2_000, snapshot);
        assert!(duplicate.is_empty());
    }

    fn available_snapshot(
        focused_window: Option<ProtectableWindow>,
        windows: Vec<ProtectableWindow>,
    ) -> WindowMetadataSnapshot {
        WindowMetadataSnapshot {
            focused_window,
            provider: Some("linux-test".to_string()),
            status: ProtectableWindowProviderStatus::Available,
            windows,
        }
    }

    fn window(id: &str, process_name: &str, title: &str) -> ProtectableWindow {
        ProtectableWindow {
            app_identity: Some(process_name.to_string()),
            id: id.to_string(),
            platform: "linux".to_string(),
            process_id: Some(42),
            process_name: Some(process_name.to_string()),
            title: Some(title.to_string()),
        }
    }
}
