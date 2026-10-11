use super::*;

fn finalize_sources(connection: &rusqlite::Connection, segment_id: &str) {
    storage::finalize_recording_segment(
        connection,
        &storage::FinalizeRecordingSegmentInput {
            desktop_audio_stream_index: Some(1),
            duration_ms: 10_000,
            ended_at_ms: 10_001,
            id: segment_id.to_string(),
            microphone_audio_stream_index: Some(2),
            probe_json: None,
            updated_at_ms: 10_001,
            video_stream_index: Some(0),
        },
    )
    .expect("finalize source indices");
}

#[test]
fn privacy_library_audio_redaction_hides_real_uuid_transcript() {
    let storage = storage::test_storage(PathBuf::new());
    let recording_id = storage
        .with_connection_mut(|connection| {
            let (chunk, track) = test_transcription_chunk(connection);
            finalize_sources(connection, chunk.segment_id.as_deref().unwrap());
            persist_transcriptions_for_chunk(
                connection,
                &chunk,
                &track,
                "local-whisper-cpp",
                "small",
                chunk.segment_id.as_deref().unwrap(),
                vec![PendingTranscription {
                    base_offset_ms: 0,
                    transcription: test_verbose_transcription(1.0, 2.0, "private speech", vec![]),
                }],
            )?;
            storage::create_redaction(
                connection,
                &storage::CreateRedactionInput {
                    created_at_ms: 20_000,
                    end_offset_ms: 2_000,
                    id: Uuid::now_v7().to_string(),
                    reason: None,
                    recording_id: chunk.recording_id.clone(),
                    start_offset_ms: 1_000,
                    track_ids_json: "[\"microphone-2\"]".to_string(),
                },
            )?;
            Ok(chunk.recording_id)
        })
        .unwrap();
    assert!(transcript_for_recording(&storage, &recording_id)
        .unwrap()
        .tracks
        .is_empty());
    storage
        .with_connection(|connection| {
            storage::delete_redactions_for_recording(connection, &recording_id)
        })
        .unwrap();
    assert_eq!(
        transcript_for_recording(&storage, &recording_id)
            .unwrap()
            .tracks[0]
            .segments[0]
            .text,
        "private speech",
        "unredaction remains reversible"
    );
}

#[test]
fn privacy_library_video_redaction_hides_real_uuid_visual_results() {
    let storage = storage::test_storage(PathBuf::new());
    let recording_id = storage
        .with_connection_mut(|connection| {
            let fixture = test_visual_recording(connection);
            finalize_sources(connection, &fixture.segment_id);
            create_test_ocr_region(
                connection,
                &fixture,
                &fixture.ocr_chunk_id,
                1_500,
                "private screen",
            )?;
            create_test_vlm_observation(
                connection,
                &fixture,
                &fixture.vlm_chunk_id,
                1_500,
                "private description",
            )?;
            storage::create_redaction(
                connection,
                &storage::CreateRedactionInput {
                    created_at_ms: 20_000,
                    end_offset_ms: 2_000,
                    id: Uuid::now_v7().to_string(),
                    reason: None,
                    recording_id: fixture.recording_id.clone(),
                    start_offset_ms: 1_000,
                    track_ids_json: "[\"video-0\"]".to_string(),
                },
            )?;
            Ok(fixture.recording_id)
        })
        .unwrap();
    let visible = visual_observations_for_recording(&storage, &recording_id).unwrap();
    assert!(visible.ocr_regions.is_empty());
    assert!(visible.vlm_observations.is_empty());
}

#[test]
fn privacy_remote_vlm_request_body_excludes_redacted_prior_text() {
    let storage = storage::test_storage(PathBuf::new());
    let recording_id = storage
        .with_connection_mut(|connection| {
            let fixture = test_visual_recording(connection);
            finalize_sources(connection, &fixture.segment_id);
            create_test_ocr_region(
                connection,
                &fixture,
                &fixture.ocr_chunk_id,
                1_500,
                "hidden OCR secret",
            )?;
            create_test_vlm_observation(
                connection,
                &fixture,
                &fixture.vlm_chunk_id,
                1_500,
                "hidden VLM secret",
            )?;
            create_test_ocr_region(
                connection,
                &fixture,
                &fixture.ocr_chunk_id,
                2_500,
                "public context",
            )?;
            storage::create_redaction(
                connection,
                &storage::CreateRedactionInput {
                    created_at_ms: 20_000,
                    end_offset_ms: 2_000,
                    id: Uuid::now_v7().to_string(),
                    reason: None,
                    recording_id: fixture.recording_id.clone(),
                    start_offset_ms: 1_000,
                    track_ids_json: "[\"video-0\"]".to_string(),
                },
            )?;
            Ok(fixture.recording_id)
        })
        .unwrap();
    let path = std::env::temp_dir().join(format!("daydream-context-{}.png", Uuid::now_v7()));
    fs::write(&path, b"test public frame").unwrap();
    let frame = ExtractedVisualFrame {
        abrupt_transition: false,
        after_context: None,
        before_context: None,
        frame_timestamp_ms: 3_000,
        full_checkpoint: true,
        height: 10,
        path: path.clone(),
        region: VisualRegion {
            height: 10,
            width: 10,
            x: 0,
            y: 0,
        },
        signature: "public".to_string(),
        source_frame_ref: "public-frame".to_string(),
        width: 10,
    };
    let config = VisionRequestConfig {
        route: None,
        storage: None,
        api_key: "test".to_string(),
        base_url: "http://localhost".to_string(),
        model: "test".to_string(),
        provider: "openai-compatible".to_string(),
    };
    let prior = previous_visual_observations(&storage, &recording_id).unwrap();
    let body = vlm_chat_completions_body(&config, &frame, &[], &prior)
        .unwrap()
        .to_string();
    fs::remove_file(path).unwrap();
    assert!(body.contains("public context"));
    assert!(!body.contains("hidden OCR secret"));
    assert!(!body.contains("hidden VLM secret"));
}

#[test]
fn privacy_restart_drains_stale_chunks_under_paused_and_cancelled_parents() {
    for parent_status in ["paused", "cancelled"] {
        let storage = storage::test_storage(PathBuf::new());
        let (chunk_id, job_id) = storage
            .with_connection(|connection| {
                let (chunk, _) = test_transcription_chunk(connection);
                storage::update_processing_job_status(
                    connection,
                    &chunk.job_id,
                    parent_status,
                    None,
                    20_000,
                )?;
                Ok((chunk.id, chunk.job_id))
            })
            .unwrap();
        recover_stale_processing(&storage).unwrap();
        let chunk = storage
            .with_connection(|connection| storage::get_processing_chunk(connection, &chunk_id))
            .unwrap()
            .unwrap();
        assert_ne!(
            chunk.status, "running",
            "{parent_status} parent retains vanished worker"
        );
        let job = storage
            .with_connection(|connection| storage::get_processing_job(connection, &job_id))
            .unwrap()
            .unwrap();
        assert_eq!(job.status, parent_status);
        if parent_status == "paused" {
            update_processing_job_for_action(&storage, &job_id, "queued").unwrap();
        } else {
            retry_processing_job_for_action(&storage, &job_id).unwrap();
        }
    }
}

#[test]
fn privacy_dispatch_rechecks_redactions_added_after_job_planning() {
    let storage = storage::test_storage(PathBuf::new());
    let (chunk_id, job_id) = storage
        .with_connection_mut(|connection| {
            let (chunk, _) = test_transcription_chunk(connection);
            finalize_sources(connection, chunk.segment_id.as_deref().unwrap());
            storage::update_processing_chunk_status(
                connection, &chunk.id, "queued", 0.0, None, 20_000,
            )?;
            storage::create_redaction(
                connection,
                &storage::CreateRedactionInput {
                    created_at_ms: 20_000,
                    end_offset_ms: 2_000,
                    id: Uuid::now_v7().to_string(),
                    reason: None,
                    recording_id: chunk.recording_id.clone(),
                    start_offset_ms: 1_000,
                    track_ids_json: "[\"microphone-2\"]".to_string(),
                },
            )?;
            Ok((chunk.id, chunk.job_id))
        })
        .unwrap();
    assert!(claim_next_processing_chunk(&storage, &job_id)
        .unwrap()
        .is_none());
    let chunk = storage
        .with_connection(|connection| storage::get_processing_chunk(connection, &chunk_id))
        .unwrap()
        .unwrap();
    assert_ne!(chunk.status, "running");
}

#[test]
fn privacy_dispatch_blocks_open_and_unmaterialized_protection_spans() {
    for closed in [false, true] {
        let storage = storage::test_storage(PathBuf::new());
        let job_id = storage
            .with_connection(|connection| {
                let (chunk, _) = test_transcription_chunk(connection);
                finalize_sources(connection, chunk.segment_id.as_deref().unwrap());
                storage::update_processing_chunk_status(
                    connection, &chunk.id, "queued", 0.0, None, 20_000,
                )?;
                let span = storage::create_recording_protection_span(
                    connection,
                    &storage::CreateRecordingProtectionSpanInput {
                        action: "redact".to_string(),
                        id: Uuid::now_v7().to_string(),
                        opened_at_ms: 20_000,
                        recording_id: chunk.recording_id.clone(),
                        rule_id: "private".to_string(),
                        rule_label: "Private".to_string(),
                        start_offset_ms: 1_000,
                    },
                )?;
                if closed {
                    storage::close_recording_protection_span(connection, &span.id, 2_000, 21_000)?;
                }
                Ok(chunk.job_id)
            })
            .unwrap();
        assert!(
            claim_next_processing_chunk(&storage, &job_id).is_err(),
            "must visibly fail while protection is unresolved"
        );
    }
}

#[test]
fn privacy_recording_deletion_removes_crash_extracted_artifacts() {
    let root = std::env::temp_dir().join(format!("daydream-delete-regression-{}", Uuid::now_v7()));
    let recordings_root = root.join("recordings");
    let storage = storage::test_storage(PathBuf::new());
    let (recording, job_id) = storage
        .with_connection(|connection| {
            let (chunk, _) = test_transcription_chunk(connection);
            let directory = recordings_root.join(&chunk.recording_id);
            fs::create_dir_all(&directory).unwrap();
            connection.execute(
                "UPDATE recordings SET storage_dir = ?2, status = 'completed' WHERE id = ?1",
                rusqlite::params![chunk.recording_id, directory.to_string_lossy()],
            )?;
            Ok((
                storage::get_recording(connection, &chunk.recording_id)?,
                chunk.job_id,
            ))
        })
        .unwrap();
    for kind in ["transcription", "visual"] {
        let directory = root.join("processing").join(kind).join(&job_id);
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join("private-extraction"), b"private content").unwrap();
    }
    crate::library::delete_recording_row(&storage, &recordings_root, &recording).unwrap();
    let leftovers = ["transcription", "visual"]
        .iter()
        .any(|kind| root.join("processing").join(kind).join(&job_id).exists());
    fs::remove_dir_all(root).unwrap();
    assert!(
        !leftovers,
        "deletion left private extracted content on disk"
    );
}

#[test]
fn privacy_cancelled_or_superseded_attempt_cannot_persist_transcript() {
    for superseded in [false, true] {
        let storage = storage::test_storage(PathBuf::new());
        let recording_id = storage
            .with_connection_mut(|connection| {
                let (chunk, track) = test_transcription_chunk(connection);
                if superseded {
                    storage::update_processing_chunk_status(
                        connection, &chunk.id, "running", 0.0, None, 20_000,
                    )?;
                } else {
                    storage::update_processing_chunk_status(
                        connection,
                        &chunk.id,
                        "cancelled",
                        0.0,
                        Some("ai_processing_privacy_changed"),
                        20_000,
                    )?;
                }
                let stats = persist_transcriptions_for_chunk(
                    connection,
                    &chunk,
                    &track,
                    "local-whisper-cpp",
                    "small",
                    chunk.segment_id.as_deref().unwrap(),
                    vec![PendingTranscription {
                        base_offset_ms: 0,
                        transcription: test_verbose_transcription(
                            1.0,
                            2.0,
                            "late private speech",
                            vec![],
                        ),
                    }],
                )?;
                assert_eq!(stats.segment_count, 0);
                Ok(chunk.recording_id)
            })
            .unwrap();
        assert!(storage
            .with_connection(|connection| storage::list_transcript_segments(
                connection,
                &recording_id
            ))
            .unwrap()
            .is_empty());
    }
}

#[test]
fn privacy_cancelled_visual_work_cannot_persist_late_private_text() {
    let storage = storage::test_storage(PathBuf::new());
    let (fixture, chunk) = storage
        .with_connection(|connection| {
            let fixture = test_visual_recording(connection);
            finalize_sources(connection, &fixture.segment_id);
            let chunk = storage::update_processing_chunk_status(
                connection,
                &fixture.ocr_chunk_id,
                "running",
                0.0,
                None,
                20_000,
            )?;
            storage::update_processing_chunk_status(
                connection,
                &fixture.ocr_chunk_id,
                "cancelled",
                0.0,
                Some("ai_processing_privacy_changed"),
                20_001,
            )?;
            Ok((fixture, chunk))
        })
        .unwrap();
    let input = CreateVisualRegionInput {
        chunk_id: chunk.id.clone(),
        confidence: None,
        frame_timestamp_ms: 1_500,
        id: Uuid::now_v7().to_string(),
        job_id: fixture.job_id.clone(),
        model: Some("tesseract".to_string()),
        observation_kind: None,
        prompt_version: None,
        provider: "tesseract".to_string(),
        recording_id: fixture.recording_id.clone(),
        region_height: 10,
        region_width: 10,
        region_x: 0,
        region_y: 0,
        source_frame_ref: "private-source".to_string(),
        text: "late private OCR".to_string(),
    };
    persist_visual_output_for_test(
        &storage,
        &chunk,
        PendingVisualOutputForTest {
            ocr_regions: vec![input],
            ..Default::default()
        },
    )
    .unwrap();
    assert!(storage
        .with_connection(|connection| storage::list_ocr_regions(connection, &fixture.recording_id))
        .unwrap()
        .is_empty());
}

#[test]
fn privacy_retention_defers_active_processing_then_removes_all_owned_artifacts() {
    tauri::async_runtime::block_on(async {
        let root =
            std::env::temp_dir().join(format!("daydream-retention-regression-{}", Uuid::now_v7()));
        let recordings_root = root.join("recordings");
        let storage = storage::test_storage(PathBuf::new());
        let (recording_id, job_id) = storage
            .with_connection(|connection| {
                let (chunk, _) = test_transcription_chunk(connection);
                let directory = recordings_root.join(&chunk.recording_id);
                fs::create_dir_all(&directory).unwrap();
                connection.execute(
                    "UPDATE recordings SET storage_dir = ?2, status = 'completed' WHERE id = ?1",
                    rusqlite::params![chunk.recording_id, directory.to_string_lossy()],
                )?;
                Ok((chunk.recording_id, chunk.job_id))
            })
            .unwrap();
        let artifact = root
            .join("processing/visual")
            .join(&job_id)
            .join("private.png");
        fs::create_dir_all(artifact.parent().unwrap()).unwrap();
        fs::write(&artifact, b"private frame").unwrap();
        let lease = crate::recording_privacy::processing_lease(&recording_id).await;
        let settings = settings::default_settings().storage;
        let timestamp = 100 * 86_400_000;
        let deferred = crate::storage_limits::enforce_storage_cleanup_at(
            &storage,
            &recordings_root,
            &settings,
            timestamp,
        );
        assert_eq!(deferred.deleted_recording_count, 0);
        assert!(artifact.exists());
        drop(lease);
        let cleaned = crate::storage_limits::enforce_storage_cleanup_at(
            &storage,
            &recordings_root,
            &settings,
            timestamp,
        );
        assert_eq!(cleaned.deleted_recording_count, 1);
        assert!(!artifact.exists());
        assert!(!recordings_root.join(&recording_id).exists());
        fs::remove_dir_all(root).unwrap();
    });
}
