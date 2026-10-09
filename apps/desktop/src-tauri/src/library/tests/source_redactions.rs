use super::*;

struct Fixture {
    recording: String,
    a: String,
    b: String,
    directory: PathBuf,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.directory);
    }
}
fn seed(storage: &AppStorage) -> Fixture {
    let directory =
        std::env::temp_dir().join(format!("daydream-redaction-review-{}", Uuid::now_v7()));
    fs::create_dir_all(&directory).unwrap();
    let file = directory.join("synthetic.mkv");
    fs::write(&file, b"synthetic source metadata").unwrap();
    let recording = Uuid::now_v7().to_string();
    let a = Uuid::now_v7().to_string();
    let b = Uuid::now_v7().to_string();
    storage
        .with_connection(|connection| {
            storage::create_recording(
                connection,
                &storage::CreateRecordingInput {
                    id: recording.clone(),
                    active_scene_id: None,
                    created_at_ms: 1,
                    default_fps: 1.0,
                    notes: None,
                    output_root: None,
                    quality_label: "balanced-ocr".into(),
                    storage_dir: path_to_string(&directory),
                },
            )?;
            let segment = storage::create_recording_segment(
                connection,
                &storage::CreateRecordingSegmentInput {
                    id: Uuid::now_v7().to_string(),
                    recording_id: recording.clone(),
                    segment_index: 1,
                    file_path: path_to_string(&file),
                    started_at_ms: 1,
                },
            )?;
            storage::finalize_recording_segment(
                connection,
                &storage::FinalizeRecordingSegmentInput {
                    id: segment.id.clone(),
                    duration_ms: 2000,
                    ended_at_ms: 2001,
                    updated_at_ms: 2001,
                    video_stream_index: Some(0),
                    desktop_audio_stream_index: Some(2),
                    microphone_audio_stream_index: None,
                    probe_json: None,
                },
            )?;
            let sources =
                [(&a, "a"), (&b, "b")].map(|(track, source)| storage::RecordingVideoSource {
                    track_id: track.clone(),
                    source_id: source.into(),
                    label: source.into(),
                    width: 4,
                    height: 2,
                    config_json: "{}".into(),
                });
            storage::save_recording_video_sources(connection, &recording, &sources, 1)?;
            storage::save_segment_video_streams(connection, &segment.id, &sources, 1.0)?;
            storage::mark_recording_completed(connection, &recording, 2001)?;
            storage::create_redaction(
                connection,
                &storage::CreateRedactionInput {
                    id: Uuid::now_v7().to_string(),
                    recording_id: recording.clone(),
                    start_offset_ms: 200,
                    end_offset_ms: 1000,
                    reason: Some("synthetic protection cut".into()),
                    created_at_ms: 2001,
                    track_ids_json: "[]".into(),
                },
            )?;
            Ok(())
        })
        .unwrap();
    Fixture {
        recording,
        a,
        b,
        directory,
    }
}
fn hidden(detail: &LibraryRecordingDetail, track: &str, time: i64) -> bool {
    detail.redactions.iter().any(|cut| {
        cut.start_offset_ms <= time
            && time < cut.end_offset_ms
            && cut.track_ids.iter().any(|id| id == track)
    })
}

#[test]
fn review_screen_redaction_mutation_accepts_durable_source_ids() {
    tauri::async_runtime::block_on(async {
        let app = tauri::test::mock_app();
        let storage = storage::test_storage(PathBuf::new());
        let fixture = seed(&storage);
        app.manage(storage);
        let renamed = commands::rename_library_recording(
            RenameLibraryRecordingInput {
                recording_id: fixture.recording.clone(),
                title: Some("Synthetic renamed recording".into()),
            },
            app.handle().clone(),
            app.state::<AppStorage>(),
        )
        .unwrap();
        assert!(
            renamed.tracks.iter().any(|track| track.id == fixture.a),
            "Renaming must retain the source IDs returned to Studio"
        );
        let detail = commands::create_library_redactions(
            app.handle().clone(),
            CreateLibraryRedactionsInput {
                recording_id: fixture.recording.clone(),
                ranges: vec![CreateLibraryRedactionRangeInput {
                    start_offset_ms: 1200,
                    end_offset_ms: 1500,
                    track_ids: vec![fixture.a.clone()],
                }],
            },
            app.state::<AppStorage>(),
        )
        .await
        .expect("screen UUID must be accepted");
        assert!(hidden(&detail, &fixture.a, 1300));
        assert!(!hidden(&detail, &fixture.b, 1300));
        assert!(hidden(&detail, &fixture.a, 500));
        assert!(hidden(&detail, &fixture.b, 500));
    });
}
#[test]
fn review_screen_unredaction_mutation_keeps_other_sources_protected() {
    tauri::async_runtime::block_on(async {
        let app = tauri::test::mock_app();
        let storage = storage::test_storage(PathBuf::new());
        let fixture = seed(&storage);
        app.manage(storage);
        let detail = commands::unredact_library_redactions(
            app.handle().clone(),
            UnredactLibraryRedactionsInput {
                recording_id: fixture.recording.clone(),
                ranges: vec![CreateLibraryRedactionRangeInput {
                    start_offset_ms: 400,
                    end_offset_ms: 600,
                    track_ids: vec![fixture.a.clone()],
                }],
            },
            app.state::<AppStorage>(),
        )
        .await
        .expect("screen UUID must be accepted");
        assert!(!hidden(&detail, &fixture.a, 500));
        assert!(hidden(&detail, &fixture.b, 500));
        assert!(hidden(&detail, &fixture.a, 300));
    });
}
#[test]
fn review_audio_redaction_mutation_preserves_all_track_protection_on_independent_screens() {
    tauri::async_runtime::block_on(async {
        let app = tauri::test::mock_app();
        let storage = storage::test_storage(PathBuf::new());
        let fixture = seed(&storage);
        app.manage(storage);
        let detail = commands::create_library_redactions(
            app.handle().clone(),
            CreateLibraryRedactionsInput {
                recording_id: fixture.recording.clone(),
                ranges: vec![CreateLibraryRedactionRangeInput {
                    start_offset_ms: 1200,
                    end_offset_ms: 1500,
                    track_ids: vec!["desktop-audio-2".into()],
                }],
            },
            app.state::<AppStorage>(),
        )
        .await
        .unwrap();
        assert!(hidden(&detail, &fixture.a, 500));
        assert!(hidden(&detail, &fixture.b, 500));
    });
}
