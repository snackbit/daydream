use std::{
    collections::{HashMap, HashSet, VecDeque},
    fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::Path,
    process::{Command, Stdio},
    sync::{mpsc, Arc, Condvar, Mutex},
    thread,
};

use uuid::Uuid;

use crate::{
    ffmpeg,
    library::{
        LibraryPlaybackAssets, LibraryPlaybackAudioAsset, LibraryPlaybackSegment,
        LibraryPlaybackThumbnail, LibraryTrackKind,
    },
    storage::{RecordingRow, RecordingSegmentRow},
};

const PREVIEW_LONG_EDGE_PX: i64 = 3_840;
const MAX_PLAYBACK_VIDEO_FPS: f64 = 60.0;
const MIN_PLAYBACK_VIDEO_FPS: f64 = 1.0;
const THUMBNAIL_WIDTH_PX: i64 = 320;
const THUMBNAIL_INTERVAL_MS: i64 = 3_000;
const THUMBNAIL_WORKER_COUNT: usize = 2;

#[derive(Clone)]
pub struct PlaybackServer {
    address: String,
    sessions: Arc<Mutex<HashMap<String, PlaybackSession>>>,
    thumbnails: ThumbnailService,
}

#[derive(Clone)]
struct PlaybackSession {
    segments: HashMap<String, PlaybackSegment>,
    token: String,
}

#[derive(Clone)]
struct PlaybackSegment {
    desktop_audio_stream_index: Option<i64>,
    file_path: String,
    microphone_audio_stream_index: Option<i64>,
    playback_fps: f64,
    video_stream_index: Option<i64>,
}

#[derive(Clone)]
struct ThumbnailService {
    sessions: Arc<Mutex<HashMap<String, PlaybackSession>>>,
    state: Arc<(Mutex<ThumbnailState>, Condvar)>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct ThumbnailKey {
    offset_ms: i64,
    segment_id: String,
    session_id: String,
}

struct ThumbnailJob {
    key: ThumbnailKey,
    offset_ms: i64,
    segment: PlaybackSegment,
    video_stream_index: i64,
}

struct ThumbnailState {
    cache: HashMap<ThumbnailKey, Arc<Vec<u8>>>,
    in_progress: HashMap<ThumbnailKey, Vec<mpsc::Sender<ThumbnailResult>>>,
    queue: VecDeque<ThumbnailJob>,
    queued_keys: HashSet<ThumbnailKey>,
    running_keys: HashSet<ThumbnailKey>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ThumbnailError {
    Failed,
    StaleSession,
}

#[derive(Debug, PartialEq, Eq)]
enum ThumbnailEnqueueOutcome {
    Cached,
    Deduplicated,
    Enqueued,
}

type ThumbnailResult = Result<Arc<Vec<u8>>, ThumbnailError>;

#[derive(Clone, Debug, PartialEq, Eq)]
enum VideoEncoder {
    Amf,
    Libx264,
    Nvenc,
    Qsv,
    Vaapi,
}

enum PlaybackRequest {
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

impl PlaybackServer {
    pub fn start() -> Result<Self, Box<dyn std::error::Error>> {
        let listener = TcpListener::bind(("127.0.0.1", 0))?;
        let address = format!("http://{}", listener.local_addr()?);
        let sessions = Arc::new(Mutex::new(HashMap::new()));
        let thumbnails = ThumbnailService::new(Arc::clone(&sessions), THUMBNAIL_WORKER_COUNT);
        let sessions_for_thread = Arc::clone(&sessions);
        let thumbnails_for_thread = thumbnails.clone();
        let encoder = detect_video_encoder();
        let encoder_for_thread = encoder.clone();

        thread::Builder::new()
            .name("daydream-playback-server".to_string())
            .spawn(move || {
                for stream in listener.incoming().flatten() {
                    let sessions_for_request = Arc::clone(&sessions_for_thread);
                    let thumbnails_for_request = thumbnails_for_thread.clone();
                    let encoder_for_request = encoder_for_thread.clone();
                    let _ = thread::Builder::new()
                        .name("daydream-playback-request".to_string())
                        .spawn(move || {
                            handle_stream(
                                stream,
                                sessions_for_request,
                                thumbnails_for_request,
                                encoder_for_request,
                            );
                        });
                }
            })?;

        Ok(Self {
            address,
            sessions,
            thumbnails,
        })
    }

    pub fn create_session(
        &self,
        recording: &RecordingRow,
        segments: &[RecordingSegmentRow],
    ) -> Result<LibraryPlaybackAssets, String> {
        let session_id = Uuid::now_v7().to_string();
        let token = Uuid::now_v7().to_string();
        let mut session_segments = HashMap::new();
        let mut response_segments = Vec::new();

        for segment in segments.iter().filter(|segment| {
            segment.state == "finalized" && Path::new(&segment.file_path).is_file()
        }) {
            let duration_ms = segment.duration_ms.unwrap_or(0).max(1);
            session_segments.insert(
                segment.id.clone(),
                PlaybackSegment {
                    desktop_audio_stream_index: segment.desktop_audio_stream_index,
                    file_path: segment.file_path.clone(),
                    microphone_audio_stream_index: segment.microphone_audio_stream_index,
                    playback_fps: playback_video_fps(recording.default_fps),
                    video_stream_index: segment.video_stream_index,
                },
            );

            let base = format!(
                "{}/playback/{}/{}/segments/{}",
                self.address, session_id, token, segment.id
            );
            let mut audio = Vec::new();
            if let Some(stream_index) = segment.desktop_audio_stream_index {
                audio.push(LibraryPlaybackAudioAsset {
                    kind: LibraryTrackKind::DesktopAudio,
                    stream_index,
                    track_id: format!("desktop-audio-{stream_index}"),
                    url: format!("{base}/audio/desktop-audio-{stream_index}"),
                });
            }
            if let Some(stream_index) = segment.microphone_audio_stream_index {
                audio.push(LibraryPlaybackAudioAsset {
                    kind: LibraryTrackKind::Microphone,
                    stream_index,
                    track_id: format!("microphone-{stream_index}"),
                    url: format!("{base}/audio/microphone-{stream_index}"),
                });
            }

            response_segments.push(LibraryPlaybackSegment {
                audio,
                segment_id: segment.id.clone(),
                thumbnails: thumbnail_offsets(duration_ms)
                    .into_iter()
                    .map(|offset_ms| LibraryPlaybackThumbnail {
                        offset_ms,
                        url: format!("{base}/thumbnail/{offset_ms}.jpg"),
                    })
                    .collect(),
                video_url: format!("{base}/video"),
            });
        }

        self.sessions
            .lock()
            .map_err(|_error| "Unable to lock playback sessions.".to_string())?
            .insert(
                session_id.clone(),
                PlaybackSession {
                    segments: session_segments,
                    token,
                },
            );

        Ok(LibraryPlaybackAssets {
            error: None,
            recording_id: recording.id.clone(),
            segments: response_segments,
            session_id,
        })
    }

    pub fn close_session(&self, session_id: &str) -> Result<(), String> {
        self.sessions
            .lock()
            .map_err(|_error| "Unable to lock playback sessions.".to_string())?
            .remove(session_id);
        self.thumbnails.close_session(session_id);
        Ok(())
    }
}

impl ThumbnailService {
    fn new(sessions: Arc<Mutex<HashMap<String, PlaybackSession>>>, worker_count: usize) -> Self {
        let service = Self {
            sessions,
            state: Arc::new((Mutex::new(ThumbnailState::new()), Condvar::new())),
        };
        for worker_index in 0..worker_count {
            let service_for_worker = service.clone();
            let _ = thread::Builder::new()
                .name(format!("daydream-thumbnail-worker-{worker_index}"))
                .spawn(move || service_for_worker.worker_loop());
        }

        service
    }

    fn request_thumbnail(
        &self,
        key: ThumbnailKey,
        segment: PlaybackSegment,
        video_stream_index: i64,
        offset_ms: i64,
    ) -> ThumbnailResult {
        let (sender, receiver) = mpsc::channel();
        let job = ThumbnailJob {
            key: key.clone(),
            offset_ms,
            segment,
            video_stream_index,
        };
        let (lock, condvar) = &*self.state;
        {
            let mut state = lock.lock().map_err(|_error| ThumbnailError::Failed)?;
            if let ThumbnailEnqueueOutcome::Cached = state.enqueue(key, job, sender)? {
                return receiver.recv().unwrap_or(Err(ThumbnailError::Failed));
            }
        }
        condvar.notify_one();

        receiver.recv().unwrap_or(Err(ThumbnailError::Failed))
    }

    fn close_session(&self, session_id: &str) {
        let waiters = {
            let (lock, condvar) = &*self.state;
            let Ok(mut state) = lock.lock() else {
                return;
            };
            let waiters = state.close_session(session_id);
            condvar.notify_all();
            waiters
        };
        for waiter in waiters {
            let _ = waiter.send(Err(ThumbnailError::StaleSession));
        }
    }

    fn worker_loop(&self) {
        loop {
            let job = {
                let (lock, condvar) = &*self.state;
                let mut state = match lock.lock() {
                    Ok(state) => state,
                    Err(_) => return,
                };
                loop {
                    if let Some(job) = state.pop_next_job() {
                        break job;
                    }
                    state = match condvar.wait(state) {
                        Ok(state) => state,
                        Err(_) => return,
                    };
                }
            };

            let result = if self.session_exists(&job.key.session_id) {
                generate_thumbnail(&job.segment, job.video_stream_index, job.offset_ms)
                    .map(Arc::new)
                    .map_err(|_error| ThumbnailError::Failed)
            } else {
                Err(ThumbnailError::StaleSession)
            };
            let result = if result.is_ok() && !self.session_exists(&job.key.session_id) {
                Err(ThumbnailError::StaleSession)
            } else {
                result
            };
            self.complete_job(job.key, result);
        }
    }

    fn complete_job(&self, key: ThumbnailKey, result: ThumbnailResult) {
        let waiters = {
            let (lock, _condvar) = &*self.state;
            let Ok(mut state) = lock.lock() else {
                return;
            };
            state.complete_job(key, result.clone())
        };
        for waiter in waiters {
            let _ = waiter.send(result.clone());
        }
    }

    fn session_exists(&self, session_id: &str) -> bool {
        self.sessions
            .lock()
            .is_ok_and(|sessions| sessions.contains_key(session_id))
    }
}

impl ThumbnailState {
    fn new() -> Self {
        Self {
            cache: HashMap::new(),
            in_progress: HashMap::new(),
            queue: VecDeque::new(),
            queued_keys: HashSet::new(),
            running_keys: HashSet::new(),
        }
    }

    fn enqueue(
        &mut self,
        key: ThumbnailKey,
        job: ThumbnailJob,
        sender: mpsc::Sender<ThumbnailResult>,
    ) -> Result<ThumbnailEnqueueOutcome, ThumbnailError> {
        if let Some(cached) = self.cache.get(&key) {
            sender
                .send(Ok(Arc::clone(cached)))
                .map_err(|_error| ThumbnailError::Failed)?;
            return Ok(ThumbnailEnqueueOutcome::Cached);
        }
        if let Some(waiters) = self.in_progress.get_mut(&key) {
            waiters.push(sender);
            return Ok(ThumbnailEnqueueOutcome::Deduplicated);
        }

        self.in_progress.insert(key.clone(), vec![sender]);
        self.queued_keys.insert(key);
        self.queue.push_back(job);
        Ok(ThumbnailEnqueueOutcome::Enqueued)
    }

    fn pop_next_job(&mut self) -> Option<ThumbnailJob> {
        let job = self.queue.pop_front()?;
        self.queued_keys.remove(&job.key);
        self.running_keys.insert(job.key.clone());
        Some(job)
    }

    fn complete_job(
        &mut self,
        key: ThumbnailKey,
        result: ThumbnailResult,
    ) -> Vec<mpsc::Sender<ThumbnailResult>> {
        self.running_keys.remove(&key);
        if let Ok(bytes) = &result {
            self.cache.insert(key.clone(), Arc::clone(bytes));
        }
        self.in_progress.remove(&key).unwrap_or_default()
    }

    fn close_session(&mut self, session_id: &str) -> Vec<mpsc::Sender<ThumbnailResult>> {
        self.cache
            .retain(|key, _bytes| key.session_id.as_str() != session_id);
        let mut removed_queued_keys = HashSet::new();
        self.queue.retain(|job| {
            let keep = job.key.session_id != session_id;
            if !keep {
                removed_queued_keys.insert(job.key.clone());
                self.queued_keys.remove(&job.key);
            }
            keep
        });

        let mut waiters = Vec::new();
        for key in removed_queued_keys {
            if let Some(mut next_waiters) = self.in_progress.remove(&key) {
                waiters.append(&mut next_waiters);
            }
        }

        waiters
    }
}

fn handle_stream(
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
    let Some((session, segment)) = sessions.lock().ok().and_then(|sessions| {
        let session = sessions.get(&session_id)?;
        if session.token != token {
            return None;
        }
        let segment_id = match &playback_request {
            PlaybackRequest::Audio { segment_id, .. }
            | PlaybackRequest::Thumbnail { segment_id, .. }
            | PlaybackRequest::Video { segment_id, .. } => segment_id,
        };
        let segment = session.segments.get(segment_id)?;
        Some((session.clone(), segment.clone()))
    }) else {
        write_response(&mut stream, 403, "text/plain", b"Forbidden");
        return;
    };
    drop(session);

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

fn parse_request(request: &str) -> Option<(String, String, PlaybackRequest)> {
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

fn input_seek_ffmpeg_args(start_ms: i64, file_path: &str) -> Vec<String> {
    vec![
        "-i".to_string(),
        file_path.to_string(),
        "-ss".to_string(),
        format_seconds(start_ms),
    ]
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

fn generate_thumbnail(
    segment: &PlaybackSegment,
    video_stream_index: i64,
    offset_ms: i64,
) -> Result<Vec<u8>, String> {
    let output = Command::new(ffmpeg::binary_path())
        .args(thumbnail_ffmpeg_args(
            segment,
            video_stream_index,
            offset_ms,
        ))
        .output()
        .map_err(|error| error.to_string())?;

    if output.status.success() && !output.stdout.is_empty() {
        Ok(output.stdout)
    } else {
        Err("Unable to generate thumbnail".to_string())
    }
}

fn thumbnail_ffmpeg_args(
    segment: &PlaybackSegment,
    video_stream_index: i64,
    offset_ms: i64,
) -> Vec<String> {
    vec![
        "-hide_banner".to_string(),
        "-v".to_string(),
        "error".to_string(),
        "-i".to_string(),
        segment.file_path.clone(),
        "-ss".to_string(),
        format_seconds(offset_ms),
        "-map".to_string(),
        format!("0:{video_stream_index}"),
        "-frames:v".to_string(),
        "1".to_string(),
        "-an".to_string(),
        "-vf".to_string(),
        format!("scale={THUMBNAIL_WIDTH_PX}:-1,format=yuvj420p"),
        "-q:v".to_string(),
        "4".to_string(),
        "-f".to_string(),
        "image2pipe".to_string(),
        "pipe:1".to_string(),
    ]
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

fn append_video_encoder_args(command: &mut Command, encoder: &VideoEncoder) {
    append_video_encoder_args_with_fps(command, encoder, MIN_PLAYBACK_VIDEO_FPS);
}

fn append_video_encoder_args_with_fps(command: &mut Command, encoder: &VideoEncoder, fps: f64) {
    let fps_filter = format!("fps={:.3}", playback_video_fps(fps));
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

fn playback_video_fps(recording_fps: f64) -> f64 {
    if recording_fps.is_finite() && recording_fps > 0.0 {
        recording_fps.clamp(MIN_PLAYBACK_VIDEO_FPS, MAX_PLAYBACK_VIDEO_FPS)
    } else {
        MIN_PLAYBACK_VIDEO_FPS
    }
}

fn detect_video_encoder() -> VideoEncoder {
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

fn vaapi_device_path() -> Option<String> {
    ["/dev/dri/renderD128", "/dev/dri/renderD129"]
        .into_iter()
        .find(|path| fs::metadata(path).is_ok())
        .map(ToString::to_string)
}

fn thumbnail_offsets(duration_ms: i64) -> Vec<i64> {
    let mut offsets = Vec::new();
    let mut offset_ms = 0;
    while offset_ms < duration_ms.max(1) {
        offsets.push(offset_ms);
        offset_ms += THUMBNAIL_INTERVAL_MS;
    }
    offsets
}

fn format_seconds(ms: i64) -> String {
    format!("{:.3}", ms.max(0) as f64 / 1000.0)
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use super::*;

    #[test]
    fn parser_rejects_requests_without_session_token() {
        assert!(parse_request("GET /playback/session/segments/one/video HTTP/1.1\r\n").is_none());
    }

    #[test]
    fn parser_reads_video_start_time() {
        let parsed = parse_request(
            "GET /playback/session/token/segments/segment-1/video?startMs=1200 HTTP/1.1\r\n",
        )
        .expect("parse request");

        assert_eq!(parsed.0, "session");
        assert_eq!(parsed.1, "token");
        match parsed.2 {
            PlaybackRequest::Video {
                segment_id,
                start_ms,
            } => {
                assert_eq!(segment_id, "segment-1");
                assert_eq!(start_ms, 1200);
            }
            _ => panic!("expected video request"),
        }
    }

    #[test]
    fn thumbnails_cover_segment_from_start_at_fixed_interval() {
        assert_eq!(thumbnail_offsets(9_500), vec![0, 3_000, 6_000, 9_000]);
    }

    #[test]
    fn thumbnail_state_deduplicates_matching_queued_requests() {
        let mut state = ThumbnailState::new();
        let key = thumbnail_key("session-1", "segment-1", 3_000);
        let (first_sender, _first_receiver) = mpsc::channel();
        let (second_sender, _second_receiver) = mpsc::channel();

        assert_eq!(
            state
                .enqueue(key.clone(), thumbnail_job(key.clone()), first_sender)
                .expect("enqueue first"),
            ThumbnailEnqueueOutcome::Enqueued,
        );
        assert_eq!(
            state
                .enqueue(key.clone(), thumbnail_job(key.clone()), second_sender)
                .expect("enqueue second"),
            ThumbnailEnqueueOutcome::Deduplicated,
        );

        assert_eq!(state.queue.len(), 1);
        assert_eq!(state.in_progress.get(&key).map(Vec::len), Some(2));
    }

    #[test]
    fn thumbnail_state_pops_jobs_in_fifo_order() {
        let mut state = ThumbnailState::new();
        let first = thumbnail_key("session-1", "segment-1", 0);
        let second = thumbnail_key("session-1", "segment-1", 3_000);
        let (first_sender, _first_receiver) = mpsc::channel();
        let (second_sender, _second_receiver) = mpsc::channel();

        state
            .enqueue(first.clone(), thumbnail_job(first.clone()), first_sender)
            .expect("enqueue first");
        state
            .enqueue(second.clone(), thumbnail_job(second.clone()), second_sender)
            .expect("enqueue second");

        assert_eq!(state.pop_next_job().map(|job| job.key), Some(first));
        assert_eq!(state.pop_next_job().map(|job| job.key), Some(second));
    }

    #[test]
    fn thumbnail_state_returns_cache_hits_without_queueing() {
        let mut state = ThumbnailState::new();
        let key = thumbnail_key("session-1", "segment-1", 0);
        let bytes = Arc::new(vec![1, 2, 3]);
        let (sender, receiver) = mpsc::channel();
        state.cache.insert(key.clone(), Arc::clone(&bytes));

        assert_eq!(
            state
                .enqueue(key.clone(), thumbnail_job(key), sender)
                .expect("cache hit"),
            ThumbnailEnqueueOutcome::Cached,
        );

        assert_eq!(state.queue.len(), 0);
        assert_eq!(
            receiver.recv().expect("cached result").expect("thumbnail"),
            bytes,
        );
    }

    #[test]
    fn thumbnail_state_close_session_removes_queued_jobs_and_cache() {
        let mut state = ThumbnailState::new();
        let closed_cached = thumbnail_key("session-1", "segment-1", 0);
        let closed_queued = thumbnail_key("session-1", "segment-1", 3_000);
        let retained_cached = thumbnail_key("session-2", "segment-1", 0);
        let retained_queued = thumbnail_key("session-2", "segment-1", 3_000);
        let (closed_sender, _closed_receiver) = mpsc::channel();
        let (retained_sender, _retained_receiver) = mpsc::channel();
        state.cache.insert(closed_cached.clone(), Arc::new(vec![1]));
        state
            .cache
            .insert(retained_cached.clone(), Arc::new(vec![2]));

        state
            .enqueue(
                closed_queued.clone(),
                thumbnail_job(closed_queued.clone()),
                closed_sender,
            )
            .expect("enqueue closed");
        state
            .enqueue(
                retained_queued.clone(),
                thumbnail_job(retained_queued.clone()),
                retained_sender,
            )
            .expect("enqueue retained");

        let waiters = state.close_session("session-1");

        assert_eq!(waiters.len(), 1);
        assert!(!state.cache.contains_key(&closed_cached));
        assert!(state.cache.contains_key(&retained_cached));
        assert_eq!(state.queue.len(), 1);
        assert_eq!(
            state.queue.front().map(|job| &job.key),
            Some(&retained_queued),
        );
    }

    #[test]
    fn stream_args_seek_after_input_for_non_zero_start_time() {
        assert_eq!(
            input_seek_ffmpeg_args(1_200, "/tmp/daydream/segment.mkv"),
            vec![
                "-i".to_string(),
                "/tmp/daydream/segment.mkv".to_string(),
                "-ss".to_string(),
                "1.200".to_string(),
            ],
        );
    }

    #[test]
    fn playback_video_fps_clamps_to_browser_safe_range() {
        assert_eq!(playback_video_fps(0.2), 1.0);
        assert_eq!(playback_video_fps(30.0), 30.0);
        assert_eq!(playback_video_fps(1_000.0), 60.0);
        assert_eq!(playback_video_fps(f64::NAN), 1.0);
    }

    #[test]
    fn video_encoder_args_force_clamped_output_fps() {
        let mut command = Command::new("ffmpeg");
        append_video_encoder_args_with_fps(&mut command, &VideoEncoder::Libx264, 1_000.0);
        let args = command
            .get_args()
            .map(|arg| arg.to_string_lossy().to_string())
            .collect::<Vec<_>>();

        assert!(args.contains(&"-vf".to_string()));
        assert!(args.iter().any(|arg| arg.starts_with("fps=60.000,scale=")));
    }

    #[test]
    fn thumbnail_args_seek_after_input_and_write_jpeg_pipe() {
        let segment = PlaybackSegment {
            desktop_audio_stream_index: None,
            file_path: "/tmp/daydream/segment.mkv".to_string(),
            microphone_audio_stream_index: None,
            playback_fps: 1.0,
            video_stream_index: Some(0),
        };
        let args = thumbnail_ffmpeg_args(&segment, 0, 3_000);

        let input_position = args.iter().position(|arg| arg == "-i").expect("input flag");
        let seek_position = args.iter().position(|arg| arg == "-ss").expect("seek flag");

        assert!(input_position < seek_position);
        assert_eq!(args[seek_position + 1], "3.000");
        assert!(args.contains(&"scale=320:-1,format=yuvj420p".to_string()));
        assert!(args.contains(&"image2pipe".to_string()));
        assert_eq!(args.last(), Some(&"pipe:1".to_string()));
    }

    #[test]
    fn software_encoder_is_available_as_fallback() {
        let encoder = detect_video_encoder();

        assert!(matches!(
            encoder,
            VideoEncoder::Amf
                | VideoEncoder::Libx264
                | VideoEncoder::Nvenc
                | VideoEncoder::Qsv
                | VideoEncoder::Vaapi
        ));
    }

    fn thumbnail_key(session_id: &str, segment_id: &str, offset_ms: i64) -> ThumbnailKey {
        ThumbnailKey {
            offset_ms,
            segment_id: segment_id.to_string(),
            session_id: session_id.to_string(),
        }
    }

    fn thumbnail_job(key: ThumbnailKey) -> ThumbnailJob {
        ThumbnailJob {
            offset_ms: key.offset_ms,
            segment: PlaybackSegment {
                desktop_audio_stream_index: None,
                file_path: "/tmp/daydream/segment.mkv".to_string(),
                microphone_audio_stream_index: None,
                playback_fps: 1.0,
                video_stream_index: Some(0),
            },
            video_stream_index: 0,
            key,
        }
    }
}
