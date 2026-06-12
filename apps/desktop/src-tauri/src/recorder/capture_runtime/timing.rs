use std::sync::{Arc, Mutex};

use super::super::types::{CaptureFrameStorageKind, CaptureTimingDiagnostics};

pub(crate) type SharedCaptureTiming = Arc<Mutex<CaptureTimingDiagnostics>>;

pub(crate) fn new_capture_timing() -> SharedCaptureTiming {
    Arc::new(Mutex::new(CaptureTimingDiagnostics::default()))
}

pub(crate) fn capture_timing_snapshot(timing: &SharedCaptureTiming) -> CaptureTimingDiagnostics {
    timing
        .lock()
        .map(|timing| timing.clone())
        .unwrap_or_default()
}

pub(crate) fn record_capture_frame(
    timing: &SharedCaptureTiming,
    source_id: &str,
    storage: CaptureFrameStorageKind,
    timestamp_ms: u64,
) {
    if let Ok(mut timing) = timing.lock() {
        match storage {
            CaptureFrameStorageKind::CpuReadable => timing.cpu_readable_frames_received += 1,
            CaptureFrameStorageKind::DmaBuf => timing.dmabuf_frames_received += 1,
        }
        *timing
            .frames_received_by_source
            .entry(source_id.to_string())
            .or_insert(0) += 1;
        timing
            .latest_frame_storage_by_source
            .insert(source_id.to_string(), storage);
        timing.source_timestamp_min_ms = Some(
            timing
                .source_timestamp_min_ms
                .map_or(timestamp_ms, |current| current.min(timestamp_ms)),
        );
        timing.source_timestamp_max_ms = Some(
            timing
                .source_timestamp_max_ms
                .map_or(timestamp_ms, |current| current.max(timestamp_ms)),
        );
    }
}

pub(crate) fn record_dmabuf_frame_read_back_to_cpu(timing: &SharedCaptureTiming) {
    if let Ok(mut timing) = timing.lock() {
        timing.dmabuf_frames_read_back_to_cpu += 1;
    }
}

pub(crate) fn record_composed_frame(
    timing: &SharedCaptureTiming,
    timestamp_ms: u64,
    reused_latest_frame: bool,
) {
    if let Ok(mut timing) = timing.lock() {
        timing.composed_frames += 1;
        timing.last_composed_timestamp_ms = Some(timestamp_ms);
        if reused_latest_frame {
            timing.reused_frame_compositions += 1;
        }
    }
}

pub(crate) fn record_scheduler_tick(timing: &SharedCaptureTiming, missed_deadlines: u64) {
    if let Ok(mut timing) = timing.lock() {
        timing.scheduler_ticks += 1;
        timing.scheduler_missed_deadlines += missed_deadlines;
    }
}

pub(crate) fn record_compose_duration(timing: &SharedCaptureTiming, duration_ms: u64) {
    if let Ok(mut timing) = timing.lock() {
        timing.total_compose_duration_ms =
            timing.total_compose_duration_ms.saturating_add(duration_ms);
        timing.max_compose_duration_ms = timing.max_compose_duration_ms.max(duration_ms);
    }
}

pub(crate) fn record_frame_sent_to_writer(timing: &SharedCaptureTiming) {
    if let Ok(mut timing) = timing.lock() {
        timing.frames_sent_to_writer += 1;
    }
}
