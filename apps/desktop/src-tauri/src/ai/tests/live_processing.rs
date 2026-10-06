use super::*;

#[test]
fn explicit_audio_reprocessing_preserves_other_ai_sections_and_replans_both_tracks() {
    let root = std::env::temp_dir().join(format!("daydream-audio-reprocess-{}", Uuid::now_v7()));
    fs::create_dir_all(&root).unwrap();
    let file = root.join("synthetic.mkv");
    fs::write(&file, b"synthetic media metadata fixture").unwrap();
    let storage = storage::test_storage(PathBuf::new());
    let fixture = storage
        .with_connection(|connection| {
            let fixture = test_visual_recording(connection);
            connection.execute(
                "UPDATE recording_segments SET file_path=?2 WHERE id=?1",
                rusqlite::params![fixture.segment_id, file.to_string_lossy()],
            )?;
            storage::finalize_recording_segment(
                connection,
                &storage::FinalizeRecordingSegmentInput {
                    id: fixture.segment_id.clone(),
                    duration_ms: 10_000,
                    ended_at_ms: 10_001,
                    updated_at_ms: 10_001,
                    video_stream_index: Some(0),
                    desktop_audio_stream_index: Some(1),
                    microphone_audio_stream_index: Some(2),
                    probe_json: None,
                },
            )?;
            storage::mark_recording_completed(connection, &fixture.recording_id, 10_001)?;
            storage::update_processing_chunk_status(
                connection,
                &fixture.ocr_chunk_id,
                "succeeded",
                1.0,
                None,
                11_000,
            )?;
            let job = storage::get_processing_chunk(connection, &fixture.ocr_chunk_id)?
                .unwrap()
                .job_id;
            storage::update_processing_job_status(connection, &job, "succeeded", None, 11_000)?;
            Ok(fixture)
        })
        .unwrap();
    let job = reprocess_recording_audio(&storage, &fixture.recording_id).unwrap();
    storage
        .with_connection(|connection| {
            assert_eq!(
                storage::get_processing_chunk(connection, &fixture.ocr_chunk_id)?
                    .unwrap()
                    .status,
                "succeeded"
            );
            assert_eq!(
                storage::list_processing_chunks(connection, &job.id)?
                    .iter()
                    .filter(|chunk| chunk.source_kind == "audio" && chunk.status == "queued")
                    .count(),
                2
            );
            Ok(())
        })
        .unwrap();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn live_work_waits_for_classification_but_a_later_protected_span_does_not_block_safe_history() {
    let storage = storage::test_storage(PathBuf::new());
    let chunk = storage
        .with_connection(|connection| {
            let (chunk, _) = test_transcription_chunk(connection);
            storage::mark_recording_started(connection, &chunk.recording_id, 1)?;
            storage::finalize_recording_segment(
                connection,
                &storage::FinalizeRecordingSegmentInput {
                    id: chunk.segment_id.clone().unwrap(),
                    duration_ms: 10_000,
                    ended_at_ms: 10_001,
                    updated_at_ms: 10_001,
                    video_stream_index: Some(0),
                    desktop_audio_stream_index: Some(1),
                    microphone_audio_stream_index: Some(2),
                    probe_json: None,
                },
            )?;
            connection.execute(
                "INSERT INTO recording_live_state VALUES(?1,1,0)",
                [&chunk.recording_id],
            )?;
            connection.execute(
                "UPDATE processing_chunks SET start_offset_ms=0,end_offset_ms=5000 WHERE id=?1",
                [&chunk.id],
            )?;
            storage::update_processing_chunk_status(connection, &chunk.id, "queued", 0.0, None, 2)
        })
        .unwrap();
    assert!(claim_next_processing_chunk(&storage, &chunk.job_id)
        .unwrap()
        .is_none());
    storage
        .with_connection(|connection| {
            connection.execute(
                "UPDATE recording_live_state SET classified_through_ms=10000 WHERE recording_id=?1",
                [&chunk.recording_id],
            )?;
            storage::create_recording_protection_span(
                connection,
                &storage::CreateRecordingProtectionSpanInput {
                    id: Uuid::now_v7().to_string(),
                    recording_id: chunk.recording_id.clone(),
                    start_offset_ms: 6000,
                    opened_at_ms: 6001,
                    action: "redact".into(),
                    rule_id: "synthetic".into(),
                    rule_label: "Synthetic protected window".into(),
                },
            )?;
            Ok(())
        })
        .unwrap();
    let claimed = claim_next_processing_chunk(&storage, &chunk.job_id)
        .unwrap()
        .unwrap();
    assert!(claimed.end_offset_ms.unwrap() <= 6000);
}

#[test]
fn interrupted_live_capture_masks_unclassified_tail_and_closes_job_input_atomically() {
    let storage = storage::test_storage(PathBuf::new());
    let chunk = storage
        .with_connection(|connection| {
            let (chunk, _) = test_transcription_chunk(connection);
            storage::mark_recording_started(connection, &chunk.recording_id, 1)?;
            storage::finalize_recording_segment(
                connection,
                &storage::FinalizeRecordingSegmentInput {
                    id: chunk.segment_id.clone().unwrap(),
                    duration_ms: 10_000,
                    ended_at_ms: 10_001,
                    updated_at_ms: 10_001,
                    video_stream_index: Some(0),
                    desktop_audio_stream_index: Some(1),
                    microphone_audio_stream_index: Some(2),
                    probe_json: None,
                },
            )?;
            connection.execute(
                "INSERT INTO recording_live_state VALUES(?1,1,8000)",
                [&chunk.recording_id],
            )?;
            storage::set_processing_job_runtime(connection, &chunk.job_id, true, "{}")?;
            storage::recover_interrupted_recordings(connection, 11_000)?;
            Ok(chunk)
        })
        .unwrap();
    storage
        .with_connection(|connection| {
            assert!(!storage::processing_job_input_open(
                connection,
                &chunk.job_id
            )?);
            let cuts = storage::list_redactions(connection, &chunk.recording_id)?;
            assert!(cuts
                .iter()
                .any(|cut| cut.start_offset_ms == 8000 && cut.end_offset_ms == 10_000));
            assert!(
                storage::list_recording_segments(connection, &chunk.recording_id)?
                    .iter()
                    .any(|segment| Some(&segment.id) == chunk.segment_id.as_ref()
                        && segment.state == "finalized")
            );
            Ok(())
        })
        .unwrap();
}
