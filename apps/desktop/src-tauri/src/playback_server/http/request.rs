use std::{io::Read, net::TcpStream};

pub(in crate::playback_server) enum PlaybackRequest {
    Audio {
        segment_id: String,
        start_ms: i64,
        track_id: String,
    },
    Thumbnail {
        stream_index: Option<i64>,
        offset_ms: i64,
        segment_id: String,
    },
    Video {
        stream_index: Option<i64>,
        segment_id: String,
        start_ms: i64,
    },
}

pub(super) fn read_request(stream: &mut TcpStream) -> Result<String, String> {
    let mut buffer = [0_u8; 4096];
    let size = stream
        .read(&mut buffer)
        .map_err(|error| error.to_string())?;
    String::from_utf8(buffer[..size].to_vec()).map_err(|error| error.to_string())
}

pub(in crate::playback_server) fn parse_request(
    request: &str,
) -> Option<(String, String, PlaybackRequest)> {
    let first_line = request.lines().next()?;
    let mut parts = first_line.split_whitespace();
    let method = parts.next()?;
    let url = parts.next()?;
    if method != "GET" {
        return None;
    }
    let (path, query) = url.split_once('?').unwrap_or((url, ""));
    let segments = path.trim_start_matches('/').split('/').collect::<Vec<_>>();
    if segments.len() < 6
        || segments[0] != "playback"
        || !matches!(segments[3], "segments" | "runs")
    {
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
                stream_index: segments
                    .get(6)
                    .map(|value| value.parse::<i64>())
                    .transpose()
                    .ok()?,
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
                    stream_index: segments
                        .get(7)
                        .map(|value| value.parse::<i64>())
                        .transpose()
                        .ok()?,
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
