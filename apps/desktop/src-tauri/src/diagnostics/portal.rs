use super::types::PortalRestoreTokenDiagnostics;
use crate::storage::{
    get_portal_restore_token, AppStorage, LINUX_SCREENCAST_MONITOR_RESTORE_TOKEN_ID,
};

pub(super) fn collect_portal_restore_token_diagnostics(
    storage: &AppStorage,
) -> PortalRestoreTokenDiagnostics {
    match storage.with_connection(|connection| {
        get_portal_restore_token(connection, LINUX_SCREENCAST_MONITOR_RESTORE_TOKEN_ID)
    }) {
        Ok(Some(row)) => PortalRestoreTokenDiagnostics {
            created_at_ms: Some(row.created_at_ms),
            error: None,
            expected_id: LINUX_SCREENCAST_MONITOR_RESTORE_TOKEN_ID.to_string(),
            last_used_at_ms: row.last_used_at_ms,
            portal: Some(row.portal),
            present: true,
            token_length: Some(row.token.len()),
            updated_at_ms: Some(row.updated_at_ms),
        },
        Ok(None) => PortalRestoreTokenDiagnostics {
            created_at_ms: None,
            error: None,
            expected_id: LINUX_SCREENCAST_MONITOR_RESTORE_TOKEN_ID.to_string(),
            last_used_at_ms: None,
            portal: None,
            present: false,
            token_length: None,
            updated_at_ms: None,
        },
        Err(error) => PortalRestoreTokenDiagnostics {
            created_at_ms: None,
            error: Some(error.to_string()),
            expected_id: LINUX_SCREENCAST_MONITOR_RESTORE_TOKEN_ID.to_string(),
            last_used_at_ms: None,
            portal: None,
            present: false,
            token_length: None,
            updated_at_ms: None,
        },
    }
}
