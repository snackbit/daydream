use super::{BUS_NAME, INTERFACE_NAME, OBJECT_PATH, PROVIDER_NAME};
use std::{
    fs::{DirBuilder, OpenOptions},
    io::Write,
    os::unix::fs::{DirBuilderExt, OpenOptionsExt},
    path::PathBuf,
};

pub(super) fn write_kwin_script(token: &str) -> std::io::Result<PathBuf> {
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

pub(super) fn kwin_script(token: &str) -> String {
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

pub(super) fn js_string_literal(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "\"\"".to_string())
}
