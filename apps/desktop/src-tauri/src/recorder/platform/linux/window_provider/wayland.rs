use std::collections::BTreeMap;

use wayland_client::{
    globals::{registry_queue_init, GlobalListContents},
    protocol::{wl_output, wl_registry},
    Connection, Dispatch, Proxy, QueueHandle, WEnum,
};
use wayland_protocols_wlr::foreign_toplevel::v1::client::{
    zwlr_foreign_toplevel_handle_v1::{
        self, State as WlrToplevelState, ZwlrForeignToplevelHandleV1,
    },
    zwlr_foreign_toplevel_manager_v1::{self, ZwlrForeignToplevelManagerV1},
};

use super::LinuxWindowProvider;
use crate::recorder::types::{
    ProtectableWindow, ProtectableWindowList, ProtectableWindowProviderStatus,
    WindowProtectionStatus,
};

pub(super) struct WlrForeignToplevelProvider {
    focused: Option<ProtectableWindow>,
    windows: Vec<ProtectableWindow>,
}

impl WlrForeignToplevelProvider {
    pub(super) fn open() -> Option<Self> {
        let conn = Connection::connect_to_env().ok()?;
        let (globals, mut event_queue) =
            registry_queue_init::<WlrForeignToplevelSnapshot>(&conn).ok()?;
        let qh = event_queue.handle();
        let _manager: ZwlrForeignToplevelManagerV1 = globals.bind(&qh, 1..=3, ()).ok()?;
        let mut snapshot = WlrForeignToplevelSnapshot::default();

        for _ in 0..2 {
            event_queue.roundtrip(&mut snapshot).ok()?;
        }

        let windows = snapshot.protectable_windows();
        let focused = snapshot.focused_window();

        Some(Self { focused, windows })
    }
}

impl LinuxWindowProvider for WlrForeignToplevelProvider {
    fn focused_window(&self) -> Option<ProtectableWindow> {
        self.focused.clone()
    }

    fn list_windows(&self) -> ProtectableWindowList {
        ProtectableWindowList {
            provider: Some("linux-wayland-wlr-foreign-toplevel".to_string()),
            status: ProtectableWindowProviderStatus::Available,
            windows: self.windows.clone(),
        }
    }

    fn status(&self) -> WindowProtectionStatus {
        WindowProtectionStatus {
            message: "Focused-window detection is available through wlr foreign toplevel metadata."
                .to_string(),
            platform: "linux".to_string(),
            provider: Some("linux-wayland-wlr-foreign-toplevel".to_string()),
            status: ProtectableWindowProviderStatus::Available,
        }
    }
}

#[derive(Default)]
struct WlrForeignToplevelSnapshot {
    windows: BTreeMap<String, WlrWindowSnapshot>,
}

impl WlrForeignToplevelSnapshot {
    fn focused_window(&self) -> Option<ProtectableWindow> {
        self.windows
            .values()
            .filter(|window| !window.closed)
            .find(|window| window.activated)
            .map(WlrWindowSnapshot::to_protectable_window)
    }

    fn protectable_windows(&self) -> Vec<ProtectableWindow> {
        self.windows
            .values()
            .filter(|window| !window.closed)
            .map(WlrWindowSnapshot::to_protectable_window)
            .collect()
    }
}

#[derive(Clone, Debug, Default)]
struct WlrWindowSnapshot {
    activated: bool,
    app_id: Option<String>,
    closed: bool,
    id: String,
    title: Option<String>,
}

impl WlrWindowSnapshot {
    fn to_protectable_window(&self) -> ProtectableWindow {
        ProtectableWindow {
            app_identity: self.app_id.clone(),
            id: self.id.clone(),
            platform: "linux".to_string(),
            process_id: None,
            process_name: self.app_id.clone(),
            title: self.title.clone(),
        }
    }
}

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for WlrForeignToplevelSnapshot {
    fn event(
        _: &mut Self,
        _: &wl_registry::WlRegistry,
        _: wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwlrForeignToplevelManagerV1, ()> for WlrForeignToplevelSnapshot {
    fn event(
        state: &mut Self,
        _: &ZwlrForeignToplevelManagerV1,
        event: zwlr_foreign_toplevel_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let zwlr_foreign_toplevel_manager_v1::Event::Toplevel { toplevel } = event {
            let id = toplevel_id(&toplevel);
            state
                .windows
                .entry(id.clone())
                .or_insert(WlrWindowSnapshot {
                    id,
                    ..WlrWindowSnapshot::default()
                });
        }
    }
}

impl Dispatch<ZwlrForeignToplevelHandleV1, ()> for WlrForeignToplevelSnapshot {
    fn event(
        state: &mut Self,
        handle: &ZwlrForeignToplevelHandleV1,
        event: zwlr_foreign_toplevel_handle_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let id = toplevel_id(handle);
        let window = state
            .windows
            .entry(id.clone())
            .or_insert(WlrWindowSnapshot {
                id,
                ..WlrWindowSnapshot::default()
            });

        match event {
            zwlr_foreign_toplevel_handle_v1::Event::Title { title } => {
                window.title = non_empty_string(title);
            }
            zwlr_foreign_toplevel_handle_v1::Event::AppId { app_id } => {
                window.app_id = non_empty_string(app_id);
            }
            zwlr_foreign_toplevel_handle_v1::Event::State { state } => {
                window.activated = states_contain_activated(&state);
            }
            zwlr_foreign_toplevel_handle_v1::Event::Closed => {
                window.closed = true;
            }
            zwlr_foreign_toplevel_handle_v1::Event::OutputEnter { .. }
            | zwlr_foreign_toplevel_handle_v1::Event::OutputLeave { .. }
            | zwlr_foreign_toplevel_handle_v1::Event::Done
            | zwlr_foreign_toplevel_handle_v1::Event::Parent { .. } => {}
            _ => {}
        }
    }
}

impl Dispatch<wl_output::WlOutput, ()> for WlrForeignToplevelSnapshot {
    fn event(
        _: &mut Self,
        _: &wl_output::WlOutput,
        _: wl_output::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

fn toplevel_id(handle: &ZwlrForeignToplevelHandleV1) -> String {
    format!("wayland-wlr:{:?}", handle.id())
}

fn non_empty_string(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn states_contain_activated(states: &[u8]) -> bool {
    states
        .chunks_exact(4)
        .map(|chunk| u32::from_ne_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .any(|value| {
            matches!(
                WEnum::<WlrToplevelState>::from(value),
                WEnum::Value(WlrToplevelState::Activated)
            )
        })
}

#[cfg(test)]
mod tests {
    use super::{non_empty_string, states_contain_activated};
    use wayland_protocols_wlr::foreign_toplevel::v1::client::zwlr_foreign_toplevel_handle_v1::State;

    #[test]
    fn decodes_activated_state_from_wayland_state_array() {
        let mut states = Vec::new();
        states.extend_from_slice(&(State::Minimized as u32).to_ne_bytes());
        states.extend_from_slice(&(State::Activated as u32).to_ne_bytes());

        assert!(states_contain_activated(&states));
    }

    #[test]
    fn trims_empty_wayland_strings() {
        assert_eq!(non_empty_string("  ".to_string()), None);
        assert_eq!(
            non_empty_string(" Terminal ".to_string()),
            Some("Terminal".to_string())
        );
    }
}
