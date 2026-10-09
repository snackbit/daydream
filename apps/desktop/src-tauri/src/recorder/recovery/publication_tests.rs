use super::*;
use crate::recorder::media_writer::{MediaSegmentProbe, MediaSegmentVideoProbe};
fn fixture(
    connection: &Connection,
) -> (
    storage::RecordingRow,
    RecoveredSegment,
    String,
    String,
    String,
) {
    let id = uuid::Uuid::now_v7().to_string();
    let segment_id = uuid::Uuid::now_v7().to_string();
    let track = uuid::Uuid::now_v7().to_string();
    let operation = uuid::Uuid::now_v7().to_string();
    let recovered = uuid::Uuid::now_v7().to_string();
    storage::create_recording(
        connection,
        &storage::CreateRecordingInput {
            id: id.clone(),
            active_scene_id: None,
            created_at_ms: 1000,
            default_fps: 1.0,
            notes: None,
            output_root: None,
            quality_label: "balanced-ocr".into(),
            storage_dir: "/tmp/synthetic-original".into(),
        },
    )
    .unwrap();
    connection
        .execute(
            "UPDATE recordings SET status='failed',started_at_ms=1000,ended_at_ms=2000 WHERE id=?1",
            [&id],
        )
        .unwrap();
    storage::save_recording_video_sources(
        connection,
        &id,
        &[storage::RecordingVideoSource {
            track_id: track.clone(),
            source_id: "synthetic-monitor".into(),
            label: "Synthetic display".into(),
            width: 64,
            height: 64,
            config_json: "{}".into(),
        }],
        1000,
    )
    .unwrap();
    let segment = storage::create_recording_segment(
        connection,
        &storage::CreateRecordingSegmentInput {
            id: segment_id.clone(),
            recording_id: id.clone(),
            segment_index: 1,
            file_path: "/tmp/synthetic-original/segment.mkv".into(),
            started_at_ms: 1000,
        },
    )
    .unwrap();
    connection
        .execute(
            "INSERT INTO recording_segment_videos VALUES(?1,?2,0,64,64,1)",
            params![segment_id, track],
        )
        .unwrap();
    storage::create_redaction(
        connection,
        &storage::CreateRedactionInput {
            id: uuid::Uuid::now_v7().to_string(),
            recording_id: id.clone(),
            start_offset_ms: 600,
            end_offset_ms: 900,
            track_ids_json: serde_json::to_string(&vec![&track]).unwrap(),
            reason: Some("synthetic privacy mask".into()),
            created_at_ms: 1000,
        },
    )
    .unwrap();
    recovery::start_operation(connection, &operation, &id, &recovered, 1000).unwrap();
    let video = MediaSegmentVideoProbe {
        codec_name: Some("h264".into()),
        duration_ms: Some(1000),
        estimated_frame_count: Some(1),
        height: Some(64),
        index: 0,
        pixel_format: None,
        width: Some(64),
    };
    let repaired = RecoveredSegment {
        original: segment,
        id: uuid::Uuid::now_v7().to_string(),
        path: "/tmp/synthetic-copy/segment.mkv".into(),
        probe: MediaSegmentProbe {
            videos: vec![video.clone()],
            audio: vec![],
            available: true,
            duration_ms: Some(1000),
            error: None,
            probe_backend: "synthetic".into(),
            streams: vec![],
            video: Some(video),
        },
        start_pts_ms: 500,
        end_pts_ms: 1500,
        stream_mappings: vec![(0, 0, 500, 1500)],
    };
    (
        storage::get_recording(connection, &id).unwrap(),
        repaired,
        operation,
        track,
        recovered,
    )
}
#[test]
fn recovered_copy_preserves_source_privacy_and_original_without_automatic_analysis() {
    let database = storage::test_storage("/tmp/synthetic-recovery-publish".into());
    database
        .with_connection_mut(|connection| {
            let (original, segment, operation, original_track, recovered) = fixture(connection);
            let coverage = publish(
                connection,
                &operation,
                &original,
                &recovered,
                Path::new("/tmp/synthetic-copy"),
                &[segment],
            )?;
            assert_eq!(coverage[0].source_start_offset_ms, 500);
            assert_eq!(coverage[0].streams[0].source_end_offset_ms, 1500);
            assert_eq!(storage::get_recording(connection, &original.id)?, original);
            assert_eq!(
                storage::get_recording(connection, &recovered)?.status,
                "recovered-partial"
            );
            let masks = storage::list_redactions(connection, &recovered)?;
            assert_eq!(
                (masks[0].start_offset_ms, masks[0].end_offset_ms),
                (600, 900)
            );
            assert!(!masks[0].track_ids_json.contains(&original_track));
            assert_eq!(storage::list_redactions(connection, &original.id)?.len(), 1);
            assert!(storage::list_processing_jobs(connection, Some(&recovered))?.is_empty());
            assert_eq!(
                storage::list_recording_segments(connection, &recovered)?[0].timeline_base_ms,
                Some(500)
            );
            Ok(())
        })
        .unwrap();
}
#[test]
fn publication_failure_exposes_no_recording_or_unprotected_copy() {
    let database = storage::test_storage("/tmp/synthetic-recovery-publication-failure".into());
    database.with_connection_mut(|connection| {
            let (original,segment,operation,_,recovered)=fixture(connection);
            connection.execute_batch("CREATE TRIGGER reject_recovery BEFORE UPDATE OF status ON recordings WHEN NEW.status='recovered-partial' BEGIN SELECT RAISE(FAIL,'synthetic publication failure'); END;")?;
            assert!(publish(connection,&operation,&original,&recovered,Path::new("/tmp/synthetic-copy"),&[segment]).is_err());
            assert!(storage::get_recording(connection,&recovered).is_err());
            assert_eq!(storage::get_recording(connection,&original.id)?,original);
            assert_eq!(storage::list_recordings(connection)?.len(),1);
            Ok(())
        }).unwrap();
}

#[test]
fn publication_failure_removes_unpublished_private_media_after_repair_finishes() {
    let _worker_slot = crate::recorder::media_worker::PROCESS_TEST.lock().unwrap();
    let root = std::env::temp_dir().join(format!(
        "daydream-publication-cleanup-{}",
        uuid::Uuid::now_v7()
    ));
    let database = storage::test_storage(root.clone());
    let (original, mut segment, operation, _, recovered) = database
        .with_connection(|connection| Ok(fixture(connection)))
        .unwrap();
    let output = root.join("recordings").join(&recovered);
    std::fs::create_dir_all(&output).unwrap();
    segment.path = output.join("synthetic.mkv");
    std::fs::write(&segment.path, b"synthetic repaired media").unwrap();
    database.with_connection(|connection| connection.execute_batch("CREATE TRIGGER reject_recovery BEFORE UPDATE OF status ON recordings WHEN NEW.status='recovered-partial' BEGIN SELECT RAISE(FAIL,'synthetic publication failure'); END;")).unwrap();
    assert!(super::super::publish_recovered(
        &database,
        &operation,
        &original,
        &recovered,
        &output,
        &[segment]
    )
    .is_err());
    assert!(
        !output.exists(),
        "failed publication must not orphan private copies"
    );
    assert!(database
        .with_connection(|connection| recovery::pending_outputs(connection, Some(&original.id)))
        .unwrap()
        .is_empty());
    assert_eq!(
        database
            .with_connection(|connection| storage::get_recording(connection, &original.id))
            .unwrap(),
        original
    );
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn unknown_privacy_source_prevents_publication() {
    let database = storage::test_storage("/tmp/synthetic-recovery-invalid-source".into());
    database.with_connection_mut(|connection| { let (original,segment,operation,_,recovered)=fixture(connection); connection.execute("UPDATE redactions SET track_ids_json='[\"unknown-private-source\"]' WHERE recording_id=?1",[&original.id])?; assert!(publish(connection,&operation,&original,&recovered,Path::new("/tmp/synthetic-copy"),&[segment]).is_err()); assert_eq!(storage::list_recordings(connection)?.len(),1); Ok(()) }).unwrap();
}
