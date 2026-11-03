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

mod audio_cleanup;
#[cfg(test)]
#[path = "tests/day_slice.rs"]
mod day_slice_tests;
mod encoders;
mod ffmpeg_args;
mod http;
pub(crate) mod prepared;
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
pub(crate) use runs::concat_manifest as cleanup_concat_manifest;
use thumbnails::{thumbnail_offsets, ThumbnailService, THUMBNAIL_WORKER_COUNT};

#[derive(Clone)]
pub struct PlaybackServer {
    address: String,
    thumbnail_address: String,
    sessions: Arc<Mutex<HashMap<String, PlaybackSession>>>,
    thumbnails: ThumbnailService,
    resources: resources::PlaybackResources,
}

mod sessions;
use sessions::{CleanupGuard, PlaybackSegment, PlaybackSession};

impl PlaybackServer {
    pub(crate) fn decoder_warmup_url(&self) -> String {
        format!("{}/decoder-warmup.mp4", self.address)
    }

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

    #[cfg(test)]
    pub fn create_source_session(
        &self,
        recording: &RecordingRow,
        segments: &[RecordingSegmentRow],
        sources: &std::collections::BTreeMap<String, Vec<crate::storage::SegmentVideoStream>>,
    ) -> Result<LibraryPlaybackAssets, String> {
        self.create_source_session_with_options(recording, segments, sources, false, true)
    }

    fn create_source_session_with_options(
        &self,
        recording: &RecordingRow,
        segments: &[RecordingSegmentRow],
        sources: &std::collections::BTreeMap<String, Vec<crate::storage::SegmentVideoStream>>,
        lazy_thumbnails: bool,
        include_runs: bool,
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
                    microphone_fragments: vec![],
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
                thumbnail_base_url: lazy_thumbnails.then(|| format!("{thumbnail_base}/thumbnail")),
                can_seek_in_stream: false,
                videos: sources
                    .get(&segment.id)
                    .into_iter()
                    .flatten()
                    .map(|video| crate::library::LibraryPlaybackVideoAsset {
                        thumbnail_base_url: lazy_thumbnails
                            .then(|| format!("{thumbnail_base}/thumbnail")),
                        track_id: video.track_id.clone(),
                        stream_index: video.stream_index,
                        url: format!("{base}/video/{}", video.stream_index),
                        thumbnails: (if lazy_thumbnails {
                            vec![]
                        } else {
                            thumbnail_offsets(duration_ms)
                        })
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
                thumbnails: (if lazy_thumbnails {
                    vec![]
                } else {
                    thumbnail_offsets(duration_ms)
                })
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
        for group in if include_runs {
            runs::groups(segments, sources)
        } else {
            vec![]
        } {
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
            assets.thumbnail_base_url = None;
            assets.video_url = assets.video_url.replace(&old_base, &new_base);
            for video in &mut assets.videos {
                video.thumbnail_base_url = None;
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
            can_seek_in_stream: false,
            runs: response_runs,
            error: None,
            recording_id: recording.id.clone(),
            segments: response_segments,
            session_id,
        })
    }

    pub fn close_session(&self, session_id: &str) -> Result<(), String> {
        let session = self
            .sessions
            .lock()
            .map_err(|_error| "Unable to lock playback sessions.".to_string())?
            .remove(session_id);
        self.thumbnails.close_session(session_id);
        self.resources.cancel_session(session_id);
        if let Some(dir) = session.and_then(|session| session.cleanup_dir) {
            // The readers own child completion; unlinking previews does not reuse paths.
            let _ = std::fs::remove_dir_all(dir);
        }
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
