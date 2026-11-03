use std::collections::HashMap;

#[derive(Clone)]
pub(super) struct PlaybackSession {
    pub(super) segments: HashMap<String, PlaybackSegment>,
    pub(super) token: String,
    pub(super) cleanup_dir: Option<std::path::PathBuf>,
    pub(super) cleanup_guard: Option<CleanupGuard>,
    pub(super) prepared_files: HashMap<String, std::path::PathBuf>,
    pub(super) prepared_leases:
        HashMap<String, std::sync::Arc<super::prepared::lifecycle::FileLease>>,
}

#[derive(Clone)]
pub(super) struct CleanupGuard {
    pub(super) masks: std::sync::Arc<super::prepared::privacy::RecordingMasks>,
    pub(super) storage: crate::storage::AppStorage,
    pub(super) recording_id: String,
    pub(super) fingerprint: String,
    pub(super) revision: Option<String>,
}

pub(super) struct PlaybackRequestSnapshot {
    pub(super) segment: PlaybackSegment,
    pub(super) prepared_path: Option<std::path::PathBuf>,
    guard: Option<CleanupGuard>,
    _prepared_lease: Option<std::sync::Arc<super::prepared::lifecycle::FileLease>>,
}
impl PlaybackRequestSnapshot {
    pub(super) fn authorized(&self) -> bool {
        self.guard.as_ref().is_none_or(CleanupGuard::valid)
    }
    pub(super) fn thumbnail_namespace(&self) -> &str {
        self.guard
            .as_ref()
            .map(|guard| guard.fingerprint.as_str())
            .unwrap_or("source")
    }
    pub(super) fn allows_thumbnail(&self, id: &str, index: i64, offset: i64) -> bool {
        self.guard
            .as_ref()
            .is_none_or(|guard| guard.masks.allows_thumbnail(id, index, offset))
    }
}
impl CleanupGuard {
    fn valid(&self) -> bool {
        self.storage.with_read_connection(|c|{
            if !crate::recording_privacy::protection_ready(c,&self.recording_id)? {return Ok(false);}
            if let Some(revision)=&self.revision {
                let ready:bool=c.query_row("SELECT EXISTS(SELECT 1 FROM audio_cleanup_generations WHERE id=?1 AND status='ready')",[revision],|r|r.get(0))?;
                if !ready {return Ok(false);}
            }
            Ok(crate::library::audio_cleanup::privacy_fingerprint(c,&self.recording_id)?==self.fingerprint)
        }).unwrap_or(false)
    }
}

#[derive(Clone)]
pub(super) struct PlaybackSegment {
    pub(super) fragments: Vec<(String, i64)>,
    pub(super) microphone_fragments: Vec<(String, i64)>,
    pub(super) duration_ms: i64,
    pub(super) video_streams: Vec<crate::storage::SegmentVideoStream>,
    pub(super) desktop_audio_stream_index: Option<i64>,
    pub(super) file_path: String,
    pub(super) microphone_audio_stream_index: Option<i64>,
    pub(super) playback_fps: f64,
    pub(super) video_stream_index: Option<i64>,
}

impl PlaybackSession {
    pub(super) fn new(segments: HashMap<String, PlaybackSegment>, token: String) -> Self {
        Self {
            segments,
            token,
            cleanup_dir: None,
            cleanup_guard: None,
            prepared_files: HashMap::new(),
            prepared_leases: HashMap::new(),
        }
    }

    pub(super) fn request_snapshot(
        &self,
        token: &str,
        segment_id: &str,
        asset_key: Option<&str>,
    ) -> Option<PlaybackRequestSnapshot> {
        if self.token != token {
            return None;
        }
        Some(PlaybackRequestSnapshot {
            segment: self.segments.get(segment_id)?.clone(),
            prepared_path: asset_key.and_then(|key| {
                self.prepared_files
                    .get(&format!("{segment_id}/{key}"))
                    .cloned()
            }),
            guard: self.cleanup_guard.clone(),
            _prepared_lease: asset_key.and_then(|key| {
                self.prepared_leases
                    .get(&format!("{segment_id}/{key}"))
                    .cloned()
            }),
        })
    }
}
