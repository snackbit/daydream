use std::{
    fs::{DirBuilder, OpenOptions},
    io::Write,
    os::unix::fs::{DirBuilderExt, OpenOptionsExt},
    path::PathBuf,
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use serde::Deserialize;
use uuid::Uuid;
use zbus::{blocking::connection::Builder as ConnectionBuilder, blocking::Connection, interface};

use super::LinuxWindowProvider;
use crate::recorder::types::{
    ProtectableWindow, ProtectableWindowList, ProtectableWindowProviderStatus,
    WindowProtectionStatus,
};

const PROVIDER_NAME: &str = "linux-kde-kwin-script";
const BUS_NAME: &str = "com.snackbit.Daydream.WindowMetadata";
const OBJECT_PATH: &str = "/com/snackbit/Daydream/WindowMetadata";
const INTERFACE_NAME: &str = "com.snackbit.Daydream.WindowMetadata";
const PLUGIN_NAME: &str = "daydream-window-metadata";
const MAX_PAYLOAD_BYTES: usize = 1024 * 1024;
const SNAPSHOT_WAIT: Duration = Duration::from_millis(750);

static BRIDGE: Mutex<Option<Arc<KdeKwinBridge>>> = Mutex::new(None);

pub(super) struct KdeKwinScriptProvider {
    snapshot: KwinSnapshot,
}

impl KdeKwinScriptProvider {
    pub(super) fn open() -> Option<Self> {
        if !is_kde_session() {
            return None;
        }

        let bridge = bridge()?;
        let Some(snapshot) = bridge.wait_for_snapshot(SNAPSHOT_WAIT) else {
            clear_bridge(&bridge);
            return None;
        };
        Some(Self { snapshot })
    }
}

impl LinuxWindowProvider for KdeKwinScriptProvider {
    fn focused_window(&self) -> Option<ProtectableWindow> {
        self.snapshot.focused_window()
    }

    fn list_windows(&self) -> ProtectableWindowList {
        ProtectableWindowList {
            provider: Some(PROVIDER_NAME.to_string()),
            status: ProtectableWindowProviderStatus::Available,
            windows: self.snapshot.protectable_windows(),
        }
    }

    fn status(&self) -> WindowProtectionStatus {
        WindowProtectionStatus {
            message: "Focused-window detection is available through KDE KWin scripting."
                .to_string(),
            platform: "linux".to_string(),
            provider: Some(PROVIDER_NAME.to_string()),
            status: ProtectableWindowProviderStatus::Available,
        }
    }
}

struct KdeKwinBridge {
    _connection: Connection,
    _script_path: PathBuf,
    state: Arc<KdeKwinBridgeState>,
}

impl KdeKwinBridge {
    fn start() -> Option<Arc<Self>> {
        let token = Uuid::now_v7().to_string();
        let state = Arc::new(KdeKwinBridgeState::new(token));
        let connection = ConnectionBuilder::session()
            .ok()?
            .serve_at(
                OBJECT_PATH,
                KdeKwinBridgeInterface {
                    state: Arc::clone(&state),
                },
            )
            .ok()?
            .name(BUS_NAME)
            .ok()?
            .build()
            .ok()?;
        let script_path = write_kwin_script(&state.token).ok()?;
        load_kwin_script(&script_path).ok()?;

        Some(Arc::new(Self {
            _connection: connection,
            _script_path: script_path,
            state,
        }))
    }

    fn wait_for_snapshot(&self, timeout: Duration) -> Option<KwinSnapshot> {
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(snapshot) = self.state.snapshot() {
                return Some(snapshot);
            }

            if Instant::now() >= deadline {
                return None;
            }

            thread::sleep(Duration::from_millis(25));
        }
    }
}

struct KdeKwinBridgeState {
    latest: Mutex<Option<KwinSnapshot>>,
    token: String,
}

impl KdeKwinBridgeState {
    fn new(token: String) -> Self {
        Self {
            latest: Mutex::new(None),
            token,
        }
    }

    fn accept_snapshot(&self, token: &str, payload: &str) {
        if token != self.token || payload.len() > MAX_PAYLOAD_BYTES {
            return;
        }

        let Ok(snapshot) = parse_snapshot(payload) else {
            return;
        };
        if snapshot.provider != PROVIDER_NAME {
            return;
        }

        let Ok(mut latest) = self.latest.lock() else {
            return;
        };

        if latest
            .as_ref()
            .is_some_and(|current| snapshot.sequence <= current.sequence)
        {
            return;
        }

        *latest = Some(snapshot);
    }

    fn snapshot(&self) -> Option<KwinSnapshot> {
        self.latest.lock().ok().and_then(|latest| latest.clone())
    }
}

struct KdeKwinBridgeInterface {
    state: Arc<KdeKwinBridgeState>,
}

#[interface(name = "com.snackbit.Daydream.WindowMetadata")]
impl KdeKwinBridgeInterface {
    #[zbus(name = "UpdateSnapshot")]
    fn update_snapshot(&self, token: &str, payload: &str) {
        self.state.accept_snapshot(token, payload);
    }
}

fn bridge() -> Option<Arc<KdeKwinBridge>> {
    let mut bridge = BRIDGE.lock().ok()?;
    if let Some(existing) = bridge.as_ref() {
        return Some(Arc::clone(existing));
    }

    let next = KdeKwinBridge::start()?;
    *bridge = Some(Arc::clone(&next));
    Some(next)
}

fn clear_bridge(current: &Arc<KdeKwinBridge>) {
    let Ok(mut bridge) = BRIDGE.lock() else {
        return;
    };

    if bridge
        .as_ref()
        .is_some_and(|existing| Arc::ptr_eq(existing, current))
    {
        *bridge = None;
    }
}

fn load_kwin_script(script_path: &PathBuf) -> zbus::Result<()> {
    let connection = ConnectionBuilder::session()?
        .method_timeout(Duration::from_secs(2))
        .build()?;
    let proxy = zbus::blocking::Proxy::new(
        &connection,
        "org.kde.KWin",
        "/Scripting",
        "org.kde.kwin.Scripting",
    )?;

    let _: zbus::Result<()> = proxy.call("unloadScript", &(PLUGIN_NAME,));
    let script_path = script_path.to_string_lossy().to_string();
    let _: i32 = proxy.call("loadScript", &(script_path, PLUGIN_NAME))?;
    proxy.call("start", &())
}

fn write_kwin_script(token: &str) -> std::io::Result<PathBuf> {
    let script_dir = kwin_script_dir();
    DirBuilder::new()
        .mode(0o700)
        .recursive(true)
        .create(&script_dir)?;
    let script_path = script_dir.join("daydream-window-metadata.js");
    let script = kwin_script(token);
    let mut file = OpenOptions::new()
        .create(true)
        .mode(0o600)
        .truncate(true)
        .write(true)
        .open(&script_path)?;
    file.write_all(script.as_bytes())?;
    Ok(script_path)
}

fn kwin_script_dir() -> PathBuf {
    std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join(format!("daydream-kwin-{}", std::process::id()))
}

fn kwin_script(token: &str) -> String {
    format!(
        r#"const DAYDREAM_SERVICE = {service};
const DAYDREAM_PATH = {path};
const DAYDREAM_INTERFACE = {interface};
const DAYDREAM_TOKEN = {token};
const DAYDREAM_PROVIDER = {provider};

let daydreamSequence = 0;
const daydreamConnectedWindows = new Map();

function daydreamString(value) {{
    if (value === undefined || value === null) {{
        return null;
    }}
    const text = String(value).trim();
    return text.length > 0 ? text : null;
}}

function daydreamWindowId(window) {{
    return daydreamString(window.internalId) ||
        daydreamString(window.windowId) ||
        daydreamString(window.pid) ||
        daydreamString(window.caption) ||
        "window-" + String(window.stackingOrder);
}}

function daydreamProcessId(window) {{
    return typeof window.pid === "number" && Number.isFinite(window.pid) ? window.pid : null;
}}

function daydreamWindowSnapshot(window) {{
    const desktopFileName = daydreamString(window.desktopFileName);
    const resourceClass = daydreamString(window.resourceClass);
    const resourceName = daydreamString(window.resourceName);
    return {{
        id: daydreamWindowId(window),
        appIdentity: desktopFileName || resourceClass || resourceName,
        processName: resourceClass || resourceName || desktopFileName,
        processId: daydreamProcessId(window),
        title: daydreamString(window.caption)
    }};
}}

function daydreamWindows() {{
    return workspace.windowList()
        .filter(window => window.normalWindow)
        .map(window => {{
            daydreamConnectWindow(window);
            return daydreamWindowSnapshot(window);
        }});
}}

function daydreamEmitSnapshot() {{
    const activeWindow = workspace.activeWindow;
    const payload = JSON.stringify({{
        provider: DAYDREAM_PROVIDER,
        sequence: ++daydreamSequence,
        activeWindowId: activeWindow && activeWindow.normalWindow ? daydreamWindowId(activeWindow) : null,
        windows: daydreamWindows()
    }});
    callDBus(DAYDREAM_SERVICE, DAYDREAM_PATH, DAYDREAM_INTERFACE, "UpdateSnapshot", DAYDREAM_TOKEN, payload);
}}

function daydreamConnectSignal(object, signalName) {{
    if (object && object[signalName]) {{
        object[signalName].connect(daydreamEmitSnapshot);
    }}
}}

function daydreamConnectWindow(window) {{
    if (daydreamConnectedWindows.has(window)) {{
        return;
    }}

    daydreamConnectedWindows.set(window, true);
    daydreamConnectSignal(window, "activeChanged");
    daydreamConnectSignal(window, "captionChanged");
    daydreamConnectSignal(window, "desktopFileNameChanged");
    daydreamConnectSignal(window, "resourceClassChanged");
    daydreamConnectSignal(window, "resourceNameChanged");
    if (window.closed) {{
        window.closed.connect(() => {{
            daydreamConnectedWindows.delete(window);
            daydreamEmitSnapshot();
        }});
    }}
}}

function daydreamMain() {{
    workspace.windowList().forEach(daydreamConnectWindow);
    workspace.windowAdded.connect(window => {{
        daydreamConnectWindow(window);
        daydreamEmitSnapshot();
    }});
    daydreamConnectSignal(workspace, "currentDesktopChanged");
    daydreamConnectSignal(workspace, "currentActivityChanged");
    daydreamConnectSignal(workspace, "activeWindowChanged");
    daydreamEmitSnapshot();
}}

daydreamMain();
"#,
        service = js_string_literal(BUS_NAME),
        path = js_string_literal(OBJECT_PATH),
        interface = js_string_literal(INTERFACE_NAME),
        token = js_string_literal(token),
        provider = js_string_literal(PROVIDER_NAME),
    )
}

fn js_string_literal(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "\"\"".to_string())
}

fn parse_snapshot(payload: &str) -> Result<KwinSnapshot, serde_json::Error> {
    let mut snapshot: KwinSnapshot = serde_json::from_str(payload)?;
    snapshot.windows.retain(|window| window.id.trim().len() > 0);
    snapshot.windows.iter_mut().for_each(KwinWindow::sanitize);
    Ok(snapshot)
}

fn is_kde_session() -> bool {
    std::env::var("XDG_CURRENT_DESKTOP")
        .ok()
        .is_some_and(|value| {
            value
                .split(':')
                .any(|part| part.eq_ignore_ascii_case("KDE"))
        })
        || std::env::var("KDE_FULL_SESSION")
            .ok()
            .is_some_and(|value| value.eq_ignore_ascii_case("true"))
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct KwinSnapshot {
    active_window_id: Option<String>,
    provider: String,
    sequence: u64,
    windows: Vec<KwinWindow>,
}

impl KwinSnapshot {
    fn focused_window(&self) -> Option<ProtectableWindow> {
        let active_window_id = self.active_window_id.as_deref()?;
        self.windows
            .iter()
            .find(|window| window.id == active_window_id)
            .map(KwinWindow::to_protectable_window)
    }

    fn protectable_windows(&self) -> Vec<ProtectableWindow> {
        self.windows
            .iter()
            .map(KwinWindow::to_protectable_window)
            .collect()
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct KwinWindow {
    app_identity: Option<String>,
    id: String,
    process_id: Option<u32>,
    process_name: Option<String>,
    title: Option<String>,
}

impl KwinWindow {
    fn sanitize(&mut self) {
        self.id = self.id.trim().to_string();
        self.app_identity = non_empty_string(self.app_identity.take());
        self.process_name = non_empty_string(self.process_name.take());
        self.title = non_empty_string(self.title.take());
    }

    fn to_protectable_window(&self) -> ProtectableWindow {
        ProtectableWindow {
            app_identity: self.app_identity.clone(),
            id: format!("kde-kwin:{}", self.id),
            platform: "linux".to_string(),
            process_id: self.process_id,
            process_name: self.process_name.clone(),
            title: self.title.clone(),
        }
    }
}

fn non_empty_string(value: Option<String>) -> Option<String> {
    let value = value?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        js_string_literal, kwin_script, parse_snapshot, KdeKwinBridgeState, PROVIDER_NAME,
    };

    #[test]
    fn parses_kde_snapshot_into_protectable_windows() {
        let snapshot = parse_snapshot(
            r#"{
                "provider": "linux-kde-kwin-script",
                "sequence": 7,
                "activeWindowId": "abc",
                "windows": [
                    {
                        "id": " abc ",
                        "appIdentity": "org.kde.konsole",
                        "processName": "konsole",
                        "processId": 123,
                        "title": " Terminal "
                    },
                    {
                        "id": "   ",
                        "appIdentity": "ignored",
                        "processName": "ignored",
                        "processId": null,
                        "title": "ignored"
                    }
                ]
            }"#,
        )
        .expect("snapshot should parse");

        assert_eq!(snapshot.windows.len(), 1);
        assert_eq!(
            snapshot.focused_window().expect("focused").id,
            "kde-kwin:abc"
        );
        assert_eq!(
            snapshot.protectable_windows()[0].title.as_deref(),
            Some("Terminal")
        );
    }

    #[test]
    fn rejects_foreign_or_stale_kde_updates() {
        let state = KdeKwinBridgeState::new("current-token".to_string());
        let first_payload = format!(
            r#"{{
                "provider": "{PROVIDER_NAME}",
                "sequence": 2,
                "activeWindowId": null,
                "windows": []
            }}"#
        );
        let stale_payload = format!(
            r#"{{
                "provider": "{PROVIDER_NAME}",
                "sequence": 1,
                "activeWindowId": null,
                "windows": []
            }}"#
        );
        let wrong_provider_payload = r#"{
            "provider": "foreign-provider",
            "sequence": 3,
            "activeWindowId": null,
            "windows": []
        }"#;

        state.accept_snapshot("wrong-token", &first_payload);
        assert!(state.snapshot().is_none());

        state.accept_snapshot("current-token", &first_payload);
        assert_eq!(state.snapshot().expect("snapshot").sequence, 2);

        state.accept_snapshot("current-token", &stale_payload);
        assert_eq!(state.snapshot().expect("snapshot").sequence, 2);

        state.accept_snapshot("current-token", wrong_provider_payload);
        assert_eq!(state.snapshot().expect("snapshot").sequence, 2);
    }

    #[test]
    fn generated_kwin_script_targets_daydream_bridge() {
        let script = kwin_script("token-with-\"quote");

        assert!(script.contains("callDBus"));
        assert!(script.contains("UpdateSnapshot"));
        assert!(script.contains(&js_string_literal("token-with-\"quote")));
        assert!(script.contains(PROVIDER_NAME));
    }
}
