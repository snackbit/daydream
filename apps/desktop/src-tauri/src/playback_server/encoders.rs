use std::{
    fs,
    process::{Command, Stdio},
};

use crate::ffmpeg;

const PREVIEW_LONG_EDGE_PX: i64 = 3_840;
const MAX_PLAYBACK_VIDEO_FPS: f64 = 60.0;
const MIN_PLAYBACK_VIDEO_FPS: f64 = 1.0;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum VideoEncoder {
    Amf,
    Libx264,
    Nvenc,
    Qsv,
    Vaapi,
}
fn append_video_encoder_args(command: &mut Command, encoder: &VideoEncoder) {
    append_video_encoder_args_with_fps(command, encoder, MIN_PLAYBACK_VIDEO_FPS);
}

pub(super) fn append_video_encoder_args_with_fps(
    command: &mut Command,
    encoder: &VideoEncoder,
    fps: f64,
) {
    append_video_encoder_args_with_tail(command, encoder, fps, 0);
}

pub(super) fn append_video_encoder_args_with_tail(
    command: &mut Command,
    encoder: &VideoEncoder,
    fps: f64,
    duration_ms: i64,
) {
    let fps_filter = format!(
        "fps={:.3}:start_time=0,tpad=stop_mode=clone:stop_duration={:.3}",
        playback_video_fps(fps),
        duration_ms.max(0) as f64 / 1000.
    );
    let scale_filter = format!(
        "scale=w='if(gte(iw,ih),min(iw,{PREVIEW_LONG_EDGE_PX}),-2)':h='if(gte(iw,ih),-2,min(ih,{PREVIEW_LONG_EDGE_PX}))'"
    );
    match encoder {
        VideoEncoder::Amf => {
            command
                .args([
                    "-vf",
                    &format!("{fps_filter},{scale_filter},format=yuv420p"),
                ])
                .args(["-c:v", "h264_amf", "-quality", "speed"]);
        }
        VideoEncoder::Nvenc => {
            command
                .args([
                    "-vf",
                    &format!("{fps_filter},{scale_filter},format=yuv420p"),
                ])
                .args(["-c:v", "h264_nvenc", "-preset", "p3", "-tune", "ull"]);
        }
        VideoEncoder::Qsv => {
            command
                .args(["-vf", &format!("{fps_filter},{scale_filter},format=nv12")])
                .args(["-c:v", "h264_qsv", "-preset", "veryfast"]);
        }
        VideoEncoder::Vaapi => {
            command
                .args([
                    "-vf",
                    &format!("{fps_filter},{scale_filter},format=nv12,hwupload"),
                ])
                .args(["-c:v", "h264_vaapi"]);
        }
        VideoEncoder::Libx264 => {
            command
                .args([
                    "-vf",
                    &format!("{fps_filter},{scale_filter},format=yuv420p"),
                ])
                .args([
                    "-c:v",
                    "libx264",
                    "-preset",
                    "veryfast",
                    "-tune",
                    "zerolatency",
                    "-profile:v",
                    "baseline",
                    "-level",
                    "5.1",
                    "-crf",
                    "23",
                ]);
        }
    }
}

pub(super) fn playback_video_fps(recording_fps: f64) -> f64 {
    if recording_fps.is_finite() && recording_fps > 0.0 {
        recording_fps.clamp(MIN_PLAYBACK_VIDEO_FPS, MAX_PLAYBACK_VIDEO_FPS)
    } else {
        MIN_PLAYBACK_VIDEO_FPS
    }
}

pub(super) fn detect_video_encoder() -> VideoEncoder {
    for encoder in [
        VideoEncoder::Vaapi,
        VideoEncoder::Qsv,
        VideoEncoder::Nvenc,
        VideoEncoder::Amf,
    ] {
        if encoder_probe_succeeds(&encoder) {
            return encoder;
        }
    }
    VideoEncoder::Libx264
}

fn encoder_probe_succeeds(encoder: &VideoEncoder) -> bool {
    if *encoder == VideoEncoder::Vaapi && vaapi_device_path().is_none() {
        return false;
    }
    let mut command = Command::new(ffmpeg::binary_path());
    command
        .args([
            "-hide_banner",
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=128x72:rate=1",
            "-frames:v",
            "1",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if *encoder == VideoEncoder::Vaapi {
        if let Some(device_path) = vaapi_device_path() {
            command.args(["-vaapi_device", &device_path]);
        }
    }
    append_video_encoder_args(&mut command, encoder);
    command.args(["-f", "null", "-"]);

    command.status().is_ok_and(|status| status.success())
}

pub(super) fn vaapi_device_path() -> Option<String> {
    ["/dev/dri/renderD128", "/dev/dri/renderD129"]
        .into_iter()
        .find(|path| fs::metadata(path).is_ok())
        .map(ToString::to_string)
}
