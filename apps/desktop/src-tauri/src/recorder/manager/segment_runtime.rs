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

    pub(crate) fn poll_error(&mut self) -> Option<RecorderError> {
        self.capture
            .as_ref()
            .and_then(|capture| capture.poll_error())
            .or_else(|| self.media_writer.poll_error())
    }

    pub(crate) fn abort(mut self, timestamp_ms: u64) {
        let _ = self.stop_capture(timestamp_ms);
        self.media_writer.abort();
    }

    pub(crate) fn drain_fragments(&mut self) -> Vec<super::super::media_writer::FragmentEvent> {
        self.media_writer.drain_fragments()
    }

    pub(super) fn stop(
        mut self,
        stop_at_ms: u64,
        deadline: std::time::Instant,
    ) -> Result<super::super::media_writer::FinalizedSegmentMedia, RecorderError> {
        #[cfg(target_os = "linux")]
        let cutoff_error = self.media_writer.apply_cutoff(stop_at_ms).err();
        #[cfg(not(target_os = "linux"))]
        let cutoff_error: Option<RecorderError> = None;
        let capture_error = self.stop_capture(stop_at_ms).err();
        #[cfg(target_os = "linux")]
        let media_writer_result = self
            .media_writer
            .stop_with_probe_until(stop_at_ms, deadline);
        #[cfg(not(target_os = "linux"))]
        let media_writer_result = self.media_writer.stop(stop_at_ms).map(|events| {
            super::super::media_writer::FinalizedSegmentMedia {
                events,
                probe: None,
            }
        });
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
        if let Some(error) = cutoff_error {
            return Err(error);
        }

        media_writer_result
    }

    fn stop_capture(&mut self, timestamp_ms: u64) -> Result<(), RecorderError> {
        let activity = self.activity_monitor.take();
        let capture = self.capture.take();
        if activity.is_none() && capture.is_none() {
            return Ok(());
        }
        let (sender, result) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            if let Some(activity) = activity {
                activity.stop();
            }
            let stopped = capture.map_or(Ok(()), |capture| capture.stop(timestamp_ms));
            let _ = sender.send(stopped);
        });
        result
            .recv_timeout(std::time::Duration::from_secs(2))
            .unwrap_or_else(|_| {
                #[cfg(target_os = "linux")]
                super::super::media_worker::quarantine();
                Err(super::super::errors::lifecycle_error(
                    "recorder_capture_stop_timeout",
                    "Capture cleanup timed out. Recording files have been preserved.",
                ))
            })
    }
}
