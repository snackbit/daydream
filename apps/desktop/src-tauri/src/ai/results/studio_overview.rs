use super::super::*;
use crate::ai::types::StudioTimelineBucket;
use rusqlite::params;

/// Overview reads aggregate eligible metadata in SQLite; no transcript text or
/// words, captured titles, OCR text, or frame images cross this boundary.
pub(crate) fn read(
    storage: &AppStorage,
    id: &str,
    start: i64,
    end: i64,
    bin: i64,
) -> Result<Vec<StudioTimelineBucket>, AiError> {
    if start < 0 || end <= start || bin <= 0 || (end - start) / bin > 512 {
        return Err(ai_error(
            "ai_processing_selection_invalid",
            "Choose a valid overview resolution.",
        ));
    }
    storage.with_connection(|connection| {
        let segments = storage::list_recording_segments(connection, id)?;
        let media = crate::timeline_lanes::media_track_ids_from_segments(&segments);
        let cuts = storage::list_redactions(connection, id)?.into_iter().map(|cut| serde_json::json!({
            "start":cut.start_offset_ms,"end":cut.end_offset_ms,"targets":crate::redaction_targets::expanded_redaction_track_ids(&cut.track_ids_json,&media)
        })).collect::<Vec<_>>();
        let cuts_json = serde_json::to_string(&cuts).map_err(|_|rusqlite::Error::InvalidQuery)?;
        let mut buckets = Vec::new();
        let mut speech = connection.prepare("SELECT t.source_role,t.start_offset_ms/?4,MIN(t.start_offset_ms),MAX(t.end_offset_ms),COUNT(*) FROM published_transcript_segments t JOIN recording_segments s ON s.id=t.segment_id
          WHERE t.recording_id=?1 AND t.start_offset_ms<?3 AND t.end_offset_ms>?2
          AND NOT EXISTS(SELECT 1 FROM json_each(?5) r WHERE json_extract(r.value,'$.start')<t.end_offset_ms AND json_extract(r.value,'$.end')>t.start_offset_ms AND EXISTS(SELECT 1 FROM json_each(json_extract(r.value,'$.targets')) j WHERE j.value=t.track_id OR j.value=t.source_role||'-'||CASE WHEN t.source_role='microphone' THEN s.microphone_audio_stream_index ELSE s.desktop_audio_stream_index END))
          GROUP BY t.source_role,t.start_offset_ms/?4 ORDER BY MIN(t.start_offset_ms)")?;
        for row in speech.query_map(params![id,start,end,bin,cuts_json], |row| Ok((row.get::<_,String>(0)?,row.get::<_,i64>(2)?,row.get::<_,i64>(3)?,row.get::<_,i64>(4)?)))? {
            let (role, from, to, count) = row?;
            if let Some(track_id) = super::transcript::library_audio_track_id(&segments, &role) {
                buckets.push(StudioTimelineBucket { track_id, kind: "speech".into(), start_offset_ms: from, end_offset_ms: to, count });
            }
        }
        let mut activity = connection.prepare("SELECT a.source_kind,a.start_offset_ms/?4,MIN(a.start_offset_ms),MAX(a.end_offset_ms),COUNT(*) FROM frame_activity_intervals a WHERE a.recording_id=?1 AND a.state='active' AND a.start_offset_ms<?3 AND a.end_offset_ms>?2
          AND NOT EXISTS(SELECT 1 FROM json_each(?5) r WHERE json_extract(r.value,'$.start')<a.end_offset_ms AND json_extract(r.value,'$.end')>a.start_offset_ms AND EXISTS(SELECT 1 FROM json_each(json_extract(r.value,'$.targets')) j WHERE j.value='activity:'||a.source_kind))
          GROUP BY a.source_kind,a.start_offset_ms/?4")?;
        for row in activity.query_map(params![id,start,end,bin,cuts_json], |row| Ok((row.get::<_,String>(0)?,row.get::<_,i64>(2)?,row.get::<_,i64>(3)?,row.get::<_,i64>(4)?)))? {
            let (role, from, to, count) = row?;
            if matches!(role.as_str(), "mouse" | "keyboard") { buckets.push(StudioTimelineBucket { track_id: format!("activity:{role}"), kind: "activity".into(), start_offset_ms: from, end_offset_ms: to, count }); }
        }
        let mut focus = connection.prepare("SELECT e.offset_ms/?4,MIN(e.offset_ms),MAX(e.offset_ms)+1,COUNT(*) FROM capture_events e WHERE e.recording_id=?1 AND e.kind='focused-window' AND e.offset_ms>=?2 AND e.offset_ms<?3
          AND NOT EXISTS(SELECT 1 FROM json_each(?5) r WHERE json_extract(r.value,'$.start')<=e.offset_ms AND json_extract(r.value,'$.end')>e.offset_ms AND EXISTS(SELECT 1 FROM json_each(json_extract(r.value,'$.targets')) j WHERE j.value='activity:focus'))
          GROUP BY e.offset_ms/?4")?;
        for row in focus.query_map(params![id,start,end,bin,cuts_json], |row| Ok(StudioTimelineBucket { track_id: "activity:focus".into(), kind: "activity".into(), start_offset_ms: row.get(1)?, end_offset_ms: row.get(2)?, count: row.get(3)? }))? { buckets.push(row?); }
        Ok(buckets)
    }).map_err(storage_error).and_then(|mut buckets| {
        // Existing visibility owns committed replacements and context lineage.
        // Screen sampling is bounded independently from high-volume speech/input.
        let rows = super::visual::visible_visual_rows(storage, id, false)?;
        let mut scenes = BTreeMap::<(String, i64), StudioTimelineBucket>::new();
        for row in rows.vlm_observations {
            let to = row.confirmed_end_ms.unwrap_or(row.frame_timestamp_ms + 1);
            if row.frame_timestamp_ms >= end || to <= start { continue; }
            let track_id = row.source_frame_ref.split(":stream:").nth(1).and_then(|source| source.split(':').next()).unwrap_or("video-0").to_owned();
            let key = (track_id.clone(), row.frame_timestamp_ms / bin);
            let bucket = scenes.entry(key).or_insert(StudioTimelineBucket { track_id, kind: "observation".into(), start_offset_ms: row.frame_timestamp_ms, end_offset_ms: to, count: 0 });
            bucket.count += 1; bucket.end_offset_ms = bucket.end_offset_ms.max(to);
        }
        buckets.extend(scenes.into_values());
        buckets.sort_by_key(|bucket| bucket.start_offset_ms);
        Ok(buckets)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn studio_overview_counts_only_eligible_sources_and_detail_reads_keep_separate_review_windows()
    {
        let storage = storage::test_storage(PathBuf::new());
        let (chunk, track) = storage
            .with_connection(|connection| {
                let (chunk, track) = crate::ai::tests::test_transcription_chunk(connection);
                storage::finalize_recording_segment(
                    connection,
                    &storage::FinalizeRecordingSegmentInput {
                        id: chunk.segment_id.clone().unwrap(),
                        duration_ms: 30_000,
                        ended_at_ms: 30_001,
                        updated_at_ms: 30_001,
                        video_stream_index: None,
                        desktop_audio_stream_index: None,
                        microphone_audio_stream_index: Some(2),
                        probe_json: None,
                    },
                )?;
                storage::mark_recording_completed(connection, &chunk.recording_id, 30_001)?;
                storage::create_processing_chunk(
                    connection,
                    &storage::CreateProcessingChunkInput {
                        id: Uuid::now_v7().to_string(),
                        job_id: chunk.job_id.clone(),
                        recording_id: chunk.recording_id.clone(),
                        segment_id: chunk.segment_id.clone(),
                        track_id: chunk.track_id.clone(),
                        source_kind: "audio".into(),
                        start_offset_ms: Some(20_000),
                        end_offset_ms: Some(30_000),
                        status: "queued".into(),
                        progress: 0.,
                        attempt_count: 0,
                        error_code: None,
                        result_code: None,
                        created_at_ms: 1,
                        updated_at_ms: 1,
                    },
                )?;
                for start in [500, 6000, 9000] {
                    storage::create_transcript_segment(
                        connection,
                        &storage::CreateTranscriptSegmentInput {
                            id: Uuid::now_v7().to_string(),
                            recording_id: chunk.recording_id.clone(),
                            segment_id: chunk.segment_id.clone().unwrap(),
                            track_id: track.id.clone(),
                            source_role: "microphone".into(),
                            start_offset_ms: start,
                            end_offset_ms: start + 500,
                            provider: "local-whisper-cpp".into(),
                            model: None,
                            text: "Synthetic private utterance".into(),
                            confidence: None,
                            job_id: chunk.job_id.clone(),
                            chunk_id: chunk.id.clone(),
                            created_at_ms: 1,
                        },
                    )?;
                }
                Ok((chunk, track))
            })
            .unwrap();
        let overview = read(&storage, &chunk.recording_id, 0, 10_000, 5000).unwrap();
        assert_eq!(overview.len(), 2);
        assert_eq!(overview.iter().map(|bucket| bucket.count).sum::<i64>(), 3);
        assert!(overview
            .iter()
            .all(|bucket| bucket.track_id == "microphone-2" && bucket.kind == "speech"));
        assert!(!serde_json::to_string(&overview)
            .unwrap()
            .contains("Synthetic private"));
        let detail = super::super::processing::processing_state_with_review(
            &storage,
            &chunk.recording_id,
            Some((6000, 6500)),
            Some((500, 1000)),
        )
        .unwrap();
        let full =
            super::super::processing::processing_state_for_recording(&storage, &chunk.recording_id)
                .unwrap();
        assert_eq!(full.chunks.len(), 2);
        assert_eq!(detail.chunks.len(), 1);
        assert_eq!(
            serde_json::to_value(&detail.job).unwrap(),
            serde_json::to_value(&full.job).unwrap()
        );
        assert!(detail
            .transcription_tracks
            .iter()
            .flat_map(|track| &track.chunks)
            .all(|chunk| chunk.start_offset_ms.unwrap_or(0) < 6500));
        assert_eq!(
            detail
                .transcript
                .tracks
                .iter()
                .map(|track| track.segments.len())
                .sum::<usize>(),
            2
        );
        storage
            .with_connection(|connection| {
                storage::create_redaction(
                    connection,
                    &storage::CreateRedactionInput {
                        id: Uuid::now_v7().to_string(),
                        recording_id: chunk.recording_id.clone(),
                        start_offset_ms: 5500,
                        end_offset_ms: 7000,
                        track_ids_json: serde_json::to_string(&[track.id]).unwrap(),
                        reason: None,
                        created_at_ms: 10_002,
                    },
                )
            })
            .unwrap();
        let masked = read(&storage, &chunk.recording_id, 0, 10_000, 5000).unwrap();
        assert_eq!(masked.iter().map(|bucket| bucket.count).sum::<i64>(), 2);
        let detail = super::super::processing::processing_state_with_review(
            &storage,
            &chunk.recording_id,
            Some((6000, 6500)),
            Some((500, 1000)),
        )
        .unwrap();
        assert_eq!(
            detail
                .transcript
                .tracks
                .iter()
                .map(|track| track.segments.len())
                .sum::<usize>(),
            1
        );
        assert!(read(&storage, &chunk.recording_id, 0, 10_000, 0).is_err());
        assert!(read(&storage, &chunk.recording_id, 0, 10_000, 1).is_err());
    }
}
