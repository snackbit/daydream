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
mod resource_tests;
mod resources;
#[cfg(test)]
#[path = "tests/run_duration.rs"]
mod run_duration_tests;
#[cfg(test)]
mod run_tests;
mod runs;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod thumbnail_tests;
mod thumbnails;

use encoders::{detect_video_encoder, playback_video_fps};
use http::handle_stream;
use thumbnails::{thumbnail_offsets, ThumbnailService, THUMBNAIL_WORKER_COUNT};

#[derive(Clone)]
pub struct PlaybackServer {
    address: String,
    thumbnail_address: String,
    sessions: Arc<Mutex<HashMap<String, PlaybackSession>>>,
    thumbnails: ThumbnailService,
    resources: resources::PlaybackResources,
}

#[derive(Clone)]
pub(super) struct PlaybackSession {
    segments: HashMap<String, PlaybackSegment>,
    token: String,
}

#[derive(Clone)]
pub(super) struct PlaybackSegment {
    fragments: Vec<(String, i64)>,
    duration_ms: i64,
    video_streams: Vec<crate::storage::SegmentVideoStream>,
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
        // Browsers bound concurrent HTTP/1 connections per origin. Slow preview
        // jobs must not occupy the connections needed by playback or seeking.
        let thumbnail_listener = TcpListener::bind(("127.0.0.1", 0))?;
        let thumbnail_address = format!("http://{}", thumbnail_listener.local_addr()?);
        let sessions = Arc::new(Mutex::new(HashMap::new()));
        let resources = resources::PlaybackResources::default();
        let thumbnails = ThumbnailService::new_with_resources(
            Arc::clone(&sessions),
            THUMBNAIL_WORKER_COUNT,
            resources.clone(),
        );
        let encoder = detect_video_encoder();
        for (listener, thread_name) in [
            (listener, "daydream-playback-server"),
            (thumbnail_listener, "daydream-preview-server"),
        ] {
            let sessions_for_thread = Arc::clone(&sessions);
            let thumbnails_for_thread = thumbnails.clone();
            let encoder_for_thread = encoder.clone();
            let resources_for_thread = resources.clone();

            thread::Builder::new()
                .name(thread_name.to_string())
                .spawn(move || {
                    for stream in listener.incoming().flatten() {
                        let Ok(resource) = resources_for_thread.begin(Some(&stream), None) else {
                            continue;
                        };
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
                                    resource,
                                );
                            });
                    }
                })?;
        }

        Ok(Self {
            address,
            thumbnail_address,
            sessions,
            thumbnails,
            resources,
        })
    }

    pub fn create_source_session(
        &self,
        recording: &RecordingRow,
        segments: &[RecordingSegmentRow],
        sources: &std::collections::BTreeMap<String, Vec<crate::storage::SegmentVideoStream>>,
    ) -> Result<LibraryPlaybackAssets, String> {
        let _admission = self.resources.admit()?;
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
                    fragments: vec![],
                    duration_ms,
                    video_streams: sources.get(&segment.id).cloned().unwrap_or_default(),
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
            let thumbnail_base = format!(
                "{}/playback/{}/{}/segments/{}",
                self.thumbnail_address, session_id, token, segment.id
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
                videos: sources
                    .get(&segment.id)
                    .into_iter()
                    .flatten()
                    .map(|video| crate::library::LibraryPlaybackVideoAsset {
                        track_id: video.track_id.clone(),
                        stream_index: video.stream_index,
                        url: format!("{base}/video/{}", video.stream_index),
                        thumbnails: thumbnail_offsets(duration_ms)
                            .into_iter()
                            .map(|offset_ms| LibraryPlaybackThumbnail {
                                offset_ms,
                                url: format!(
                                    "{thumbnail_base}/thumbnail/{offset_ms}.jpg/{}",
                                    video.stream_index
                                ),
                            })
                            .collect(),
                    })
                    .collect(),
                audio,
                segment_id: segment.id.clone(),
                thumbnails: thumbnail_offsets(duration_ms)
                    .into_iter()
                    .map(|offset_ms| LibraryPlaybackThumbnail {
                        offset_ms,
                        url: format!("{thumbnail_base}/thumbnail/{offset_ms}.jpg"),
                    })
                    .collect(),
                video_url: format!("{base}/video"),
            });
        }

        let mut response_runs = Vec::new();
        let bases = segments
            .iter()
            .zip(crate::storage::segment_playback_bases(segments))
            .map(|(segment, base)| (segment.id.as_str(), base))
            .collect::<HashMap<_, _>>();
        for group in runs::groups(segments, sources) {
            let first = group[0];
            let last = group.last().expect("run fragment");
            let offset = bases[first.id.as_str()];
            let duration = bases[last.id.as_str()]
                .saturating_add(last.duration_ms.unwrap_or(0))
                .saturating_sub(offset);
            let Some(mut data) = session_segments.get(&first.id).cloned() else {
                continue;
            };
            let Some(mut assets) = response_segments
                .iter()
                .find(|segment| segment.segment_id == first.id)
                .cloned()
            else {
                continue;
            };
            let id = format!("run-{}", first.id);
            data.fragments = group
                .iter()
                .map(|segment| (segment.file_path.clone(), segment.duration_ms.unwrap_or(0)))
                .collect();
            data.duration_ms = duration;
            let old_base = format!(
                "{}/playback/{}/{}/segments/{}",
                self.address, session_id, token, first.id
            );
            let new_base = format!(
                "{}/playback/{}/{}/runs/{}",
                self.address, session_id, token, id
            );
            assets.segment_id = id.clone();
            assets.video_url = assets.video_url.replace(&old_base, &new_base);
            for video in &mut assets.videos {
                video.url = video.url.replace(&old_base, &new_base);
            }
            for audio in &mut assets.audio {
                audio.url = audio.url.replace(&old_base, &new_base);
            }
            // Thumbnail URLs keep physical fragment identity and exact offsets.
            assets.thumbnails.clear();
            for video in &mut assets.videos {
                video.thumbnails.clear();
            }
            let mut local = 0;
            for fragment in &group {
                if let Some(part) = response_segments
                    .iter()
                    .find(|segment| segment.segment_id == fragment.id)
                {
                    assets
                        .thumbnails
                        .extend(part.thumbnails.iter().cloned().map(|mut thumbnail| {
                            thumbnail.offset_ms += local;
                            thumbnail
                        }));
                    for video in &mut assets.videos {
                        if let Some(part) = part
                            .videos
                            .iter()
                            .find(|part| part.track_id == video.track_id)
                        {
                            video.thumbnails.extend(part.thumbnails.iter().cloned().map(
                                |mut thumbnail| {
                                    thumbnail.offset_ms += local;
                                    thumbnail
                                },
                            ));
                        }
                    }
                }
                local += fragment.duration_ms.unwrap_or(0);
            }
            session_segments.insert(id.clone(), data);
            response_runs.push(crate::library::LibraryPlaybackRun {
                run_id: id,
                segment_ids: group.iter().map(|segment| segment.id.clone()).collect(),
                duration_ms: duration,
                playback_offset_ms: offset,
                assets,
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
            runs: response_runs,
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
        self.resources.cancel_session(session_id);
        Ok(())
    }
    pub(crate) fn attach_runtime_registry(
        &self,
        registry: crate::maintenance::RuntimeActivityRegistry,
    ) {
        self.resources.attach_registry(registry);
    }
    pub fn close_all_and_drain(&self, timeout: std::time::Duration) -> Result<(), String> {
        self.sessions
            .lock()
            .map_err(|_| "Playback sessions unavailable.")?
            .clear();
        self.thumbnails.close_all();
        self.resources.cancel_and_drain(timeout)
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
