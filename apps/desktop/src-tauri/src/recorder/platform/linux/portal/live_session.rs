use ashpd::desktop::{screencast::Screencast, Session};
use std::os::fd::OwnedFd;
use tracing::{debug, info, warn};

use crate::storage::{self, AppStorage};

use super::super::super::super::{errors::storage_error, manager::now_ms, types::RecorderError};
use super::errors::portal_error;
use super::restore_token::{
    invalid_restore_token_error, invalidate_restore_token, restore_token_for_live_session,
    RESTORE_TOKEN_ID, RESTORE_TOKEN_PORTAL,
};
use super::selection::select_sources_for_session;
use super::streams::streams_to_live_portal_streams;

#[derive(Debug)]
pub(crate) struct LivePortalSession {
    pub(crate) fd: OwnedFd,
    pub(crate) streams: Vec<LivePortalStream>,
    session: Session<Screencast>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LivePortalStream {
    pub(crate) display_name: String,
    pub(crate) logical_height: Option<u32>,
    pub(crate) logical_width: Option<u32>,
    pub(crate) node_id: u32,
    pub(crate) source_id: String,
}

impl LivePortalSession {
    pub(crate) fn close_blocking(self, context: &'static str) -> Result<(), RecorderError> {
        let session = self.session;
        drop(self.fd);
        drop(self.streams);

        close_portal_session_blocking(session, context)
    }

    pub(crate) fn into_fd_and_session(self) -> (OwnedFd, Session<Screencast>) {
        (self.fd, self.session)
    }
}

pub(crate) async fn open_live_capture_session(
    storage: &AppStorage,
) -> Result<LivePortalSession, RecorderError> {
    let restore_token = restore_token_for_live_session(
        storage
            .with_connection(|connection| {
                storage::get_portal_restore_token(connection, RESTORE_TOKEN_ID)
            })
            .map_err(storage_error)?
            .map(|row| row.token),
    )?;
    debug!(
        restore_token_id = RESTORE_TOKEN_ID,
        restore_token_length = restore_token.len(),
        "portal_live_restore_token_loaded"
    );
    let proxy = Screencast::new().await.map_err(portal_error)?;
    let session = proxy
        .create_session(Default::default())
        .await
        .map_err(portal_error)?;
    let streams = match select_sources_for_session(&proxy, &session, Some(&restore_token)).await {
        Ok(streams) => {
            info!(
                restore_token_id = RESTORE_TOKEN_ID,
                "portal_live_restore_token_accepted"
            );
            storage
                .with_connection(|connection| {
                    storage::mark_portal_restore_token_used(
                        connection,
                        RESTORE_TOKEN_ID,
                        now_ms() as i64,
                    )
                })
                .map_err(storage_error)?;
            streams
        }
        Err(_error) => {
            warn!(
                restore_token_id = RESTORE_TOKEN_ID,
                "portal_live_restore_token_invalidated"
            );
            invalidate_restore_token(storage)?;
            return Err(invalid_restore_token_error());
        }
    };

    if let Some(token) = streams.restore_token() {
        storage
            .with_connection(|connection| {
                storage::upsert_portal_restore_token(
                    connection,
                    RESTORE_TOKEN_ID,
                    RESTORE_TOKEN_PORTAL,
                    token,
                    now_ms() as i64,
                )
            })
            .map_err(storage_error)?;
    }

    let fd = proxy
        .open_pipe_wire_remote(&session, Default::default())
        .await
        .map_err(portal_error)?;
    info!(
        stream_count = streams.streams().len(),
        source_ids = %streams
            .streams()
            .iter()
            .map(|stream| format!("pipewire-node-{}", stream.pipe_wire_node_id()))
            .collect::<Vec<_>>()
            .join(","),
        "portal_live_capture_session_opened"
    );

    Ok(LivePortalSession {
        fd,
        streams: streams_to_live_portal_streams(streams.streams()),
        session,
    })
}

pub(crate) fn close_portal_session_blocking(
    session: Session<Screencast>,
    context: &'static str,
) -> Result<(), RecorderError> {
    tauri::async_runtime::block_on(close_portal_session(&session, context))
}

pub(super) async fn close_portal_session(
    session: &Session<Screencast>,
    context: &'static str,
) -> Result<(), RecorderError> {
    info!(context, "portal_session_close_requested");
    match session.close().await {
        Ok(()) => {
            info!(context, "portal_session_close_succeeded");
            Ok(())
        }
        Err(error) => {
            let error = portal_error(error);
            warn!(
                context,
                error_code = %error.code,
                error_category = ?error.category,
                "portal_session_close_failed"
            );
            Err(error)
        }
    }
}
