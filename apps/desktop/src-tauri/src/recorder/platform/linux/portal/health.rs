use ashpd::desktop::screencast::{CursorMode, Screencast, SourceType};
use enumflags2::BitFlags;

use super::super::super::super::types::RecorderPortalHealth;

pub async fn collect_portal_health() -> RecorderPortalHealth {
    match Screencast::new().await {
        Ok(proxy) => {
            let source_types = proxy
                .available_source_types()
                .await
                .map(source_type_labels)
                .unwrap_or_default();
            let cursor_modes = proxy
                .available_cursor_modes()
                .await
                .map(cursor_mode_labels)
                .unwrap_or_default();

            RecorderPortalHealth {
                available: source_types.iter().any(|source| source == "monitor"),
                cursor_modes,
                error: None,
                screen_cast_version: Some(proxy.version()),
                source_types,
            }
        }
        Err(error) => RecorderPortalHealth {
            available: false,
            cursor_modes: Vec::new(),
            error: Some(format!("ScreenCast portal is unavailable: {error}")),
            screen_cast_version: None,
            source_types: Vec::new(),
        },
    }
}

pub(super) fn source_type_labels(flags: BitFlags<SourceType>) -> Vec<String> {
    let mut labels = Vec::new();

    if flags.contains(SourceType::Monitor) {
        labels.push("monitor".to_string());
    }
    if flags.contains(SourceType::Window) {
        labels.push("window".to_string());
    }
    if flags.contains(SourceType::Virtual) {
        labels.push("virtual".to_string());
    }

    labels
}

pub(super) fn cursor_mode_labels(flags: BitFlags<CursorMode>) -> Vec<String> {
    let mut labels = Vec::new();

    if flags.contains(CursorMode::Hidden) {
        labels.push("hidden".to_string());
    }
    if flags.contains(CursorMode::Embedded) {
        labels.push("embedded".to_string());
    }
    if flags.contains(CursorMode::Metadata) {
        labels.push("metadata".to_string());
    }

    labels
}

pub(super) fn preferred_cursor_mode(flags: BitFlags<CursorMode>) -> Option<CursorMode> {
    if flags.contains(CursorMode::Embedded) {
        Some(CursorMode::Embedded)
    } else if flags.contains(CursorMode::Metadata) {
        Some(CursorMode::Metadata)
    } else if flags.contains(CursorMode::Hidden) {
        Some(CursorMode::Hidden)
    } else {
        None
    }
}
