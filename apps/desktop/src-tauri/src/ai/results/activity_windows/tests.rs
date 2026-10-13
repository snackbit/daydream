use super::*;

#[test]
fn detached_review_pages_include_focus_mouse_and_keyboard_without_gap_events() {
    let storage = storage::test_storage(PathBuf::new());
    let recording = storage.with_connection(|connection| {
        let (chunk, _) = crate::ai::tests::test_transcription_chunk(connection);
        for offset in [100, 2000, 5100] {
            for kind in ["focused-window", "open-windows"] {
                storage::create_capture_event(connection, &storage::CreateCaptureEventInput { id: Uuid::now_v7().to_string(), recording_id: chunk.recording_id.clone(), kind: kind.into(), offset_ms: offset, occurred_at_ms: offset, payload_json: r#"{"applicationName":"Synthetic editor","title":"Synthetic window","windows":[]}"#.into() })?;
            }
            for source in ["mouse", "keyboard"] {
                storage::create_activity_interval(connection, &storage::CreateActivityIntervalInput { id: Uuid::now_v7().to_string(), recording_id: chunk.recording_id.clone(), job_id: chunk.job_id.clone(), chunk_id: Some(chunk.id.clone()), source_kind: source.into(), state: "active".into(), start_offset_ms: offset, end_offset_ms: offset + 100, score: Some(1.0), regions_json: None })?;
            }
        }
        Ok(chunk.recording_id)
    }).unwrap();
    for (timeline, review) in [
        ((0, 1000), (5000, 6000)),
        ((5000, 6000), (0, 1000)),
        ((0, 6000), (0, 1000)),
    ] {
        let result =
            activity_for_recording_with_review(&storage, &recording, Some(timeline), Some(review))
                .unwrap();
        let offsets = if timeline == (0, 6000) {
            vec![100, 2000, 5100]
        } else {
            vec![100, 5100]
        };
        assert_eq!(
            result
                .events
                .iter()
                .map(|event| event.offset_ms)
                .collect::<Vec<_>>(),
            offsets
        );
        assert_eq!(
            result
                .window_snapshots
                .iter()
                .map(|event| event.offset_ms)
                .collect::<Vec<_>>(),
            offsets
        );
        assert_eq!(result.intervals.len(), offsets.len() * 2);
    }
    let timeline =
        activity_for_recording_with_review(&storage, &recording, Some((0, 1000)), None).unwrap();
    assert_eq!(timeline.events.len(), 1);
    let all =
        activity_for_recording_with_review(&storage, &recording, None, Some((0, 1000))).unwrap();
    assert_eq!(all.events.len(), 3);
    storage
        .with_connection(|connection| {
            storage::create_redaction(
                connection,
                &storage::CreateRedactionInput {
                    id: Uuid::now_v7().to_string(),
                    recording_id: recording.clone(),
                    start_offset_ms: 5000,
                    end_offset_ms: 6000,
                    track_ids_json: "[]".into(),
                    reason: None,
                    created_at_ms: 1,
                },
            )
            .map(|_| ())
        })
        .unwrap();
    let hidden = activity_for_recording_with_review(
        &storage,
        &recording,
        Some((0, 1000)),
        Some((5000, 6000)),
    )
    .unwrap();
    assert_eq!(hidden.events.len(), 1);
    assert_eq!(hidden.intervals.len(), 2);
}
