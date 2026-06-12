use super::types::MediaWriterDiagnostics;
use crate::recorder::media_writer;

pub(super) fn collect_media_writer_diagnostics() -> MediaWriterDiagnostics {
    let health = media_writer::collect_media_writer_health();

    MediaWriterDiagnostics {
        available: health.available,
        capabilities: health.capabilities,
        error: health.error,
        version: health.version,
        writer: health.writer,
    }
}
