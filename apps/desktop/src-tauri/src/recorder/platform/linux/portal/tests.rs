use ashpd::desktop::screencast::{CursorMode, SourceType};
use enumflags2::BitFlags;

use super::errors::portal_error;
use super::health::{cursor_mode_labels, preferred_cursor_mode, source_type_labels};
use super::restore_token::{
    invalid_restore_token_error, missing_restore_token_error, restore_token_for_live_session,
    restore_token_for_selection,
};
use super::streams::capture_source_from_portal_fields;
use crate::recorder::types::RecorderErrorCategory;

#[test]
fn labels_available_source_types() {
    let labels = source_type_labels(SourceType::Monitor | SourceType::Window);

    assert_eq!(labels, vec!["monitor", "window"]);
}

#[test]
fn labels_available_cursor_modes() {
    let labels = cursor_mode_labels(CursorMode::Hidden | CursorMode::Embedded);

    assert_eq!(labels, vec!["hidden", "embedded"]);
}

#[test]
fn chooses_best_supported_cursor_mode() {
    assert_eq!(
        preferred_cursor_mode(CursorMode::Hidden | CursorMode::Metadata),
        Some(CursorMode::Metadata),
    );
    assert_eq!(
        preferred_cursor_mode(CursorMode::Hidden | CursorMode::Embedded),
        Some(CursorMode::Embedded),
    );
    assert_eq!(preferred_cursor_mode(BitFlags::<CursorMode>::empty()), None,);
}

#[test]
fn maps_portal_cancellation_to_permission_error() {
    let error = portal_error(ashpd::Error::Response(
        ashpd::desktop::ResponseError::Cancelled,
    ));

    assert_eq!(error.category, RecorderErrorCategory::Permission);
    assert_eq!(error.code, "recorder_portal_permission_denied");
}

#[test]
fn keeps_portal_logical_size_out_of_pixel_dimensions() {
    let source = capture_source_from_portal_fields(0, "monitor-1".to_string(), Some((1280, 720)));

    assert_eq!(source.width, None);
    assert_eq!(source.height, None);
    assert_eq!(source.logical_width, Some(1280));
    assert_eq!(source.logical_height, Some(720));
}

#[test]
fn force_chooser_bypasses_restore_token_reuse() {
    assert_eq!(
        restore_token_for_selection(Some("restore-token"), false),
        Some("restore-token"),
    );
    assert_eq!(
        restore_token_for_selection(Some("restore-token"), true),
        None,
    );
}

#[test]
fn live_session_requires_saved_restore_token() {
    let token =
        restore_token_for_live_session(Some("restore-token".to_string())).expect("restore token");

    assert_eq!(token, "restore-token");

    let error =
        restore_token_for_live_session(None).expect_err("missing token should fail live start");
    assert_eq!(error.category, RecorderErrorCategory::Permission);
    assert_eq!(error.code, "recorder_portal_restore_token_missing");
}

#[test]
fn live_session_restore_token_errors_are_user_repairable() {
    let missing = missing_restore_token_error();
    let invalid = invalid_restore_token_error();

    assert_eq!(missing.category, RecorderErrorCategory::Permission);
    assert_eq!(missing.code, "recorder_portal_restore_token_missing");
    assert!(missing.message.contains("Reselect monitor sources"));
    assert_eq!(invalid.category, RecorderErrorCategory::Permission);
    assert_eq!(invalid.code, "recorder_portal_restore_token_invalid");
    assert!(invalid.message.contains("Reselect monitor sources"));
}
