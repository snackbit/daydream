use super::*;
#[test]
fn review_frame_navigation_searches_past_redacted_neighborhoods() {
    let root = std::env::temp_dir().join(format!("daydream-frame-navigation-{}", Uuid::now_v7()));
    fs::create_dir_all(&root).unwrap();
    let video = root.join("synthetic.mkv");
    let output = Command::new(ffmpeg::binary_path())
        .args([
            "-hide_banner",
            "-v",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "color=c=white:s=64x32:r=1:d=30",
            "-c:v",
            "ffv1",
        ])
        .arg(&video)
        .output()
        .unwrap();
    assert!(output.status.success());
    let storage = storage::test_storage(PathBuf::new());
    let id = Uuid::now_v7().to_string();
    let segment = Uuid::now_v7().to_string();
    storage
        .with_connection(|connection| {
            storage::create_recording(
                connection,
                &storage::CreateRecordingInput {
                    id: id.clone(),
                    active_scene_id: None,
                    created_at_ms: 1,
                    default_fps: 10.0,
                    notes: None,
                    output_root: None,
                    quality_label: "balanced-ocr".into(),
                    storage_dir: path_to_string(&root),
                },
            )?;
            storage::create_recording_segment(
                connection,
                &storage::CreateRecordingSegmentInput {
                    id: segment.clone(),
                    recording_id: id.clone(),
                    segment_index: 1,
                    started_at_ms: 1,
                    file_path: path_to_string(&video),
                },
            )?;
            storage::finalize_recording_segment(
                connection,
                &storage::FinalizeRecordingSegmentInput {
                    id: segment.clone(),
                    duration_ms: 30_000,
                    ended_at_ms: 30_001,
                    updated_at_ms: 30_001,
                    video_stream_index: Some(0),
                    desktop_audio_stream_index: None,
                    microphone_audio_stream_index: None,
                    probe_json: None,
                },
            )?;
            storage::mark_recording_completed(connection, &id, 30_001)?;
            Ok(())
        })
        .unwrap();
    storage
        .with_connection(|connection| {
            storage::create_redaction(
                connection,
                &storage::CreateRedactionInput {
                    id: Uuid::now_v7().to_string(),
                    recording_id: id.clone(),
                    start_offset_ms: 5000,
                    end_offset_ms: 26000,
                    reason: None,
                    created_at_ms: 30001,
                    track_ids_json: r#"["video-0"]"#.into(),
                },
            )?;
            Ok(())
        })
        .unwrap();
    for (time, direction, expected) in [
        (4000, 1, 26000),
        (26000, -1, 4000),
        (10000, 1, 26000),
        (23000, -1, 4000),
    ] {
        assert_eq!(
            adjacent(
                &storage,
                AdjacentFrameInput {
                    recording_id: id.clone(),
                    source_track_id: None,
                    timestamp_ms: time,
                    direction
                }
            )
            .unwrap(),
            Some(expected),
            "Continue within the current segment before moving to another"
        );
    }
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn frame_navigation_uses_real_pts_and_segment_offset() {
    let points=timestamps(b"[Parsed_showinfo_0] n: 0 pts: 0 pts_time:0.020\n[Parsed_showinfo_0] n: 1 pts: 1 pts_time:0.523\n",2500);
    assert_eq!(points, vec![2520, 3023]);
}
#[test]
fn real_media_steps_use_saved_pts_instead_of_configured_fps() {
    let root = std::env::temp_dir().join(format!("daydream-frame-navigation-{}", Uuid::now_v7()));
    fs::create_dir_all(&root).unwrap();
    let video = root.join("synthetic.mkv");
    let output = Command::new(ffmpeg::binary_path())
        .args([
            "-hide_banner",
            "-v",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "color=c=white:s=64x32:r=2:d=2",
            "-c:v",
            "ffv1",
        ])
        .arg(&video)
        .output()
        .unwrap();
    assert!(output.status.success());
    let storage = storage::test_storage(PathBuf::new());
    let id = Uuid::now_v7().to_string();
    let segment = Uuid::now_v7().to_string();
    storage
        .with_connection(|connection| {
            storage::create_recording(
                connection,
                &storage::CreateRecordingInput {
                    id: id.clone(),
                    active_scene_id: None,
                    created_at_ms: 1,
                    default_fps: 10.0,
                    notes: None,
                    output_root: None,
                    quality_label: "balanced-ocr".into(),
                    storage_dir: path_to_string(&root),
                },
            )?;
            storage::create_recording_segment(
                connection,
                &storage::CreateRecordingSegmentInput {
                    id: segment.clone(),
                    recording_id: id.clone(),
                    segment_index: 1,
                    started_at_ms: 1,
                    file_path: path_to_string(&video),
                },
            )?;
            storage::finalize_recording_segment(
                connection,
                &storage::FinalizeRecordingSegmentInput {
                    id: segment,
                    duration_ms: 2000,
                    ended_at_ms: 2001,
                    updated_at_ms: 2001,
                    video_stream_index: Some(0),
                    desktop_audio_stream_index: None,
                    microphone_audio_stream_index: None,
                    probe_json: None,
                },
            )?;
            storage::mark_recording_completed(connection, &id, 2001)?;
            Ok(())
        })
        .unwrap();
    assert_eq!(
        adjacent(
            &storage,
            AdjacentFrameInput {
                recording_id: id.clone(),
                source_track_id: None,
                timestamp_ms: 100,
                direction: 1
            }
        )
        .unwrap(),
        Some(500)
    );
    assert_eq!(
        adjacent(
            &storage,
            AdjacentFrameInput {
                recording_id: id,
                source_track_id: None,
                timestamp_ms: 500,
                direction: -1
            }
        )
        .unwrap(),
        Some(0)
    );
    fs::remove_dir_all(root).unwrap();
}
