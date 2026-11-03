use std::{io::Read, net::TcpStream};

pub(in crate::playback_server) enum PlaybackRequest {
    File {
        segment_id: String,
        asset_key: String,
    },
    Audio {
        segment_id: String,
        start_ms: i64,
        duration_ms: Option<i64>,
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
        duration_ms: Option<i64>,
    },
}

pub(super) fn read_request(stream: &mut TcpStream) -> Result<String, String> {
    read_headers(stream)
}

fn read_headers(reader: &mut impl Read) -> Result<String, String> {
    let mut request = Vec::with_capacity(4096);
    let mut buffer = [0u8; 1024];
    while request.len() < 4096 {
        let remaining = buffer.len().min(4096 - request.len());
        let size = reader
            .read(&mut buffer[..remaining])
            .map_err(|_| "Playback request could not be read.")?;
        if size == 0 {
            return Err("Playback request is incomplete.".into());
        }
        request.extend_from_slice(&buffer[..size]);
        if request.windows(4).any(|bytes| bytes == b"\r\n\r\n") {
            return String::from_utf8(request).map_err(|_| "Playback request is invalid.".into());
        }
    }
    Err("Playback request is too large.".into())
}

pub(in crate::playback_server) fn parse_request(
    request: &str,
) -> Option<(String, String, PlaybackRequest)> {
    let first_line = request.lines().next()?;
    let mut parts = first_line.split_whitespace();
    let method = parts.next()?;
    let url = parts.next()?;
    if method != "GET" && method != "HEAD" {
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
    if method == "HEAD" && segments[5] != "file" {
        return None;
    }
    let start_ms = query_value(query, "startMs").unwrap_or(0).max(0);
    let duration_ms = if query.split('&').any(|pair| {
        pair.split_once('=')
            .is_some_and(|(key, _)| key == "durationMs")
    }) {
        let duration = query_value(query, "durationMs")?;
        if duration <= 0 {
            return None;
        }
        Some(duration)
    } else {
        None
    };

    match segments.get(5).copied()? {
        "file" => {
            let key = segments.get(6)?;
            let (role, index) = key.rsplit_once('-')?;
            if !matches!(role, "video" | "desktop-audio" | "microphone")
                || index.parse::<u32>().is_err()
            {
                return None;
            }
            Some((
                session_id,
                token,
                PlaybackRequest::File {
                    segment_id,
                    asset_key: (*key).into(),
                },
            ))
        }
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
                duration_ms,
            },
        )),
        "audio" => Some((
            session_id,
            token,
            PlaybackRequest::Audio {
                segment_id,
                start_ms,
                duration_ms,
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

#[cfg(test)]
mod header_tests {
    use super::*;
    struct Fragmented {
        parts: std::collections::VecDeque<&'static [u8]>,
    }
    impl Read for Fragmented {
        fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
            let Some(part) = self.parts.pop_front() else {
                return Ok(0);
            };
            let count = part.len().min(buffer.len());
            buffer[..count].copy_from_slice(&part[..count]);
            if count < part.len() {
                self.parts.push_front(&part[count..]);
            }
            Ok(count)
        }
    }
    #[test]
    fn fragmented_headers_complete_before_parsing_and_incomplete_headers_fail() {
        let mut reader = Fragmented {
            parts: std::collections::VecDeque::from([
                b"GET /playback/".as_slice(),
                b"session/token/segments/one/audio/microphone-2 HTTP/1.1\r\n".as_slice(),
                b"Host: localhost\r\n".as_slice(),
                b"\r\n".as_slice(),
            ]),
        };
        let request = read_headers(&mut reader).unwrap();
        assert!(parse_request(&request).is_some());
        assert!(read_headers(&mut "GET /playback".as_bytes()).is_err());
        assert!(read_headers(&mut vec![b'x'; 4097].as_slice()).is_err());
    }
}
