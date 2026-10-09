//! Picker-only local application icons. Capture metadata remains unchanged.
use super::types::{ProtectableWindow, ProtectableWindowList, ProtectableWindowProviderStatus};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

const MAX_CACHE_ENTRIES: usize = 128;
const MAX_PNG_BYTES: usize = 64 * 1024;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowPickerWindow {
    #[serde(flatten)]
    window: ProtectableWindow,
    icon_key: Option<String>,
}
#[derive(Serialize)]
pub struct WindowPickerList {
    provider: Option<String>,
    status: ProtectableWindowProviderStatus,
    windows: Vec<WindowPickerWindow>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetWindowApplicationIconInput {
    pub icon_key: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowApplicationIcon {
    pub icon_key: String,
    pub png_base64: Option<String>,
}

#[derive(Clone)]
struct IconEntry {
    key: String,
    names: Vec<String>,
    png_base64: Option<String>,
    resolved: bool,
}
#[derive(Default)]
struct IconCache {
    entries: VecDeque<IconEntry>,
}
impl IconCache {
    fn register(&mut self, key: String, names: Vec<String>) -> String {
        if let Some(index) = self.entries.iter().position(|entry| entry.key == key) {
            if let Some(entry) = self.entries.remove(index) {
                self.entries.push_back(entry);
            }
        } else {
            while self.entries.len() >= MAX_CACHE_ENTRIES {
                self.entries.pop_front();
            }
            self.entries.push_back(IconEntry {
                key: key.clone(),
                names,
                png_base64: None,
                resolved: false,
            });
        }
        key
    }
    fn get(&mut self, key: &str) -> Option<IconEntry> {
        let index = self.entries.iter().position(|entry| entry.key == key)?;
        let entry = self.entries.remove(index)?;
        self.entries.push_back(entry.clone());
        Some(entry)
    }
    fn resolve(&mut self, key: &str, png: Option<Vec<u8>>) -> Option<String> {
        use base64::Engine;
        let entry = self.entries.iter_mut().find(|entry| entry.key == key)?;
        entry.png_base64 = png
            .filter(|bytes| valid_png(bytes))
            .map(|bytes| base64::engine::general_purpose::STANDARD.encode(bytes));
        entry.resolved = true;
        entry.png_base64.clone()
    }
}
fn valid_png(bytes: &[u8]) -> bool {
    bytes.len() <= MAX_PNG_BYTES && bytes.starts_with(b"\x89PNG\r\n\x1a\n")
}
fn picker_list(list: ProtectableWindowList, keys: Vec<Option<String>>) -> WindowPickerList {
    WindowPickerList {
        provider: list.provider,
        status: list.status,
        windows: list
            .windows
            .into_iter()
            .zip(keys)
            .map(|(window, icon_key)| WindowPickerWindow { window, icon_key })
            .collect(),
    }
}

pub async fn enrich_picker(app: tauri::AppHandle, list: ProtectableWindowList) -> WindowPickerList {
    let fallback = vec![None; list.windows.len()];
    if list.status != ProtectableWindowProviderStatus::Available {
        return picker_list(list, fallback);
    }
    #[cfg(target_os = "linux")]
    {
        let windows = list.windows.clone();
        let (sender, receiver) = tokio::sync::oneshot::channel();
        if app
            .run_on_main_thread(move || {
                let _ = sender.send(linux::register_windows(&windows));
            })
            .is_ok()
        {
            if let Ok(Ok(keys)) =
                tokio::time::timeout(std::time::Duration::from_secs(2), receiver).await
            {
                return picker_list(list, keys);
            }
        }
    }
    #[cfg(not(target_os = "linux"))]
    let _ = app;
    picker_list(list, fallback)
}

pub async fn get_window_application_icon(
    app: tauri::AppHandle,
    input: GetWindowApplicationIconInput,
) -> WindowApplicationIcon {
    let mut result = WindowApplicationIcon {
        icon_key: input.icon_key.clone(),
        png_base64: None,
    };
    // The frontend can request only an issued cache handle, never a path/name.
    if input.icon_key.len() != 76
        || !input.icon_key.starts_with("window-icon:")
        || !input.icon_key[12..]
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return result;
    }
    #[cfg(target_os = "linux")]
    {
        let (sender, receiver) = tokio::sync::oneshot::channel();
        if app
            .run_on_main_thread(move || {
                let _ = sender.send(linux::load_icon(&input.icon_key));
            })
            .is_ok()
        {
            if let Ok(Ok(icon)) =
                tokio::time::timeout(std::time::Duration::from_secs(2), receiver).await
            {
                result.png_base64 = icon;
            }
        }
    }
    #[cfg(not(target_os = "linux"))]
    let _ = app;
    result
}

#[cfg(target_os = "linux")]
mod linux {
    use super::*;
    use gio::glib::translate::{from_glib_none, ToGlibPtr};
    use gio::prelude::*;
    use gtk::prelude::*;
    use sha2::{Digest, Sha256};
    use std::sync::{Mutex, OnceLock};
    static CACHE: OnceLock<Mutex<IconCache>> = OnceLock::new();
    fn cache() -> &'static Mutex<IconCache> {
        CACHE.get_or_init(|| Mutex::new(IconCache::default()))
    }
    fn optional_executable(app: &gio::DesktopAppInfo) -> Option<std::path::PathBuf> {
        // DBusActivatable desktop entries may legitimately omit Exec. gio0.18's
        // non-optional AppInfoExt::executable binding assumes a non-null pointer.
        // Transfer:none belongs to the live borrowed app; Option checks null and
        // from_glib_none copies the platform-native path before returning.
        unsafe {
            from_glib_none(gio::ffi::g_app_info_get_executable(
                app.upcast_ref::<gio::AppInfo>().to_glib_none().0,
            ))
        }
    }
    pub(super) fn identity_matches(identity: &str, app: &gio::DesktopAppInfo) -> bool {
        let id = app.id();
        let class = app.startup_wm_class();
        let executable = optional_executable(app);
        let basename = executable
            .as_deref()
            .and_then(std::path::Path::file_name)
            .and_then(|name| name.to_str());
        let matched = [
            id.as_deref()
                .map(|value| value.strip_suffix(".desktop").unwrap_or(value)),
            class.as_deref(),
            basename,
        ]
        .into_iter()
        .flatten()
        .any(|candidate| candidate.eq_ignore_ascii_case(identity));
        matched
    }
    pub(super) fn register_windows(windows: &[ProtectableWindow]) -> Vec<Option<String>> {
        if !gtk::is_initialized_main_thread() {
            return vec![None; windows.len()];
        }
        // This inventory is picker-only; no GIO/theme work occurs in capture callbacks.
        let apps: Vec<_> = gio::AppInfo::all()
            .into_iter()
            .take(1024)
            .filter_map(|app| app.downcast::<gio::DesktopAppInfo>().ok())
            .collect();
        windows
            .iter()
            .map(|window| {
                // App identity owns the match. A process fallback is usable only
                // when local desktop metadata identifies exactly one application.
                let mut matching: Vec<_> = window
                    .app_identity
                    .as_deref()
                    .map(|identity| {
                        apps.iter()
                            .filter(|app| identity_matches(identity, app))
                            .collect()
                    })
                    .unwrap_or_default();
                if matching.is_empty() {
                    matching = window
                        .process_name
                        .as_deref()
                        .map(|identity| {
                            apps.iter()
                                .filter(|app| identity_matches(identity, app))
                                .collect()
                        })
                        .unwrap_or_default();
                }
                if matching.len() != 1 {
                    return None;
                }
                let app = matching[0];
                let desktop_id = app.id()?;
                let themed = app.icon()?.downcast::<gio::ThemedIcon>().ok()?;
                let names: Vec<_> = themed
                    .names()
                    .into_iter()
                    .filter(|name| name.len() <= 128 && !name.contains('/') && !name.contains('\\'))
                    .take(16)
                    .map(|name| name.to_string())
                    .collect();
                if names.is_empty() {
                    return None;
                }
                let key = format!(
                    "window-icon:{}",
                    hex::encode(Sha256::digest(desktop_id.as_bytes()))
                );
                Some(cache().lock().ok()?.register(key, names))
            })
            .collect()
    }
    pub(super) fn load_icon(key: &str) -> Option<String> {
        if !gtk::is_initialized_main_thread() {
            return None;
        }
        let entry = cache().lock().ok()?.get(key)?;
        if entry.resolved {
            return entry.png_base64;
        }
        let theme = gtk::IconTheme::default()?;
        let names: Vec<_> = entry.names.iter().map(String::as_str).collect();
        let icon = gio::ThemedIcon::from_names(&names);
        let png = theme
            .lookup_by_gicon(&icon, 32, gtk::IconLookupFlags::FORCE_SIZE)
            .and_then(|info| {
                // Only installed theme entries are resolved; bound encoded input before decoding.
                let path = info.filename()?;
                if std::fs::metadata(path).ok()?.len() > 2 * 1024 * 1024 {
                    return None;
                }
                let pixbuf = info.load_icon().ok()?;
                if pixbuf.width() > 32 || pixbuf.height() > 32 {
                    return None;
                }
                pixbuf.save_to_bufferv("png", &[]).ok()
            });
        cache().lock().ok()?.resolve(key, png)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cache_evicts_old_handles_and_reuses_recent_apps() {
        let mut cache = IconCache::default();
        cache.register("first".into(), vec!["synthetic".into()]);
        for index in 1..MAX_CACHE_ENTRIES {
            cache.register(index.to_string(), vec![]);
        }
        assert!(cache.get("first").is_some());
        cache.register("latest".into(), vec![]);
        assert_eq!(cache.entries.len(), MAX_CACHE_ENTRIES);
        assert!(cache.get("1").is_none());
        assert!(cache.get("first").is_some());
    }
    #[test]
    fn oversized_or_non_png_images_never_enter_the_cache() {
        let mut cache = IconCache::default();
        cache.register("app".into(), vec![]);
        assert!(cache
            .resolve("app", Some(vec![0; MAX_PNG_BYTES + 1]))
            .is_none());
        assert!(cache.resolve("app", Some(b"<svg/>".to_vec())).is_none());
        assert!(cache
            .resolve("unknown", Some(b"\x89PNG\r\n\x1a\n".to_vec()))
            .is_none());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn dbus_activated_desktop_metadata_without_exec_remains_matchable() {
        let directory =
            std::env::temp_dir().join(format!("daydream-icon-metadata-{}", uuid::Uuid::now_v7()));
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("org.example.SyntheticNoExec.desktop");
        std::fs::write(&path, "[Desktop Entry]\nType=Application\nName=Synthetic no-exec application\nDBusActivatable=true\nStartupWMClass=SyntheticNoExec\nIcon=utilities-terminal\n").unwrap();
        let app = gio::DesktopAppInfo::from_filename(&path)
            .expect("DBus-activated metadata without Exec is valid");
        // Exercise the actual GIO reader and nullable executable boundary; no GTK
        // display, live application or launch/network operation is required.
        let matches = linux::identity_matches("SyntheticNoExec", &app);
        let unrelated = linux::identity_matches("unrelated-synthetic-process", &app);
        std::fs::remove_dir_all(directory).unwrap();
        assert!(matches);
        assert!(!unrelated);
    }

    #[cfg(target_os = "linux")]
    #[test]
    #[ignore = "requires a GTK display and installed desktop applications/icon theme"]
    fn installed_application_metadata_resolves_a_bounded_local_png() {
        use base64::Engine;
        use gio::prelude::*;
        gtk::init().expect("GTK display is required for the local icon smoke test");
        // Only installed desktop metadata is read. No live windows, recording,
        // model/network requests or personal captured content are involved.
        let identities: Vec<_> = gio::AppInfo::all()
            .into_iter()
            .take(1024)
            .filter_map(|app| app.id())
            .map(|id| id.strip_suffix(".desktop").unwrap_or(&id).to_string())
            .collect();
        let windows: Vec<_> = identities
            .into_iter()
            .map(|identity| ProtectableWindow {
                app_identity: Some(identity),
                id: "synthetic-icon-probe".into(),
                platform: "linux".into(),
                process_id: None,
                process_name: None,
                title: None,
            })
            .collect();
        let keys = linux::register_windows(&windows);
        for key in keys.into_iter().flatten() {
            let Some(encoded) = linux::load_icon(&key) else {
                continue;
            };
            let png = base64::engine::general_purpose::STANDARD
                .decode(encoded)
                .expect("local icon must be PNG base64");
            assert!(valid_png(&png));
            return;
        }
        panic!("No installed application had a resolvable local themed icon");
    }
}
