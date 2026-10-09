use super::RecoveredSegment;
use crate::storage::{
    self,
    recovery::{self, RecordingRecoveryCoverage, RecordingRecoveryStreamCoverage},
};
use rusqlite::{params, Connection, OptionalExtension};
use std::{collections::HashMap, path::Path};

pub(super) fn publish(
    connection: &mut Connection,
    operation_id: &str,
    original: &storage::RecordingRow,
    recovered_id: &str,
    output_root: &Path,
    segments: &[RecoveredSegment],
) -> rusqlite::Result<Vec<RecordingRecoveryCoverage>> {
    let transaction = connection.transaction()?;
    // Recheck the authoritative original and current privacy state after repair.
    let current = storage::get_recording(&transaction, &original.id)?;
    if current.status != "failed" {
        return Err(rusqlite::Error::InvalidQuery);
    }
    let timestamp = chrono::Utc::now().timestamp_millis();
    let original_start = original.started_at_ms.unwrap_or(original.created_at_ms);
    let mut coverage = Vec::new();
    for segment in segments {
        if !segment.probe.available
            || segment.end_pts_ms <= segment.start_pts_ms
            || segment.start_pts_ms < 0
            || segment.probe.videos.is_empty()
            || segment
                .stream_mappings
                .iter()
                .any(|(original, output, start, end)| {
                    original != output
                        || start < &segment.start_pts_ms
                        || end > &segment.end_pts_ms
                        || end <= start
                })
        {
            return Err(rusqlite::Error::InvalidQuery);
        }
        if segment.probe.videos.iter().any(|video| {
            video.width.is_none_or(|width| width == 0)
                || video.height.is_none_or(|height| height == 0)
        }) {
            return Err(rusqlite::Error::InvalidQuery);
        }
        let base = segment.original.timeline_base_ms.unwrap_or(
            segment
                .original
                .started_at_ms
                .saturating_sub(original_start)
                .max(0),
        );
        let start = base
            .checked_add(segment.start_pts_ms)
            .ok_or(rusqlite::Error::InvalidQuery)?;
        let end = base
            .checked_add(segment.end_pts_ms)
            .ok_or(rusqlite::Error::InvalidQuery)?;
        coverage.push(RecordingRecoveryCoverage {
            original_segment_id: segment.original.id.clone(),
            recovered_segment_id: segment.id.clone(),
            source_start_offset_ms: start,
            source_end_offset_ms: end,
            recovered_start_offset_ms: start,
            recovered_end_offset_ms: end,
            streams: segment
                .stream_mappings
                .iter()
                .map(
                    |(original, output, start, end)| RecordingRecoveryStreamCoverage {
                        original_stream_index: *original,
                        recovered_stream_index: *output,
                        source_start_offset_ms: base + start,
                        source_end_offset_ms: base + end,
                    },
                )
                .collect(),
        });
    }
    storage::create_recording(
        &transaction,
        &storage::CreateRecordingInput {
            id: recovered_id.into(),
            active_scene_id: original.active_scene_id.clone(),
            created_at_ms: timestamp,
            default_fps: original.default_fps,
            notes: Some(format!("Recovered partial copy of {}", original.id)),
            output_root: None,
            quality_label: original.quality_label.clone(),
            storage_dir: output_root.to_string_lossy().into(),
        },
    )?;
    let last_end = coverage
        .iter()
        .map(|range| range.recovered_end_offset_ms)
        .max()
        .ok_or(rusqlite::Error::InvalidQuery)?;
    transaction.execute("UPDATE recordings SET status='recovered-partial',started_at_ms=?2,ended_at_ms=?3,title=?4 WHERE id=?1",params![recovered_id,original_start,original_start+last_end,original.title.as_ref().map(|title|format!("{title} (recovered)"))])?;
    let mut track_map: HashMap<String, String> = HashMap::new();
    let original_tracks = storage::list_recording_tracks(&transaction, &original.id)?;
    for track in original_tracks
        .iter()
        .filter(|track| track.kind != "screen")
    {
        let id = uuid::Uuid::now_v7().to_string();
        storage::upsert_recording_track(
            &transaction,
            &storage::UpsertRecordingTrackInput {
                id: id.clone(),
                recording_id: recovered_id.into(),
                kind: track.kind.clone(),
                label: track.label.clone(),
                source_id: track.source_id.clone(),
                path: None,
                codec: track.codec.clone(),
                created_at_ms: timestamp,
            },
        )?;
        track_map.insert(track.id.clone(), id);
    }
    for (segment, range) in segments.iter().zip(&coverage) {
        let original_streams = storage::segment_video_streams(&transaction, &segment.original)?;
        if original_streams.len() != segment.probe.videos.len() {
            return Err(rusqlite::Error::InvalidQuery);
        }
        let mut sources = Vec::new();
        for stream in &original_streams {
            let video = segment
                .probe
                .videos
                .iter()
                .find(|video| video.index == stream.stream_index)
                .ok_or(rusqlite::Error::InvalidQuery)?;
            let (Some(width), Some(height)) = (video.width, video.height) else {
                return Err(rusqlite::Error::InvalidQuery);
            };
            if stream.width.is_some_and(|original| original != width)
                || stream.height.is_some_and(|original| original != height)
            {
                return Err(rusqlite::Error::InvalidQuery);
            }
            let track_id = track_map
                .entry(stream.track_id.clone())
                .or_insert_with(|| uuid::Uuid::now_v7().to_string())
                .clone();
            let legacy_id = format!("video-{}", stream.stream_index);
            if track_map
                .get(&legacy_id)
                .is_some_and(|existing| existing != &track_id)
            {
                return Err(rusqlite::Error::InvalidQuery);
            }
            track_map.insert(legacy_id, track_id.clone());
            let config: Option<String> = transaction
                .query_row(
                    "SELECT config_json FROM recording_video_sources WHERE track_id=?1",
                    [&stream.track_id],
                    |row| row.get(0),
                )
                .optional()?;
            let source = storage::RecordingVideoSource {
                track_id: track_id.clone(),
                source_id: stream
                    .source_id
                    .clone()
                    .unwrap_or_else(|| format!("legacy-{}", stream.track_id)),
                label: stream.label.clone(),
                width,
                height,
                config_json: config.unwrap_or_else(|| "{}".into()),
            };
            let exists: bool = transaction.query_row(
                "SELECT EXISTS(SELECT 1 FROM recording_video_sources WHERE track_id=?1)",
                [&track_id],
                |row| row.get(0),
            )?;
            if !exists {
                storage::save_recording_video_sources(
                    &transaction,
                    recovered_id,
                    std::slice::from_ref(&source),
                    timestamp,
                )?;
            }
            sources.push((source, stream.stream_index));
        }
        let desktop = segment.original.desktop_audio_stream_index;
        let microphone = segment.original.microphone_audio_stream_index;
        if segment.probe.audio.len()
            != usize::from(desktop.is_some()) + usize::from(microphone.is_some())
            || [desktop, microphone]
                .into_iter()
                .flatten()
                .any(|index| !segment.probe.audio.iter().any(|audio| audio.index == index))
        {
            return Err(rusqlite::Error::InvalidQuery);
        }
        storage::create_recording_segment(
            &transaction,
            &storage::CreateRecordingSegmentInput {
                id: segment.id.clone(),
                recording_id: recovered_id.into(),
                segment_index: segment.original.segment_index,
                file_path: segment.path.to_string_lossy().into(),
                started_at_ms: original_start + range.recovered_start_offset_ms,
            },
        )?;
        storage::finalize_recording_segment(
            &transaction,
            &storage::FinalizeRecordingSegmentInput {
                id: segment.id.clone(),
                duration_ms: range.recovered_end_offset_ms - range.recovered_start_offset_ms,
                ended_at_ms: original_start + range.recovered_end_offset_ms,
                updated_at_ms: timestamp.max(original_start + range.recovered_end_offset_ms),
                desktop_audio_stream_index: desktop,
                microphone_audio_stream_index: microphone,
                video_stream_index: segment.probe.videos.first().map(|video| video.index),
                probe_json: Some(
                    serde_json::to_string(&segment.probe)
                        .map_err(|_| rusqlite::Error::InvalidQuery)?,
                ),
            },
        )?;
        transaction.execute("UPDATE recording_segments SET timeline_base_ms=?2,timeline_basis_version=1 WHERE id=?1",params![segment.id,range.recovered_start_offset_ms])?;
        for (source, index) in sources {
            transaction.execute("INSERT INTO recording_segment_videos(segment_id,track_id,stream_index,width,height,fps) VALUES(?1,?2,?3,?4,?5,?6)",params![segment.id,source.track_id,index,source.width,source.height,original.default_fps])?;
        }
    }
    for redaction in storage::list_redactions(&transaction, &original.id)? {
        let targets: Vec<String> = serde_json::from_str(&redaction.track_ids_json)
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let targets = targets
            .into_iter()
            .map(|id| {
                if let Some(track) = original_tracks
                    .iter()
                    .find(|track| track.id == id && track.kind != "screen")
                {
                    let aliases = segments
                        .iter()
                        .filter_map(|segment| match track.kind.as_str() {
                            "desktop-audio" => segment
                                .original
                                .desktop_audio_stream_index
                                .map(|index| format!("desktop-audio-{index}")),
                            "microphone" => segment
                                .original
                                .microphone_audio_stream_index
                                .map(|index| format!("microphone-{index}")),
                            _ => None,
                        })
                        .collect::<Vec<_>>();
                    return if aliases.is_empty() {
                        Err(rusqlite::Error::InvalidQuery)
                    } else {
                        Ok(aliases)
                    };
                }
                if let Some(mapped) = track_map.get(&id) {
                    return Ok(vec![mapped.clone()]);
                }
                let supported = crate::timeline_lanes::ACTIVITY_TRACK_IDS.contains(&id.as_str())
                    || segments.iter().any(|segment| {
                        segment
                            .original
                            .desktop_audio_stream_index
                            .is_some_and(|index| id == format!("desktop-audio-{index}"))
                            || segment
                                .original
                                .microphone_audio_stream_index
                                .is_some_and(|index| id == format!("microphone-{index}"))
                    });
                if supported {
                    Ok(vec![id])
                } else {
                    Err(rusqlite::Error::InvalidQuery)
                }
            })
            .collect::<rusqlite::Result<Vec<_>>>()?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        let targets = serde_json::to_string(&targets).map_err(|_| rusqlite::Error::InvalidQuery)?;
        for (start, end) in recovery::intersect_coverage(
            redaction.start_offset_ms,
            redaction.end_offset_ms,
            &coverage,
        ) {
            add_mask(
                &transaction,
                recovered_id,
                start,
                end,
                &targets,
                redaction.reason.clone(),
                timestamp,
            )?;
        }
    }
    for span in storage::list_recording_protection_spans(&transaction, &original.id)? {
        for (start, end) in recovery::intersect_coverage(
            span.start_offset_ms,
            span.end_offset_ms.unwrap_or(last_end),
            &coverage,
        ) {
            add_mask(
                &transaction,
                recovered_id,
                start,
                end,
                "[]",
                Some("Recording protection preserved from original recovery".into()),
                timestamp,
            )?;
        }
    }
    let watermark:Option<(bool,i64)>=transaction.query_row("SELECT classifier_required,classified_through_ms FROM recording_live_state WHERE recording_id=?1",[&original.id],|row|Ok((row.get(0)?,row.get(1)?))).optional()?;
    if let Some((true, watermark)) = watermark {
        for (start, end) in recovery::intersect_coverage(watermark, last_end, &coverage) {
            add_mask(
                &transaction,
                recovered_id,
                start,
                end,
                "[]",
                Some("Recording protection: unclassified recovered media".into()),
                timestamp,
            )?;
        }
    }
    recovery::finish_operation(&transaction, operation_id, recovered_id, &coverage)?;
    transaction.commit()?;
    Ok(coverage)
}
fn add_mask(
    connection: &Connection,
    id: &str,
    start: i64,
    end: i64,
    targets: &str,
    reason: Option<String>,
    timestamp: i64,
) -> rusqlite::Result<()> {
    storage::create_redaction(
        connection,
        &storage::CreateRedactionInput {
            id: uuid::Uuid::now_v7().to_string(),
            recording_id: id.into(),
            start_offset_ms: start,
            end_offset_ms: end,
            track_ids_json: targets.into(),
            created_at_ms: timestamp,
            reason,
        },
    )?;
    Ok(())
}

#[cfg(test)]
#[path = "publication_tests.rs"]
mod tests;
