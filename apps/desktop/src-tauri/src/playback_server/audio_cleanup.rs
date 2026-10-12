//! Cleaned microphone assets and privacy-scoped preview sessions.
use super::*;

impl PlaybackServer {
    /// Validate derived inputs before allocation and retire any partially prepared session.
    pub(crate) fn create_recording_source_session(
        &self,
        recording: &RecordingRow,
        segments: &[RecordingSegmentRow],
        sources: &std::collections::BTreeMap<String, Vec<crate::storage::SegmentVideoStream>>,
        storage: crate::storage::AppStorage,
    ) -> Result<LibraryPlaybackAssets, String> {
        let cleanup = storage
            .with_connection(|c| crate::library::audio_cleanup::active_manifest(c, &recording.id))
            .map_err(|_| "Cleanup is unavailable. Retry cleanup or turn it off.")?;
        let assets = self.create_source_session(recording, segments, sources)?;
        if let Some(cleanup) = cleanup {
            if let Err(error) = self.use_cleaned_audio(
                &assets.session_id,
                segments,
                &cleanup,
                storage,
                &recording.id,
            ) {
                self.close_session(&assets.session_id)?;
                return Err(error);
            }
        }
        Ok(assets)
    }

    #[cfg(test)]
    pub(crate) fn session_count(&self) -> usize {
        self.sessions.lock().unwrap().len()
    }

    pub(crate) fn use_cleaned_audio(
        &self,
        session_id: &str,
        segments: &[RecordingSegmentRow],
        manifest: &crate::library::audio_cleanup::CleanupManifest,
        storage: crate::storage::AppStorage,
        recording_id: &str,
    ) -> Result<(), String> {
        let fingerprint = storage
            .with_connection(|c| {
                crate::library::audio_cleanup::privacy_fingerprint(c, recording_id)
            })
            .map_err(|_| "Cleanup sources are unavailable.")?;
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| "Playback session is unavailable.")?;
        let session = sessions
            .get_mut(session_id)
            .ok_or("Playback session is unavailable.")?;
        session.cleanup_guard = Some(CleanupGuard {
            storage,
            recording_id: recording_id.into(),
            fingerprint,
            revision: Some(manifest.revision.clone()),
        });
        for (id, part) in &mut session.segments {
            let members: Vec<_> =
                if let Some(segment) = segments.iter().find(|segment| &segment.id == id) {
                    vec![segment]
                } else {
                    segments
                        .iter()
                        .filter(|segment| {
                            part.fragments
                                .iter()
                                .any(|(path, _)| path == &segment.file_path)
                        })
                        .collect()
                };
            part.microphone_fragments = members
                .iter()
                .flat_map(|segment| {
                    manifest
                        .blocks
                        .iter()
                        .filter(|block| block.segment_id == segment.id)
                        .map(|block| (block.path.clone(), block.duration_ms))
                })
                .collect();
            if part.microphone_audio_stream_index.is_some()
                && part
                    .microphone_fragments
                    .iter()
                    .map(|(_, duration)| duration)
                    .sum::<i64>()
                    != part.duration_ms
            {
                return Err("Cleaned playback audio is incomplete.".into());
            }
        }
        Ok(())
    }

    pub(crate) fn create_cleanup_preview(
        &self,
        original: &Path,
        cleaned: &Path,
        duration: i64,
        dir: std::path::PathBuf,
        storage: crate::storage::AppStorage,
        recording_id: &str,
    ) -> Result<(String, String, String), String> {
        let fingerprint = storage
            .with_connection(|c| {
                crate::library::audio_cleanup::privacy_fingerprint(c, recording_id)
            })
            .map_err(|_| "Preview sources are unavailable.")?;
        let id = Uuid::now_v7().to_string();
        let token = Uuid::now_v7().to_string();
        let segment = |path: &Path| PlaybackSegment {
            fragments: vec![],
            microphone_fragments: vec![],
            duration_ms: duration,
            video_streams: vec![],
            desktop_audio_stream_index: None,
            file_path: path.to_string_lossy().into_owned(),
            microphone_audio_stream_index: Some(0),
            playback_fps: 1.0,
            video_stream_index: None,
        };
        let mut parts = HashMap::new();
        parts.insert("original".into(), segment(original));
        parts.insert("cleaned".into(), segment(cleaned));
        let mut session = PlaybackSession::new(parts, token.clone());
        session.cleanup_dir = Some(dir);
        session.cleanup_guard = Some(CleanupGuard {
            storage,
            recording_id: recording_id.into(),
            fingerprint,
            revision: None,
        });
        self.sessions
            .lock()
            .map_err(|_| "Preview playback is unavailable.")?
            .insert(id.clone(), session);
        let base = format!("{}/playback/{}/{}/segments", self.address, id, token);
        Ok((
            id,
            format!("{base}/original/audio/microphone-0"),
            format!("{base}/cleaned/audio/microphone-0"),
        ))
    }
    pub(crate) fn close_cleanup_recording(&self, recording_id: &str) -> Result<(), String> {
        let ids: Vec<_> = self
            .sessions
            .lock()
            .map_err(|_| "Playback sessions unavailable.")?
            .iter()
            .filter(|(_, session)| {
                session
                    .cleanup_guard
                    .as_ref()
                    .is_some_and(|guard| guard.recording_id == recording_id)
            })
            .map(|(id, _)| id.clone())
            .collect();
        for id in ids {
            self.close_session(&id)?;
            self.resources
                .drain_session(&id, std::time::Duration::from_secs(5))?;
        }
        Ok(())
    }
}
