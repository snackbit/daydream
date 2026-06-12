use std::{
    collections::HashMap,
    net::TcpListener,
    path::Path,
    sync::{Arc, Mutex},
    thread,
};

use uuid::Uuid;

use crate::{
    library::{
        LibraryPlaybackAssets, LibraryPlaybackAudioAsset, LibraryPlaybackSegment,
        LibraryPlaybackThumbnail, LibraryTrackKind,
    },
    storage::{RecordingRow, RecordingSegmentRow},
};

mod encoders;
mod ffmpeg_args;
mod http;
#[cfg(test)]
mod tests;
mod thumbnails;

use encoders::{detect_video_encoder, playback_video_fps};
use http::handle_stream;
use thumbnails::{thumbnail_offsets, ThumbnailService, THUMBNAIL_WORKER_COUNT};

#[derive(Clone)]
pub struct PlaybackServer {
    address: String,
    sessions: Arc<Mutex<HashMap<String, PlaybackSession>>>,
    thumbnails: ThumbnailService,
}

#[derive(Clone)]
pub(super) struct PlaybackSession {
    segments: HashMap<String, PlaybackSegment>,
    token: String,
}

#[derive(Clone)]
pub(super) struct PlaybackSegment {
    desktop_audio_stream_index: Option<i64>,
    file_path: String,
    microphone_audio_stream_index: Option<i64>,
    playback_fps: f64,
    video_stream_index: Option<i64>,
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
                PlaybackSession::new(session_segments, token),
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

impl PlaybackSession {
    fn new(segments: HashMap<String, PlaybackSegment>, token: String) -> Self {
        Self { segments, token }
    }

    pub(super) fn segment_for_token(
        &self,
        token: &str,
        segment_id: &str,
    ) -> Option<PlaybackSegment> {
        (self.token == token)
            .then(|| self.segments.get(segment_id).cloned())
            .flatten()
    }
}
