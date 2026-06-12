use super::*;
use std::path::{Path, PathBuf};

#[test]
fn catalog_models_default_to_missing_with_pinned_sha1() {
    let model = catalog_asset(&LOCAL_WHISPER_CATALOG[2]);

    assert_eq!(model.id, "small");
    assert_eq!(model.checksum_algorithm.as_deref(), Some("sha1"));
    assert_eq!(
        model.checksum.as_deref(),
        Some("55356645c2b361a969dfd0ef2c5a50d530afd8d5")
    );
    assert_eq!(model.download_status, "missing");
    assert_eq!(model.self_test_status, "not-run");
}

#[test]
fn visual_sampling_uses_recording_fps_when_synced() {
    let mut app_settings = settings::default_settings();
    app_settings
        .ai
        .processing
        .frame_recognition_sync_with_recording = true;
    app_settings.ai.processing.frame_recognition_fps = 5.0;

    assert_eq!(effective_frame_recognition_fps(&app_settings, 0.2), 0.2);
    assert_eq!(frame_sample_interval_ms(0.2), 5_000);
}

#[test]
fn full_frame_checkpoints_respect_interval_and_minute_cap() {
    assert!(full_checkpoint_for_frame(0, 0, 10_000, 2));
    assert!(full_checkpoint_for_frame(10_000, 0, 10_000, 2));
    assert!(!full_checkpoint_for_frame(20_000, 0, 10_000, 2));
    assert!(full_checkpoint_for_frame(60_000, 0, 10_000, 2));
}

#[test]
fn visual_delta_regions_skip_unchanged_frames() {
    let previous = test_visual_image(256, 256, &[]);
    let current = test_visual_image(256, 256, &[]);

    assert!(visual_delta_regions(&previous, &current, 64).is_empty());
}

#[test]
fn visual_delta_regions_keep_subtle_text_sized_changes() {
    let previous = test_visual_image(256, 256, &[]);
    let current = test_visual_image(256, 256, &[(130, 130), (131, 130), (132, 130)]);
    let regions = visual_delta_regions(&previous, &current, 64);

    assert_eq!(
        regions,
        vec![VisualRegion {
            height: 192,
            width: 192,
            x: 64,
            y: 64,
        }]
    );
}

#[test]
fn visual_delta_regions_merge_adjacent_changed_tiles() {
    let previous = test_visual_image(320, 256, &[]);
    let current = test_visual_image(
        320,
        256,
        &[
            (130, 130),
            (131, 130),
            (132, 130),
            (194, 130),
            (195, 130),
            (196, 130),
        ],
    );
    let regions = visual_delta_regions(&previous, &current, 64);

    assert_eq!(
        regions,
        vec![VisualRegion {
            height: 192,
            width: 256,
            x: 64,
            y: 64,
        }]
    );
}

#[test]
fn parse_tesseract_tsv_returns_text_regions() {
    let frame = ExtractedVisualFrame {
        abrupt_transition: false,
        after_context: None,
        before_context: None,
        frame_timestamp_ms: 500,
        full_checkpoint: true,
        height: 1080,
        path: PathBuf::from("/tmp/frame.png"),
        region: VisualRegion {
            height: 1080,
            width: 1920,
            x: 0,
            y: 0,
        },
        signature: "sig".to_string(),
        source_frame_ref: "segment:one:offset:500:frame:0".to_string(),
        width: 1920,
    };
    let regions = parse_tesseract_tsv(
        "level\tpage_num\tblock_num\tpar_num\tline_num\tword_num\tleft\ttop\twidth\theight\tconf\ttext\n5\t1\t1\t1\t1\t1\t10\t20\t30\t40\t88.5\tDaydream\n5\t1\t1\t1\t1\t2\t10\t20\t0\t40\t90\tignored",
        &frame,
    )
    .expect("parse TSV");

    assert_eq!(regions.len(), 1);
    assert_eq!(regions[0].text, "Daydream");
    assert_eq!(regions[0].confidence, Some(0.885));
    assert_eq!(regions[0].region.x, 10);
}

#[test]
fn parse_tesseract_tsv_translates_crop_regions_to_full_frame() {
    let frame = ExtractedVisualFrame {
        abrupt_transition: false,
        after_context: None,
        before_context: None,
        frame_timestamp_ms: 500,
        full_checkpoint: false,
        height: 1080,
        path: PathBuf::from("/tmp/crop.png"),
        region: VisualRegion {
            height: 200,
            width: 300,
            x: 100,
            y: 200,
        },
        signature: "sig".to_string(),
        source_frame_ref: "segment:one:offset:500:frame:0:region:100,200,300,200".to_string(),
        width: 1920,
    };
    let regions = parse_tesseract_tsv(
        "level\tpage_num\tblock_num\tpar_num\tline_num\tword_num\tleft\ttop\twidth\theight\tconf\ttext\n5\t1\t1\t1\t1\t1\t10\t20\t30\t40\t88.5\tDaydream",
        &frame,
    )
    .expect("parse TSV");

    assert_eq!(regions[0].region.x, 110);
    assert_eq!(regions[0].region.y, 220);
    assert_eq!(regions[0].region.width, 30);
    assert_eq!(regions[0].region.height, 40);
}

#[test]
fn tesseract_missing_binary_returns_privacy_safe_error() {
    let frame = ExtractedVisualFrame {
        abrupt_transition: false,
        after_context: None,
        before_context: None,
        frame_timestamp_ms: 500,
        full_checkpoint: false,
        height: 1080,
        path: PathBuf::from("/tmp/crop.png"),
        region: VisualRegion {
            height: 200,
            width: 300,
            x: 100,
            y: 200,
        },
        signature: "sig".to_string(),
        source_frame_ref: "segment:one:offset:500:frame:0:region:100,200,300,200".to_string(),
        width: 1920,
    };
    let error = run_tesseract_ocr_with_binary(
        "daydream-missing-tesseract-binary-for-test",
        frame.path(),
        &frame,
    )
    .expect_err("missing binary should fail");

    assert_eq!(error.code, "ai_ocr_tesseract_unavailable");
    assert!(!error.message.contains("Daydream"));
    assert!(!error.message.contains("segment:one"));
}

#[test]
fn vlm_observation_validation_requires_typed_source_links() {
    let frame = ExtractedVisualFrame {
        abrupt_transition: false,
        after_context: None,
        before_context: None,
        frame_timestamp_ms: 500,
        full_checkpoint: true,
        height: 100,
        path: PathBuf::from("/tmp/frame.png"),
        region: VisualRegion {
            height: 100,
            width: 200,
            x: 0,
            y: 0,
        },
        signature: "sig".to_string(),
        source_frame_ref: "segment:one:offset:500:frame:0".to_string(),
        width: 200,
    };
    let observations = validate_vlm_observations(
        VlmObservationPayload {
            observations: vec![VlmObservationCandidate {
                confidence: Some(1.2),
                frame_timestamp_ms: 500,
                kind: "corrected-ocr".to_string(),
                region: VlmObservationRegion {
                    height: 25,
                    width: 50,
                    x: 0,
                    y: 0,
                },
                text: "Corrected text".to_string(),
            }],
        },
        &frame,
    )
    .expect("validate observations");

    assert_eq!(observations[0].observation_kind, "corrected-ocr");
    assert_eq!(observations[0].confidence, Some(1.0));
    assert_eq!(
        observations[0].source_frame_ref,
        "segment:one:offset:500:frame:0"
    );

    let error = validate_vlm_observations(
        VlmObservationPayload {
            observations: vec![VlmObservationCandidate {
                confidence: None,
                frame_timestamp_ms: 501,
                kind: "screen-observation".to_string(),
                region: VlmObservationRegion {
                    height: 25,
                    width: 50,
                    x: 0,
                    y: 0,
                },
                text: "Mismatch".to_string(),
            }],
        },
        &frame,
    )
    .expect_err("timestamp mismatch should fail");

    assert_eq!(error.code, "ai_vision_observation_invalid");
}

#[test]
fn vlm_state_reports_provider_and_chunk_states() {
    let disabled = settings::default_settings();
    assert_eq!(
        vlm_state_for_recording(&disabled, &[], 0),
        VlmState {
            error_code: None,
            provider_status: "disabled".to_string(),
            status: "disabled".to_string(),
        }
    );

    let mut configured = settings::default_settings();
    configured.ai.vision.provider = "openai-compatible".to_string();
    configured.ai.vision.base_url = Some("http://localhost:11434/v1".to_string());
    configured.ai.vision.api_key_ref = Some("ai.secret.vision.test".to_string());
    configured.ai.vision.model = Some("llava".to_string());

    assert_eq!(
        vlm_state_for_recording(&configured, &[], 0).status,
        "not-planned"
    );
    assert_eq!(
        vlm_state_for_recording(&configured, &[test_vlm_chunk("queued", None, None)], 0).status,
        "queued"
    );
    assert_eq!(
        vlm_state_for_recording(&configured, &[test_vlm_chunk("running", None, None)], 0).status,
        "running"
    );
    assert_eq!(
        vlm_state_for_recording(
            &configured,
            &[test_vlm_chunk(
                "failed",
                Some("ai_vision_remote_timeout"),
                None
            )],
            0,
        ),
        VlmState {
            error_code: Some("ai_vision_remote_timeout".to_string()),
            provider_status: "configured".to_string(),
            status: "failed".to_string(),
        }
    );
    assert_eq!(
        vlm_state_for_recording(&configured, &[test_vlm_chunk("succeeded", None, None)], 0).status,
        "empty"
    );
    assert_eq!(
        vlm_state_for_recording(&configured, &[test_vlm_chunk("succeeded", None, None)], 1).status,
        "succeeded"
    );
}

#[test]
fn vlm_state_reports_unconfigured_provider_reasons() {
    let mut missing_base_url = settings::default_settings();
    missing_base_url.ai.vision.provider = "openai-compatible".to_string();

    assert_eq!(
        vlm_state_for_recording(&missing_base_url, &[], 0),
        VlmState {
            error_code: None,
            provider_status: "missing-base-url".to_string(),
            status: "unconfigured".to_string(),
        }
    );
}

#[test]
fn transcript_segment_offsets_use_word_bounds_when_available() {
    let segment = transcription::VerboseTranscriptionSegment {
        end: 5.0,
        id: 0,
        start: 0.0,
        text: " test test ".to_string(),
    };
    let words = vec![
        transcription::VerboseTranscriptionWord {
            end: 4.1,
            probability: Some(0.9),
            start: 3.8,
            word: "test".to_string(),
        },
        transcription::VerboseTranscriptionWord {
            end: 4.8,
            probability: Some(0.8),
            start: 4.2,
            word: "test".to_string(),
        },
    ];
    let segment_words = transcription_words_for_segment(&segment, &words);

    assert_eq!(
        transcript_segment_offsets_ms(1_000, &segment, &segment_words),
        (4_800, 5_800)
    );
}

#[test]
fn transcript_segment_offsets_fall_back_to_segment_without_words() {
    let segment = transcription::VerboseTranscriptionSegment {
        end: 2.5,
        id: 0,
        start: 0.5,
        text: "fallback".to_string(),
    };

    assert_eq!(
        transcript_segment_offsets_ms(1_000, &segment, &[]),
        (1_500, 3_500)
    );
}

#[test]
fn transcription_words_for_segment_ignores_invalid_or_empty_words() {
    let segment = transcription::VerboseTranscriptionSegment {
        end: 5.0,
        id: 0,
        start: 0.0,
        text: "ok".to_string(),
    };
    let words = vec![
        transcription::VerboseTranscriptionWord {
            end: 1.2,
            probability: None,
            start: 1.0,
            word: "ok".to_string(),
        },
        transcription::VerboseTranscriptionWord {
            end: 2.0,
            probability: None,
            start: 2.0,
            word: "zero".to_string(),
        },
        transcription::VerboseTranscriptionWord {
            end: 3.5,
            probability: None,
            start: 3.0,
            word: " ".to_string(),
        },
        transcription::VerboseTranscriptionWord {
            end: 6.0,
            probability: None,
            start: 5.5,
            word: "outside".to_string(),
        },
    ];

    let segment_words = transcription_words_for_segment(&segment, &words);

    assert_eq!(segment_words.len(), 1);
    assert_eq!(segment_words[0].word, "ok");
}

#[test]
fn persist_transcriptions_for_chunk_commits_all_completed_ranges() {
    let storage = storage::test_storage(PathBuf::new());

    storage
        .with_connection_mut(|connection| {
            let (chunk, track) = test_transcription_chunk(connection);
            let stats = persist_transcriptions_for_chunk(
                connection,
                &chunk,
                &track,
                "local-whisper-cpp",
                "small",
                chunk.segment_id.as_deref().expect("chunk segment id"),
                vec![
                    PendingTranscription {
                        base_offset_ms: 1_000,
                        transcription: test_verbose_transcription(
                            0.5,
                            1.0,
                            " first ",
                            vec![test_transcription_word(0.6, 0.8, "first", Some(0.8))],
                        ),
                    },
                    PendingTranscription {
                        base_offset_ms: 5_000,
                        transcription: test_verbose_transcription(
                            0.25,
                            0.75,
                            "second",
                            vec![test_transcription_word(0.3, 0.7, "second", Some(0.9))],
                        ),
                    },
                ],
            )
            .expect("persist completed ranges");

            let segments = storage::list_transcript_segments(connection, &chunk.recording_id)?;
            let words = segments
                .iter()
                .map(|segment| storage::list_transcript_words(connection, &segment.id))
                .collect::<rusqlite::Result<Vec<_>>>()?
                .into_iter()
                .flatten()
                .collect::<Vec<_>>();

            assert_eq!(stats.segment_count, 2);
            assert_eq!(stats.word_count, 2);
            assert_eq!(segments.len(), 2);
            assert_eq!(segments[0].text, "first");
            assert_eq!(segments[0].start_offset_ms, 1_600);
            assert_eq!(segments[0].end_offset_ms, 1_800);
            assert_eq!(segments[1].text, "second");
            assert_eq!(segments[1].start_offset_ms, 5_300);
            assert_eq!(segments[1].end_offset_ms, 5_700);
            assert_eq!(words.len(), 2);

            Ok(())
        })
        .expect("persist transcriptions");
}

#[test]
fn transcript_queries_hide_redacted_track_overlaps_without_deleting_rows() {
    let storage = storage::test_storage(PathBuf::new());
    let (recording_id, redacted_segment_id) = storage
        .with_connection_mut(|connection| {
            let (chunk, track) = test_transcription_chunk(connection);
            let other_track_id = Uuid::now_v7().to_string();
            storage::upsert_recording_track(
                connection,
                &UpsertRecordingTrackInput {
                    codec: Some("pcm_s16le".to_string()),
                    created_at_ms: 1,
                    id: other_track_id.clone(),
                    kind: "desktop-audio".to_string(),
                    label: "Desktop audio".to_string(),
                    path: None,
                    recording_id: chunk.recording_id.clone(),
                    source_id: None,
                },
            )?;
            storage::create_redaction(
                connection,
                &storage::CreateRedactionInput {
                    created_at_ms: 20,
                    end_offset_ms: 2_000,
                    id: Uuid::now_v7().to_string(),
                    reason: None,
                    recording_id: chunk.recording_id.clone(),
                    start_offset_ms: 1_000,
                    track_ids_json: serde_json::to_string(&vec![track.id.clone()])
                        .expect("serialize track ids"),
                },
            )?;

            let redacted_segment = storage::create_transcript_segment(
                connection,
                &CreateTranscriptSegmentInput {
                    chunk_id: chunk.id.clone(),
                    confidence: Some(0.9),
                    created_at_ms: 30,
                    end_offset_ms: 1_500,
                    id: Uuid::now_v7().to_string(),
                    job_id: chunk.job_id.clone(),
                    model: Some("small".to_string()),
                    provider: "local-whisper-cpp".to_string(),
                    recording_id: chunk.recording_id.clone(),
                    segment_id: chunk.segment_id.clone().expect("segment id"),
                    source_role: "microphone".to_string(),
                    start_offset_ms: 500,
                    text: "hidden words".to_string(),
                    track_id: track.id.clone(),
                },
            )?;
            storage::create_transcript_word(
                connection,
                &CreateTranscriptWordInput {
                    end_offset_ms: 800,
                    id: Uuid::now_v7().to_string(),
                    probability: Some(0.8),
                    start_offset_ms: 700,
                    text: "hidden".to_string(),
                    transcript_segment_id: redacted_segment.id.clone(),
                },
            )?;
            storage::create_transcript_segment(
                connection,
                &CreateTranscriptSegmentInput {
                    chunk_id: chunk.id.clone(),
                    confidence: Some(0.8),
                    created_at_ms: 31,
                    end_offset_ms: 1_500,
                    id: Uuid::now_v7().to_string(),
                    job_id: chunk.job_id.clone(),
                    model: Some("small".to_string()),
                    provider: "local-whisper-cpp".to_string(),
                    recording_id: chunk.recording_id.clone(),
                    segment_id: chunk.segment_id.clone().expect("segment id"),
                    source_role: "desktop-audio".to_string(),
                    start_offset_ms: 500,
                    text: "visible other track".to_string(),
                    track_id: other_track_id,
                },
            )?;
            storage::create_transcript_segment(
                connection,
                &CreateTranscriptSegmentInput {
                    chunk_id: chunk.id.clone(),
                    confidence: Some(0.7),
                    created_at_ms: 32,
                    end_offset_ms: 3_000,
                    id: Uuid::now_v7().to_string(),
                    job_id: chunk.job_id,
                    model: Some("small".to_string()),
                    provider: "local-whisper-cpp".to_string(),
                    recording_id: chunk.recording_id.clone(),
                    segment_id: chunk.segment_id.expect("segment id"),
                    source_role: "microphone".to_string(),
                    start_offset_ms: 2_000,
                    text: "visible after redaction".to_string(),
                    track_id: track.id,
                },
            )?;

            Ok((chunk.recording_id, redacted_segment.id))
        })
        .expect("seed transcript and redaction");

    let visible_segments = visible_transcript_segments_for_recording(&storage, &recording_id)
        .expect("visible transcript segments");
    let transcript = transcript_for_recording(&storage, &recording_id).expect("transcript");
    let stored_segments = storage
        .with_connection(|connection| storage::list_transcript_segments(connection, &recording_id))
        .expect("stored transcript segments");

    assert_eq!(stored_segments.len(), 3);
    assert!(stored_segments
        .iter()
        .any(|segment| segment.id == redacted_segment_id));
    let visible_text = visible_segments
        .iter()
        .map(|segment| segment.text.as_str())
        .collect::<Vec<_>>();
    assert_eq!(visible_text.len(), 2);
    assert!(visible_text.contains(&"visible other track"));
    assert!(visible_text.contains(&"visible after redaction"));

    let transcript_text = transcript
        .tracks
        .iter()
        .flat_map(|track| track.segments.iter())
        .map(|segment| segment.text.as_str())
        .collect::<Vec<_>>();
    assert_eq!(transcript_text.len(), 2);
    assert!(transcript_text.contains(&"visible other track"));
    assert!(transcript_text.contains(&"visible after redaction"));
    assert!(transcript
        .tracks
        .iter()
        .flat_map(|track| track.segments.iter())
        .flat_map(|segment| segment.words.iter())
        .all(|word| word.text != "hidden"));
}

#[test]
fn visual_observation_queries_hide_redacted_video_overlaps_without_deleting_rows() {
    let storage = storage::test_storage(PathBuf::new());
    let recording_id = storage
        .with_connection_mut(|connection| {
            let fixture = test_visual_recording(connection);
            storage::create_redaction(
                connection,
                &storage::CreateRedactionInput {
                    created_at_ms: 20,
                    end_offset_ms: 2_000,
                    id: Uuid::now_v7().to_string(),
                    reason: None,
                    recording_id: fixture.recording_id.clone(),
                    start_offset_ms: 1_000,
                    track_ids_json: serde_json::to_string(&vec![fixture.video_track_id.clone()])
                        .expect("serialize video redaction tracks"),
                },
            )?;
            storage::create_redaction(
                connection,
                &storage::CreateRedactionInput {
                    created_at_ms: 21,
                    end_offset_ms: 4_000,
                    id: Uuid::now_v7().to_string(),
                    reason: None,
                    recording_id: fixture.recording_id.clone(),
                    start_offset_ms: 3_000,
                    track_ids_json: serde_json::to_string(&vec![fixture.audio_track_id.clone()])
                        .expect("serialize audio redaction tracks"),
                },
            )?;
            create_test_ocr_region(
                connection,
                &fixture,
                &fixture.ocr_chunk_id,
                1_500,
                "hidden video text",
            )?;
            create_test_ocr_region(
                connection,
                &fixture,
                &fixture.ocr_chunk_id,
                2_500,
                "visible text",
            )?;
            create_test_ocr_region(
                connection,
                &fixture,
                &fixture.ocr_chunk_id,
                3_500,
                "visible during audio redaction",
            )?;
            create_test_vlm_observation(
                connection,
                &fixture,
                &fixture.vlm_chunk_id,
                1_500,
                "hidden video observation",
            )?;
            create_test_vlm_observation(
                connection,
                &fixture,
                &fixture.vlm_chunk_id,
                2_500,
                "visible observation",
            )?;

            Ok(fixture.recording_id)
        })
        .expect("seed visual observations and redactions");

    let observations =
        visual_observations_for_recording(&storage, &recording_id).expect("visual observations");
    let stored_counts = storage
        .with_connection(|connection| {
            Ok((
                storage::list_ocr_regions(connection, &recording_id)?.len(),
                storage::list_vlm_observations(connection, &recording_id)?.len(),
            ))
        })
        .expect("stored visual counts");

    assert_eq!(stored_counts, (3, 2));
    let ocr_text = observations
        .ocr_regions
        .iter()
        .map(|region| region.text.as_str())
        .collect::<Vec<_>>();
    assert_eq!(ocr_text.len(), 2);
    assert!(ocr_text.contains(&"visible text"));
    assert!(ocr_text.contains(&"visible during audio redaction"));
    assert!(!ocr_text.contains(&"hidden video text"));

    let vlm_text = observations
        .vlm_observations
        .iter()
        .map(|observation| observation.text.as_str())
        .collect::<Vec<_>>();
    assert_eq!(vlm_text, vec!["visible observation"]);
}

#[test]
fn visual_observation_queries_hide_all_track_redactions() {
    let storage = storage::test_storage(PathBuf::new());
    let recording_id = storage
        .with_connection_mut(|connection| {
            let fixture = test_visual_recording(connection);
            storage::create_redaction(
                connection,
                &storage::CreateRedactionInput {
                    created_at_ms: 20,
                    end_offset_ms: 2_000,
                    id: Uuid::now_v7().to_string(),
                    reason: None,
                    recording_id: fixture.recording_id.clone(),
                    start_offset_ms: 1_000,
                    track_ids_json: "[]".to_string(),
                },
            )?;
            create_test_ocr_region(
                connection,
                &fixture,
                &fixture.ocr_chunk_id,
                1_500,
                "hidden by all tracks",
            )?;
            create_test_ocr_region(
                connection,
                &fixture,
                &fixture.ocr_chunk_id,
                2_500,
                "visible after all tracks",
            )?;

            Ok(fixture.recording_id)
        })
        .expect("seed all-track visual redaction");

    let observations =
        visual_observations_for_recording(&storage, &recording_id).expect("visual observations");
    let ocr_text = observations
        .ocr_regions
        .iter()
        .map(|region| region.text.as_str())
        .collect::<Vec<_>>();

    assert_eq!(ocr_text, vec!["visible after all tracks"]);
}

#[test]
fn visual_observation_queries_hide_non_succeeded_chunk_rows() {
    let storage = storage::test_storage(PathBuf::new());
    let recording_id = storage
        .with_connection_mut(|connection| {
            let fixture = test_visual_recording(connection);
            let failed_ocr_chunk_id =
                create_test_visual_chunk(connection, &fixture, "ocr", "failed")?;
            let running_vlm_chunk_id =
                create_test_visual_chunk(connection, &fixture, "vlm", "running")?;
            create_test_ocr_region(
                connection,
                &fixture,
                &fixture.ocr_chunk_id,
                500,
                "visible completed text",
            )?;
            create_test_ocr_region(
                connection,
                &fixture,
                &failed_ocr_chunk_id,
                500,
                "hidden failed text",
            )?;
            create_test_vlm_observation(
                connection,
                &fixture,
                &fixture.vlm_chunk_id,
                500,
                "visible completed observation",
            )?;
            create_test_vlm_observation(
                connection,
                &fixture,
                &running_vlm_chunk_id,
                500,
                "hidden running observation",
            )?;

            Ok(fixture.recording_id)
        })
        .expect("seed non-succeeded visual rows");

    let observations =
        visual_observations_for_recording(&storage, &recording_id).expect("visual observations");
    let ocr_text = observations
        .ocr_regions
        .iter()
        .map(|region| region.text.as_str())
        .collect::<Vec<_>>();
    let vlm_text = observations
        .vlm_observations
        .iter()
        .map(|observation| observation.text.as_str())
        .collect::<Vec<_>>();

    assert_eq!(ocr_text, vec!["visible completed text"]);
    assert_eq!(vlm_text, vec!["visible completed observation"]);
}

#[test]
fn persist_transcriptions_for_chunk_rolls_back_failed_range_batch() {
    let storage = storage::test_storage(PathBuf::new());

    storage
        .with_connection_mut(|connection| {
            let (chunk, track) = test_transcription_chunk(connection);
            let error = persist_transcriptions_for_chunk(
                connection,
                &chunk,
                &track,
                "local-whisper-cpp",
                "small",
                chunk.segment_id.as_deref().expect("chunk segment id"),
                vec![
                    PendingTranscription {
                        base_offset_ms: 1_000,
                        transcription: test_verbose_transcription(
                            0.0,
                            1.0,
                            "would otherwise persist",
                            vec![],
                        ),
                    },
                    PendingTranscription {
                        base_offset_ms: 2_000,
                        transcription: test_verbose_transcription(
                            0.0,
                            1.0,
                            "invalid word probability",
                            vec![test_transcription_word(0.1, 0.2, "bad", Some(2.0))],
                        ),
                    },
                ],
            )
            .expect_err("invalid word probability should fail");

            assert!(matches!(error, rusqlite::Error::SqliteFailure(_, Some(_))));
            let segment_count: i64 = connection.query_row(
                "SELECT COUNT(*) FROM transcript_segments WHERE chunk_id = ?1",
                [&chunk.id],
                |row| row.get(0),
            )?;
            let word_count: i64 = connection.query_row(
                r#"
                SELECT COUNT(*)
                FROM transcript_words
                WHERE transcript_segment_id IN (
                    SELECT id FROM transcript_segments WHERE chunk_id = ?1
                )
                "#,
                [&chunk.id],
                |row| row.get(0),
            )?;

            assert_eq!(segment_count, 0);
            assert_eq!(word_count, 0);

            Ok(())
        })
        .expect("rollback checked");
}

#[test]
fn manager_queues_one_download_while_another_is_active() {
    let manager = AiModelManager::new();

    assert_eq!(
        manager.request_download("tiny"),
        DownloadRequestState::Started
    );
    assert_eq!(
        manager.request_download("small"),
        DownloadRequestState::Queued
    );
    assert_eq!(manager.finish_download("tiny"), Some("small".to_string()));
}

#[test]
fn manager_clears_cancelled_state_when_retrying_active_download() {
    let manager = AiModelManager::new();

    assert_eq!(
        manager.request_download("tiny"),
        DownloadRequestState::Started
    );
    assert!(manager.cancel_download("tiny"));
    assert!(manager.is_cancelled("tiny"));

    assert_eq!(
        manager.request_download("tiny"),
        DownloadRequestState::AlreadyRunning
    );
    assert!(!manager.is_cancelled("tiny"));
}

#[test]
fn processing_completion_stays_active_until_chunks_are_terminal() {
    for active_status in ["queued", "running"] {
        let chunks = vec![test_processing_chunk(active_status)];

        assert!(matches!(
            summarize_processing_completion(&chunks),
            ProcessingCompletion::Active
        ));
    }
}

#[test]
fn processing_completion_succeeds_only_after_all_chunks_succeed() {
    let chunks = vec![
        test_processing_chunk("succeeded"),
        test_processing_chunk("succeeded"),
    ];

    assert!(matches!(
        summarize_processing_completion(&chunks),
        ProcessingCompletion::Terminal {
            error: None,
            status: "succeeded"
        }
    ));
}

#[test]
fn transcription_chunk_state_preserves_skipped_silence_result() {
    let mut chunk = test_processing_chunk("succeeded");
    chunk.result_code = Some(TRANSCRIPTION_RESULT_SKIPPED_SILENCE.to_string());

    let state = transcription_chunk_state(&processing_chunk_from_row(chunk), &[]);

    assert_eq!(state.result, TRANSCRIPTION_RESULT_SKIPPED_SILENCE);
    assert_eq!(
        state.result_code.as_deref(),
        Some(TRANSCRIPTION_RESULT_SKIPPED_SILENCE)
    );
    assert_eq!(state.text, None);
}

#[test]
fn activity_events_expand_to_active_window_without_private_payload() {
    let events = vec![CaptureEventRow {
        id: "event-1".to_string(),
        kind: "keyboard-activity".to_string(),
        occurred_at_ms: 1_000,
        offset_ms: 1_000,
        payload_json: r#"{"source":"evdev","bucketMs":500}"#.to_string(),
        recording_id: "recording-1".to_string(),
    }];

    let intervals =
        activity_intervals_from_events("keyboard", "keyboard-activity", &events, 0, 5_000, 2_000);

    assert_eq!(
        intervals
            .iter()
            .map(|interval| (
                interval.state.as_str(),
                interval.start_offset_ms,
                interval.end_offset_ms
            ))
            .collect::<Vec<_>>(),
        vec![
            ("idle", 0, 1_000),
            ("active", 1_000, 3_000),
            ("idle", 3_000, 5_000)
        ]
    );
    assert!(!events[0].payload_json.contains("KEY_"));
    assert!(!events[0].payload_json.contains("BTN_"));
}

#[test]
fn unavailable_input_source_produces_unknown_interval() {
    let events = vec![CaptureEventRow {
        id: "event-1".to_string(),
        kind: "capture-state".to_string(),
        occurred_at_ms: 0,
        offset_ms: 0,
        payload_json: r#"{"reason":"input-device-access-unavailable"}"#.to_string(),
        recording_id: "recording-1".to_string(),
    }];

    let intervals =
        activity_intervals_from_events("mouse", "mouse-activity", &events, 0, 5_000, 1_000);

    assert_eq!(intervals.len(), 1);
    assert_eq!(intervals[0].state, "unknown");
    assert_eq!(intervals[0].start_offset_ms, 0);
    assert_eq!(intervals[0].end_offset_ms, 5_000);
}

#[test]
fn frame_delta_scores_changed_pixels() {
    let previous = vec![0; 100];
    let mut current = vec![0; 100];
    current.iter_mut().take(10).for_each(|pixel| *pixel = 255);

    assert_eq!(frame_delta_score(&previous, &current, 100), 0.1);
}

#[test]
fn merged_activity_prefers_active_then_idle_over_unknown() {
    let intervals = vec![
        planned_interval("keyboard", "unknown", 0, 5_000, None, None),
        planned_interval("frame-delta", "idle", 0, 2_000, Some(0.0), None),
        planned_interval("frame-delta", "active", 2_000, 3_000, Some(1.0), None),
        planned_interval("frame-delta", "idle", 3_000, 5_000, Some(0.0), None),
    ];

    let merged = merge_activity_intervals(&intervals, 0, 5_000);

    assert_eq!(
        merged
            .iter()
            .map(|interval| (
                interval.state.as_str(),
                interval.start_offset_ms,
                interval.end_offset_ms
            ))
            .collect::<Vec<_>>(),
        vec![
            ("idle", 0, 2_000),
            ("active", 2_000, 3_000),
            ("idle", 3_000, 5_000)
        ]
    );
}

#[test]
fn stale_cancel_does_not_overwrite_available_model() {
    let storage = storage::test_storage(PathBuf::new());
    let catalog = &LOCAL_WHISPER_CATALOG[2];

    storage
        .with_connection(|connection| {
            save_catalog_model(
                connection,
                catalog,
                ModelAssetPatch {
                    download_status: "available".into(),
                    downloaded_bytes: catalog.size_bytes,
                    local_path: Some("/tmp/ggml-small.bin".into()),
                    self_test_status: "passed".into(),
                    total_bytes: Some(catalog.size_bytes),
                    verification_status: "passed".into(),
                    ..ModelAssetPatch::missing()
                },
                10,
            )?;

            let notification =
                save_cancel_download_result(connection, catalog, None, false, None, 20)?;

            assert_eq!(notification.status, "warning");
            assert_eq!(notification.title, "Model download not running");

            let model = storage::get_ai_model_asset(connection, catalog.id)?
                .expect("model asset remains saved");
            assert_eq!(model.download_status, "available");
            assert_eq!(model.local_path.as_deref(), Some("/tmp/ggml-small.bin"));
            assert_eq!(model.verification_status, "passed");

            Ok(())
        })
        .expect("save stale cancel");
}

#[test]
fn cancel_persisted_download_records_partial_bytes() {
    let storage = storage::test_storage(PathBuf::new());
    let catalog = &LOCAL_WHISPER_CATALOG[2];

    storage
        .with_connection(|connection| {
            save_catalog_model(
                connection,
                catalog,
                ModelAssetPatch {
                    download_status: "downloading".into(),
                    downloaded_bytes: 10,
                    total_bytes: Some(catalog.size_bytes),
                    verification_status: "pending".into(),
                    ..ModelAssetPatch::missing()
                },
                10,
            )?;

            let notification =
                save_cancel_download_result(connection, catalog, None, true, Some(42), 20)?;

            assert_eq!(notification.status, "cancelled");
            assert_eq!(notification.title, "Model download cancelled");

            let model = storage::get_ai_model_asset(connection, catalog.id)?
                .expect("model asset remains saved");
            assert_eq!(model.download_status, "cancelled");
            assert_eq!(model.downloaded_bytes, 42);
            assert_eq!(
                model.error_code.as_deref(),
                Some("ai_model_download_cancelled")
            );

            Ok(())
        })
        .expect("cancel persisted download");
}

#[test]
fn complete_partial_file_can_be_recovered_before_ranged_download() {
    let path = test_temp_path("complete-partial").join("model.part");
    fs::create_dir_all(path.parent().expect("partial parent")).expect("create temp dir");
    let bytes = b"complete model bytes";
    fs::write(&path, bytes).expect("write partial");
    let sha1 = Box::leak(test_sha1(bytes).into_boxed_str());
    let catalog = CatalogModel {
        id: "test",
        label: "Test model",
        sha1,
        size_bytes: bytes.len() as i64,
    };

    assert_eq!(
        prepare_partial_download(&catalog, &path).expect("prepare partial"),
        PartialDownloadResume::Complete
    );

    delete_file_if_exists(&path).expect("delete partial");
}

#[test]
fn corrupt_complete_partial_file_restarts_from_zero() {
    let path = test_temp_path("corrupt-partial").join("model.part");
    fs::create_dir_all(path.parent().expect("partial parent")).expect("create temp dir");
    fs::write(&path, b"corrupt").expect("write partial");
    let sha1 = Box::leak(test_sha1(b"expected").into_boxed_str());
    let catalog = CatalogModel {
        id: "test",
        label: "Test model",
        sha1,
        size_bytes: 1,
    };

    assert_eq!(
        prepare_partial_download(&catalog, &path).expect("prepare partial"),
        PartialDownloadResume::Resume(0)
    );
    assert!(!path.exists());
}

#[test]
fn recovery_uses_verified_final_model_file() {
    let dir = test_temp_path("recovery-final");
    fs::create_dir_all(&dir).expect("create temp dir");
    let final_path = dir.join("model.bin");
    let partial_path = dir.join("model.part");
    let bytes = b"complete model bytes";
    fs::write(&final_path, bytes).expect("write final");
    let sha1 = Box::leak(test_sha1(bytes).into_boxed_str());
    let catalog = CatalogModel {
        id: "test",
        label: "Test model",
        sha1,
        size_bytes: bytes.len() as i64,
    };

    assert_eq!(
        recover_model_files(&catalog, &partial_path, &final_path).expect("recover files"),
        RecoveredModelFiles::VerifiedFinal
    );

    delete_file_if_exists(&final_path).expect("delete final");
}

#[test]
fn recovery_deletes_corrupt_final_and_resumes_partial() {
    let dir = test_temp_path("recovery-corrupt-final");
    fs::create_dir_all(&dir).expect("create temp dir");
    let final_path = dir.join("model.bin");
    let partial_path = dir.join("model.part");
    fs::write(&final_path, b"corrupt").expect("write final");
    fs::write(&partial_path, b"partial").expect("write partial");
    let sha1 = Box::leak(test_sha1(b"expected").into_boxed_str());
    let catalog = CatalogModel {
        id: "test",
        label: "Test model",
        sha1,
        size_bytes: 100,
    };

    assert_eq!(
        recover_model_files(&catalog, &partial_path, &final_path).expect("recover files"),
        RecoveredModelFiles::Partial(7)
    );
    assert!(!final_path.exists());

    delete_file_if_exists(&partial_path).expect("delete partial");
}

#[test]
fn recovery_reports_no_files_when_partial_is_missing() {
    let dir = test_temp_path("recovery-missing");
    fs::create_dir_all(&dir).expect("create temp dir");
    let final_path = dir.join("model.bin");
    let partial_path = dir.join("model.part");
    let sha1 = Box::leak(test_sha1(b"expected").into_boxed_str());
    let catalog = CatalogModel {
        id: "test",
        label: "Test model",
        sha1,
        size_bytes: 100,
    };

    assert_eq!(
        recover_model_files(&catalog, &partial_path, &final_path).expect("recover files"),
        RecoveredModelFiles::None
    );
}

#[test]
fn cancelled_finalized_download_moves_final_file_back_to_partial() {
    let dir = test_temp_path("rollback-finalized");
    fs::create_dir_all(&dir).expect("create temp dir");
    let partial_path = dir.join("model.part");
    let final_path = dir.join("model.bin");
    fs::write(&final_path, b"complete model bytes").expect("write final");

    rollback_finalized_model_file(&partial_path, &final_path).expect("rollback final file");

    assert!(!final_path.exists());
    assert_eq!(
        fs::read(&partial_path).expect("read restored partial"),
        b"complete model bytes"
    );
    delete_file_if_exists(&partial_path).expect("delete partial");
}

#[test]
fn cancelled_finalized_download_deletes_final_file_when_partial_exists() {
    let dir = test_temp_path("rollback-finalized-with-partial");
    fs::create_dir_all(&dir).expect("create temp dir");
    let partial_path = dir.join("model.part");
    let final_path = dir.join("model.bin");
    fs::write(&partial_path, b"existing partial").expect("write partial");
    fs::write(&final_path, b"complete model bytes").expect("write final");

    rollback_finalized_model_file(&partial_path, &final_path).expect("rollback final file");

    assert!(!final_path.exists());
    assert_eq!(
        fs::read(&partial_path).expect("read existing partial"),
        b"existing partial"
    );
    delete_file_if_exists(&partial_path).expect("delete partial");
}

#[test]
fn retry_and_cancel_actions_keep_model_id_in_payload() {
    let retry = model_download_retry_action("small");
    let cancel = model_download_cancel_action("small");

    assert_eq!(retry.handler, "model-download-retry");
    assert_eq!(retry.payload.get("modelId"), Some(&"small".to_string()));
    assert_eq!(cancel.handler, "model-download-cancel");
    assert_eq!(cancel.payload.get("modelId"), Some(&"small".to_string()));
}

#[test]
fn audio_chunk_plans_split_long_segments_by_source_and_preserve_pause_gap() {
    let dir = test_temp_path("chunk-plans");
    fs::create_dir_all(&dir).expect("create temp dir");
    let segment_1_path = dir.join("segment-1.mkv");
    let segment_2_path = dir.join("segment-2.mkv");
    fs::write(&segment_1_path, b"segment 1").expect("write segment 1");
    fs::write(&segment_2_path, b"segment 2").expect("write segment 2");
    let segment_1_duration_ms = (31 * 60 * 1_000) as i64;
    let segment_2_duration_ms = 5 * 60 * 1_000;
    let pause_after_segment_1_ms = 2 * 60 * 1_000;
    let segment_1 = test_segment(TestSegmentInput {
        desktop_audio_stream_index: Some(1),
        duration_ms: segment_1_duration_ms,
        id: "segment-1",
        microphone_audio_stream_index: Some(2),
        path: &segment_1_path,
        segment_index: 1,
        started_at_ms: 1_000,
        state: "finalized",
    });
    let segment_2 = test_segment(TestSegmentInput {
        desktop_audio_stream_index: Some(1),
        duration_ms: segment_2_duration_ms,
        id: "segment-2",
        microphone_audio_stream_index: None,
        path: &segment_2_path,
        segment_index: 2,
        started_at_ms: segment_1.ended_at_ms.expect("segment 1 ended") + pause_after_segment_1_ms,
        state: "finalized",
    });
    let failed_segment = test_segment(TestSegmentInput {
        desktop_audio_stream_index: Some(1),
        duration_ms: segment_2_duration_ms,
        id: "failed-segment",
        microphone_audio_stream_index: None,
        path: &segment_2_path,
        segment_index: 3,
        started_at_ms: segment_2.ended_at_ms.expect("segment 2 ended"),
        state: "failed",
    });

    let plans = audio_chunk_plans(&[segment_1.clone(), segment_2.clone(), failed_segment]);
    let desktop_segment_1 = plans
        .iter()
        .filter(|plan| plan.segment_id == "segment-1" && plan.source_role == "desktop-audio")
        .collect::<Vec<_>>();
    let microphone_segment_1 = plans
        .iter()
        .filter(|plan| plan.segment_id == "segment-1" && plan.source_role == "microphone")
        .collect::<Vec<_>>();
    let desktop_segment_2 = plans
        .iter()
        .filter(|plan| plan.segment_id == "segment-2" && plan.source_role == "desktop-audio")
        .collect::<Vec<_>>();

    assert_eq!(desktop_segment_1.len(), 3);
    assert_eq!(microphone_segment_1.len(), 3);
    assert_eq!(desktop_segment_1[0].start_offset_ms, 0);
    assert_eq!(desktop_segment_1[0].end_offset_ms, 15 * 60 * 1_000);
    assert_eq!(desktop_segment_1[2].start_offset_ms, 30 * 60 * 1_000);
    assert_eq!(desktop_segment_1[2].end_offset_ms, 31 * 60 * 1_000);
    assert_eq!(desktop_segment_2.len(), 1);
    assert_eq!(
        desktop_segment_2[0].start_offset_ms,
        segment_1_duration_ms + pause_after_segment_1_ms
    );
    assert_eq!(
        plans
            .iter()
            .filter(|plan| plan.segment_id == "failed-segment")
            .count(),
        0
    );
    assert_eq!(
        playback_offset_for_segment(&[segment_1, segment_2], "segment-2"),
        Some(segment_1_duration_ms + pause_after_segment_1_ms)
    );
}

#[test]
fn ffmpeg_audio_extract_args_include_slice_seek_and_duration() {
    let args = ffmpeg_audio_extract_args("/recording/segment.mkv", 2, 900_000, 123_456);

    assert_eq!(
        args,
        vec![
            "-hide_banner",
            "-v",
            "error",
            "-y",
            "-i",
            "/recording/segment.mkv",
            "-ss",
            "900.000",
            "-t",
            "123.456",
            "-map",
            "0:2",
            "-vn",
            "-ac",
            "1",
            "-ar",
            "16000",
            "-c:a",
            "pcm_s16le",
            "-f",
            "wav",
        ]
    );
}

#[test]
fn temp_audio_file_removes_wav_on_drop() {
    let path = test_temp_path("temp-audio-cleanup").join("chunk.wav");
    fs::create_dir_all(path.parent().expect("temp parent")).expect("create temp dir");
    fs::write(&path, b"private audio").expect("write temp audio");

    {
        let _temp_audio = TempAudioFile::new(path.clone());
        assert!(path.exists());
    }

    assert!(!path.exists());
}

#[test]
fn model_download_notification_update_prefers_clicked_notification() {
    let storage = storage::test_storage(PathBuf::new());

    storage
        .with_connection(|connection| {
            let started = save_model_notification(
                connection,
                "running",
                "model-download",
                "Model download started",
                "Whisper small will download from Hugging Face.",
                &[model_download_cancel_action("small")],
                10,
            )?;
            let cancelled = save_model_notification(
                connection,
                "cancelled",
                "model-download",
                "Model download cancelled",
                "Whisper small download was cancelled.",
                &[model_download_retry_action("small")],
                20,
            )?;
            storage::dismiss_notification(connection, &started.id, 25)?;

            let updated = save_model_download_notification(
                connection,
                ModelDownloadNotificationInput {
                    actions: &[model_download_retry_action("small")],
                    message: "Whisper small download was cancelled.",
                    model_id: "small",
                    preferred_notification_id: Some(&started.id),
                    status: "cancelled",
                    timestamp_ms: 30,
                    title: "Model download cancelled",
                },
            )?;

            assert_eq!(updated.id, started.id);
            assert_eq!(updated.dismissed_at_ms, None);
            assert_eq!(updated.created_at_ms, 10);
            assert_eq!(updated.updated_at_ms, 30);

            let rows = storage::list_notifications(connection)?;
            assert_eq!(rows.len(), 2);
            let untouched = rows
                .into_iter()
                .find(|row| row.id == cancelled.id)
                .expect("cancelled duplicate remains untouched");
            assert_eq!(untouched.updated_at_ms, 20);

            Ok(())
        })
        .expect("save notifications");
}

fn test_sha1(bytes: &[u8]) -> String {
    let mut hasher = Sha1::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

fn test_visual_image(width: usize, height: usize, white_pixels: &[(usize, usize)]) -> VisualImage {
    let mut pixels = vec![0; width * height * 4];
    for pixel in pixels.chunks_exact_mut(4) {
        pixel[3] = 255;
    }
    for (x, y) in white_pixels {
        let index = ((y * width) + x) * 4;
        pixels[index] = 255;
        pixels[index + 1] = 255;
        pixels[index + 2] = 255;
    }

    VisualImage {
        height,
        pixels,
        width,
    }
}

fn test_vlm_chunk(
    status: &str,
    error_code: Option<&str>,
    result_code: Option<&str>,
) -> ProcessingChunkRow {
    ProcessingChunkRow {
        attempt_count: 0,
        created_at_ms: 0,
        end_offset_ms: Some(1_000),
        error_code: error_code.map(str::to_string),
        id: Uuid::now_v7().to_string(),
        job_id: Uuid::now_v7().to_string(),
        progress: if status == "succeeded" { 1.0 } else { 0.0 },
        recording_id: Uuid::now_v7().to_string(),
        result_code: result_code.map(str::to_string),
        segment_id: Some(Uuid::now_v7().to_string()),
        source_kind: "vlm".to_string(),
        start_offset_ms: Some(0),
        status: status.to_string(),
        track_id: None,
        updated_at_ms: 0,
    }
}

struct TestVisualRecording {
    audio_track_id: String,
    job_id: String,
    ocr_chunk_id: String,
    recording_id: String,
    segment_id: String,
    video_track_id: String,
    vlm_chunk_id: String,
}

fn test_visual_recording(connection: &rusqlite::Connection) -> TestVisualRecording {
    let recording_id = Uuid::now_v7().to_string();
    let segment_id = Uuid::now_v7().to_string();
    let video_track_id = Uuid::now_v7().to_string();
    let audio_track_id = Uuid::now_v7().to_string();
    let job_id = Uuid::now_v7().to_string();

    storage::create_recording(
        connection,
        &storage::CreateRecordingInput {
            active_scene_id: None,
            created_at_ms: 1,
            default_fps: 1.0,
            id: recording_id.clone(),
            notes: None,
            output_root: None,
            quality_label: "balanced-ocr".to_string(),
            storage_dir: "/tmp/daydream-test".to_string(),
        },
    )
    .expect("create visual recording");
    storage::create_recording_segment(
        connection,
        &storage::CreateRecordingSegmentInput {
            file_path: "/tmp/daydream-test/segments/0001.mkv".to_string(),
            id: segment_id.clone(),
            recording_id: recording_id.clone(),
            segment_index: 1,
            started_at_ms: 1,
        },
    )
    .expect("create visual segment");
    storage::upsert_recording_track(
        connection,
        &UpsertRecordingTrackInput {
            codec: Some("h264".to_string()),
            created_at_ms: 1,
            id: video_track_id.clone(),
            kind: "screen".to_string(),
            label: "Display".to_string(),
            path: None,
            recording_id: recording_id.clone(),
            source_id: None,
        },
    )
    .expect("create video track");
    storage::upsert_recording_track(
        connection,
        &UpsertRecordingTrackInput {
            codec: Some("pcm_s16le".to_string()),
            created_at_ms: 1,
            id: audio_track_id.clone(),
            kind: "microphone".to_string(),
            label: "Microphone".to_string(),
            path: None,
            recording_id: recording_id.clone(),
            source_id: None,
        },
    )
    .expect("create audio track");
    storage::create_processing_job(
        connection,
        &CreateProcessingJobInput {
            created_at_ms: 1,
            error: None,
            id: job_id.clone(),
            kind: AI_PROCESSING_JOB_KIND.to_string(),
            recording_id: recording_id.clone(),
            status: "running".to_string(),
            updated_at_ms: 1,
        },
    )
    .expect("create visual job");

    let mut fixture = TestVisualRecording {
        audio_track_id,
        job_id,
        ocr_chunk_id: String::new(),
        recording_id,
        segment_id,
        video_track_id,
        vlm_chunk_id: String::new(),
    };
    fixture.ocr_chunk_id = create_test_visual_chunk(connection, &fixture, "ocr", "succeeded")
        .expect("create ocr chunk");
    fixture.vlm_chunk_id = create_test_visual_chunk(connection, &fixture, "vlm", "succeeded")
        .expect("create vlm chunk");

    fixture
}

fn create_test_visual_chunk(
    connection: &rusqlite::Connection,
    fixture: &TestVisualRecording,
    source_kind: &str,
    status: &str,
) -> rusqlite::Result<String> {
    let chunk = storage::create_processing_chunk(
        connection,
        &CreateProcessingChunkInput {
            attempt_count: 1,
            created_at_ms: 1,
            end_offset_ms: Some(10_000),
            error_code: None,
            id: Uuid::now_v7().to_string(),
            job_id: fixture.job_id.clone(),
            progress: if status == "succeeded" { 1.0 } else { 0.0 },
            recording_id: fixture.recording_id.clone(),
            result_code: None,
            segment_id: Some(fixture.segment_id.clone()),
            source_kind: source_kind.to_string(),
            start_offset_ms: Some(0),
            status: status.to_string(),
            track_id: None,
            updated_at_ms: 1,
        },
    )?;

    Ok(chunk.id)
}

fn create_test_ocr_region(
    connection: &rusqlite::Connection,
    fixture: &TestVisualRecording,
    chunk_id: &str,
    frame_timestamp_ms: i64,
    text: &str,
) -> rusqlite::Result<()> {
    storage::create_ocr_region(
        connection,
        &CreateVisualRegionInput {
            chunk_id: chunk_id.to_string(),
            confidence: Some(0.9),
            frame_timestamp_ms,
            id: Uuid::now_v7().to_string(),
            job_id: fixture.job_id.clone(),
            model: Some("tesseract".to_string()),
            observation_kind: None,
            prompt_version: None,
            provider: "tesseract".to_string(),
            recording_id: fixture.recording_id.clone(),
            region_height: 20,
            region_width: 30,
            region_x: 1,
            region_y: 2,
            source_frame_ref: format!("segment:test:offset:{frame_timestamp_ms}:frame:0"),
            text: text.to_string(),
        },
    )
}

fn create_test_vlm_observation(
    connection: &rusqlite::Connection,
    fixture: &TestVisualRecording,
    chunk_id: &str,
    frame_timestamp_ms: i64,
    text: &str,
) -> rusqlite::Result<()> {
    storage::create_vlm_observation(
        connection,
        &CreateVisualRegionInput {
            chunk_id: chunk_id.to_string(),
            confidence: Some(0.8),
            frame_timestamp_ms,
            id: Uuid::now_v7().to_string(),
            job_id: fixture.job_id.clone(),
            model: Some("vision".to_string()),
            observation_kind: Some("screen-observation".to_string()),
            prompt_version: Some("m2-test".to_string()),
            provider: "openai-compatible".to_string(),
            recording_id: fixture.recording_id.clone(),
            region_height: 20,
            region_width: 30,
            region_x: 1,
            region_y: 2,
            source_frame_ref: format!("segment:test:offset:{frame_timestamp_ms}:frame:0"),
            text: text.to_string(),
        },
    )
}

fn test_transcription_chunk(
    connection: &rusqlite::Connection,
) -> (ProcessingChunkRow, RecordingTrackRow) {
    let recording_id = Uuid::now_v7().to_string();
    let segment_id = Uuid::now_v7().to_string();
    let track_id = Uuid::now_v7().to_string();
    let job_id = Uuid::now_v7().to_string();
    let chunk_id = Uuid::now_v7().to_string();

    storage::create_recording(
        connection,
        &storage::CreateRecordingInput {
            active_scene_id: None,
            created_at_ms: 1,
            default_fps: 1.0,
            id: recording_id.clone(),
            notes: None,
            output_root: None,
            quality_label: "balanced-ocr".to_string(),
            storage_dir: "/tmp/daydream-test".to_string(),
        },
    )
    .expect("create recording");
    storage::create_recording_segment(
        connection,
        &storage::CreateRecordingSegmentInput {
            file_path: "/tmp/daydream-test/segments/0001.mkv".to_string(),
            id: segment_id.clone(),
            recording_id: recording_id.clone(),
            segment_index: 1,
            started_at_ms: 1,
        },
    )
    .expect("create segment");
    let track = storage::upsert_recording_track(
        connection,
        &UpsertRecordingTrackInput {
            codec: Some("pcm_s16le".to_string()),
            created_at_ms: 1,
            id: track_id.clone(),
            kind: "microphone".to_string(),
            label: "Microphone".to_string(),
            path: None,
            recording_id: recording_id.clone(),
            source_id: None,
        },
    )
    .expect("create track");
    storage::create_processing_job(
        connection,
        &CreateProcessingJobInput {
            created_at_ms: 1,
            error: None,
            id: job_id.clone(),
            kind: AI_PROCESSING_JOB_KIND.to_string(),
            recording_id: recording_id.clone(),
            status: "running".to_string(),
            updated_at_ms: 1,
        },
    )
    .expect("create job");
    let chunk = storage::create_processing_chunk(
        connection,
        &CreateProcessingChunkInput {
            attempt_count: 1,
            created_at_ms: 1,
            end_offset_ms: Some(10_000),
            error_code: None,
            id: chunk_id,
            job_id,
            progress: 0.0,
            recording_id,
            result_code: None,
            segment_id: Some(segment_id),
            source_kind: "audio".to_string(),
            start_offset_ms: Some(0),
            status: "running".to_string(),
            track_id: Some(track_id),
            updated_at_ms: 1,
        },
    )
    .expect("create chunk");

    (chunk, track)
}

fn test_verbose_transcription(
    start: f64,
    end: f64,
    text: &str,
    words: Vec<transcription::VerboseTranscriptionWord>,
) -> transcription::VerboseTranscription {
    transcription::VerboseTranscription {
        duration: Some(end),
        language: Some("en".to_string()),
        segments: vec![transcription::VerboseTranscriptionSegment {
            end,
            id: 0,
            start,
            text: text.to_string(),
        }],
        task: Some("transcribe".to_string()),
        text: text.to_string(),
        words,
    }
}

fn test_transcription_word(
    start: f64,
    end: f64,
    word: &str,
    probability: Option<f64>,
) -> transcription::VerboseTranscriptionWord {
    transcription::VerboseTranscriptionWord {
        end,
        probability,
        start,
        word: word.to_string(),
    }
}

fn test_temp_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("daydream-ai-{name}-{}", Uuid::now_v7()))
}

fn test_processing_chunk(status: &str) -> ProcessingChunkRow {
    ProcessingChunkRow {
        attempt_count: 0,
        created_at_ms: 1,
        end_offset_ms: Some(1_000),
        error_code: None,
        id: Uuid::now_v7().to_string(),
        job_id: Uuid::now_v7().to_string(),
        progress: if status == "succeeded" { 1.0 } else { 0.0 },
        recording_id: Uuid::now_v7().to_string(),
        result_code: None,
        segment_id: None,
        source_kind: "audio".to_string(),
        start_offset_ms: Some(0),
        status: status.to_string(),
        track_id: None,
        updated_at_ms: 1,
    }
}

struct TestSegmentInput<'a> {
    desktop_audio_stream_index: Option<i64>,
    duration_ms: i64,
    id: &'a str,
    microphone_audio_stream_index: Option<i64>,
    path: &'a Path,
    segment_index: i64,
    started_at_ms: i64,
    state: &'a str,
}

fn test_segment(input: TestSegmentInput<'_>) -> RecordingSegmentRow {
    RecordingSegmentRow {
        desktop_audio_stream_index: input.desktop_audio_stream_index,
        duration_ms: Some(input.duration_ms),
        ended_at_ms: Some(input.started_at_ms + input.duration_ms),
        error_code: None,
        ffmpeg_exit_code: Some(0),
        file_path: input.path.to_string_lossy().to_string(),
        id: input.id.to_string(),
        microphone_audio_stream_index: input.microphone_audio_stream_index,
        probe_json: None,
        recording_id: "recording-1".to_string(),
        segment_index: input.segment_index,
        started_at_ms: input.started_at_ms,
        state: input.state.to_string(),
        video_stream_index: Some(0),
    }
}
