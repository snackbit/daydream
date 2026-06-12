use super::*;

pub(super) struct PipeWireLatestFrameLog {
    frames_since_last_log: u64,
    last_logged_at: Instant,
    latest_timestamps_by_source: HashMap<String, u64>,
}

pub(super) struct PipeWireLatestFrameLogSnapshot {
    pub(super) frames_since_last_log: u64,
    pub(super) latest_timestamps_by_source: HashMap<String, u64>,
    pub(super) window_ms: u128,
}

pub(super) struct PipeWireScheduledFrameLog {
    frames_since_last_log: u64,
    last_logged_at: Instant,
    last_timestamp_ms: u64,
    max_compose_duration_ms: u64,
    reused_frames_since_last_log: u64,
    total_compose_duration_ms: u64,
}

pub(super) struct PipeWireScheduledFrameLogSnapshot {
    pub(super) average_compose_duration_ms: u64,
    pub(super) frames_since_last_log: u64,
    pub(super) last_timestamp_ms: u64,
    pub(super) max_compose_duration_ms: u64,
    pub(super) reused_frames_since_last_log: u64,
    pub(super) window_ms: u128,
}

pub(super) type SharedPipeWireFrameSampler = Arc<Mutex<LatestPipeWireFrameSampler>>;

pub(super) enum PipeWireCapturedFrame {
    Cpu(CapturedFrame),
    DmaBuf(CapturedDmaBufFrame),
}

pub(super) struct LatestPipeWireFrameSampler {
    latest_frames: HashMap<String, PipeWireCapturedFrame>,
}

impl LatestPipeWireFrameSampler {
    pub(super) fn new(_fps: f64) -> Self {
        Self {
            latest_frames: HashMap::new(),
        }
    }

    pub(super) fn push(&mut self, frame: PipeWireCapturedFrame) -> Result<(), RecorderError> {
        match &frame {
            PipeWireCapturedFrame::Cpu(frame) => frame.validate()?,
            PipeWireCapturedFrame::DmaBuf(frame) => frame.validate()?,
        }
        self.latest_frames
            .insert(frame.source_id().to_string(), frame);

        Ok(())
    }

    pub(super) fn latest_frames(&self) -> &HashMap<String, PipeWireCapturedFrame> {
        &self.latest_frames
    }

    pub(super) fn has_frames(&self) -> bool {
        !self.latest_frames.is_empty()
    }

    #[cfg(test)]
    pub(super) fn latest_frame_count(&self) -> usize {
        self.latest_frames.len()
    }

    #[cfg(test)]
    pub(super) fn has_latest_frame(&self, source_id: &str) -> bool {
        self.latest_frames.contains_key(source_id)
    }
}

impl PipeWireLatestFrameLog {
    pub(super) fn new(now: Instant) -> Self {
        Self {
            frames_since_last_log: 0,
            last_logged_at: now,
            latest_timestamps_by_source: HashMap::new(),
        }
    }

    pub(super) fn record_frame(
        &mut self,
        source_id: &str,
        frame_timestamp_ms: u64,
        now: Instant,
    ) -> Option<PipeWireLatestFrameLogSnapshot> {
        self.frames_since_last_log += 1;
        self.latest_timestamps_by_source
            .insert(source_id.to_string(), frame_timestamp_ms);
        let window = now.duration_since(self.last_logged_at);
        if window < PIPEWIRE_LATEST_FRAME_LOG_INTERVAL {
            return None;
        }

        let snapshot = PipeWireLatestFrameLogSnapshot {
            frames_since_last_log: self.frames_since_last_log,
            latest_timestamps_by_source: std::mem::take(&mut self.latest_timestamps_by_source),
            window_ms: window.as_millis(),
        };
        self.frames_since_last_log = 0;
        self.last_logged_at = now;

        Some(snapshot)
    }
}

impl PipeWireScheduledFrameLog {
    pub(super) fn new(now: Instant) -> Self {
        Self {
            frames_since_last_log: 0,
            last_logged_at: now,
            last_timestamp_ms: 0,
            max_compose_duration_ms: 0,
            reused_frames_since_last_log: 0,
            total_compose_duration_ms: 0,
        }
    }

    pub(super) fn record_frame(
        &mut self,
        timestamp_ms: u64,
        reused_latest_frame: bool,
        compose_duration_ms: u64,
        now: Instant,
    ) -> Option<PipeWireScheduledFrameLogSnapshot> {
        self.frames_since_last_log += 1;
        self.last_timestamp_ms = timestamp_ms;
        if reused_latest_frame {
            self.reused_frames_since_last_log += 1;
        }
        self.total_compose_duration_ms = self
            .total_compose_duration_ms
            .saturating_add(compose_duration_ms);
        self.max_compose_duration_ms = self.max_compose_duration_ms.max(compose_duration_ms);

        let window = now.duration_since(self.last_logged_at);
        if window < PIPEWIRE_LATEST_FRAME_LOG_INTERVAL {
            return None;
        }

        let frames_since_last_log = self.frames_since_last_log.max(1);
        let snapshot = PipeWireScheduledFrameLogSnapshot {
            average_compose_duration_ms: self.total_compose_duration_ms / frames_since_last_log,
            frames_since_last_log,
            last_timestamp_ms: self.last_timestamp_ms,
            max_compose_duration_ms: self.max_compose_duration_ms,
            reused_frames_since_last_log: self.reused_frames_since_last_log,
            window_ms: window.as_millis(),
        };
        self.frames_since_last_log = 0;
        self.last_logged_at = now;
        self.max_compose_duration_ms = 0;
        self.reused_frames_since_last_log = 0;
        self.total_compose_duration_ms = 0;

        Some(snapshot)
    }
}

pub(super) fn frame_interval_ms(fps: f64) -> u64 {
    if fps.is_finite() && fps > 0.0 {
        Duration::from_secs_f64(1.0 / fps).as_millis().max(1) as u64
    } else {
        1000
    }
}

impl PipeWireCapturedFrame {
    pub(super) fn source_id(&self) -> &str {
        match self {
            Self::Cpu(frame) => &frame.source_id,
            Self::DmaBuf(frame) => &frame.source_id,
        }
    }

    pub(super) fn timestamp_ms(&self) -> u64 {
        match self {
            Self::Cpu(frame) => frame.timestamp_ms,
            Self::DmaBuf(frame) => frame.timestamp_ms,
        }
    }

    pub(super) fn format_label(&self) -> &'static str {
        match self {
            Self::Cpu(frame) => frame.format.label(),
            Self::DmaBuf(frame) => frame.format.label(),
        }
    }

    pub(super) fn try_clone(&self) -> Result<Self, RecorderError> {
        match self {
            Self::Cpu(frame) => Ok(Self::Cpu(frame.clone())),
            Self::DmaBuf(frame) => Ok(Self::DmaBuf(CapturedDmaBufFrame::new(
                CapturedDmaBufFrameInput {
                    fd: frame.fd.as_fd().try_clone_to_owned().map_err(|error| {
                        capture_adapter_failed_error(format!(
                            "Failed to duplicate DMA-BUF fd for capture source {}: {error}.",
                            frame.source_id
                        ))
                    })?,
                    format: frame.format,
                    fourcc: frame.fourcc,
                    height: frame.height,
                    modifier: frame.modifier,
                    offset: frame.offset,
                    source_id: frame.source_id.clone(),
                    stride: frame.stride,
                    timestamp_ms: frame.timestamp_ms,
                    width: frame.width,
                },
            )?)),
        }
    }
}
