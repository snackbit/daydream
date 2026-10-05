mod kde;
mod wayland;
mod x11;

use crate::recorder::types::{
    ProtectableWindow, ProtectableWindowList, ProtectableWindowProviderStatus,
    WindowMetadataSnapshot, WindowProtectionStatus,
};

trait LinuxWindowProvider {
    fn focused_window(&self) -> Option<ProtectableWindow>;
    fn list_windows(&self) -> ProtectableWindowList;
    fn status(&self) -> WindowProtectionStatus;
}

pub(super) fn list_protectable_windows() -> ProtectableWindowList {
    provider()
        .map(|provider| provider.list_windows())
        .unwrap_or_else(unavailable_window_list)
}

pub(super) fn window_protection_status() -> WindowProtectionStatus {
    provider()
        .map(|provider| provider.status())
        .unwrap_or_else(unavailable_window_status)
}

pub(super) fn window_metadata_snapshot() -> WindowMetadataSnapshot {
    let Some(provider) = provider() else {
        let list = unavailable_window_list();
        return WindowMetadataSnapshot {
            focused_window: None,
            provider: list.provider,
            status: list.status,
            windows: list.windows,
        };
    };

    let focused_window = provider.focused_window();
    let list = provider.list_windows();
    WindowMetadataSnapshot {
        focused_window,
        provider: list.provider,
        status: list.status,
        windows: list.windows,
    }
}

fn provider() -> Option<Box<dyn LinuxWindowProvider>> {
    match session_kind() {
        LinuxSessionKind::Wayland => wayland::WlrForeignToplevelProvider::open()
            .map(|provider| Box::new(provider) as Box<dyn LinuxWindowProvider>)
            .or_else(|| {
                kde::KdeKwinScriptProvider::open()
                    .map(|provider| Box::new(provider) as Box<dyn LinuxWindowProvider>)
            }),
        LinuxSessionKind::X11 => x11::X11WindowProvider::open()
            .map(|provider| Box::new(provider) as Box<dyn LinuxWindowProvider>),
        LinuxSessionKind::Unknown => x11::X11WindowProvider::open()
            .map(|provider| Box::new(provider) as Box<dyn LinuxWindowProvider>),
    }
}

fn unavailable_window_list() -> ProtectableWindowList {
    ProtectableWindowList {
        provider: Some(provider_name()),
        status: unavailable_status(),
        windows: Vec::new(),
    }
}

fn unavailable_window_status() -> WindowProtectionStatus {
    let status = unavailable_status();
    WindowProtectionStatus {
        message: unavailable_message(&status),
        platform: "linux".to_string(),
        provider: Some(provider_name()),
        status,
    }
}

fn provider_name() -> String {
    match session_kind() {
        LinuxSessionKind::Wayland => "linux-wayland".to_string(),
        LinuxSessionKind::X11 => "linux-x11".to_string(),
        LinuxSessionKind::Unknown => "linux-unknown".to_string(),
    }
}

fn unavailable_status() -> ProtectableWindowProviderStatus {
    match session_kind() {
        LinuxSessionKind::Wayland => ProtectableWindowProviderStatus::ProviderUnavailable,
        LinuxSessionKind::X11 | LinuxSessionKind::Unknown => {
            ProtectableWindowProviderStatus::ProviderUnavailable
        }
    }
}

fn unavailable_message(status: &ProtectableWindowProviderStatus) -> String {
    unavailable_message_for_session(session_kind(), status)
}

fn unavailable_message_for_session(
    session: LinuxSessionKind,
    status: &ProtectableWindowProviderStatus,
) -> String {
    match (session, status) {
        (LinuxSessionKind::Wayland, ProtectableWindowProviderStatus::ProviderUnavailable) => {
            "Focused-window detection requires a Wayland compositor that exposes wlr-foreign-toplevel-management or KDE KWin scripting. This session does not advertise a supported window metadata protocol.".to_string()
        }
        (LinuxSessionKind::X11 | LinuxSessionKind::Unknown, _) => {
            "Focused-window detection requires an X11 display.".to_string()
        }
        _ => "Focused-window detection is unavailable.".to_string(),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LinuxSessionKind {
    Unknown,
    Wayland,
    X11,
}

fn session_kind() -> LinuxSessionKind {
    match std::env::var("XDG_SESSION_TYPE") {
        Ok(value) if value.eq_ignore_ascii_case("wayland") => LinuxSessionKind::Wayland,
        Ok(value) if value.eq_ignore_ascii_case("x11") => LinuxSessionKind::X11,
        _ => LinuxSessionKind::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::{unavailable_message_for_session, LinuxSessionKind};
    use crate::recorder::types::ProtectableWindowProviderStatus;

    #[test]
    fn wayland_unavailable_message_names_required_protocol() {
        let message = unavailable_message_for_session(
            LinuxSessionKind::Wayland,
            &ProtectableWindowProviderStatus::ProviderUnavailable,
        );

        assert!(message.contains("wlr-foreign-toplevel-management"));
        assert!(message.contains("KDE KWin scripting"));
        assert!(message.contains("supported window metadata protocol"));
    }
}
