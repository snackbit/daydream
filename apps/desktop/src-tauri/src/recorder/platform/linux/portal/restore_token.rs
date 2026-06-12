use tracing::warn;

use crate::storage::{self, AppStorage};

use super::super::super::super::{
    errors::{permission_error, storage_error},
    types::RecorderError,
};

pub(super) const RESTORE_TOKEN_ID: &str = storage::LINUX_SCREENCAST_MONITOR_RESTORE_TOKEN_ID;
pub(super) const RESTORE_TOKEN_PORTAL: &str = "linux-screencast";

pub(super) fn restore_token_for_live_session(
    stored_restore_token: Option<String>,
) -> Result<String, RecorderError> {
    if stored_restore_token.is_none() {
        warn!(
            restore_token_id = RESTORE_TOKEN_ID,
            "portal_live_restore_token_missing"
        );
    }

    stored_restore_token.ok_or_else(missing_restore_token_error)
}

pub(super) fn missing_restore_token_error() -> RecorderError {
    permission_error(
        "recorder_portal_restore_token_missing",
        "Screen permission is missing. Reselect monitor sources.",
    )
}

pub(super) fn invalid_restore_token_error() -> RecorderError {
    permission_error(
        "recorder_portal_restore_token_invalid",
        "Screen permission expired. Reselect monitor sources.",
    )
}

pub(super) fn invalidate_restore_token(storage: &AppStorage) -> Result<(), RecorderError> {
    storage
        .with_connection(|connection| {
            storage::delete_portal_restore_token(connection, RESTORE_TOKEN_ID)
        })
        .map_err(storage_error)
}

#[cfg(test)]
pub(super) fn restore_token_for_selection(
    stored_restore_token: Option<&str>,
    force_chooser: bool,
) -> Option<&str> {
    if force_chooser {
        None
    } else {
        stored_restore_token
    }
}
