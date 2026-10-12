use std::{
    collections::HashMap,
    io::{Read, Write},
    net::TcpStream,
    process::{Command, Stdio},
    sync::{Arc, Mutex},
};

use crate::ffmpeg;

use super::{
    encoders::{vaapi_device_path, VideoEncoder},
    ffmpeg_args::input_seek_ffmpeg_args,
    thumbnails::{ThumbnailError, ThumbnailKey, ThumbnailService},
    PlaybackSegment, PlaybackSession,
};

mod request;
use request::read_request;
pub(super) use request::{parse_request, PlaybackRequest};
struct PlaybackRange {
    start_ms: i64,
    duration_ms: Option<i64>,
}
pub(super) fn handle_stream(
    mut stream: TcpStream,
    sessions: Arc<Mutex<HashMap<String, PlaybackSession>>>,
    thumbnails: ThumbnailService,
    encoder: VideoEncoder,
    resource: super::resources::ResourceLease,
) {
    let span = tracing::info_span!("playback_http", request_id = %uuid::Uuid::new_v4());
    let _entered = span.enter();
    let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(10)));
    let _ = stream.set_write_timeout(Some(std::time::Duration::from_secs(2)));
    let Ok(request) = read_request(&mut stream) else {
        write_response(&mut stream, 400, "text/plain", b"Bad request");
        return;
    };
    let Some((session_id, token, playback_request)) = parse_request(&request) else {
        write_response(&mut stream, 404, "text/plain", b"Not found");
        return;
    };
    resource.set_session(&session_id);
    let Some(segment) = sessions.lock().ok().and_then(|sessions| {
        let session = sessions.get(&session_id)?;
        let segment_id = match &playback_request {
            PlaybackRequest::Audio { segment_id, .. }
            | PlaybackRequest::Thumbnail { segment_id, .. }
            | PlaybackRequest::Video { segment_id, .. } => segment_id,
        };
        session.segment_for_token(&token, segment_id)
    }) else {
        write_response(&mut stream, 403, "text/plain", b"Forbidden");
        return;
    };

    match playback_request {
        PlaybackRequest::Audio {
            start_ms,
            track_id,
            duration_ms,
            ..
        } => stream_audio(
            &mut stream,
            &segment,
            &track_id,
            PlaybackRange {
                start_ms,
                duration_ms,
            },
            &resource,
        ),
        PlaybackRequest::Thumbnail {
            stream_index,
            offset_ms,
            segment_id,
        } => stream_thumbnail(
            &mut stream,
            &thumbnails,
            session_id,
            segment_id,
            &segment,
            offset_ms,
            stream_index,
        ),
        PlaybackRequest::Video {
            start_ms,
            stream_index,
            duration_ms,
            ..
        } => stream_video(
            &mut stream,
            &segment,
            &encoder,
            PlaybackRange {
                start_ms,
                duration_ms,
            },
            stream_index,
            &resource,
        ),
    }
}

fn stream_video(
    stream: &mut TcpStream,
    segment: &PlaybackSegment,
    encoder: &VideoEncoder,
    range: PlaybackRange,
    requested_stream: Option<i64>,
    resource: &super::resources::ResourceLease,
) {
    let start_ms = range.start_ms;
    let Some(video_stream_index) = selected_video_stream(segment, requested_stream) else {
        write_response(stream, 404, "text/plain", b"Segment has no video stream");
        return;
    };
    let mut command = Command::new(ffmpeg::binary_path());
    command.args(["-hide_banner", "-v", "error"]);
    if *encoder == VideoEncoder::Vaapi {
        if let Some(device_path) = vaapi_device_path() {
            command.args(["-vaapi_device", &device_path]);
        }
    }
    let Ok(input) = playback_input_args(segment, start_ms, resource) else {
        write_response(stream, 404, "text/plain", b"Playback media unavailable");
        return;
    };
    command
        .args(input)
        .args(["-map", &format!("0:{video_stream_index}")])
        .arg("-an");
    super::encoders::append_video_encoder_args_with_tail(
        &mut command,
        encoder,
        segment.playback_fps,
        segment.duration_ms,
    );
    command.args([
        "-t",
        &format!(
            "{:.3}",
            output_duration_ms(segment.duration_ms, start_ms, range.duration_ms) as f64 / 1000.
        ),
    ]);
    command
        .args(super::ffmpeg_args::video_muxer_ffmpeg_args())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());

    stream_ffmpeg_stdout(stream, command, "video/mp4", resource);
}

fn stream_audio(
    stream: &mut TcpStream,
    segment: &PlaybackSegment,
    track_id: &str,
    range: PlaybackRange,
    resource: &super::resources::ResourceLease,
) {
    let start_ms = range.start_ms;
    let stream_index = if track_id.starts_with("desktop-audio-") {
        segment.desktop_audio_stream_index
    } else if track_id.starts_with("microphone-") {
        segment.microphone_audio_stream_index
    } else {
        None
    };
    let Some(stream_index) = stream_index else {
        write_response(
            stream,
            404,
            "text/plain",
            b"Segment has no matching audio stream",
        );
        return;
    };

    let cleaned = track_id.starts_with("microphone-") && !segment.microphone_fragments.is_empty();
    let input = if cleaned {
        let mut audio = segment.clone();
        audio.fragments = segment.microphone_fragments.clone();
        playback_input_args(&audio, start_ms, resource)
    } else {
        playback_input_args(segment, start_ms, resource)
    };
    let Ok(input) = input else {
        write_response(stream, 404, "text/plain", b"Playback media unavailable");
        return;
    };
    let mut command = Command::new(ffmpeg::binary_path());
    command
        .args(["-hide_banner", "-v", "error"])
        .args(input)
        .args([
            "-map",
            &format!("0:{}", if cleaned { 0 } else { stream_index }),
        ])
        .args([
            "-t",
            &format!(
                "{:.3}",
                output_duration_ms(segment.duration_ms, start_ms, range.duration_ms) as f64 / 1000.
            ),
        ])
        .args([
            "-vn",
            "-af",
            &ffmpeg::audio_timeline_filter(48_000, segment.duration_ms),
            "-c:a",
            "aac",
            "-b:a",
            "128k",
            "-movflags",
            "empty_moov+default_base_moof",
            // Audio has no video keyframes. Bound fragments so streaming does
            // not accumulate a run-sized index on the non-seekable stdout pipe.
            "-frag_duration",
            "1000000",
            "-f",
            "mp4",
            "pipe:1",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null());

    stream_ffmpeg_stdout(stream, command, "audio/mp4", resource);
}
pub(super) fn output_duration_ms(duration_ms: i64, start_ms: i64, requested: Option<i64>) -> i64 {
    let remaining = duration_ms.saturating_sub(start_ms).max(1);
    requested.unwrap_or(remaining).clamp(1, remaining)
}

pub(super) fn playback_input_args(
    segment: &PlaybackSegment,
    start_ms: i64,
    resource: &super::resources::ResourceLease,
) -> Result<Vec<String>, String> {
    if segment.fragments.is_empty() {
        return Ok(input_seek_ffmpeg_args(start_ms, &segment.file_path));
    }
    let manifest = resource
        .manifest(&super::runs::concat_manifest(&segment.fragments)?)
        .map_err(|_| "Playback manifest unavailable.")?;
    Ok(vec![
        "-f".into(),
        "concat".into(),
        "-safe".into(),
        "0".into(),
        "-i".into(),
        manifest.to_string_lossy().into_owned(),
        "-ss".into(),
        format!("{:.3}", start_ms.max(0) as f64 / 1000.),
    ])
}
fn stream_thumbnail(
    stream: &mut TcpStream,
    thumbnails: &ThumbnailService,
    session_id: String,
    segment_id: String,
    segment: &PlaybackSegment,
    offset_ms: i64,
    requested_stream: Option<i64>,
) {
    let Some(video_stream_index) = selected_video_stream(segment, requested_stream) else {
        write_response(stream, 404, "text/plain", b"Segment has no video stream");
        return;
    };
    let key = ThumbnailKey {
        video_stream_index,
        offset_ms,
        segment_id,
        session_id,
    };
    match thumbnails.request_thumbnail(key, segment.clone(), video_stream_index, offset_ms) {
        Ok(bytes) => write_response(stream, 200, "image/jpeg", &bytes),
        Err(ThumbnailError::StaleSession) => {
            write_response(stream, 403, "text/plain", b"Forbidden")
        }
        Err(ThumbnailError::Failed) => {
            write_response(stream, 500, "text/plain", b"Unable to generate thumbnail")
        }
    }
}

fn selected_video_stream(segment: &PlaybackSegment, requested: Option<i64>) -> Option<i64> {
    match requested {
        None => segment.video_stream_index,
        Some(index)
            if segment
                .video_streams
                .iter()
                .any(|video| video.stream_index == index) =>
        {
            Some(index)
        }
        Some(index)
            if segment.video_streams.is_empty() && segment.video_stream_index == Some(index) =>
        {
            Some(index)
        }
        _ => None,
    }
}
fn stream_ffmpeg_stdout(
    stream: &mut TcpStream,
    mut command: Command,
    content_type: &str,
    resource: &super::resources::ResourceLease,
) {
    let Ok(mut stdout) = resource.spawn(&mut command) else {
        write_response(stream, 500, "text/plain", b"Unable to start FFmpeg");
        return;
    };

    let header = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n"
    );
    if stream.write_all(header.as_bytes()).is_err() {
        resource.kill();
        return;
    }

    let mut buffer = [0_u8; 64 * 1024];
    loop {
        match stdout.read(&mut buffer) {
            Ok(0) => break,
            Ok(size) => {
                if stream.write_all(&buffer[..size]).is_err() {
                    resource.kill();
                    break;
                }
            }
            Err(_) => {
                resource.kill();
                break;
            }
        }
    }
    let _ = resource.wait();
}

fn write_response(stream: &mut TcpStream, status: u16, content_type: &str, body: &[u8]) {
    if status >= 400 {
        tracing::warn!(http_status = status, "playback_http_response_failed");
    }
    let status_label = match status {
        200 => "OK",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        500 => "Internal Server Error",
        _ => "OK",
    };
    let header = format!(
        "HTTP/1.1 {status} {status_label}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n",
        body.len()
    );
    if let Err(error) = stream
        .write_all(header.as_bytes())
        .and_then(|_| stream.write_all(body))
    {
        tracing::info!(http_status = status, category = ?error.kind(), "playback_client_disconnected");
    }
}
