use super::super::media_writer::{FragmentEvent, MediaWriterStreamIndexes};
use super::*;

#[test]
fn fragment_closure_waits_for_its_successor_across_polls_and_drains_on_stop() {
    let mut queue = super::fragments::FragmentPersistenceQueue::default();
    let event = |index, timestamp_ms, opened| FragmentEvent {
        path: PathBuf::from(format!("{index:04}.mkv")),
        timestamp_ms,
        opened,
    };
    assert_eq!(queue.ready(vec![event(1, 0, true)]).len(), 1);
    assert!(queue.ready(vec![event(1, 25_000, false)]).is_empty());
    assert!(queue.ready(vec![]).is_empty());
    let ready = queue.ready(vec![event(2, 30_000, true)]);
    assert_eq!(ready.len(), 2);
    assert!(!ready[0].opened);
    assert!(ready[1].opened);
    // A delayed older close is already safe; the current close stays pending.
    assert_eq!(
        queue
            .ready(vec![event(1, 25_000, false), event(2, 55_000, false)])
            .len(),
        1
    );
    let pending = queue.finish();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].path, PathBuf::from("0002.mkv"));
    assert!(queue.finish().is_empty());
}

#[test]
fn rollover_persists_durable_bases_before_predecessor_finalization() {
    for origin_base in [Some(0), Some(65_000), None] {
        assert_rollover_bases(origin_base);
    }
}

fn assert_rollover_bases(origin_base: Option<i64>) {
    let root = std::env::temp_dir().join(format!("daydream-fragment-bases-{}", Uuid::now_v7()));
    std::fs::create_dir(&root).unwrap();
    let storage = storage::test_storage(root.clone());
    let manager = RecorderManager::new(root.clone());
    let recording_id = Uuid::now_v7().to_string();
    let segments_dir = root.join("segments");
    let path = segments_dir.join("0001.mkv");
    let first = storage
        .with_connection(|connection| {
            storage::create_recording(
                connection,
                &CreateRecordingInput {
                    id: recording_id.clone(),
                    active_scene_id: None,
                    created_at_ms: 1_000,
                    default_fps: 0.2,
                    notes: None,
                    output_root: None,
                    quality_label: "balanced-ocr".into(),
                    storage_dir: root.to_string_lossy().into(),
                },
            )?;
            storage::create_recording_segment(
                connection,
                &CreateRecordingSegmentInput {
                    id: Uuid::now_v7().to_string(),
                    recording_id: recording_id.clone(),
                    segment_index: 1,
                    file_path: path.to_string_lossy().into(),
                    started_at_ms: 1_000,
                },
            )
        })
        .unwrap();
    let segment = ActiveSegment {
        capture_started_at_ms: 1_000,
        file_path: path,
        id: first.id.clone(),
        runtime: None,
        started_at_ms: 1_000,
        stream_indexes: MediaWriterStreamIndexes {
            video_stream_index: 0,
            desktop_audio_stream_index: None,
            microphone_audio_stream_index: None,
        },
        timing: None,
    };
    let active = ActiveRecording {
        capture_run_id: "synthetic-run".into(),
        video_sources: vec![],
        active_segment: Some(segment.clone()),
        canvas_height: 18,
        canvas_width: 32,
        desktop_audio_source_id: None,
        fps: 0.2,
        media_writer: "synthetic".into(),
        microphone_audio_source_id: None,
        recording_id: recording_id.clone(),
        scene_id: "synthetic-scene".into(),
        segments_dir: segments_dir.clone(),
        next_segment_index: 2,
        video_bitrate_kbps: 64,
    };
    storage
        .with_connection(|connection| {
            connection.execute(
                "UPDATE recording_segments SET capture_run_id=?2, timeline_base_ms=?3,
                 timeline_basis_version=CASE WHEN ?3 IS NOT NULL THEN 1 ELSE NULL END WHERE id=?1",
                rusqlite::params![first.id, active.capture_run_id, origin_base],
            )
        })
        .unwrap();
    let events =
        [(2, 31_279), (3, 61_279), (4, 91_279)].map(|(index, timestamp_ms)| FragmentEvent {
            path: segments_dir.join(format!("{index:04}.mkv")),
            timestamp_ms,
            opened: true,
        });
    manager
        .save_fragment_events(&storage, &active, &segment, &events)
        .unwrap();
    let rows = storage
        .with_connection(|connection| storage::list_recording_segments(connection, &recording_id))
        .unwrap();
    assert!(rows.iter().all(|row| row.state == "recording"));
    assert_eq!(
        rows.iter()
            .map(|row| row.timeline_base_ms)
            .collect::<Vec<_>>(),
        [0, 30_279, 60_279, 90_279].map(|delta| origin_base.map(|base| base + delta))
    );
    assert!(rows
        .iter()
        .all(|row| row.timeline_basis_version == origin_base.map(|_| 1)));
    // Replay must keep the same durable offsets, without waiting for closure.
    manager
        .save_fragment_events(&storage, &active, &segment, &events)
        .unwrap();
    assert_eq!(
        storage
            .with_connection(|connection| storage::list_recording_segments(
                connection,
                &recording_id
            ))
            .unwrap(),
        rows
    );
    if origin_base.is_some() {
        // A conflicting durable base rejects the entire batch, including any
        // earlier newly opened fragment. It cannot silently rewrite provenance.
        storage
            .with_connection(|connection| {
                connection.execute(
                    "UPDATE recording_segments SET timeline_base_ms=timeline_base_ms+1 WHERE id=?1",
                    [&rows[1].id],
                )
            })
            .unwrap();
        let before = storage
            .with_connection(|connection| {
                storage::list_recording_segments(connection, &recording_id)
            })
            .unwrap();
        let future = FragmentEvent {
            path: segments_dir.join("0005.mkv"),
            timestamp_ms: 121_279,
            opened: true,
        };
        assert!(manager
            .save_fragment_events(&storage, &active, &segment, &[future, events[0].clone()])
            .is_err());
        assert_eq!(
            storage
                .with_connection(|connection| storage::list_recording_segments(
                    connection,
                    &recording_id
                ))
                .unwrap(),
            before
        );
    }
    std::fs::remove_dir_all(root).unwrap();
}
