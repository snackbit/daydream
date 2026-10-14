use ashpd::desktop::{
    screencast::{Screencast, SelectSourcesOptions, SourceType, StartCastOptions},
    PersistMode, Session,
};
use enumflags2::BitFlags;
use tracing::{debug, info, warn};

use crate::storage::{self, AppStorage};

use super::super::super::super::{
    errors::storage_error,
    manager::{now_ms, RecorderManager},
    types::{CaptureSourceList, RecorderError},
};
use super::errors::portal_error;
use super::health::preferred_cursor_mode;
use super::restore_token::{invalidate_restore_token, RESTORE_TOKEN_ID, RESTORE_TOKEN_PORTAL};
use super::streams::streams_to_capture_sources;

pub(crate) async fn select_capture_sources(
    storage: &AppStorage,
    manager: &RecorderManager,
    force_chooser: bool,
) -> Result<CaptureSourceList, RecorderError> {
    let restore_token = if force_chooser {
        info!("portal_select_sources_force_chooser");
        None
    } else {
        storage
            .with_connection(|connection| {
                storage::get_portal_restore_token(connection, RESTORE_TOKEN_ID)
            })
            .map_err(storage_error)?
            .map(|row| row.token)
    };
    debug!(
        restore_token_id = RESTORE_TOKEN_ID,
        restore_token_present = restore_token.is_some(),
        "portal_select_sources_restore_token_lookup"
    );

    let streams = match restore_token.as_deref() {
        Some(token) => match select_sources_once(Some(token)).await {
            Ok(streams) => {
                info!(
                    restore_token_id = RESTORE_TOKEN_ID,
                    restore_token_length = token.len(),
                    "portal_select_sources_restore_token_reused"
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
                    "portal_select_sources_restore_token_invalidated"
                );
                invalidate_restore_token(storage)?;
                select_sources_once(None).await?
            }
        },
        None => select_sources_once(None).await?,
    };
    let sources = streams_to_capture_sources(streams.streams())?;
    info!(
        source_count = sources.len(),
        source_ids = %sources
            .iter()
            .map(|source| source.id.as_str())
            .collect::<Vec<_>>()
            .join(","),
        "portal_select_sources_completed"
    );

    if let Some(token) = streams.restore_token() {
        info!(
            restore_token_id = RESTORE_TOKEN_ID,
            restore_token_length = token.len(),
            "portal_restore_token_stored"
        );
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

    manager.set_capture_sources(sources)
}

pub(super) async fn select_sources_once(
    restore_token: Option<&str>,
) -> Result<ashpd::desktop::screencast::Streams, RecorderError> {
    let _timing = crate::recorder::operation_timing::OperationTimer::start("portal_selection");
    let proxy = Screencast::new().await.map_err(portal_error)?;
    let session = proxy
        .create_session(Default::default())
        .await
        .map_err(portal_error)?;
    let result = select_sources_for_session(&proxy, &session, restore_token).await;
    let _ = super::live_session::close_portal_session(&session, "select-sources").await;

    result
}

pub(super) async fn select_sources_for_session(
    proxy: &Screencast,
    session: &Session<Screencast>,
    restore_token: Option<&str>,
) -> Result<ashpd::desktop::screencast::Streams, RecorderError> {
    let _timing =
        crate::recorder::operation_timing::OperationTimer::start("portal_select_and_start");
    let cursor_mode = match proxy.available_cursor_modes().await {
        Ok(cursor_modes) => preferred_cursor_mode(cursor_modes),
        Err(_error) => None,
    };
    let mut options = SelectSourcesOptions::default()
        .set_sources(BitFlags::from_flag(SourceType::Monitor))
        .set_multiple(true)
        .set_persist_mode(PersistMode::ExplicitlyRevoked);

    if let Some(cursor_mode) = cursor_mode {
        options = options.set_cursor_mode(cursor_mode);
    }

    if let Some(token) = restore_token {
        options = options.set_restore_token(token);
    }

    proxy
        .select_sources(session, options)
        .await
        .map_err(portal_error)?
        .response()
        .map_err(portal_error)?;

    proxy
        .start(session, None, StartCastOptions::default())
        .await
        .map_err(portal_error)?
        .response()
        .map_err(portal_error)
}
