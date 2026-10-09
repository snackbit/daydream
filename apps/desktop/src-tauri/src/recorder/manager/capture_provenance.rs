//! Persist immutable source identities before a fragment can be interrupted.
use super::{ActiveRecording, ActiveSegment};
use crate::storage::{self, UpsertRecordingTrackInput};
use rusqlite::{params, Connection};

pub(super) fn persist_segment_provenance(
    connection: &Connection,
    active: &ActiveRecording,
    segment: &ActiveSegment,
    id: &str,
    timestamp: u64,
) -> rusqlite::Result<()> {
    connection.execute("UPDATE recording_segments SET video_stream_index=?2,desktop_audio_stream_index=?3,microphone_audio_stream_index=?4 WHERE id=?1",params![id,segment.stream_indexes.video_stream_index,segment.stream_indexes.desktop_audio_stream_index,segment.stream_indexes.microphone_audio_stream_index])?;
    for (kind, label, index, source) in [
        (
            "desktop-audio",
            "Desktop audio",
            segment.stream_indexes.desktop_audio_stream_index,
            active.desktop_audio_source_id.as_deref(),
        ),
        (
            "microphone",
            "Microphone",
            segment.stream_indexes.microphone_audio_stream_index,
            active.microphone_audio_source_id.as_deref(),
        ),
    ] {
        match (index, source) {
            (None, None) => continue,
            (Some(_), Some(source)) => {
                let existing =
                    storage::get_recording_track_by_kind(connection, &active.recording_id, kind)?;
                if existing
                    .as_ref()
                    .and_then(|track| track.source_id.as_deref())
                    .is_some_and(|original| original != source)
                {
                    return Err(rusqlite::Error::InvalidQuery);
                }
                storage::upsert_recording_track(
                    connection,
                    &UpsertRecordingTrackInput {
                        id: existing
                            .map(|track| track.id)
                            .unwrap_or_else(|| uuid::Uuid::now_v7().to_string()),
                        recording_id: active.recording_id.clone(),
                        kind: kind.into(),
                        label: label.into(),
                        source_id: Some(source.into()),
                        path: None,
                        codec: None,
                        created_at_ms: timestamp as i64,
                    },
                )?;
            }
            _ => return Err(rusqlite::Error::InvalidQuery),
        }
    }
    Ok(())
}
