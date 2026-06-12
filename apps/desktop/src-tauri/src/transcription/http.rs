use std::{
    collections::BTreeMap,
    io::{Read, Write},
    net::TcpStream,
    time::Duration,
};

use serde::Serialize;

use super::{network_io_error, transcription_error, TranscriptionError, HTTP_HEADER_LIMIT_BYTES};

#[derive(Debug)]
pub(super) struct HttpRequest {
    pub(super) body: Vec<u8>,
    pub(super) headers: BTreeMap<String, String>,
    pub(super) method: String,
    pub(super) path: String,
}

#[derive(Debug)]
pub(super) struct HttpResponse {
    pub(super) body: Vec<u8>,
    content_type: &'static str,
    status: u16,
}

impl HttpResponse {
    pub(super) fn json(status: u16, value: &impl Serialize) -> Self {
        Self {
            body: serde_json::to_vec(value).unwrap_or_else(|_error| b"{}".to_vec()),
            content_type: "application/json",
            status,
        }
    }
}

pub(super) fn error_response(error: &TranscriptionError) -> HttpResponse {
    let status = match error.code.as_str() {
        "ai_transcription_token_required" | "ai_transcription_token_invalid" => 401,
        "ai_transcription_input_too_large" => 413,
        "ai_transcription_route_not_found" => 404,
        _ => 400,
    };
    HttpResponse::json(status, error)
}

pub(super) fn read_http_request(
    stream: &mut TcpStream,
    max_body_bytes: u64,
) -> Result<HttpRequest, TranscriptionError> {
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .map_err(network_io_error)?;
    let mut buffer = Vec::new();
    let mut temp = [0_u8; 4096];
    let header_end;
    loop {
        let read = stream.read(&mut temp).map_err(network_io_error)?;
        if read == 0 {
            return Err(transcription_error(
                "ai_transcription_request_invalid",
                "HTTP request ended before headers were complete.",
            ));
        }
        buffer.extend_from_slice(&temp[..read]);
        if let Some(index) = find_bytes(&buffer, b"\r\n\r\n") {
            header_end = index + 4;
            break;
        }
        if buffer.len() > HTTP_HEADER_LIMIT_BYTES {
            return Err(transcription_error(
                "ai_transcription_request_invalid",
                "HTTP request headers are too large.",
            ));
        }
    }
    let header_text = std::str::from_utf8(&buffer[..header_end]).map_err(|_error| {
        transcription_error(
            "ai_transcription_request_invalid",
            "HTTP request headers are not valid UTF-8.",
        )
    })?;
    let mut lines = header_text.split("\r\n");
    let request_line = lines.next().ok_or_else(|| {
        transcription_error(
            "ai_transcription_request_invalid",
            "HTTP request line is missing.",
        )
    })?;
    let mut request_parts = request_line.split_whitespace();
    let method = request_parts.next().unwrap_or_default().to_string();
    let path = request_parts.next().unwrap_or_default().to_string();
    let mut headers = BTreeMap::new();
    for line in lines.filter(|line| !line.is_empty()) {
        if let Some((name, value)) = line.split_once(':') {
            headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_string());
        }
    }
    let content_length = headers
        .get("content-length")
        .and_then(|value| value.parse::<u64>().ok())
        .ok_or_else(|| {
            transcription_error(
                "ai_transcription_request_invalid",
                "HTTP request content length is missing.",
            )
        })?;
    if content_length > max_body_bytes {
        return Err(transcription_error(
            "ai_transcription_input_too_large",
            "Transcription input exceeds the upload limit.",
        ));
    }
    let mut body = buffer[header_end..].to_vec();
    while body.len() < content_length as usize {
        let read = stream.read(&mut temp).map_err(network_io_error)?;
        if read == 0 {
            break;
        }
        body.extend_from_slice(&temp[..read]);
    }
    body.truncate(content_length as usize);

    Ok(HttpRequest {
        body,
        headers,
        method,
        path,
    })
}

pub(super) fn write_http_response(
    stream: &mut TcpStream,
    response: HttpResponse,
) -> Result<(), TranscriptionError> {
    let status_text = match response.status {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        413 => "Payload Too Large",
        _ => "Internal Server Error",
    };
    write!(
        stream,
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        response.status,
        status_text,
        response.content_type,
        response.body.len()
    )
    .map_err(network_io_error)?;
    stream.write_all(&response.body).map_err(network_io_error)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct MultipartForm {
    fields: Vec<MultipartField>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct MultipartField {
    pub(super) data: Vec<u8>,
    pub(super) filename: Option<String>,
    pub(super) name: String,
}

impl MultipartForm {
    pub(super) fn file(&self, name: &str) -> Result<&MultipartField, TranscriptionError> {
        self.fields
            .iter()
            .find(|field| field.name == name && field.filename.is_some())
            .ok_or_else(|| {
                transcription_error(
                    "ai_transcription_request_invalid",
                    "Transcription request is missing the audio file.",
                )
            })
    }

    pub(super) fn text(&self, name: &str) -> Result<&str, TranscriptionError> {
        self.texts(name).into_iter().next().ok_or_else(|| {
            transcription_error(
                "ai_transcription_request_invalid",
                "Transcription request is missing a required form field.",
            )
        })
    }

    pub(super) fn optional_text(&self, name: &str) -> Option<&str> {
        self.texts(name).into_iter().next()
    }

    pub(super) fn texts(&self, name: &str) -> Vec<&str> {
        self.fields
            .iter()
            .filter(|field| field.name == name && field.filename.is_none())
            .filter_map(|field| std::str::from_utf8(&field.data).ok())
            .collect()
    }
}

pub(super) fn multipart_boundary(content_type: &str) -> Result<String, TranscriptionError> {
    for part in content_type.split(';') {
        let trimmed = part.trim();
        if let Some(boundary) = trimmed.strip_prefix("boundary=") {
            return Ok(boundary.trim_matches('"').to_string());
        }
    }
    Err(transcription_error(
        "ai_transcription_request_invalid",
        "Multipart boundary is missing.",
    ))
}

pub(super) fn parse_multipart_form(
    body: &[u8],
    boundary: &str,
) -> Result<MultipartForm, TranscriptionError> {
    let marker = format!("--{boundary}");
    let marker = marker.as_bytes();
    let mut fields = Vec::new();
    for raw_part in split_bytes(body, marker) {
        let mut part = raw_part;
        if let Some(stripped) = part.strip_prefix(b"\r\n") {
            part = stripped;
        }
        if part.is_empty() {
            continue;
        }
        if part.starts_with(b"--") {
            continue;
        }
        let header_end = find_bytes(part, b"\r\n\r\n").ok_or_else(|| {
            transcription_error(
                "ai_transcription_request_invalid",
                "Multipart part headers are invalid.",
            )
        })?;
        let header_text = std::str::from_utf8(&part[..header_end]).map_err(|_error| {
            transcription_error(
                "ai_transcription_request_invalid",
                "Multipart part headers are invalid.",
            )
        })?;
        let data = strip_one_trailing_crlf(&part[header_end + 4..]).to_vec();
        let disposition = header_text
            .split("\r\n")
            .find_map(|line| {
                line.split_once(':').and_then(|(name, value)| {
                    name.eq_ignore_ascii_case("content-disposition")
                        .then(|| value.trim())
                })
            })
            .ok_or_else(|| {
                transcription_error(
                    "ai_transcription_request_invalid",
                    "Multipart part disposition is missing.",
                )
            })?;
        let parameters = content_disposition_parameters(disposition);
        let name = parameters.get("name").cloned().ok_or_else(|| {
            transcription_error(
                "ai_transcription_request_invalid",
                "Multipart field name is missing.",
            )
        })?;
        fields.push(MultipartField {
            data,
            filename: parameters.get("filename").cloned(),
            name,
        });
    }

    Ok(MultipartForm { fields })
}

fn strip_one_trailing_crlf(input: &[u8]) -> &[u8] {
    input.strip_suffix(b"\r\n").unwrap_or(input)
}

fn content_disposition_parameters(value: &str) -> BTreeMap<String, String> {
    let mut parameters = BTreeMap::new();
    for part in value.split(';').skip(1) {
        if let Some((name, raw_value)) = part.trim().split_once('=') {
            parameters.insert(
                name.trim().to_ascii_lowercase(),
                raw_value.trim().trim_matches('"').to_string(),
            );
        }
    }
    parameters
}

fn split_bytes<'a>(input: &'a [u8], needle: &[u8]) -> Vec<&'a [u8]> {
    let mut parts = Vec::new();
    let mut start = 0;
    while let Some(relative) = find_bytes(&input[start..], needle) {
        parts.push(&input[start..start + relative]);
        start += relative + needle.len();
    }
    parts.push(&input[start..]);
    parts
}

pub(super) fn find_bytes(input: &[u8], needle: &[u8]) -> Option<usize> {
    input
        .windows(needle.len())
        .position(|window| window == needle)
}
