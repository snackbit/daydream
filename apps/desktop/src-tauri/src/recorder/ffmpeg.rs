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

use super::{
    errors::dependency_error,
    types::{RecorderError, RecorderFfmpegCapabilities, RecorderFfmpegHealth},
};

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
    let canvas_width = input.canvas_width;
    let canvas_height = input.canvas_height;
    let frame_interval = frame_interval(input.fps);
    let frame_pump = thread::spawn(move || {
        pump_synthetic_frames(stdin, canvas_width, canvas_height, frame_interval, receiver)
    });

    Ok(RunningFfmpegSegment {
        child,
        frame_pump: Some(frame_pump),
        stop_frames: Some(stop_frames),
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
        "yuv444p".to_string(),
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
        "yuv444p".to_string(),
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

fn ffmpeg_executable_names() -> Vec<String> {
    let executable_suffix = env::consts::EXE_SUFFIX;
    if executable_suffix.is_empty() {
        vec!["ffmpeg".to_string()]
    } else {
        vec![format!("ffmpeg{executable_suffix}"), "ffmpeg".to_string()]
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
        build_segment_args, parse_capabilities, selected_ffmpeg_path_from, stream_indexes,
        FfmpegPathSource, FfmpegSegmentInput,
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
                "yuv444p",
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
