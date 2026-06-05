use std::{
    env,
    ffi::OsStr,
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    sync::mpsc,
    thread,
    time::Duration,
};

use serde::{Deserialize, Serialize};

use super::{
    capture_runtime::{record_frame_sent_to_ffmpeg, FrameReceiver, SharedCaptureTiming},
    errors::dependency_error,
    types::{RecorderError, RecorderFfmpegCapabilities, RecorderFfmpegHealth},
};

const RAW_VIDEO_PIXEL_FORMAT: &str = "yuv444p";
const ENCODED_VIDEO_PIXEL_FORMAT: &str = "yuv420p";
const EVEN_DIMENSION_PAD_FILTER: &str = "pad=ceil(iw/2)*2:ceil(ih/2)*2";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FfmpegProbe {
    pub(crate) path: String,
    pub(crate) version_line: String,
    pub(crate) capabilities: RecorderFfmpegCapabilities,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum FfmpegPathSource {
    DaydreamFfmpegEnv,
    Path,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SelectedFfmpegPath {
    pub(crate) path: String,
    pub(crate) source: FfmpegPathSource,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct FfmpegSegmentInput {
    pub(crate) canvas_height: u32,
    pub(crate) canvas_width: u32,
    pub(crate) desktop_audio_source_id: Option<String>,
    pub(crate) fps: f64,
    pub(crate) microphone_audio_source_id: Option<String>,
    pub(crate) output_path: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FfmpegStreamIndexes {
    pub(crate) desktop_audio_stream_index: Option<i64>,
    pub(crate) microphone_audio_stream_index: Option<i64>,
    pub(crate) video_stream_index: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FfmpegSegmentProbe {
    pub(crate) audio: Vec<FfmpegSegmentAudioProbe>,
    pub(crate) available: bool,
    pub(crate) duration_ms: Option<u64>,
    pub(crate) error: Option<String>,
    pub(crate) ffprobe_path: Option<String>,
    pub(crate) streams: Vec<FfmpegSegmentStreamProbe>,
    pub(crate) video: Option<FfmpegSegmentVideoProbe>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FfmpegSegmentStreamProbe {
    pub(crate) codec_name: Option<String>,
    pub(crate) codec_type: Option<String>,
    pub(crate) duration_ms: Option<u64>,
    pub(crate) index: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FfmpegSegmentVideoProbe {
    pub(crate) codec_name: Option<String>,
    pub(crate) duration_ms: Option<u64>,
    pub(crate) estimated_frame_count: Option<u64>,
    pub(crate) height: Option<u32>,
    pub(crate) index: i64,
    pub(crate) pixel_format: Option<String>,
    pub(crate) width: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FfmpegSegmentAudioProbe {
    pub(crate) channels: Option<u32>,
    pub(crate) codec_name: Option<String>,
    pub(crate) duration_ms: Option<u64>,
    pub(crate) index: i64,
    pub(crate) sample_rate: Option<u32>,
}

#[derive(Debug)]
pub(crate) struct RunningFfmpegSegment {
    child: Child,
    frame_pump: Option<thread::JoinHandle<Result<(), String>>>,
    stop_frames: Option<mpsc::Sender<()>>,
}

pub(crate) fn collect_ffmpeg_health() -> RecorderFfmpegHealth {
    match probe_selected_ffmpeg() {
        Ok(probe) => {
            let error = capability_error(&probe.capabilities, false);
            RecorderFfmpegHealth {
                available: error.is_none(),
                capabilities: probe.capabilities,
                error,
                path: Some(probe.path),
                version_line: Some(probe.version_line),
            }
        }
        Err(error) => RecorderFfmpegHealth {
            available: false,
            capabilities: RecorderFfmpegCapabilities::default(),
            error: Some(error),
            path: selected_ffmpeg_path_details()
                .ok()
                .map(|selected| selected.path),
            version_line: None,
        },
    }
}

pub(crate) fn probe_selected_ffmpeg() -> Result<FfmpegProbe, String> {
    let path = selected_ffmpeg_path_details()?.path;
    let version_output = run_ffmpeg_command(&path, &["-version"])?;
    let devices_output = run_ffmpeg_command(&path, &["-hide_banner", "-devices"])?;
    let formats_output = run_ffmpeg_command(&path, &["-hide_banner", "-formats"])?;
    let encoders_output = run_ffmpeg_command(&path, &["-hide_banner", "-encoders"])?;

    Ok(FfmpegProbe {
        path,
        version_line: first_non_empty_line(&version_output)
            .unwrap_or("unknown FFmpeg version")
            .to_string(),
        capabilities: parse_capabilities(&devices_output, &formats_output, &encoders_output),
    })
}

pub(crate) fn probe_segment_best_effort(
    ffmpeg_path: &str,
    segment_path: &Path,
) -> FfmpegSegmentProbe {
    let ffprobe_path = match selected_ffprobe_path(ffmpeg_path, env::var_os("PATH").as_deref()) {
        Some(path) => path,
        None => {
            return FfmpegSegmentProbe {
                audio: Vec::new(),
                available: false,
                duration_ms: None,
                error: Some("ffprobe was not found beside FFmpeg or on PATH.".to_string()),
                ffprobe_path: None,
                streams: Vec::new(),
                video: None,
            };
        }
    };

    let output = Command::new(&ffprobe_path)
        .args([
            "-v",
            "error",
            "-show_format",
            "-show_streams",
            "-of",
            "json",
            &segment_path.to_string_lossy(),
        ])
        .output();

    match output {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            parse_segment_probe_json(&stdout, Some(ffprobe_path.clone()))
                .unwrap_or_else(|error| unavailable_segment_probe(Some(ffprobe_path), error))
        }
        Ok(output) => unavailable_segment_probe(
            Some(ffprobe_path),
            format!("ffprobe exited with {}.", output.status),
        ),
        Err(error) => unavailable_segment_probe(
            Some(ffprobe_path),
            format!("Failed to run ffprobe: {error}"),
        ),
    }
}

pub(crate) fn probe_json_string(probe: &FfmpegSegmentProbe) -> String {
    serde_json::to_string(probe).unwrap_or_else(|error| {
        serde_json::to_string(&unavailable_segment_probe(None, error.to_string()))
            .expect("serialize unavailable probe")
    })
}

pub(crate) fn parse_segment_probe_json(
    value: &str,
    ffprobe_path: Option<String>,
) -> Result<FfmpegSegmentProbe, String> {
    let raw: RawFfprobeOutput = serde_json::from_str(value)
        .map_err(|error| format!("Failed to parse ffprobe JSON: {error}"))?;
    let streams = raw.streams.unwrap_or_default();
    let duration_ms = raw
        .format
        .as_ref()
        .and_then(|format| duration_string_ms(format.duration.as_deref()))
        .or_else(|| streams.iter().filter_map(raw_stream_duration_ms).max());
    let video = streams
        .iter()
        .find(|stream| stream.codec_type.as_deref() == Some("video"))
        .map(raw_video_probe);
    let audio = streams
        .iter()
        .filter(|stream| stream.codec_type.as_deref() == Some("audio"))
        .map(raw_audio_probe)
        .collect::<Vec<_>>();
    let streams = streams.iter().map(raw_stream_probe).collect::<Vec<_>>();

    Ok(FfmpegSegmentProbe {
        audio,
        available: true,
        duration_ms,
        error: None,
        ffprobe_path,
        streams,
        video,
    })
}

#[derive(Debug, Deserialize)]
struct RawFfprobeOutput {
    format: Option<RawFfprobeFormat>,
    streams: Option<Vec<RawFfprobeStream>>,
}

#[derive(Debug, Deserialize)]
struct RawFfprobeFormat {
    duration: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawFfprobeStream {
    index: i64,
    codec_name: Option<String>,
    codec_type: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    pix_fmt: Option<String>,
    sample_rate: Option<String>,
    channels: Option<u32>,
    duration: Option<String>,
    nb_frames: Option<String>,
    avg_frame_rate: Option<String>,
    r_frame_rate: Option<String>,
    tags: Option<RawFfprobeStreamTags>,
}

#[derive(Debug, Deserialize)]
struct RawFfprobeStreamTags {
    #[serde(rename = "DURATION")]
    duration: Option<String>,
}

fn raw_stream_probe(stream: &RawFfprobeStream) -> FfmpegSegmentStreamProbe {
    FfmpegSegmentStreamProbe {
        codec_name: stream.codec_name.clone(),
        codec_type: stream.codec_type.clone(),
        duration_ms: raw_stream_duration_ms(stream),
        index: stream.index,
    }
}

fn raw_video_probe(stream: &RawFfprobeStream) -> FfmpegSegmentVideoProbe {
    let duration_ms = raw_stream_duration_ms(stream);
    FfmpegSegmentVideoProbe {
        codec_name: stream.codec_name.clone(),
        duration_ms,
        estimated_frame_count: stream
            .nb_frames
            .as_deref()
            .and_then(parse_u64)
            .or_else(|| estimate_frame_count(duration_ms, stream)),
        height: stream.height,
        index: stream.index,
        pixel_format: stream.pix_fmt.clone(),
        width: stream.width,
    }
}

fn raw_audio_probe(stream: &RawFfprobeStream) -> FfmpegSegmentAudioProbe {
    FfmpegSegmentAudioProbe {
        channels: stream.channels,
        codec_name: stream.codec_name.clone(),
        duration_ms: raw_stream_duration_ms(stream),
        index: stream.index,
        sample_rate: stream.sample_rate.as_deref().and_then(parse_u32),
    }
}

fn raw_stream_duration_ms(stream: &RawFfprobeStream) -> Option<u64> {
    duration_string_ms(stream.duration.as_deref()).or_else(|| {
        stream
            .tags
            .as_ref()
            .and_then(|tags| matroska_duration_ms(tags.duration.as_deref()))
    })
}

fn duration_string_ms(value: Option<&str>) -> Option<u64> {
    let seconds = value?.parse::<f64>().ok()?;
    if seconds.is_finite() && seconds >= 0.0 {
        Some((seconds * 1000.0).round() as u64)
    } else {
        None
    }
}

fn matroska_duration_ms(value: Option<&str>) -> Option<u64> {
    let value = value?;
    let mut parts = value.split(':');
    let hours = parts.next()?.parse::<f64>().ok()?;
    let minutes = parts.next()?.parse::<f64>().ok()?;
    let seconds = parts.next()?.parse::<f64>().ok()?;
    if parts.next().is_some() {
        return None;
    }
    let total_seconds = hours * 3600.0 + minutes * 60.0 + seconds;
    if total_seconds.is_finite() && total_seconds >= 0.0 {
        Some((total_seconds * 1000.0).round() as u64)
    } else {
        None
    }
}

fn estimate_frame_count(duration_ms: Option<u64>, stream: &RawFfprobeStream) -> Option<u64> {
    let duration_seconds = duration_ms? as f64 / 1000.0;
    let frame_rate = stream
        .avg_frame_rate
        .as_deref()
        .and_then(parse_rational)
        .or_else(|| stream.r_frame_rate.as_deref().and_then(parse_rational))?;
    if duration_seconds.is_finite() && duration_seconds >= 0.0 && frame_rate > 0.0 {
        Some((duration_seconds * frame_rate).round() as u64)
    } else {
        None
    }
}

fn parse_rational(value: &str) -> Option<f64> {
    if let Some((numerator, denominator)) = value.split_once('/') {
        let numerator = numerator.parse::<f64>().ok()?;
        let denominator = denominator.parse::<f64>().ok()?;
        if denominator == 0.0 {
            return None;
        }
        let result = numerator / denominator;
        return (result.is_finite() && result > 0.0).then_some(result);
    }

    let result = value.parse::<f64>().ok()?;
    (result.is_finite() && result > 0.0).then_some(result)
}

fn parse_u64(value: &str) -> Option<u64> {
    value.parse::<u64>().ok()
}

fn parse_u32(value: &str) -> Option<u32> {
    value.parse::<u32>().ok()
}

fn unavailable_segment_probe(
    ffprobe_path: Option<String>,
    error: impl Into<String>,
) -> FfmpegSegmentProbe {
    FfmpegSegmentProbe {
        audio: Vec::new(),
        available: false,
        duration_ms: None,
        error: Some(error.into()),
        ffprobe_path,
        streams: Vec::new(),
        video: None,
    }
}

pub(crate) fn validate_segment_capabilities(
    capabilities: &RecorderFfmpegCapabilities,
    audio_selected: bool,
) -> Result<(), RecorderError> {
    if let Some(error) = capability_error(capabilities, audio_selected) {
        return Err(dependency_error("recorder_ffmpeg_unavailable", error));
    }

    Ok(())
}

pub(crate) fn start_synthetic_segment(
    ffmpeg_path: &Path,
    input: &FfmpegSegmentInput,
) -> Result<RunningFfmpegSegment, RecorderError> {
    let canvas_width = input.canvas_width;
    let canvas_height = input.canvas_height;
    let frame_interval = frame_interval(input.fps);

    start_segment_with_frame_pump(ffmpeg_path, input, move |stdin, stop| {
        thread::spawn(move || {
            pump_synthetic_frames(stdin, canvas_width, canvas_height, frame_interval, stop)
        })
    })
}

#[allow(dead_code)]
pub(crate) fn start_segment_from_frames(
    ffmpeg_path: &Path,
    input: &FfmpegSegmentInput,
    frames: FrameReceiver,
    timing: Option<SharedCaptureTiming>,
) -> Result<RunningFfmpegSegment, RecorderError> {
    start_segment_with_frame_pump(ffmpeg_path, input, move |stdin, stop| {
        thread::spawn(move || pump_composed_frames(stdin, frames, stop, timing))
    })
}

impl RunningFfmpegSegment {
    pub(crate) fn stop(mut self) -> Result<ExitStatus, RecorderError> {
        if let Some(stop_frames) = self.stop_frames.take() {
            let _ = stop_frames.send(());
        }

        if let Some(frame_pump) = self.frame_pump.take() {
            match frame_pump.join() {
                Ok(Ok(())) => {}
                Ok(Err(error)) => {
                    let _ = self.child.kill();
                    let _ = self.child.wait();
                    return Err(dependency_error(
                        "recorder_ffmpeg_frame_write_failed",
                        format!("Failed to feed FFmpeg video frames: {error}"),
                    ));
                }
                Err(_error) => {
                    let _ = self.child.kill();
                    let _ = self.child.wait();
                    return Err(dependency_error(
                        "recorder_ffmpeg_frame_write_failed",
                        "FFmpeg frame writer thread panicked.",
                    ));
                }
            }
        }

        self.child.wait().map_err(|error| {
            dependency_error(
                "recorder_ffmpeg_wait_failed",
                format!("Failed to wait for FFmpeg: {error}"),
            )
        })
    }
}

fn start_segment_with_frame_pump(
    ffmpeg_path: &Path,
    input: &FfmpegSegmentInput,
    spawn_frame_pump: impl FnOnce(
        std::process::ChildStdin,
        mpsc::Receiver<()>,
    ) -> thread::JoinHandle<Result<(), String>>,
) -> Result<RunningFfmpegSegment, RecorderError> {
    let args = build_segment_args(input);
    let mut child = Command::new(ffmpeg_path)
        .args(&args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| {
            dependency_error(
                "recorder_ffmpeg_spawn_failed",
                format!("Failed to start FFmpeg: {error}"),
            )
        })?;
    let stdin = child.stdin.take().ok_or_else(|| {
        dependency_error(
            "recorder_ffmpeg_stdin_unavailable",
            "FFmpeg stdin is unavailable.",
        )
    })?;
    let (stop_frames, receiver) = mpsc::channel();
    let frame_pump = spawn_frame_pump(stdin, receiver);

    Ok(RunningFfmpegSegment {
        child,
        frame_pump: Some(frame_pump),
        stop_frames: Some(stop_frames),
    })
}

pub(crate) fn build_segment_args(input: &FfmpegSegmentInput) -> Vec<String> {
    let mut args = vec![
        "-hide_banner".to_string(),
        "-loglevel".to_string(),
        "error".to_string(),
        "-nostdin".to_string(),
        "-y".to_string(),
        "-f".to_string(),
        "rawvideo".to_string(),
        "-pixel_format".to_string(),
        RAW_VIDEO_PIXEL_FORMAT.to_string(),
        "-video_size".to_string(),
        format!("{}x{}", input.canvas_width, input.canvas_height),
        "-framerate".to_string(),
        format_fps(input.fps),
        "-i".to_string(),
        "pipe:0".to_string(),
    ];

    if let Some(source_id) = &input.desktop_audio_source_id {
        args.extend([
            "-f".to_string(),
            "pulse".to_string(),
            "-i".to_string(),
            source_id.clone(),
        ]);
    }

    if let Some(source_id) = &input.microphone_audio_source_id {
        args.extend([
            "-f".to_string(),
            "pulse".to_string(),
            "-i".to_string(),
            source_id.clone(),
        ]);
    }

    args.extend([
        "-map".to_string(),
        "0:v:0".to_string(),
        "-c:v".to_string(),
        "libx264".to_string(),
        "-preset".to_string(),
        "ultrafast".to_string(),
        "-pix_fmt".to_string(),
        ENCODED_VIDEO_PIXEL_FORMAT.to_string(),
        "-vf".to_string(),
        EVEN_DIMENSION_PAD_FILTER.to_string(),
        "-r".to_string(),
        format_fps(input.fps),
    ]);

    let mut input_index = 1;
    if input.desktop_audio_source_id.is_some() {
        args.extend(["-map".to_string(), format!("{input_index}:a:0")]);
        input_index += 1;
    }
    if input.microphone_audio_source_id.is_some() {
        args.extend(["-map".to_string(), format!("{input_index}:a:0")]);
    }
    if input.desktop_audio_source_id.is_some() || input.microphone_audio_source_id.is_some() {
        args.extend(["-c:a".to_string(), "libopus".to_string()]);
    }

    args.extend([
        "-shortest".to_string(),
        "-f".to_string(),
        "matroska".to_string(),
        input.output_path.to_string_lossy().to_string(),
    ]);

    args
}

pub(crate) fn stream_indexes(input: &FfmpegSegmentInput) -> FfmpegStreamIndexes {
    let mut next_stream_index = 1;
    let desktop_audio_stream_index = input.desktop_audio_source_id.as_ref().map(|_source_id| {
        let index = next_stream_index;
        next_stream_index += 1;
        index
    });
    let microphone_audio_stream_index =
        input.microphone_audio_source_id.as_ref().map(|_source_id| {
            let index = next_stream_index;
            next_stream_index += 1;
            index
        });

    FfmpegStreamIndexes {
        desktop_audio_stream_index,
        microphone_audio_stream_index,
        video_stream_index: 0,
    }
}

pub(crate) fn parse_capabilities(
    devices_output: &str,
    formats_output: &str,
    encoders_output: &str,
) -> RecorderFfmpegCapabilities {
    RecorderFfmpegCapabilities {
        libopus_encoder: has_named_component(encoders_output, "libopus"),
        libx264_encoder: has_named_component(encoders_output, "libx264"),
        matroska_muxer: has_muxing_format(formats_output, "matroska"),
        pulse_input: has_demuxing_device(devices_output, "pulse"),
        rawvideo_demuxer: has_demuxing_format(formats_output, "rawvideo"),
    }
}

pub(crate) fn selected_ffmpeg_path_details() -> Result<SelectedFfmpegPath, String> {
    selected_ffmpeg_path_from(
        env::var("DAYDREAM_FFMPEG_PATH").ok(),
        env::var_os("PATH").as_deref(),
    )
}

fn selected_ffmpeg_path_from(
    configured_path: Option<String>,
    path_env: Option<&OsStr>,
) -> Result<SelectedFfmpegPath, String> {
    if let Some(path) = configured_path {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            return Ok(SelectedFfmpegPath {
                path: trimmed.to_string(),
                source: FfmpegPathSource::DaydreamFfmpegEnv,
            });
        }
    }

    find_ffmpeg_on_path(path_env)
        .map(|path| SelectedFfmpegPath {
            path,
            source: FfmpegPathSource::Path,
        })
        .ok_or_else(|| {
            "DAYDREAM_FFMPEG_PATH is not configured and ffmpeg was not found on PATH.".to_string()
        })
}

fn selected_ffprobe_path(ffmpeg_path: &str, path_env: Option<&OsStr>) -> Option<String> {
    sibling_ffprobe_path(Path::new(ffmpeg_path)).or_else(|| find_ffprobe_on_path(path_env))
}

fn sibling_ffprobe_path(ffmpeg_path: &Path) -> Option<String> {
    let parent = ffmpeg_path.parent()?;
    ffprobe_executable_names()
        .iter()
        .map(|name| parent.join(name))
        .find(|candidate| is_executable_file(candidate))
        .map(|candidate| candidate.to_string_lossy().to_string())
}

fn run_ffmpeg_command(path: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(path)
        .args(args)
        .output()
        .map_err(|error| format!("Failed to run FFmpeg: {error}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}\n{stderr}");

    if !output.status.success() {
        let reason = first_non_empty_line(&combined).unwrap_or("unknown error");
        return Err(format!("FFmpeg exited with {}: {reason}", output.status));
    }

    Ok(combined)
}

fn pump_synthetic_frames(
    mut stdin: std::process::ChildStdin,
    canvas_width: u32,
    canvas_height: u32,
    frame_interval: Duration,
    stop: mpsc::Receiver<()>,
) -> Result<(), String> {
    let frame = black_yuv444p_frame(canvas_width, canvas_height)?;

    loop {
        stdin.write_all(&frame).map_err(|error| error.to_string())?;
        stdin.flush().map_err(|error| error.to_string())?;

        match stop.recv_timeout(frame_interval) {
            Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => return Ok(()),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
    }
}

#[allow(dead_code)]
fn pump_composed_frames(
    mut stdin: std::process::ChildStdin,
    frames: FrameReceiver,
    stop: mpsc::Receiver<()>,
    timing: Option<SharedCaptureTiming>,
) -> Result<(), String> {
    loop {
        match stop.try_recv() {
            Ok(()) | Err(mpsc::TryRecvError::Disconnected) => return Ok(()),
            Err(mpsc::TryRecvError::Empty) => {}
        }

        match frames.recv_timeout(Duration::from_millis(50)) {
            Ok(frame) => {
                let frame = frame.into_yuv444p().map_err(|error| error.message)?;
                stdin.write_all(&frame).map_err(|error| error.to_string())?;
                stdin.flush().map_err(|error| error.to_string())?;
                if let Some(timing) = &timing {
                    record_frame_sent_to_ffmpeg(timing);
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => return Ok(()),
        }
    }
}

fn black_yuv444p_frame(canvas_width: u32, canvas_height: u32) -> Result<Vec<u8>, String> {
    let plane_len = (canvas_width as usize)
        .checked_mul(canvas_height as usize)
        .ok_or_else(|| "Frame dimensions overflow.".to_string())?;
    let frame_len = plane_len
        .checked_mul(3)
        .ok_or_else(|| "Frame dimensions overflow.".to_string())?;
    let mut frame = vec![0; frame_len];
    frame[plane_len..].fill(128);

    Ok(frame)
}

fn frame_interval(fps: f64) -> Duration {
    if fps.is_finite() && fps > 0.0 {
        Duration::from_secs_f64(1.0 / fps)
    } else {
        Duration::from_secs(1)
    }
}

fn format_fps(fps: f64) -> String {
    if fps.fract() == 0.0 {
        format!("{fps:.0}")
    } else {
        fps.to_string()
    }
}

fn capability_error(
    capabilities: &RecorderFfmpegCapabilities,
    audio_selected: bool,
) -> Option<String> {
    let mut missing = Vec::new();
    if !capabilities.rawvideo_demuxer {
        missing.push("rawvideo demuxer");
    }
    if !capabilities.matroska_muxer {
        missing.push("Matroska muxer");
    }
    if !capabilities.libx264_encoder {
        missing.push("libx264 encoder");
    }
    if audio_selected {
        if !capabilities.pulse_input {
            missing.push("Pulse input");
        }
        if !capabilities.libopus_encoder {
            missing.push("libopus encoder");
        }
    }

    if missing.is_empty() {
        None
    } else {
        Some(format!(
            "Selected FFmpeg is missing required capabilities: {}.",
            missing.join(", "),
        ))
    }
}

fn has_demuxing_device(output: &str, name: &str) -> bool {
    output.lines().any(|line| {
        let trimmed = line.trim_start();
        trimmed.starts_with('D') && has_named_component(trimmed, name)
    })
}

fn has_demuxing_format(output: &str, name: &str) -> bool {
    output.lines().any(|line| {
        let trimmed = line.trim_start();
        trimmed.starts_with('D') && has_named_component(trimmed, name)
    })
}

fn has_muxing_format(output: &str, name: &str) -> bool {
    output.lines().any(|line| {
        let trimmed = line.trim_start();
        (trimmed.starts_with('E') || trimmed.chars().nth(1) == Some('E'))
            && has_named_component(trimmed, name)
    })
}

fn has_named_component(output: &str, name: &str) -> bool {
    output
        .split(|character: char| character.is_whitespace() || character == ',')
        .any(|component| component == name)
}

fn find_ffmpeg_on_path(path_env: Option<&OsStr>) -> Option<String> {
    let path_env = path_env?;
    env::split_paths(path_env).find_map(|directory| {
        ffmpeg_executable_names()
            .iter()
            .map(|name| directory.join(name))
            .find(|candidate| is_executable_file(candidate))
            .map(|candidate| candidate.to_string_lossy().to_string())
    })
}

fn find_ffprobe_on_path(path_env: Option<&OsStr>) -> Option<String> {
    let path_env = path_env?;
    env::split_paths(path_env).find_map(|directory| {
        ffprobe_executable_names()
            .iter()
            .map(|name| directory.join(name))
            .find(|candidate| is_executable_file(candidate))
            .map(|candidate| candidate.to_string_lossy().to_string())
    })
}

fn ffmpeg_executable_names() -> Vec<String> {
    let executable_suffix = env::consts::EXE_SUFFIX;
    if executable_suffix.is_empty() {
        vec!["ffmpeg".to_string()]
    } else {
        vec![format!("ffmpeg{executable_suffix}"), "ffmpeg".to_string()]
    }
}

fn ffprobe_executable_names() -> Vec<String> {
    let executable_suffix = env::consts::EXE_SUFFIX;
    if executable_suffix.is_empty() {
        vec!["ffprobe".to_string()]
    } else {
        vec![format!("ffprobe{executable_suffix}"), "ffprobe".to_string()]
    }
}

fn is_executable_file(path: &Path) -> bool {
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }

    is_executable_metadata(&metadata)
}

#[cfg(unix)]
fn is_executable_metadata(metadata: &fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;

    metadata.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn is_executable_metadata(_metadata: &fs::Metadata) -> bool {
    true
}

fn first_non_empty_line(value: &str) -> Option<&str> {
    value.lines().map(str::trim).find(|line| !line.is_empty())
}

#[cfg(test)]
mod tests {
    use std::{ffi::OsStr, fs, path::PathBuf};

    use super::{
        build_segment_args, parse_capabilities, parse_segment_probe_json,
        selected_ffmpeg_path_from, selected_ffprobe_path, stream_indexes, FfmpegPathSource,
        FfmpegSegmentInput,
    };

    #[test]
    fn parses_required_ffmpeg_capabilities() {
        let capabilities = parse_capabilities(
            " D  lavfi\n DE pulse\n",
            " DE rawvideo\n  E matroska\n",
            " V....D libx264\n A..... libopus\n",
        );

        assert!(capabilities.rawvideo_demuxer);
        assert!(capabilities.pulse_input);
        assert!(capabilities.matroska_muxer);
        assert!(capabilities.libx264_encoder);
        assert!(capabilities.libopus_encoder);
    }

    #[test]
    fn matroska_capability_requires_muxing_flag() {
        let muxing_capabilities =
            parse_capabilities("", " DE rawvideo\n  E matroska\n", " V....D libx264\n");
        let demux_only_capabilities =
            parse_capabilities("", " DE rawvideo\n D  matroska\n", " V....D libx264\n");

        assert!(muxing_capabilities.matroska_muxer);
        assert!(!demux_only_capabilities.matroska_muxer);
    }

    #[test]
    fn selected_ffmpeg_path_prefers_non_empty_env_override() {
        let selected = selected_ffmpeg_path_from(
            Some(" /custom/bin/ffmpeg ".to_string()),
            Some(OsStr::new("/ignored")),
        )
        .expect("select configured path");

        assert_eq!(selected.path, "/custom/bin/ffmpeg");
        assert_eq!(selected.source, FfmpegPathSource::DaydreamFfmpegEnv);
    }

    #[test]
    fn selected_ffmpeg_path_falls_back_to_path_lookup() {
        let directory =
            std::env::temp_dir().join(format!("daydream-ffmpeg-path-test-{}", std::process::id()));
        fs::create_dir_all(&directory).expect("create temp path directory");
        let ffmpeg_path = directory.join("ffmpeg");
        fs::write(&ffmpeg_path, "#!/bin/sh\nexit 0\n").expect("write fake ffmpeg");
        make_executable(&ffmpeg_path);

        let selected =
            selected_ffmpeg_path_from(None, Some(directory.as_os_str())).expect("find ffmpeg");

        assert_eq!(selected.path, ffmpeg_path.to_string_lossy());
        assert_eq!(selected.source, FfmpegPathSource::Path);
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn selected_ffmpeg_path_reports_missing_binary() {
        let error = selected_ffmpeg_path_from(None, Some(OsStr::new("")))
            .expect_err("missing ffmpeg should fail");

        assert!(error.contains("ffmpeg was not found on PATH"));
    }

    #[test]
    fn selected_ffprobe_path_prefers_sibling_binary() {
        let directory =
            std::env::temp_dir().join(format!("daydream-ffprobe-path-test-{}", std::process::id()));
        fs::create_dir_all(&directory).expect("create temp path directory");
        let ffmpeg_path = directory.join("ffmpeg");
        let ffprobe_path = directory.join("ffprobe");
        fs::write(&ffmpeg_path, "#!/bin/sh\nexit 0\n").expect("write fake ffmpeg");
        fs::write(&ffprobe_path, "#!/bin/sh\nexit 0\n").expect("write fake ffprobe");
        make_executable(&ffmpeg_path);
        make_executable(&ffprobe_path);

        let selected = selected_ffprobe_path(&ffmpeg_path.to_string_lossy(), Some(OsStr::new("")))
            .expect("select ffprobe");

        assert_eq!(selected, ffprobe_path.to_string_lossy());
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn parses_segment_probe_json_for_video_and_two_audio_streams() {
        let probe = parse_segment_probe_json(
            r#"
            {
              "streams": [
                {
                  "index": 0,
                  "codec_name": "h264",
                  "codec_type": "video",
                  "width": 5120,
                  "height": 1440,
                  "pix_fmt": "yuv420p",
                  "avg_frame_rate": "1/1",
                  "duration": "28.000000"
                },
                {
                  "index": 1,
                  "codec_name": "opus",
                  "codec_type": "audio",
                  "sample_rate": "48000",
                  "channels": 2,
                  "tags": {
                    "DURATION": "00:00:27.209000000"
                  }
                },
                {
                  "index": 2,
                  "codec_name": "opus",
                  "codec_type": "audio",
                  "sample_rate": "48000",
                  "channels": 2,
                  "tags": {
                    "DURATION": "00:00:25.600000000"
                  }
                }
              ],
              "format": {
                "duration": "28.000000"
              }
            }
            "#,
            Some("/usr/bin/ffprobe".to_string()),
        )
        .expect("parse probe json");

        assert!(probe.available);
        assert_eq!(probe.duration_ms, Some(28_000));
        assert_eq!(probe.ffprobe_path.as_deref(), Some("/usr/bin/ffprobe"));
        assert_eq!(
            probe.video.as_ref().and_then(|video| video.width),
            Some(5120)
        );
        assert_eq!(
            probe
                .video
                .as_ref()
                .and_then(|video| video.pixel_format.as_deref()),
            Some("yuv420p"),
        );
        assert_eq!(
            probe
                .video
                .as_ref()
                .and_then(|video| video.estimated_frame_count),
            Some(28),
        );
        assert_eq!(probe.audio.len(), 2);
        assert_eq!(probe.audio[0].duration_ms, Some(27_209));
        assert_eq!(probe.audio[1].sample_rate, Some(48_000));
    }

    #[test]
    fn builds_video_only_segment_args() {
        let input = segment_input(None, None);
        let args = build_segment_args(&input);

        assert_eq!(
            args,
            vec![
                "-hide_banner",
                "-loglevel",
                "error",
                "-nostdin",
                "-y",
                "-f",
                "rawvideo",
                "-pixel_format",
                "yuv444p",
                "-video_size",
                "1280x720",
                "-framerate",
                "1",
                "-i",
                "pipe:0",
                "-map",
                "0:v:0",
                "-c:v",
                "libx264",
                "-preset",
                "ultrafast",
                "-pix_fmt",
                "yuv420p",
                "-vf",
                "pad=ceil(iw/2)*2:ceil(ih/2)*2",
                "-r",
                "1",
                "-shortest",
                "-f",
                "matroska",
                "/tmp/daydream-test.mkv",
            ],
        );
    }

    #[test]
    fn builds_segment_args_with_optional_audio_inputs() {
        let input = segment_input(Some("desktop.monitor"), Some("mic"));
        let args = build_segment_args(&input);

        assert!(contains_subsequence(
            &args,
            &["-f", "pulse", "-i", "desktop.monitor"]
        ));
        assert!(contains_subsequence(&args, &["-f", "pulse", "-i", "mic"]));
        assert!(contains_subsequence(&args, &["-map", "1:a:0"]));
        assert!(contains_subsequence(&args, &["-map", "2:a:0"]));
        assert!(contains_subsequence(&args, &["-c:a", "libopus"]));
    }

    #[test]
    fn yuv420p_segment_args_pad_odd_canvas_dimensions() {
        let mut input = segment_input(None, None);
        input.canvas_width = 1279;
        input.canvas_height = 719;
        let args = build_segment_args(&input);

        assert!(contains_subsequence(&args, &["-video_size", "1279x719"]));
        assert!(contains_subsequence(
            &args,
            &["-vf", "pad=ceil(iw/2)*2:ceil(ih/2)*2"],
        ));
    }

    #[test]
    fn computes_stream_indexes_from_selected_audio_roles() {
        assert_eq!(
            stream_indexes(&segment_input(None, None)).desktop_audio_stream_index,
            None,
        );
        assert_eq!(
            stream_indexes(&segment_input(Some("desktop"), None)).desktop_audio_stream_index,
            Some(1),
        );
        assert_eq!(
            stream_indexes(&segment_input(None, Some("mic"))).microphone_audio_stream_index,
            Some(1),
        );
        assert_eq!(
            stream_indexes(&segment_input(Some("desktop"), Some("mic")))
                .microphone_audio_stream_index,
            Some(2),
        );
    }

    fn segment_input(
        desktop_audio_source_id: Option<&str>,
        microphone_audio_source_id: Option<&str>,
    ) -> FfmpegSegmentInput {
        FfmpegSegmentInput {
            canvas_height: 720,
            canvas_width: 1280,
            desktop_audio_source_id: desktop_audio_source_id.map(ToOwned::to_owned),
            fps: 1.0,
            microphone_audio_source_id: microphone_audio_source_id.map(ToOwned::to_owned),
            output_path: PathBuf::from("/tmp/daydream-test.mkv"),
        }
    }

    fn contains_subsequence(values: &[String], expected: &[&str]) -> bool {
        values.windows(expected.len()).any(|window| {
            window
                .iter()
                .zip(expected)
                .all(|(left, right)| left == right)
        })
    }

    #[cfg(unix)]
    fn make_executable(path: &std::path::Path) {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = fs::metadata(path)
            .expect("fake ffmpeg metadata")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).expect("make fake ffmpeg executable");
    }

    #[cfg(not(unix))]
    fn make_executable(_path: &std::path::Path) {}
}
