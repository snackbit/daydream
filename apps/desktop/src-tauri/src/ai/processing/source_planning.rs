//! Expand visual work by durable source identity, without recomposing video.
use super::{
    super::*,
    planning::{visual_chunk_plans, VisualChunkPlan},
};

pub(in crate::ai) fn visual_source_chunk_plans(
    connection: &rusqlite::Connection,
    segments: &[RecordingSegmentRow],
    settings: &settings::AppSettings,
    redactions: &[RedactionRow],
    kind: &str,
) -> rusqlite::Result<Vec<VisualChunkPlan>> {
    let mut output = Vec::new();
    for plan in visual_chunk_plans(segments, settings, &[], kind) {
        let segment = segments
            .iter()
            .find(|segment| segment.id == plan.segment_id)
            .ok_or(rusqlite::Error::InvalidQuery)?;
        for source in storage::segment_video_streams(connection, segment)? {
            for (start, end) in super::planning::visible_ranges(
                plan.start_offset_ms,
                plan.end_offset_ms,
                redactions,
                Some(&source.track_id),
            ) {
                output.push(VisualChunkPlan {
                    segment_id: plan.segment_id.clone(),
                    start_offset_ms: start,
                    end_offset_ms: end,
                    track_id: source.source_id.as_ref().map(|_| source.track_id.clone()),
                });
            }
        }
    }
    Ok(output)
}

pub(in crate::ai) fn source_segment(
    connection: &rusqlite::Connection,
    chunk: &ProcessingChunkRow,
    mut segment: RecordingSegmentRow,
) -> rusqlite::Result<RecordingSegmentRow> {
    let sources = storage::segment_video_streams(connection, &segment)?;
    // Old invalid OCR jobs must not substitute a protected first stream for an
    // unredacted source plan. Only legacy composite work may omit provenance.
    if chunk.track_id.is_none() && sources.iter().any(|source| source.source_id.is_some()) {
        return Err(rusqlite::Error::InvalidQuery);
    }
    if let Some(track) = &chunk.track_id {
        let source = sources
            .into_iter()
            .find(|source| &source.track_id == track)
            .ok_or(rusqlite::Error::InvalidQuery)?;
        segment.video_stream_index = Some(source.stream_index);
    }
    Ok(segment)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn redacting_one_video_source_preserves_the_other_sources_processing_ranges() {
        let storage = storage::test_storage(std::env::temp_dir().join("synthetic-source-plans"));
        let file =
            std::env::temp_dir().join(format!("daydream-source-plans-{}.mkv", Uuid::now_v7()));
        fs::write(&file, b"synthetic media metadata fixture").unwrap();
        storage
            .with_connection(|connection| {
                let id = Uuid::now_v7().to_string();
                storage::create_recording(
                    connection,
                    &storage::CreateRecordingInput {
                        id: id.clone(),
                        active_scene_id: None,
                        created_at_ms: 1,
                        default_fps: 1.0,
                        notes: None,
                        output_root: None,
                        quality_label: "balanced-ocr".into(),
                        storage_dir: "/tmp/synthetic-source-plans".into(),
                    },
                )?;
                let segment = storage::create_recording_segment(
                    connection,
                    &storage::CreateRecordingSegmentInput {
                        id: Uuid::now_v7().to_string(),
                        recording_id: id.clone(),
                        segment_index: 1,
                        file_path: file.to_string_lossy().into_owned(),
                        started_at_ms: 1,
                    },
                )?;
                storage::finalize_recording_segment(
                    connection,
                    &storage::FinalizeRecordingSegmentInput {
                        id: segment.id.clone(),
                        ended_at_ms: 1001,
                        duration_ms: 1000,
                        video_stream_index: Some(0),
                        desktop_audio_stream_index: None,
                        microphone_audio_stream_index: None,
                        probe_json: None,
                        updated_at_ms: 1001,
                    },
                )?;
                let a = Uuid::now_v7().to_string();
                let b = Uuid::now_v7().to_string();
                let sources = [
                    storage::RecordingVideoSource {
                        track_id: a.clone(),
                        source_id: "a".into(),
                        label: "Synthetic A".into(),
                        width: 4,
                        height: 2,
                        config_json: "{}".into(),
                    },
                    storage::RecordingVideoSource {
                        track_id: b.clone(),
                        source_id: "b".into(),
                        label: "Synthetic B".into(),
                        width: 2,
                        height: 2,
                        config_json: "{}".into(),
                    },
                ];
                storage::save_recording_video_sources(connection, &id, &sources, 1)?;
                storage::save_segment_video_streams(connection, &segment.id, &sources, 1.0)?;
                let cut = storage::RedactionRow {
                    id: Uuid::now_v7().to_string(),
                    recording_id: id.clone(),
                    start_offset_ms: 100,
                    end_offset_ms: 300,
                    created_at_ms: 1,
                    reason: None,
                    track_ids_json: serde_json::to_string(std::slice::from_ref(&a)).unwrap(),
                };
                let plans = visual_source_chunk_plans(
                    connection,
                    &storage::list_recording_segments(connection, &id)?,
                    &settings::default_settings(),
                    &[cut],
                    "ocr",
                )?;
                let ranges = |track: &str| {
                    plans
                        .iter()
                        .filter(|plan| plan.track_id.as_deref() == Some(track))
                        .map(|plan| (plan.start_offset_ms, plan.end_offset_ms))
                        .collect::<Vec<_>>()
                };
                assert_eq!(ranges(&a), vec![(0, 100), (300, 1000)]);
                assert_eq!(ranges(&b), vec![(0, 1000)]);
                Ok(())
            })
            .unwrap();
        fs::remove_file(file).unwrap();
    }
}
