#![allow(dead_code)]

use std::{
    collections::HashMap,
    fmt,
    sync::mpsc::{self, Receiver, SyncSender},
    thread,
    time::Duration,
};

use super::{
    compositor::SceneCompositorPlan,
    errors::{
        capture_adapter_failed_error, capture_cleanup_failed_error, capture_dmabuf_required_error,
        capture_format_unsupported_error,
    },
    types::{CaptureSourceKind, RecorderError},
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FfmpegFrameHandoffPlan {
    PassThroughYuv444p,
    ConvertToYuv444p { source_format: CapturedFrameFormat },
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

pub(crate) trait CaptureSession: Send {
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

impl CapturedFrameFormat {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Yuv444p => "yuv444p",
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
            other => Err(capture_format_unsupported_error(format!(
                "PipeWire frame format {other} is not supported."
            ))),
        }
    }

    pub(crate) fn bytes_per_pixel(self) -> Option<u32> {
        match self {
            Self::Yuv444p => None,
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

    pub(crate) fn ffmpeg_handoff_plan(&self) -> Result<FfmpegFrameHandoffPlan, RecorderError> {
        self.validate()?;

        match self.format {
            CapturedFrameFormat::Yuv444p => Ok(FfmpegFrameHandoffPlan::PassThroughYuv444p),
            CapturedFrameFormat::Rgbx
            | CapturedFrameFormat::Rgba
            | CapturedFrameFormat::Xrgb
            | CapturedFrameFormat::Argb
            | CapturedFrameFormat::Bgrx
            | CapturedFrameFormat::Bgra => Ok(FfmpegFrameHandoffPlan::ConvertToYuv444p {
                source_format: self.format,
            }),
        }
    }

    pub(crate) fn into_yuv444p(self) -> Result<Vec<u8>, RecorderError> {
        match self.ffmpeg_handoff_plan()? {
            FfmpegFrameHandoffPlan::PassThroughYuv444p => Ok(self.data),
            FfmpegFrameHandoffPlan::ConvertToYuv444p { source_format } => {
                convert_packed_rgba_to_yuv444p(&self, source_format)
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

impl FrameSender {
    pub(crate) fn send(&self, frame: CapturedFrame) -> Result<(), RecorderError> {
        frame.validate()?;
        self.inner.send(frame).map_err(|_error| {
            capture_adapter_failed_error("Capture frame receiver is no longer available.")
        })
    }
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

fn convert_packed_rgba_to_yuv444p(
    frame: &CapturedFrame,
    format: CapturedFrameFormat,
) -> Result<Vec<u8>, RecorderError> {
    let pixel_count = (frame.width as usize)
        .checked_mul(frame.height as usize)
        .ok_or_else(|| {
            capture_format_unsupported_error(format!(
                "Capture frame from source {} has overflowing dimensions {}x{}.",
                frame.source_id, frame.width, frame.height
            ))
        })?;
    let plane_len = pixel_count;
    let mut output = vec![0; plane_len * 3];

    for y in 0..frame.height as usize {
        let row_start = y * frame.stride as usize;
        for x in 0..frame.width as usize {
            let input_index = row_start + x * 4;
            let (red, green, blue) = match format {
                CapturedFrameFormat::Rgbx | CapturedFrameFormat::Rgba => (
                    frame.data[input_index],
                    frame.data[input_index + 1],
                    frame.data[input_index + 2],
                ),
                CapturedFrameFormat::Xrgb | CapturedFrameFormat::Argb => (
                    frame.data[input_index + 1],
                    frame.data[input_index + 2],
                    frame.data[input_index + 3],
                ),
                CapturedFrameFormat::Bgrx | CapturedFrameFormat::Bgra => (
                    frame.data[input_index + 2],
                    frame.data[input_index + 1],
                    frame.data[input_index],
                ),
                CapturedFrameFormat::Yuv444p => {
                    return Err(capture_format_unsupported_error(
                        "YUV frame conversion must use pass-through.",
                    ));
                }
            };
            let output_index = y * frame.width as usize + x;
            let (y_value, u_value, v_value) = rgb_to_yuv(red, green, blue);
            output[output_index] = y_value;
            output[plane_len + output_index] = u_value;
            output[plane_len * 2 + output_index] = v_value;
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
        bounded_frame_channel, CapturedFrame, CapturedFrameFormat, CapturedFrameStorage,
        FfmpegFrameHandoffPlan, LatestFrameSampler, RuntimeWorker,
    };

    #[test]
    fn supported_formats_are_accepted_and_plan_ffmpeg_handoff() {
        for (format, label) in [
            (CapturedFrameFormat::Yuv444p, "yuv444p"),
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

        let yuv = CapturedFrame::new(
            "monitor-1",
            1,
            2,
            2,
            2,
            CapturedFrameFormat::Yuv444p,
            vec![0; 12],
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
            yuv.ffmpeg_handoff_plan().expect("yuv handoff plan"),
            FfmpegFrameHandoffPlan::PassThroughYuv444p,
        );
        assert_eq!(
            rgba.ffmpeg_handoff_plan().expect("rgba handoff plan"),
            FfmpegFrameHandoffPlan::ConvertToYuv444p {
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

    fn test_frame(timestamp_ms: u64) -> CapturedFrame {
        CapturedFrame::new(
            "monitor-1",
            timestamp_ms,
            1,
            1,
            1,
            CapturedFrameFormat::Yuv444p,
            vec![0, 128, 128],
        )
        .expect("valid test frame")
    }
}
