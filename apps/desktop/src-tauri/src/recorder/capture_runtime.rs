#![allow(dead_code)]

use std::{
    collections::HashMap,
    fmt::{self, Debug},
    os::fd::{AsRawFd, OwnedFd},
    sync::{
        mpsc::{self, Receiver, SyncSender, TrySendError},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};

use super::{
    compositor::SceneCompositorPlan,
    errors::{
        capture_adapter_failed_error, capture_cleanup_failed_error, capture_dmabuf_required_error,
        capture_format_unsupported_error,
    },
    types::{CaptureSourceKind, CaptureTimingDiagnostics, RecorderError},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CaptureRuntimeSourceHandle {
    BackendId(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CaptureRuntimeSource {
    pub(crate) display_name: String,
    pub(crate) handle: CaptureRuntimeSourceHandle,
    pub(crate) height: Option<u32>,
    pub(crate) kind: CaptureSourceKind,
    pub(crate) source_id: String,
    pub(crate) width: Option<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CapturedFrameFormat {
    Yuv444p,
    Yuv420p,
    Rgbx,
    Rgba,
    Xrgb,
    Argb,
    Bgrx,
    Bgra,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct CapturedFrame {
    pub(crate) data: Vec<u8>,
    pub(crate) format: CapturedFrameFormat,
    pub(crate) height: u32,
    pub(crate) source_id: String,
    pub(crate) stride: u32,
    pub(crate) timestamp_ms: u64,
    pub(crate) width: u32,
}

pub(crate) struct CapturedDmaBufFrame {
    pub(crate) fd: OwnedFd,
    pub(crate) format: CapturedFrameFormat,
    pub(crate) fourcc: u32,
    pub(crate) height: u32,
    pub(crate) modifier: u64,
    pub(crate) offset: u32,
    pub(crate) source_id: String,
    pub(crate) stride: u32,
    pub(crate) timestamp_ms: u64,
    pub(crate) width: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WriterFrameHandoffPlan {
    PassThroughYuv420p,
    ConvertToYuv420p { source_format: CapturedFrameFormat },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CapturedFrameStorage {
    CpuReadable,
    DmaBuf,
    Unknown(String),
}

#[derive(Debug)]
pub(crate) struct FrameSender {
    inner: SyncSender<CapturedFrame>,
}

#[derive(Debug)]
pub(crate) struct FrameReceiver {
    inner: Receiver<CapturedFrame>,
}

pub(crate) trait CaptureAdapter: Send + Sync {
    fn runtime_sources(&self) -> Result<Vec<CaptureRuntimeSource>, RecorderError>;

    fn start_session(
        &self,
        plan: SceneCompositorPlan,
        frames: FrameSender,
    ) -> Result<Box<dyn CaptureSession>, RecorderError>;
}

pub(crate) trait CaptureSession: Send + Debug {
    fn stop(self: Box<Self>, stop_at_ms: u64) -> Result<(), RecorderError>;
}

pub(crate) trait CapturePreviewSession: Send + Debug {
    fn stop(self: Box<Self>) -> Result<(), RecorderError>;
}

#[derive(Debug)]
pub(crate) struct RuntimeWorker {
    name: &'static str,
    handle: thread::JoinHandle<Result<(), RecorderError>>,
}

#[derive(Debug)]
pub(crate) struct LatestFrameSampler {
    frame_interval_ms: u64,
    last_composed_at_ms: Option<u64>,
    latest_frames: HashMap<String, CapturedFrame>,
}

#[derive(Debug)]
pub(crate) struct LatestDmaBufFrameSampler {
    frame_interval_ms: u64,
    last_composed_at_ms: Option<u64>,
    latest_frames: HashMap<String, CapturedDmaBufFrame>,
}

pub(crate) type SharedCaptureTiming = Arc<Mutex<CaptureTimingDiagnostics>>;

impl CapturedFrameFormat {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Yuv444p => "yuv444p",
            Self::Yuv420p => "yuv420p",
            Self::Rgbx => "rgbx",
            Self::Rgba => "rgba",
            Self::Xrgb => "xrgb",
            Self::Argb => "argb",
            Self::Bgrx => "bgrx",
            Self::Bgra => "bgra",
        }
    }

    pub(crate) fn from_label(label: &str) -> Result<Self, RecorderError> {
        match label {
            "yuv444p" => Ok(Self::Yuv444p),
            "yuv420p" | "i420" => Ok(Self::Yuv420p),
            "rgbx" => Ok(Self::Rgbx),
            "rgba" => Ok(Self::Rgba),
            "xrgb" => Ok(Self::Xrgb),
            "argb" => Ok(Self::Argb),
            "bgrx" => Ok(Self::Bgrx),
            "bgra" => Ok(Self::Bgra),
            other => Err(capture_format_unsupported_error(format!(
                "Capture frame format {other} is not supported."
            ))),
        }
    }

    pub(crate) fn from_pipewire_label(label: &str) -> Result<Self, RecorderError> {
        match label {
            "RGBx" | "rgbx" => Ok(Self::Rgbx),
            "RGBA" | "rgba" => Ok(Self::Rgba),
            "xRGB" | "XRGB" | "xrgb" => Ok(Self::Xrgb),
            "ARGB" | "argb" => Ok(Self::Argb),
            "BGRx" | "bgrx" => Ok(Self::Bgrx),
            "BGRA" | "bgra" => Ok(Self::Bgra),
            "YUV444P" | "YUV444p" | "yuv444p" => Ok(Self::Yuv444p),
            "I420" | "YUV420P" | "YUV420p" | "yuv420p" => Ok(Self::Yuv420p),
            other => Err(capture_format_unsupported_error(format!(
                "PipeWire frame format {other} is not supported."
            ))),
        }
    }

    pub(crate) fn bytes_per_pixel(self) -> Option<u32> {
        match self {
            Self::Yuv444p | Self::Yuv420p => None,
            Self::Rgbx | Self::Rgba | Self::Xrgb | Self::Argb | Self::Bgrx | Self::Bgra => Some(4),
        }
    }
}

impl CapturedFrameStorage {
    pub(crate) fn from_pipewire_data_type(label: &str) -> Self {
        match label {
            "MemPtr" | "MemFd" | "memptr" | "memfd" => Self::CpuReadable,
            "DmaBuf" | "dmabuf" => Self::DmaBuf,
            other => Self::Unknown(other.to_string()),
        }
    }

    pub(crate) fn ensure_cpu_readable(&self, source_id: &str) -> Result<(), RecorderError> {
        match self {
            Self::CpuReadable => Ok(()),
            Self::DmaBuf => Err(capture_dmabuf_required_error(format!(
                "Capture source {source_id} requires DMA-BUF import before it can be recorded."
            ))),
            Self::Unknown(label) => Err(capture_format_unsupported_error(format!(
                "Capture source {source_id} uses unsupported PipeWire buffer storage {label}."
            ))),
        }
    }
}

impl CapturedFrame {
    pub(crate) fn new(
        source_id: impl Into<String>,
        timestamp_ms: u64,
        width: u32,
        height: u32,
        stride: u32,
        format: CapturedFrameFormat,
        data: Vec<u8>,
    ) -> Result<Self, RecorderError> {
        let frame = Self {
            data,
            format,
            height,
            source_id: source_id.into(),
            stride,
            timestamp_ms,
            width,
        };
        frame.validate()?;

        Ok(frame)
    }

    pub(crate) fn validate(&self) -> Result<(), RecorderError> {
        if self.timestamp_ms == 0 {
            return Err(capture_format_unsupported_error(format!(
                "Capture frame from source {} has an invalid timestamp.",
                self.source_id
            )));
        }
        if self.width == 0 || self.height == 0 || self.stride == 0 {
            return Err(capture_format_unsupported_error(format!(
                "Capture frame from source {} has invalid dimensions {}x{} and stride {}.",
                self.source_id, self.width, self.height, self.stride
            )));
        }

        let minimum_len = self.minimum_data_len()?;
        if self.data.len() < minimum_len {
            return Err(capture_format_unsupported_error(format!(
                "Capture frame from source {} has {} bytes, but {} bytes are required for {} {}x{} stride {}.",
                self.source_id,
                self.data.len(),
                minimum_len,
                self.format.label(),
                self.width,
                self.height,
                self.stride
            )));
        }

        Ok(())
    }

    pub(crate) fn writer_handoff_plan(&self) -> Result<WriterFrameHandoffPlan, RecorderError> {
        self.validate()?;

        match self.format {
            CapturedFrameFormat::Yuv420p => Ok(WriterFrameHandoffPlan::PassThroughYuv420p),
            CapturedFrameFormat::Yuv444p => Ok(WriterFrameHandoffPlan::ConvertToYuv420p {
                source_format: self.format,
            }),
            CapturedFrameFormat::Rgbx
            | CapturedFrameFormat::Rgba
            | CapturedFrameFormat::Xrgb
            | CapturedFrameFormat::Argb
            | CapturedFrameFormat::Bgrx
            | CapturedFrameFormat::Bgra => Ok(WriterFrameHandoffPlan::ConvertToYuv420p {
                source_format: self.format,
            }),
        }
    }

    pub(crate) fn into_yuv420p(self) -> Result<Vec<u8>, RecorderError> {
        match self.writer_handoff_plan()? {
            WriterFrameHandoffPlan::PassThroughYuv420p => Ok(self.data),
            WriterFrameHandoffPlan::ConvertToYuv420p { source_format } => {
                convert_frame_to_yuv420p(&self, source_format)
            }
        }
    }

    fn minimum_data_len(&self) -> Result<usize, RecorderError> {
        let height = self.height as usize;
        let stride = self.stride as usize;

        match self.format {
            CapturedFrameFormat::Yuv444p => stride
                .checked_mul(height)
                .and_then(|plane_len| plane_len.checked_mul(3))
                .ok_or_else(|| {
                    capture_format_unsupported_error(format!(
                        "Capture frame from source {} has overflowing yuv444p dimensions {}x{} stride {}.",
                        self.source_id, self.width, self.height, self.stride
                    ))
                }),
            CapturedFrameFormat::Yuv420p => {
                if self.width % 2 != 0 || self.height % 2 != 0 || self.stride % 2 != 0 {
                    return Err(capture_format_unsupported_error(format!(
                        "Capture frame from source {} has odd yuv420p dimensions {}x{} or stride {}.",
                        self.source_id, self.width, self.height, self.stride
                    )));
                }
                let y_len = stride.checked_mul(height).ok_or_else(|| {
                    capture_format_unsupported_error(format!(
                        "Capture frame from source {} has overflowing yuv420p dimensions {}x{} stride {}.",
                        self.source_id, self.width, self.height, self.stride
                    ))
                })?;
                let chroma_len = (stride / 2)
                    .checked_mul(height / 2)
                    .and_then(|plane_len| plane_len.checked_mul(2))
                    .ok_or_else(|| {
                        capture_format_unsupported_error(format!(
                            "Capture frame from source {} has overflowing yuv420p chroma dimensions {}x{} stride {}.",
                            self.source_id, self.width, self.height, self.stride
                        ))
                    })?;
                y_len.checked_add(chroma_len).ok_or_else(|| {
                    capture_format_unsupported_error(format!(
                        "Capture frame from source {} has overflowing yuv420p frame size.",
                        self.source_id
                    ))
                })
            }
            format => {
                let Some(bytes_per_pixel) = format.bytes_per_pixel() else {
                    return Err(capture_format_unsupported_error(format!(
                        "Capture frame format {} is not supported.",
                        format.label()
                    )));
                };
                let minimum_stride = self.width.checked_mul(bytes_per_pixel).ok_or_else(|| {
                    capture_format_unsupported_error(format!(
                        "Capture frame from source {} has overflowing dimensions {}x{}.",
                        self.source_id, self.width, self.height
                    ))
                })?;
                if self.stride < minimum_stride {
                    return Err(capture_format_unsupported_error(format!(
                        "Capture frame from source {} has stride {} below required stride {} for {}.",
                        self.source_id,
                        self.stride,
                        minimum_stride,
                        format.label()
                    )));
                }

                stride.checked_mul(height).ok_or_else(|| {
                    capture_format_unsupported_error(format!(
                        "Capture frame from source {} has overflowing dimensions {}x{} stride {}.",
                        self.source_id, self.width, self.height, self.stride
                    ))
                })
            }
        }
    }
}

impl CapturedDmaBufFrame {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        source_id: impl Into<String>,
        timestamp_ms: u64,
        width: u32,
        height: u32,
        stride: u32,
        format: CapturedFrameFormat,
        fourcc: u32,
        modifier: u64,
        offset: u32,
        fd: OwnedFd,
    ) -> Result<Self, RecorderError> {
        let frame = Self {
            fd,
            format,
            fourcc,
            height,
            modifier,
            offset,
            source_id: source_id.into(),
            stride,
            timestamp_ms,
            width,
        };
        frame.validate()?;

        Ok(frame)
    }

    pub(crate) fn validate(&self) -> Result<(), RecorderError> {
        if self.timestamp_ms == 0 {
            return Err(capture_format_unsupported_error(format!(
                "Capture DMA-BUF frame from source {} has an invalid timestamp.",
                self.source_id
            )));
        }
        if self.width == 0 || self.height == 0 || self.stride == 0 {
            return Err(capture_format_unsupported_error(format!(
                "Capture DMA-BUF frame from source {} has invalid dimensions {}x{} and stride {}.",
                self.source_id, self.width, self.height, self.stride
            )));
        }
        let Some(bytes_per_pixel) = self.format.bytes_per_pixel() else {
            return Err(capture_format_unsupported_error(format!(
                "Capture DMA-BUF frame from source {} uses unsupported import format {}.",
                self.source_id,
                self.format.label()
            )));
        };
        let minimum_stride = self.width.checked_mul(bytes_per_pixel).ok_or_else(|| {
            capture_format_unsupported_error(format!(
                "Capture DMA-BUF frame from source {} has overflowing dimensions {}x{}.",
                self.source_id, self.width, self.height
            ))
        })?;
        if self.stride < minimum_stride {
            return Err(capture_format_unsupported_error(format!(
                "Capture DMA-BUF frame from source {} has stride {} below required stride {} for {}.",
                self.source_id,
                self.stride,
                minimum_stride,
                self.format.label()
            )));
        }
        if self.fourcc == 0 {
            return Err(capture_format_unsupported_error(format!(
                "Capture DMA-BUF frame from source {} has an invalid DRM format.",
                self.source_id
            )));
        }

        Ok(())
    }
}

impl fmt::Debug for CapturedFrame {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CapturedFrame")
            .field("source_id", &self.source_id)
            .field("timestamp_ms", &self.timestamp_ms)
            .field("width", &self.width)
            .field("height", &self.height)
            .field("stride", &self.stride)
            .field("format", &self.format)
            .field("data_len", &self.data.len())
            .finish()
    }
}

impl fmt::Debug for CapturedDmaBufFrame {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CapturedDmaBufFrame")
            .field("source_id", &self.source_id)
            .field("timestamp_ms", &self.timestamp_ms)
            .field("width", &self.width)
            .field("height", &self.height)
            .field("stride", &self.stride)
            .field("offset", &self.offset)
            .field("format", &self.format)
            .field("fourcc", &self.fourcc)
            .field("modifier", &self.modifier)
            .field("fd", &self.fd.as_raw_fd())
            .finish()
    }
}

impl FrameSender {
    pub(crate) fn send(&self, frame: CapturedFrame) -> Result<(), RecorderError> {
        frame.validate()?;
        self.inner.send(frame).map_err(|_error| {
            capture_adapter_failed_error("Capture frame receiver is no longer available.")
        })
    }

    pub(crate) fn try_send(&self, frame: CapturedFrame) -> Result<(), FrameTrySendError> {
        frame.validate().map_err(FrameTrySendError::InvalidFrame)?;
        self.inner.try_send(frame).map_err(|error| match error {
            TrySendError::Full(frame) => FrameTrySendError::Full(frame),
            TrySendError::Disconnected(_frame) => FrameTrySendError::Disconnected,
        })
    }
}

#[derive(Debug)]
pub(crate) enum FrameTrySendError {
    Disconnected,
    Full(CapturedFrame),
    InvalidFrame(RecorderError),
}

impl Clone for FrameSender {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl FrameReceiver {
    pub(crate) fn recv(&self) -> Result<CapturedFrame, mpsc::RecvError> {
        self.inner.recv()
    }

    pub(crate) fn recv_timeout(
        &self,
        timeout: std::time::Duration,
    ) -> Result<CapturedFrame, mpsc::RecvTimeoutError> {
        self.inner.recv_timeout(timeout)
    }
}

impl RuntimeWorker {
    pub(crate) fn spawn(
        name: &'static str,
        run: impl FnOnce() -> Result<(), RecorderError> + Send + 'static,
    ) -> Self {
        Self {
            name,
            handle: thread::spawn(run),
        }
    }

    pub(crate) fn join(self) -> Result<(), RecorderError> {
        match self.handle.join() {
            Ok(Ok(())) => Ok(()),
            Ok(Err(error)) => Err(error),
            Err(_error) => Err(capture_cleanup_failed_error(format!(
                "Capture worker {} panicked during cleanup.",
                self.name
            ))),
        }
    }
}

impl LatestFrameSampler {
    pub(crate) fn new(fps: f64) -> Self {
        Self {
            frame_interval_ms: frame_interval_ms(fps),
            last_composed_at_ms: None,
            latest_frames: HashMap::new(),
        }
    }

    pub(crate) fn push(&mut self, frame: CapturedFrame) -> Result<(), RecorderError> {
        frame.validate()?;
        self.latest_frames.insert(frame.source_id.clone(), frame);

        Ok(())
    }

    pub(crate) fn should_compose(&self, now_ms: u64) -> bool {
        match self.last_composed_at_ms {
            None => !self.latest_frames.is_empty(),
            Some(last_composed_at_ms) => {
                now_ms.saturating_sub(last_composed_at_ms) >= self.frame_interval_ms
                    && !self.latest_frames.is_empty()
            }
        }
    }

    pub(crate) fn mark_composed(&mut self, timestamp_ms: u64) {
        self.last_composed_at_ms = Some(timestamp_ms);
    }

    pub(crate) fn latest_frames(&self) -> &HashMap<String, CapturedFrame> {
        &self.latest_frames
    }

    pub(crate) fn frame_interval(&self) -> Duration {
        Duration::from_millis(self.frame_interval_ms)
    }
}

impl LatestDmaBufFrameSampler {
    pub(crate) fn new(fps: f64) -> Self {
        Self {
            frame_interval_ms: frame_interval_ms(fps),
            last_composed_at_ms: None,
            latest_frames: HashMap::new(),
        }
    }

    pub(crate) fn push(&mut self, frame: CapturedDmaBufFrame) -> Result<(), RecorderError> {
        frame.validate()?;
        self.latest_frames.insert(frame.source_id.clone(), frame);

        Ok(())
    }

    pub(crate) fn should_compose(&self, now_ms: u64) -> bool {
        match self.last_composed_at_ms {
            None => !self.latest_frames.is_empty(),
            Some(last_composed_at_ms) => {
                now_ms.saturating_sub(last_composed_at_ms) >= self.frame_interval_ms
                    && !self.latest_frames.is_empty()
            }
        }
    }

    pub(crate) fn mark_composed(&mut self, timestamp_ms: u64) {
        self.last_composed_at_ms = Some(timestamp_ms);
    }

    pub(crate) fn latest_frames(&self) -> &HashMap<String, CapturedDmaBufFrame> {
        &self.latest_frames
    }
}

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
    timestamp_ms: u64,
) {
    if let Ok(mut timing) = timing.lock() {
        *timing
            .frames_received_by_source
            .entry(source_id.to_string())
            .or_insert(0) += 1;
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

pub(crate) fn bounded_frame_channel(capacity: usize) -> (FrameSender, FrameReceiver) {
    let (sender, receiver) = mpsc::sync_channel(capacity);

    (
        FrameSender { inner: sender },
        FrameReceiver { inner: receiver },
    )
}

fn frame_interval_ms(fps: f64) -> u64 {
    if fps.is_finite() && fps > 0.0 {
        Duration::from_secs_f64(1.0 / fps).as_millis().max(1) as u64
    } else {
        1000
    }
}

fn convert_frame_to_yuv420p(
    frame: &CapturedFrame,
    format: CapturedFrameFormat,
) -> Result<Vec<u8>, RecorderError> {
    if frame.width % 2 != 0 || frame.height % 2 != 0 {
        return Err(capture_format_unsupported_error(format!(
            "Capture frame from source {} has odd dimensions {}x{}; yuv420p requires even dimensions.",
            frame.source_id, frame.width, frame.height
        )));
    }
    match format {
        CapturedFrameFormat::Yuv444p => convert_yuv444p_to_yuv420p(frame),
        CapturedFrameFormat::Yuv420p => Err(capture_format_unsupported_error(
            "yuv420p frame conversion must use pass-through.",
        )),
        CapturedFrameFormat::Rgbx | CapturedFrameFormat::Rgba => {
            convert_packed_rgba_to_yuv420p(frame, [0, 1, 2])
        }
        CapturedFrameFormat::Xrgb | CapturedFrameFormat::Argb => {
            convert_packed_rgba_to_yuv420p(frame, [1, 2, 3])
        }
        CapturedFrameFormat::Bgrx | CapturedFrameFormat::Bgra => {
            convert_packed_rgba_to_yuv420p(frame, [2, 1, 0])
        }
    }
}

fn yuv420p_layout(
    source_id: &str,
    width: u32,
    height: u32,
) -> Result<(usize, usize, usize), RecorderError> {
    let width = width as usize;
    let height = height as usize;
    let y_len = width.checked_mul(height).ok_or_else(|| {
        capture_format_unsupported_error(format!(
            "Capture frame from source {} has overflowing dimensions {}x{}.",
            source_id, width, height
        ))
    })?;
    let chroma_width = width / 2;
    let chroma_height = height / 2;
    let chroma_len = chroma_width.checked_mul(chroma_height).ok_or_else(|| {
        capture_format_unsupported_error(format!(
            "Capture frame from source {} has overflowing chroma dimensions {}x{}.",
            source_id, width, height
        ))
    })?;
    Ok((y_len, chroma_width, chroma_len))
}

fn convert_yuv444p_to_yuv420p(frame: &CapturedFrame) -> Result<Vec<u8>, RecorderError> {
    let (y_len, chroma_width, chroma_len) =
        yuv420p_layout(&frame.source_id, frame.width, frame.height)?;
    let width = frame.width as usize;
    let height = frame.height as usize;
    let mut output = vec![0; y_len + chroma_len * 2];
    let u_plane_offset = y_len;
    let v_plane_offset = y_len + chroma_len;
    let source_y_plane_len = frame.stride as usize * height;

    for y in 0..height {
        let source_start = y * frame.stride as usize;
        let output_start = y * width;
        output[output_start..output_start + width]
            .copy_from_slice(&frame.data[source_start..source_start + width]);
    }

    for block_y in 0..height / 2 {
        for block_x in 0..chroma_width {
            let mut u_sum = 0_u16;
            let mut v_sum = 0_u16;
            for local_y in 0..2 {
                for local_x in 0..2 {
                    let x = block_x * 2 + local_x;
                    let y = block_y * 2 + local_y;
                    let source_index = y * frame.stride as usize + x;
                    let u_value = frame.data[source_y_plane_len + source_index];
                    let v_value = frame.data[source_y_plane_len * 2 + source_index];
                    u_sum += u_value as u16;
                    v_sum += v_value as u16;
                }
            }
            let chroma_index = block_y * chroma_width + block_x;
            output[u_plane_offset + chroma_index] = (u_sum / 4) as u8;
            output[v_plane_offset + chroma_index] = (v_sum / 4) as u8;
        }
    }

    Ok(output)
}

fn convert_packed_rgba_to_yuv420p(
    frame: &CapturedFrame,
    channels: [usize; 3],
) -> Result<Vec<u8>, RecorderError> {
    let (y_len, chroma_width, chroma_len) =
        yuv420p_layout(&frame.source_id, frame.width, frame.height)?;
    let width = frame.width as usize;
    let height = frame.height as usize;
    let mut output = vec![0; y_len + chroma_len * 2];
    let u_plane_offset = y_len;
    let v_plane_offset = y_len + chroma_len;

    for block_y in 0..height / 2 {
        for block_x in 0..chroma_width {
            let mut u_sum = 0_u16;
            let mut v_sum = 0_u16;
            for local_y in 0..2 {
                for local_x in 0..2 {
                    let x = block_x * 2 + local_x;
                    let y = block_y * 2 + local_y;
                    let input_index = y * frame.stride as usize + x * 4;
                    let (y_value, u_value, v_value) = rgb_to_yuv(
                        frame.data[input_index + channels[0]],
                        frame.data[input_index + channels[1]],
                        frame.data[input_index + channels[2]],
                    );
                    output[y * width + x] = y_value;
                    u_sum += u_value as u16;
                    v_sum += v_value as u16;
                }
            }
            let chroma_index = block_y * chroma_width + block_x;
            output[u_plane_offset + chroma_index] = (u_sum / 4) as u8;
            output[v_plane_offset + chroma_index] = (v_sum / 4) as u8;
        }
    }

    Ok(output)
}

fn rgb_to_yuv(red: u8, green: u8, blue: u8) -> (u8, u8, u8) {
    let red = red as i32;
    let green = green as i32;
    let blue = blue as i32;
    let y = ((77 * red + 150 * green + 29 * blue) >> 8).clamp(0, 255);
    let u = (((-43 * red - 85 * green + 128 * blue) >> 8) + 128).clamp(0, 255);
    let v = (((128 * red - 107 * green - 21 * blue) >> 8) + 128).clamp(0, 255);

    (y as u8, u as u8, v as u8)
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{Arc, Barrier},
        time::Duration,
    };

    use super::{
        bounded_frame_channel, capture_timing_snapshot, new_capture_timing, record_capture_frame,
        record_compose_duration, record_composed_frame, record_frame_sent_to_writer,
        record_scheduler_tick, CapturedFrame, CapturedFrameFormat, CapturedFrameStorage,
        LatestFrameSampler, RuntimeWorker, WriterFrameHandoffPlan,
    };

    #[test]
    fn supported_formats_are_accepted_and_plan_writer_handoff() {
        for (format, label) in [
            (CapturedFrameFormat::Yuv444p, "yuv444p"),
            (CapturedFrameFormat::Yuv420p, "yuv420p"),
            (CapturedFrameFormat::Rgbx, "rgbx"),
            (CapturedFrameFormat::Rgba, "rgba"),
            (CapturedFrameFormat::Xrgb, "xrgb"),
            (CapturedFrameFormat::Argb, "argb"),
            (CapturedFrameFormat::Bgrx, "bgrx"),
            (CapturedFrameFormat::Bgra, "bgra"),
        ] {
            assert_eq!(
                CapturedFrameFormat::from_label(label).expect("parse supported format"),
                format,
            );
        }
        assert_eq!(
            CapturedFrameFormat::from_pipewire_label("xRGB").expect("parse xrgb alias"),
            CapturedFrameFormat::Xrgb,
        );
        assert_eq!(
            CapturedFrameFormat::from_pipewire_label("ARGB").expect("parse argb alias"),
            CapturedFrameFormat::Argb,
        );

        let yuv420 = CapturedFrame::new(
            "monitor-1",
            1,
            2,
            2,
            2,
            CapturedFrameFormat::Yuv420p,
            vec![0; 6],
        )
        .expect("valid yuv frame");
        let rgba = CapturedFrame::new(
            "monitor-1",
            1,
            2,
            2,
            8,
            CapturedFrameFormat::Rgba,
            vec![0; 16],
        )
        .expect("valid rgba frame");

        assert_eq!(
            yuv420.writer_handoff_plan().expect("yuv handoff plan"),
            WriterFrameHandoffPlan::PassThroughYuv420p,
        );
        assert_eq!(
            rgba.writer_handoff_plan().expect("rgba handoff plan"),
            WriterFrameHandoffPlan::ConvertToYuv420p {
                source_format: CapturedFrameFormat::Rgba,
            },
        );
    }

    #[test]
    fn unsupported_formats_and_invalid_dimensions_fail() {
        let format_error =
            CapturedFrameFormat::from_label("nv12").expect_err("unsupported format should fail");
        let dimension_error = CapturedFrame::new(
            "monitor-1",
            1,
            0,
            2,
            8,
            CapturedFrameFormat::Rgba,
            vec![0; 16],
        )
        .expect_err("zero width should fail");

        assert_eq!(format_error.code, "recorder_capture_format_unsupported");
        assert_eq!(dimension_error.code, "recorder_capture_format_unsupported",);
    }

    #[test]
    fn dmabuf_storage_gets_specific_gate_h2_error() {
        let dmabuf = CapturedFrameStorage::from_pipewire_data_type("DmaBuf");
        let unknown = CapturedFrameStorage::from_pipewire_data_type("Other");

        let dmabuf_error = dmabuf
            .ensure_cpu_readable("monitor-1")
            .expect_err("dmabuf should require gpu path");
        let unknown_error = unknown
            .ensure_cpu_readable("monitor-1")
            .expect_err("unknown storage should fail");

        assert_eq!(dmabuf_error.code, "recorder_capture_dmabuf_required");
        assert_eq!(unknown_error.code, "recorder_capture_format_unsupported");
    }

    #[test]
    fn frame_debug_output_does_not_include_frame_bytes() {
        let frame = CapturedFrame::new(
            "monitor-1",
            1,
            1,
            1,
            4,
            CapturedFrameFormat::Rgbx,
            vec![1, 2, 3, 4],
        )
        .expect("valid frame");
        let debug = format!("{frame:?}");

        assert!(debug.contains("data_len"));
        assert!(!debug.contains("[1, 2, 3, 4]"));
    }

    #[test]
    fn bounded_frame_channel_sends_receives_and_shuts_down() {
        let (sender, receiver) = bounded_frame_channel(1);
        let barrier = Arc::new(Barrier::new(2));
        let worker_barrier = Arc::clone(&barrier);
        let worker = RuntimeWorker::spawn("test-capture-worker", move || {
            sender.send(test_frame(1))?;
            worker_barrier.wait();
            sender.send(test_frame(2))
        });

        let first = receiver.recv().expect("first frame");
        assert_eq!(first.timestamp_ms, 1);
        barrier.wait();
        let second = receiver.recv().expect("second frame");
        assert_eq!(second.timestamp_ms, 2);

        worker.join().expect("worker joins");
        assert!(receiver.recv_timeout(Duration::from_millis(10)).is_err());
    }

    #[test]
    fn latest_frame_sampler_keeps_latest_frame_per_source_and_paces_composition() {
        let mut sampler = LatestFrameSampler::new(1.0);

        assert!(!sampler.should_compose(100));
        sampler.push(test_frame(100)).expect("push first frame");
        assert!(sampler.should_compose(100));
        sampler.mark_composed(100);
        assert!(!sampler.should_compose(999));

        sampler.push(test_frame(200)).expect("replace source frame");
        assert!(!sampler.should_compose(999));
        assert!(sampler.should_compose(1100));
        assert_eq!(
            sampler
                .latest_frames()
                .get("monitor-1")
                .expect("latest frame")
                .timestamp_ms,
            200,
        );
        assert_eq!(sampler.frame_interval(), Duration::from_millis(1000));
    }

    #[test]
    fn capture_timing_counters_track_live_frame_flow() {
        let timing = new_capture_timing();

        record_capture_frame(&timing, "monitor-1", 100);
        record_capture_frame(&timing, "monitor-2", 90);
        record_capture_frame(&timing, "monitor-1", 130);
        record_composed_frame(&timing, 150, true);
        record_scheduler_tick(&timing, 2);
        record_compose_duration(&timing, 7);
        record_compose_duration(&timing, 3);
        record_frame_sent_to_writer(&timing);

        let snapshot = capture_timing_snapshot(&timing);

        assert_eq!(snapshot.composed_frames, 1);
        assert_eq!(
            snapshot.frames_received_by_source.get("monitor-1"),
            Some(&2)
        );
        assert_eq!(
            snapshot.frames_received_by_source.get("monitor-2"),
            Some(&1)
        );
        assert_eq!(snapshot.frames_sent_to_writer, 1);
        assert_eq!(snapshot.last_composed_timestamp_ms, Some(150));
        assert_eq!(snapshot.max_compose_duration_ms, 7);
        assert_eq!(snapshot.total_compose_duration_ms, 10);
        assert_eq!(snapshot.reused_frame_compositions, 1);
        assert_eq!(snapshot.scheduler_missed_deadlines, 2);
        assert_eq!(snapshot.scheduler_ticks, 1);
        assert_eq!(snapshot.source_timestamp_min_ms, Some(90));
        assert_eq!(snapshot.source_timestamp_max_ms, Some(130));
    }

    fn test_frame(timestamp_ms: u64) -> CapturedFrame {
        CapturedFrame::new(
            "monitor-1",
            timestamp_ms,
            1,
            1,
            4,
            CapturedFrameFormat::Rgba,
            vec![0, 0, 0, 255],
        )
        .expect("valid test frame")
    }
}
