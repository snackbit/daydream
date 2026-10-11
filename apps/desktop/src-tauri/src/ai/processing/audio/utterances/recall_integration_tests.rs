use super::*;

#[test]
fn published_cross_fragment_speech_reaches_studio_search_and_summary_inputs() {
    let storage = storage::test_storage(PathBuf::new());
    let path = std::env::temp_dir().join(format!("daydream-cross-speech-{}.mkv", Uuid::now_v7()));
    std::fs::write(&path, b"synthetic").unwrap();
    let (chunk, track) = storage
        .with_connection(|c| {
            let (chunk, track) = crate::ai::tests::test_transcription_chunk(c);
            let first = chunk.segment_id.as_ref().unwrap();
            c.execute(
                "UPDATE recording_segments SET file_path=?1 WHERE id=?2",
                rusqlite::params![path.to_string_lossy(), first],
            )?;
            storage::finalize_recording_segment(
                c,
                &storage::FinalizeRecordingSegmentInput {
                    id: first.clone(),
                    duration_ms: 10000,
                    ended_at_ms: 10001,
                    updated_at_ms: 10001,
                    video_stream_index: Some(0),
                    desktop_audio_stream_index: Some(1),
                    microphone_audio_stream_index: Some(2),
                    probe_json: None,
                },
            )?;
            let tail = Uuid::now_v7().to_string();
            storage::create_recording_segment(
                c,
                &storage::CreateRecordingSegmentInput {
                    id: tail.clone(),
                    recording_id: chunk.recording_id.clone(),
                    segment_index: 2,
                    file_path: path.to_string_lossy().into(),
                    started_at_ms: 10001,
                },
            )?;
            storage::finalize_recording_segment(
                c,
                &storage::FinalizeRecordingSegmentInput {
                    id: tail.clone(),
                    duration_ms: 10000,
                    ended_at_ms: 20001,
                    updated_at_ms: 20001,
                    video_stream_index: Some(0),
                    desktop_audio_stream_index: Some(1),
                    microphone_audio_stream_index: Some(2),
                    probe_json: None,
                },
            )?;
            storage::create_processing_chunk(
                c,
                &storage::CreateProcessingChunkInput {
                    id: Uuid::now_v7().to_string(),
                    recording_id: chunk.recording_id.clone(),
                    job_id: chunk.job_id.clone(),
                    segment_id: Some(tail),
                    source_kind: "audio".into(),
                    track_id: Some(track.id.clone()),
                    start_offset_ms: Some(10000),
                    end_offset_ms: Some(20000),
                    status: "running".into(),
                    attempt_count: 1,
                    progress: 0.0,
                    created_at_ms: 1,
                    updated_at_ms: 1,
                    error_code: None,
                    result_code: None,
                },
            )?;
            Ok((chunk, track))
        })
        .unwrap();
    let unit = storage::SpeechUnit {
        id: Uuid::now_v7().to_string(),
        job_id: chunk.job_id.clone(),
        track_id: track.id.clone(),
        owner_chunk_id: chunk.id.clone(),
        start_ms: 9000,
        end_ms: 12000,
        submitted_start_ms: 9000,
        submitted_end_ms: 12000,
        right_open: false,
        status: "queued".into(),
        fingerprint: "synthetic-cross-speech".into(),
        output_json: None,
    };
    storage
        .with_connection(|c| {
            storage::enable_speech_strategy(c, &unit.job_id)?;
            storage::save_speech_unit(c, &unit)
        })
        .unwrap();
    let settings = settings::load_app_settings(&storage).unwrap();
    publication::publish(
        &storage,
        &chunk,
        &track,
        &settings,
        &unit,
        crate::ai::tests::test_verbose_transcription(
            0.0,
            3.0,
            "Synthetic crossing utterance",
            vec![],
        ),
    )
    .unwrap();
    let studio = crate::ai::processing_state_for_recording(&storage, &chunk.recording_id).unwrap();
    assert_eq!(studio.transcript.tracks[0].segments[0].end_offset_ms, 12000);
    storage
        .with_connection(|c| {
            storage::refresh_evidence(c)?;
            let sources = storage::list_evidence_range(c, 0, i64::MAX)?;
            assert_eq!(sources.len(), 1);
            assert_eq!(sources[0].text, "Synthetic crossing utterance");
            assert_eq!(
                sources[0].citation.segment_id,
                chunk.segment_id.as_ref().unwrap().clone()
            );
            assert_eq!(sources[0].citation.end_offset_ms, 12000);
            assert_eq!(
                c.query_row(
                    "SELECT count(*) FROM recall_fts WHERE recall_fts MATCH 'crossing'",
                    [],
                    |r| r.get::<_, i64>(0)
                )?,
                1
            );
            Ok(())
        })
        .unwrap();
    std::fs::remove_file(path).unwrap();
}
