use ashpd::desktop::{
    screencast::{
        CursorMode, Screencast, SelectSourcesOptions, SourceType, StartCastOptions,
        Stream as ScreencastStream,
    },
    PersistMode, Session,
};
use enumflags2::BitFlags;
use std::os::fd::OwnedFd;
use tracing::{debug, info, warn};

use crate::storage::{self, AppStorage};

use super::super::super::{
    errors::{dependency_error, permission_error, storage_error},
    manager::{now_ms, RecorderManager},
    types::{
        CaptureSource, CaptureSourceKind, CaptureSourceList, RecorderError, RecorderPortalHealth,
    },
};

const RESTORE_TOKEN_ID: &str = storage::LINUX_SCREENCAST_MONITOR_RESTORE_TOKEN_ID;
const RESTORE_TOKEN_PORTAL: &str = "linux-screencast";

#[derive(Debug)]
#[allow(dead_code)]
pub(crate) struct LivePortalSession {
    pub(crate) fd: OwnedFd,
    pub(crate) streams: Vec<LivePortalStream>,
    session: Session<Screencast>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(dead_code)]
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

pub async fn select_capture_sources(
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

#[allow(dead_code)]
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

async fn close_portal_session(
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

fn restore_token_for_live_session(
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

fn missing_restore_token_error() -> RecorderError {
    permission_error(
        "recorder_portal_restore_token_missing",
        "Screen permission is missing. Reselect monitor sources.",
    )
}

fn invalid_restore_token_error() -> RecorderError {
    permission_error(
        "recorder_portal_restore_token_invalid",
        "Screen permission expired. Reselect monitor sources.",
    )
}

fn invalidate_restore_token(storage: &AppStorage) -> Result<(), RecorderError> {
    storage
        .with_connection(|connection| {
            storage::delete_portal_restore_token(connection, RESTORE_TOKEN_ID)
        })
        .map_err(storage_error)
}

async fn select_sources_once(
    restore_token: Option<&str>,
) -> Result<ashpd::desktop::screencast::Streams, RecorderError> {
    let proxy = Screencast::new().await.map_err(portal_error)?;
    let session = proxy
        .create_session(Default::default())
        .await
        .map_err(portal_error)?;
    let result = select_sources_for_session(&proxy, &session, restore_token).await;
    let _ = close_portal_session(&session, "select-sources").await;

    result
}

async fn select_sources_for_session(
    proxy: &Screencast,
    session: &Session<Screencast>,
    restore_token: Option<&str>,
) -> Result<ashpd::desktop::screencast::Streams, RecorderError> {
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
        .select_sources(&session, options)
        .await
        .map_err(portal_error)?
        .response()
        .map_err(portal_error)?;

    proxy
        .start(&session, None, StartCastOptions::default())
        .await
        .map_err(portal_error)?
        .response()
        .map_err(portal_error)
}

fn streams_to_capture_sources(
    streams: &[ashpd::desktop::screencast::Stream],
) -> Result<Vec<CaptureSource>, RecorderError> {
    Ok(streams
        .iter()
        .enumerate()
        .map(|(index, stream)| stream_to_capture_source(index, stream))
        .collect())
}

fn stream_to_capture_source(
    index: usize,
    stream: &ashpd::desktop::screencast::Stream,
) -> CaptureSource {
    let id = stream
        .id()
        .or_else(|| stream.mapping_id())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| format!("pipewire-node-{}", stream.pipe_wire_node_id()));

    capture_source_from_portal_fields(index, id, stream.size())
}

#[allow(dead_code)]
fn streams_to_live_portal_streams(streams: &[ScreencastStream]) -> Vec<LivePortalStream> {
    streams
        .iter()
        .enumerate()
        .map(|(index, stream)| live_portal_stream_from_fields(index, stream))
        .collect()
}

#[allow(dead_code)]
fn live_portal_stream_from_fields(index: usize, stream: &ScreencastStream) -> LivePortalStream {
    let source_id = stream
        .id()
        .or_else(|| stream.mapping_id())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| format!("pipewire-node-{}", stream.pipe_wire_node_id()));
    let (logical_width, logical_height) = stream
        .size()
        .filter(|(width, height)| *width > 0 && *height > 0)
        .map(|(width, height)| (Some(width as u32), Some(height as u32)))
        .unwrap_or((None, None));

    LivePortalStream {
        display_name: format!("Monitor {}", index + 1),
        logical_height,
        logical_width,
        node_id: stream.pipe_wire_node_id(),
        source_id,
    }
}

fn capture_source_from_portal_fields(
    index: usize,
    id: String,
    logical_size: Option<(i32, i32)>,
) -> CaptureSource {
    let (logical_width, logical_height) = logical_size
        .filter(|(width, height)| *width > 0 && *height > 0)
        .map(|(width, height)| (Some(width as u32), Some(height as u32)))
        .unwrap_or((None, None));

    CaptureSource {
        display_name: format!("Monitor {}", index + 1),
        height: None,
        id,
        kind: CaptureSourceKind::Monitor,
        logical_height,
        logical_width,
        scale_factor: None,
        width: None,
    }
}

#[cfg(test)]
fn restore_token_for_selection(
    stored_restore_token: Option<&str>,
    force_chooser: bool,
) -> Option<&str> {
    if force_chooser {
        None
    } else {
        stored_restore_token
    }
}

fn source_type_labels(flags: BitFlags<SourceType>) -> Vec<String> {
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

fn cursor_mode_labels(flags: BitFlags<CursorMode>) -> Vec<String> {
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

fn preferred_cursor_mode(flags: BitFlags<CursorMode>) -> Option<CursorMode> {
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

fn portal_error(error: ashpd::Error) -> RecorderError {
    if matches!(
        error,
        ashpd::Error::Response(ashpd::desktop::ResponseError::Cancelled)
            | ashpd::Error::Portal(ashpd::PortalError::Cancelled(_))
    ) {
        return permission_error(
            "recorder_portal_permission_denied",
            "ScreenCast source selection was denied or cancelled.",
        );
    }

    dependency_error(
        "recorder_portal_error",
        format!("ScreenCast portal request failed: {error}"),
    )
}

#[cfg(test)]
mod tests {
    use ashpd::desktop::screencast::{CursorMode, SourceType};
    use enumflags2::BitFlags;

    use super::{
        capture_source_from_portal_fields, cursor_mode_labels, invalid_restore_token_error,
        missing_restore_token_error, portal_error, preferred_cursor_mode,
        restore_token_for_live_session, restore_token_for_selection, source_type_labels,
    };
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
        let source =
            capture_source_from_portal_fields(0, "monitor-1".to_string(), Some((1280, 720)));

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
        let token = restore_token_for_live_session(Some("restore-token".to_string()))
            .expect("restore token");

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
}
