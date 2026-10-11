use crate::storage::{RecordingSegmentRow, RedactionRow};
use rusqlite::{params, Connection};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::ops::Bound::Excluded;

pub(super) struct Source {
    pub(super) id: String,
    pub(super) segment_id: String,
    pub(super) track: String,
    pub(super) start: i64,
    pub(super) end: i64,
    pub(super) text: String,
    pub(super) provenance: String,
    pub(super) kind: String,
}

/// Build once per recording; each source visits only its covered media fragments.
pub(super) struct EvidenceMedia<'a> {
    segments: &'a BTreeMap<&'a str, &'a RecordingSegmentRow>,
    starts: BTreeMap<i64, Vec<&'a RecordingSegmentRow>>,
    redactions: &'a [RedactionRow],
}

impl<'a> EvidenceMedia<'a> {
    pub(super) fn new(
        segments: &'a BTreeMap<&'a str, &'a RecordingSegmentRow>,
        redactions: &'a [RedactionRow],
    ) -> Self {
        let mut starts: BTreeMap<i64, Vec<&RecordingSegmentRow>> = BTreeMap::new();
        for segment in segments.values() {
            if let Some(base) = segment.timeline_base_ms {
                starts.entry(base).or_default().push(segment);
            }
        }
        Self {
            segments,
            starts,
            redactions,
        }
    }

    fn span(&self, source: &mut Source) -> Option<(i64, i64)> {
        let anchor = *self.segments.get(source.segment_id.as_str())?;
        let base = anchor.timeline_base_ms?;
        let limit = base.checked_add(anchor.duration_ms?)?;
        if source.start < base || source.start >= limit || source.end <= source.start {
            return None;
        }
        let audio = matches!(source.kind.as_str(), "microphone" | "desktop-audio");
        if !audio && source.end > limit {
            return None;
        }
        let mut parts = vec![anchor];
        if audio {
            if self.starts.get(&base)?.len() != 1 {
                return None;
            }
            for (_, candidates) in self.starts.range((Excluded(base), Excluded(source.end))) {
                if candidates.len() != 1 {
                    return None;
                }
                parts.push(candidates[0]);
            }
        }
        let mut provenance: serde_json::Value = serde_json::from_str(&source.provenance).ok()?;
        let processing_track = provenance
            .get("processingTrackId")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned);
        let mut cursor = source.start;
        let mut absolute_start = None;
        let mut absolute_end = None;
        let mut previous: Option<&RecordingSegmentRow> = None;
        let mut support = Vec::new();
        for segment in parts {
            let base = segment.timeline_base_ms?;
            let end = base.checked_add(segment.duration_ms?)?.min(source.end);
            if cursor < base
                || end <= cursor
                || previous.is_some_and(|part| {
                    base != cursor || part.ended_at_ms != Some(segment.started_at_ms)
                })
            {
                return None;
            }
            let track = if audio {
                crate::recording_privacy::source_track_id(segment, &source.kind)?
            } else {
                source.track.clone()
            };
            if previous.is_none() && track != source.track {
                return None;
            }
            let mut ids = vec![track.clone()];
            if let Some(id) = &processing_track {
                ids.push(id.clone());
            }
            if crate::recording_privacy::range_redacted(self.redactions, &ids, cursor, end) {
                return None;
            }
            let start_utc = segment.started_at_ms.checked_add(cursor - base)?;
            let end_utc = segment.started_at_ms.checked_add(end - base)?;
            absolute_start.get_or_insert(start_utc);
            absolute_end = Some(end_utc);
            support.push(serde_json::json!({"segmentId":segment.id,"sourceTrackId":track,"startOffsetMs":cursor,"endOffsetMs":end,"absoluteStartMs":start_utc,"absoluteEndMs":end_utc}));
            cursor = end;
            previous = Some(segment);
        }
        if cursor != source.end {
            return None;
        }
        let (start, end) = (absolute_start?, absolute_end?);
        if end <= start {
            return None;
        }
        if support.len() > 1 {
            provenance
                .as_object_mut()?
                .insert("mediaFragments".into(), serde_json::Value::Array(support));
            source.provenance = provenance.to_string();
        }
        Some((start, end))
    }
}

pub(super) fn insert_source(
    connection: &Connection,
    recording: &str,
    media: &EvidenceMedia<'_>,
    mut source: Source,
) -> rusqlite::Result<()> {
    let Some((start, end)) = media.span(&mut source) else {
        return Ok(());
    };
    let source_id = source
        .id
        .split_once(':')
        .map(|(_, id)| id)
        .ok_or(rusqlite::Error::InvalidQuery)?;
    let fragments = crate::storage::recall_fragments::text_fragments(&source.text);
    for (ordinal, text) in fragments.iter().enumerate() {
        let id = if fragments.len() == 1 {
            source.id.clone()
        } else {
            format!("{}#fragment:{ordinal}", source.id)
        };
        let mut hasher = Sha256::new();
        for value in [
            crate::storage::recall_fragments::FRAGMENT_VERSION,
            &id,
            &source.segment_id,
            &source.track,
            &source.kind,
            text,
            &source.provenance,
        ] {
            hasher.update((value.len() as u64).to_le_bytes());
            hasher.update(value.as_bytes());
        }
        for value in [source.start, source.end, start, end] {
            hasher.update(value.to_le_bytes());
        }
        let revision = format!("{:x}", hasher.finalize());
        connection.execute(
            "INSERT INTO recall_next VALUES(?1,?2,?3,?4,?5,?13,?6,?7,?8,?9,?10,?11,?12)",
            params![
                id,
                recording,
                source.segment_id,
                source.kind,
                source.track,
                source.start,
                source.end,
                start,
                end,
                revision,
                text,
                source.provenance,
                source_id
            ],
        )?;
    }
    Ok(())
}
