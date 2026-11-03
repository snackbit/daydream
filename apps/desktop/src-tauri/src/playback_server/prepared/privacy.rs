//! Shared source identity and eligibility for prepared media and thumbnail reads.
use crate::storage::{self, AppStorage, RecordingTrackRow, RedactionRow, SegmentVideoStream};
use std::collections::{BTreeMap, HashMap, HashSet};

pub(in crate::playback_server) struct RecordingMasks {
    bases: HashMap<String, i64>,
    durations: HashMap<String, i64>,
    videos: BTreeMap<String, Vec<SegmentVideoStream>>,
    tracks: Vec<RecordingTrackRow>,
    cuts: Vec<RedactionRow>,
    eligible: HashSet<String>,
}
impl RecordingMasks {
    pub(in crate::playback_server) fn load(storage: &AppStorage, id: &str) -> Result<Self, String> {
        storage
            .with_read_connection(|c| {
                let readiness = crate::recording_privacy::protection_readiness(c, id)?;
                let segments = storage::list_recording_segments(c, id)?;
                let bases = storage::segment_playback_bases(&segments);
                Ok(Self {
                    bases: segments
                        .iter()
                        .zip(&bases)
                        .map(|(s, b)| (s.id.clone(), *b))
                        .collect(),
                    eligible: segments
                        .iter()
                        .zip(&bases)
                        .filter(|(s, b)| {
                            s.state == "finalized"
                                && readiness.allows(**b, **b + s.duration_ms.unwrap_or(0))
                        })
                        .map(|(s, _)| s.id.clone())
                        .collect(),
                    durations: segments
                        .iter()
                        .map(|s| (s.id.clone(), s.duration_ms.unwrap_or(0)))
                        .collect(),
                    videos: storage::recording_video_streams(c, id)?,
                    tracks: storage::list_recording_tracks(c, id)?,
                    cuts: storage::list_redactions(c, id)?,
                })
            })
            .map_err(|_| "Playback privacy coverage is not ready.".into())
    }
    fn targets(&self, id: &str, role: &str, index: i64) -> Vec<String> {
        let mut ids = vec![format!("{role}-{index}"), role.into()];
        if role == "video" {
            if let Some(source) = self
                .videos
                .get(id)
                .and_then(|v| v.iter().find(|v| v.stream_index == index))
            {
                ids.push(source.track_id.clone());
            }
        } else {
            ids.extend(
                self.tracks
                    .iter()
                    .filter(|t| t.kind == role)
                    .map(|t| t.id.clone()),
            );
        }
        ids
    }
    pub(in crate::playback_server) fn allows_thumbnail(
        &self,
        id: &str,
        index: i64,
        offset: i64,
    ) -> bool {
        if !self.eligible.contains(id) {
            return false;
        }
        let Some(base) = self.bases.get(id) else {
            return false;
        };
        if offset < 0 || offset >= self.durations.get(id).copied().unwrap_or(0) {
            return false;
        }
        !crate::recording_privacy::range_redacted(
            &self.cuts,
            &self.targets(id, "video", index),
            base + offset,
            base + offset + 1,
        )
    }
    pub(super) fn segment_eligible(&self, id: &str) -> bool {
        self.eligible.contains(id)
    }
    pub(super) fn allows_media(&self, id: &str, role: &str, index: i64) -> bool {
        let Some(base) = self.bases.get(id) else {
            return false;
        };
        self.segment_eligible(id)
            && !crate::recording_privacy::range_redacted(
                &self.cuts,
                &self.targets(id, role, index),
                *base,
                base + self.durations.get(id).copied().unwrap_or(0),
            )
    }
}
