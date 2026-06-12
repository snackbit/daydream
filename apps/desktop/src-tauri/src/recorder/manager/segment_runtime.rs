use tracing::info;

use super::super::{
    activity_input::ActivityMonitorHandle,
    capture_runtime::{capture_timing_snapshot, CaptureSession, SharedCaptureTiming},
    media_writer::MediaWriterSegment,
    types::RecorderError,
};

#[derive(Debug)]
pub(crate) struct RecordingSegmentRuntime {
    activity_monitor: Option<ActivityMonitorHandle>,
    capture: Option<Box<dyn CaptureSession>>,
    media_writer: MediaWriterSegment,
    timing: Option<SharedCaptureTiming>,
}

impl RecordingSegmentRuntime {
    pub(crate) fn new(
        media_writer: MediaWriterSegment,
        capture: Option<Box<dyn CaptureSession>>,
        timing: Option<SharedCaptureTiming>,
    ) -> Self {
        Self {
            activity_monitor: None,
            capture,
            media_writer,
            timing,
        }
    }

    pub(crate) fn attach_activity_monitor(&mut self, activity_monitor: ActivityMonitorHandle) {
        self.activity_monitor = Some(activity_monitor);
    }

    pub(crate) fn capture_timing(&self) -> Option<SharedCaptureTiming> {
        self.timing.clone()
    }

    pub(super) fn stop(mut self, stop_at_ms: u64) -> Result<(), RecorderError> {
        if let Some(activity_monitor) = self.activity_monitor.take() {
            activity_monitor.stop();
        }
        let capture_error = self
            .capture
            .take()
            .and_then(|capture| capture.stop(stop_at_ms).err());
        let media_writer_result = self.media_writer.stop(stop_at_ms);
        if let Some(timing) = &self.timing {
            let snapshot = capture_timing_snapshot(timing);
            info!(
                frames_received_by_source = ?snapshot.frames_received_by_source,
                composed_frames = snapshot.composed_frames,
                frames_sent_to_writer = snapshot.frames_sent_to_writer,
                reused_frame_compositions = snapshot.reused_frame_compositions,
                scheduler_ticks = snapshot.scheduler_ticks,
                scheduler_missed_deadlines = snapshot.scheduler_missed_deadlines,
                max_compose_duration_ms = snapshot.max_compose_duration_ms,
                total_compose_duration_ms = snapshot.total_compose_duration_ms,
                "recorder_segment_capture_timing_final"
            );
        }

        if let Some(error) = capture_error {
            return Err(error);
        }

        media_writer_result
    }
}
