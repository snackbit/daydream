use super::*;

#[test]
fn review_windows_return_complete_visual_chunk_frames_without_changing_work() {
    let storage = storage::test_storage(PathBuf::new());
    let fixture = storage.with_connection(|connection| {
        let fixture = test_visual_recording(connection);
        connection.execute("UPDATE processing_chunks SET status='running' WHERE id=?1", [&fixture.ocr_chunk_id])?;
        let outside = create_test_visual_chunk(connection, &fixture, "ocr", "queued")?;
        connection.execute("UPDATE processing_chunks SET start_offset_ms=20000,end_offset_ms=30000 WHERE id=?1", [&outside])?;
        for (chunk_id, timestamp, status) in [
            (&fixture.ocr_chunk_id, 0, "succeeded"),
            (&fixture.ocr_chunk_id, 2000, "running"),
            (&fixture.ocr_chunk_id, 5000, "succeeded"),
            (&fixture.ocr_chunk_id, 8000, "queued"),
            (&outside, 20000, "queued"),
        ] {
            storage::upsert_visual_frame_processing_state(connection, &storage::CreateVisualFrameProcessingStateInput {
                id: Uuid::now_v7().to_string(), recording_id: fixture.recording_id.clone(),
                job_id: fixture.job_id.clone(), chunk_id: chunk_id.clone(),
                source_kind: "ocr".into(), segment_id: Some(fixture.segment_id.clone()),
                frame_timestamp_ms: timestamp, source_frame_ref: format!("synthetic:{timestamp}"),
                region_x: 0, region_y: 0, region_width: 64, region_height: 36,
                status: status.into(), progress: if status == "succeeded" { 1.0 } else { 0.0 },
                error_code: None, output_count: 0, created_at_ms: 1, updated_at_ms: 1,
            })?;
        }
        Ok(fixture)
    }).unwrap();
    let before = processing_state_for_recording(&storage, &fixture.recording_id).unwrap();
    for (viewport, review) in [
        (Some((2500, 3500)), None),
        (Some((6500, 7000)), Some((0, 1000))),
        (Some((12000, 13000)), Some((8500, 9500))),
    ] {
        let state = results::processing_state_with_review(
            &storage,
            &fixture.recording_id,
            viewport,
            review,
        )
        .unwrap();
        assert_eq!(
            state.chunks.len(),
            2,
            "unrelated chunks stay out of the read"
        );
        assert_eq!(
            state
                .visual_frames
                .iter()
                .map(|frame| (frame.frame_timestamp_ms, frame.status.as_str()))
                .collect::<Vec<_>>(),
            vec![
                (0, "succeeded"),
                (2000, "running"),
                (5000, "succeeded"),
                (8000, "queued")
            ]
        );
        assert!(state
            .visual_frames
            .iter()
            .all(|frame| frame.chunk_id == fixture.ocr_chunk_id));
        assert!(state
            .chunks
            .iter()
            .all(|chunk| chunk.start_offset_ms == Some(0) && chunk.end_offset_ms == Some(10000)));
        assert_eq!(
            serde_json::to_value(&state.job).unwrap(),
            serde_json::to_value(&before.job).unwrap()
        );
    }
    let empty = results::processing_state_with_review(
        &storage,
        &fixture.recording_id,
        Some((10000, 20000)),
        None,
    )
    .unwrap();
    assert!(empty.chunks.is_empty());
    assert!(empty.visual_frames.is_empty());
    let after = processing_state_for_recording(&storage, &fixture.recording_id).unwrap();
    assert_eq!(
        serde_json::to_value(after).unwrap(),
        serde_json::to_value(before).unwrap(),
        "seeking is read-only"
    );
}
