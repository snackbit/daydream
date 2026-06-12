use ashpd::desktop::screencast::Stream as ScreencastStream;

use super::super::super::super::types::{CaptureSource, CaptureSourceKind, RecorderError};
use super::live_session::LivePortalStream;

pub(super) fn streams_to_capture_sources(
    streams: &[ScreencastStream],
) -> Result<Vec<CaptureSource>, RecorderError> {
    Ok(streams
        .iter()
        .enumerate()
        .map(|(index, stream)| stream_to_capture_source(index, stream))
        .collect())
}

fn stream_to_capture_source(index: usize, stream: &ScreencastStream) -> CaptureSource {
    let id = stream
        .id()
        .or_else(|| stream.mapping_id())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| format!("pipewire-node-{}", stream.pipe_wire_node_id()));

    capture_source_from_portal_fields(index, id, stream.size())
}

pub(super) fn streams_to_live_portal_streams(
    streams: &[ScreencastStream],
) -> Vec<LivePortalStream> {
    streams
        .iter()
        .enumerate()
        .map(|(index, stream)| live_portal_stream_from_fields(index, stream))
        .collect()
}

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

pub(super) fn capture_source_from_portal_fields(
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
