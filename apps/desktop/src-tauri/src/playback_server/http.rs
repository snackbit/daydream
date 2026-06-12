use std::{
    collections::HashMap,
    io::{Read, Write},
    net::TcpStream,
    process::{Command, Stdio},
    sync::{Arc, Mutex},
};

use crate::ffmpeg;

use super::{
    encoders::{append_video_encoder_args_with_fps, vaapi_device_path, VideoEncoder},
    ffmpeg_args::input_seek_ffmpeg_args,
    thumbnails::{ThumbnailError, ThumbnailKey, ThumbnailService},
    PlaybackSegment, PlaybackSession,
};

pub(super) enum PlaybackRequest {
    Audio {
        segment_id: String,
        start_ms: i64,
        track_id: String,
    },
    Thumbnail {
        offset_ms: i64,
        segment_id: String,
    },
    Video {
        segment_id: String,
        start_ms: i64,
    },
}
pub(super) fn handle_stream(
    mut stream: TcpStream,
    sessions: Arc<Mutex<HashMap<String, PlaybackSession>>>,
    thumbnails: ThumbnailService,
    encoder: VideoEncoder,
) {
    let Ok(request) = read_request(&mut stream) else {
        write_response(&mut stream, 400, "text/plain", b"Bad request");
        return;
    };
    let Some((session_id, token, playback_request)) = parse_request(&request) else {
        write_response(&mut stream, 404, "text/plain", b"Not found");
        return;
    };
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
            start_ms, track_id, ..
        } => stream_audio(&mut stream, &segment, &track_id, start_ms),
        PlaybackRequest::Thumbnail {
            offset_ms,
            segment_id,
        } => stream_thumbnail(
            &mut stream,
            &thumbnails,
            session_id,
            segment_id,
            &segment,
            offset_ms,
        ),
        PlaybackRequest::Video { start_ms, .. } => {
            stream_video(&mut stream, &segment, &encoder, start_ms)
        }
    }
}

fn read_request(stream: &mut TcpStream) -> Result<String, String> {
    let mut buffer = [0_u8; 4096];
    let size = stream
        .read(&mut buffer)
        .map_err(|error| error.to_string())?;
    String::from_utf8(buffer[..size].to_vec()).map_err(|error| error.to_string())
}

pub(super) fn parse_request(request: &str) -> Option<(String, String, PlaybackRequest)> {
    let first_line = request.lines().next()?;
    let mut parts = first_line.split_whitespace();
    let method = parts.next()?;
    let url = parts.next()?;
    if method != "GET" {
        return None;
    }
    let (path, query) = url.split_once('?').unwrap_or((url, ""));
    let segments = path.trim_start_matches('/').split('/').collect::<Vec<_>>();
    if segments.len() < 6 || segments[0] != "playback" || segments[3] != "segments" {
        return None;
    }
    let session_id = segments[1].to_string();
    let token = segments[2].to_string();
    let segment_id = segments[4].to_string();
    let start_ms = query_value(query, "startMs").unwrap_or(0).max(0);

    match segments.get(5).copied()? {
        "video" => Some((
            session_id,
            token,
            PlaybackRequest::Video {
                segment_id,
                start_ms,
            },
        )),
        "audio" => Some((
            session_id,
            token,
            PlaybackRequest::Audio {
                segment_id,
                start_ms,
                track_id: segments.get(6)?.to_string(),
            },
        )),
        "thumbnail" => {
            let offset = segments.get(6)?.strip_suffix(".jpg")?.parse().ok()?;
            Some((
                session_id,
                token,
                PlaybackRequest::Thumbnail {
                    offset_ms: offset,
                    segment_id,
                },
            ))
        }
        _ => None,
    }
}

fn query_value(query: &str, key: &str) -> Option<i64> {
    query.split('&').find_map(|pair| {
        let (pair_key, value) = pair.split_once('=')?;
        (pair_key == key).then(|| value.parse().ok()).flatten()
    })
}

fn stream_video(
    stream: &mut TcpStream,
    segment: &PlaybackSegment,
    encoder: &VideoEncoder,
    start_ms: i64,
) {
    let Some(video_stream_index) = segment.video_stream_index else {
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
    command
        .args(input_seek_ffmpeg_args(start_ms, &segment.file_path))
        .args(["-map", &format!("0:{video_stream_index}")])
        .arg("-an");
    append_video_encoder_args_with_fps(&mut command, encoder, segment.playback_fps);
    command
        .args([
            "-movflags",
            "empty_moov+frag_keyframe+default_base_moof",
            "-f",
            "mp4",
            "pipe:1",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null());

    stream_ffmpeg_stdout(stream, command, "video/mp4");
}

fn stream_audio(stream: &mut TcpStream, segment: &PlaybackSegment, track_id: &str, start_ms: i64) {
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

    let mut command = Command::new(ffmpeg::binary_path());
    command
        .args(["-hide_banner", "-v", "error"])
        .args(input_seek_ffmpeg_args(start_ms, &segment.file_path))
        .args(["-map", &format!("0:{stream_index}")])
        .args([
            "-vn",
            "-c:a",
            "aac",
            "-b:a",
            "128k",
            "-movflags",
            "empty_moov+frag_keyframe+default_base_moof",
            "-f",
            "mp4",
            "pipe:1",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null());

    stream_ffmpeg_stdout(stream, command, "audio/mp4");
}
fn stream_thumbnail(
    stream: &mut TcpStream,
    thumbnails: &ThumbnailService,
    session_id: String,
    segment_id: String,
    segment: &PlaybackSegment,
    offset_ms: i64,
) {
    let Some(video_stream_index) = segment.video_stream_index else {
        write_response(stream, 404, "text/plain", b"Segment has no video stream");
        return;
    };
    let key = ThumbnailKey {
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
fn stream_ffmpeg_stdout(stream: &mut TcpStream, mut command: Command, content_type: &str) {
    let Ok(mut child) = command.spawn() else {
        write_response(stream, 500, "text/plain", b"Unable to start FFmpeg");
        return;
    };
    let Some(mut stdout) = child.stdout.take() else {
        let _ = child.kill();
        write_response(stream, 500, "text/plain", b"Unable to read FFmpeg output");
        return;
    };

    let header = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n"
    );
    if stream.write_all(header.as_bytes()).is_err() {
        let _ = child.kill();
        return;
    }

    let mut buffer = [0_u8; 64 * 1024];
    loop {
        match stdout.read(&mut buffer) {
            Ok(0) => break,
            Ok(size) => {
                if stream.write_all(&buffer[..size]).is_err() {
                    let _ = child.kill();
                    break;
                }
            }
            Err(_) => {
                let _ = child.kill();
                break;
            }
        }
    }
    let _ = child.wait();
}

fn write_response(stream: &mut TcpStream, status: u16, content_type: &str, body: &[u8]) {
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
    let _ = stream.write_all(header.as_bytes());
    let _ = stream.write_all(body);
}
