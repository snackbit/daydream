//! Serve indexed prepared files without a decoder or a process per seek.
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
    net::TcpStream,
    path::Path,
};

/// This public route contains only the bundled synthetic frame, never recordings.
pub(super) fn serve_warmup(stream: &mut TcpStream, request: &str) -> bool {
    let mut fields = request
        .lines()
        .next()
        .unwrap_or_default()
        .split_whitespace();
    let method = fields.next().unwrap_or_default();
    if !matches!(method, "GET" | "HEAD") || fields.next() != Some("/decoder-warmup.mp4") {
        return false;
    }
    const FRAME: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../src/features/startup/media/decoder-warmup.mp4"
    ));
    let header = format!("HTTP/1.1 200 OK\r\nContent-Type: video/mp4\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nCache-Control: private, max-age=86400\r\nConnection: close\r\n\r\n", FRAME.len());
    if stream.write_all(header.as_bytes()).is_ok() && method == "GET" {
        let _ = stream.write_all(FRAME);
    }
    true
}

fn range(header: Option<&str>, length: u64) -> Option<(u64, u64)> {
    if length == 0 {
        return None;
    }
    let Some(header) = header else {
        return Some((0, length - 1));
    };
    let value = header.strip_prefix("bytes=")?;
    if value.contains(',') {
        return None;
    }
    let (start, end) = value.split_once('-')?;
    if start.is_empty() {
        let count = end.parse::<u64>().ok()?;
        return (count > 0).then_some((length.saturating_sub(count), length - 1));
    }
    let start = start.parse::<u64>().ok()?;
    let end = if end.is_empty() {
        length - 1
    } else {
        end.parse::<u64>().ok()?.min(length - 1)
    };
    (start < length && start <= end).then_some((start, end))
}

pub(super) fn serve(stream: &mut TcpStream, path: &Path, request: &str, video: bool) {
    let Ok(mut file) = File::open(path) else {
        super::write_response(stream, 404, "text/plain", b"Prepared media unavailable");
        return;
    };
    let Ok(metadata) = file.metadata() else {
        return;
    };
    let length = metadata.len();
    let requested = request.lines().skip(1).find_map(|line| {
        line.split_once(':')
            .filter(|(name, _)| name.eq_ignore_ascii_case("range"))
            .map(|(_, value)| value.trim())
    });
    let Some((start, end)) = range(requested, length) else {
        let _=write!(stream,"HTTP/1.1 416 Range Not Satisfiable\r\nContent-Range: bytes */{length}\r\nContent-Length: 0\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n");
        return;
    };
    let partial = requested.is_some();
    let content_type = if video { "video/mp4" } else { "audio/mp4" };
    let content_range = if partial {
        format!("Content-Range: bytes {start}-{end}/{length}\r\n")
    } else {
        String::new()
    };
    let header=format!("HTTP/1.1 {}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nAccept-Ranges: bytes\r\n{content_range}Cache-Control: private, no-store\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Expose-Headers: Accept-Ranges, Content-Range, Content-Length\r\nConnection: close\r\n\r\n",if partial{"206 Partial Content"}else{"200 OK"},end-start+1);
    if stream.write_all(header.as_bytes()).is_err() || request.starts_with("HEAD ") {
        return;
    }
    if file.seek(SeekFrom::Start(start)).is_ok() {
        let _ = std::io::copy(&mut file.take(end - start + 1), stream);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn byte_ranges_are_bounded_and_invalid_requests_fail_closed() {
        for (input, expected) in [
            (None, Some((0, 99))),
            (Some("bytes=10-19"), Some((10, 19))),
            (Some("bytes=95-"), Some((95, 99))),
            (Some("bytes=-10"), Some((90, 99))),
            (Some("bytes=80-999"), Some((80, 99))),
            (Some("bytes=100-"), None),
            (Some("bytes=8-2"), None),
            (Some("bytes=-0"), None),
            (Some("bytes=0-1,4-5"), None),
            (Some("broken"), None),
        ] {
            assert_eq!(range(input, 100), expected, "{input:?}");
        }
        assert_eq!(range(None, 0), None);
    }
}
