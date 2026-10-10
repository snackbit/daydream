use super::*;
use crate::storage::{OcrContentInput, OcrFrameInput};
mod reprocessing_regressions;

fn occurrence(chunk: &ProcessingChunkRow, index: i64, pixels: &str) -> OcrFrameInput {
    OcrFrameInput {
        id: Uuid::now_v7().to_string(),
        frame_timestamp_ms: index * 1000,
        sample_end_ms: (index + 1) * 1000,
        source_frame_ref: format!(
            "segment:{}:offset:{}:frame:{}",
            chunk.segment_id.as_deref().unwrap(),
            index * 1000,
            index
        ),
        source_track_id: "video-0".into(),
        width: 100,
        height: 100,
        config_fingerprint: "tesseract-v1".into(),
        engine: "tesseract".into(),
        provider: "tesseract".into(),
        model: Some("tesseract".into()),
        pixel_hash: pixels.into(),
        status: "succeeded".into(),
        error_code: None,
        content: None,
        cache_content_id: None,
        raw_regions: vec![],
    }
}
fn seed(storage: &AppStorage) -> ProcessingChunkRow {
    storage
        .with_connection(|connection| {
            let fixture = test_visual_recording(connection);
            storage::finalize_recording_segment(
                connection,
                &storage::FinalizeRecordingSegmentInput {
                    desktop_audio_stream_index: None,
                    duration_ms: 10_000,
                    ended_at_ms: 10_001,
                    id: fixture.segment_id,
                    microphone_audio_stream_index: None,
                    probe_json: None,
                    updated_at_ms: 10_001,
                    video_stream_index: Some(0),
                },
            )?;
            storage::update_processing_chunk_status(
                connection,
                &fixture.ocr_chunk_id,
                "running",
                0.0,
                None,
                20_000,
            )
        })
        .unwrap()
}

#[test]
fn reprocessing_failed_frames_keep_prior_text_and_published_frames_survive_cancellation() {
    let storage = storage::test_storage(PathBuf::new());
    let old = seed(&storage);
    for index in 0..2 {
        let mut frame = occurrence(&old, index, "old pixels");
        frame.content = Some(OcrContentInput {
            text: format!("prior frame {index}"),
            blocks: vec![],
        });
        assert!(storage
            .with_connection_mut(|c| storage::save_ocr_frame_result(c, &old, &frame))
            .unwrap());
    }
    let child=storage.with_connection(|c|{
        let job=storage::create_processing_job(c,&CreateProcessingJobInput{id:Uuid::now_v7().to_string(),recording_id:old.recording_id.clone(),kind:"ai-processing".into(),status:"running".into(),created_at_ms:30000,updated_at_ms:30000,error:None})?;
        let child=storage::create_processing_chunk(c,&CreateProcessingChunkInput{id:Uuid::now_v7().to_string(),job_id:job.id,recording_id:old.recording_id.clone(),source_kind:"ocr".into(),segment_id:old.segment_id.clone(),track_id:None,start_offset_ms:Some(0),end_offset_ms:Some(2000),attempt_count:0,created_at_ms:30000,updated_at_ms:30000,status:"running".into(),progress:0.0,error_code:None,result_code:None})?;
        let group=Uuid::now_v7().to_string();
        c.execute("INSERT INTO processing_replacement_groups(id,recording_id,job_id,source_track_id,source_kind,start_ms,end_ms) VALUES(?1,?2,?3,'video-0','ocr',0,2000)",rusqlite::params![group,child.recording_id,child.job_id])?;
        c.execute("INSERT INTO processing_replacement_chunks(chunk_id,group_id) VALUES(?1,?2)",rusqlite::params![child.id,group])?;
        Ok(child)
    }).unwrap();
    let read = |timestamp_ms| {
        storage
            .with_connection(|c| {
                storage::get_ocr_document(
                    c,
                    &GetOcrDocumentInput {
                        recording_id: old.recording_id.clone(),
                        source_track_id: "video-0".into(),
                        timestamp_ms,
                        document_id: None,
                    },
                )
            })
            .unwrap()
            .document
            .unwrap()
    };
    assert_eq!(read(0).entry.text, "prior frame 0");
    let mut failure = occurrence(&child, 0, "changed pixels");
    failure.status = "failed".into();
    failure.error_code = Some("synthetic_failure".into());
    assert!(storage
        .with_connection_mut(|c| storage::save_ocr_frame_result(c, &child, &failure))
        .unwrap());
    assert_eq!(read(0).entry.text, "prior frame 0");
    let mut success = occurrence(&child, 1, "changed pixels");
    success.content = Some(OcrContentInput {
        text: "replacement frame 1".into(),
        blocks: vec![],
    });
    assert!(storage
        .with_connection_mut(|c| storage::save_ocr_frame_result(c, &child, &success))
        .unwrap());
    assert_eq!(read(1000).entry.text, "replacement frame 1");
    storage
        .with_connection(|c| {
            storage::update_processing_chunk_status(c, &child.id, "cancelled", 0.5, None, 31000)?;
            storage::update_processing_job_status(c, &child.job_id, "cancelled", None, 31000)?;
            Ok(())
        })
        .unwrap();
    assert_eq!(read(0).entry.text, "prior frame 0");
    assert_eq!(read(1000).entry.text, "replacement frame 1");
}

#[test]
fn unchanged_frames_reuse_calls_but_keep_distinct_timestamped_occurrences() {
    tauri::async_runtime::block_on(async {
        let storage = storage::test_storage(PathBuf::new());
        let chunk = seed(&storage);
        let mut calls = 0;
        for index in 0..3 {
            let mut input = occurrence(&chunk, index, "identical-pixels");
            let content = infer_or_reuse_frame(&storage, &chunk, &mut input, || async {
                calls += 1;
                Ok(OcrContentInput {
                    text: "Synthetic meeting".into(),
                    blocks: vec![],
                })
            })
            .await
            .unwrap();
            input.content = Some(content);
            assert_eq!(input.cache_content_id.is_some(), index > 0);
            assert!(storage
                .with_connection_mut(|connection| storage::save_ocr_frame_result(
                    connection, &chunk, &input
                ))
                .unwrap());
        }
        assert_eq!(calls, 1);
        let count=storage.with_connection(|connection|connection.query_row("SELECT COUNT(DISTINCT frame_timestamp_ms) FROM ocr_frame_occurrences WHERE chunk_id=?1",[&chunk.id],|row|row.get::<_,i64>(0))).unwrap();
        assert_eq!(count, 3);
        let mut changed = occurrence(&chunk, 3, "one-pixel-changed");
        let result = infer_or_reuse_frame(&storage, &chunk, &mut changed, || async {
            calls += 1;
            Err(ai_error(
                "ai_ocr_vision_response_invalid",
                "synthetic provider failure",
            ))
        })
        .await;
        assert!(result.is_err());
        assert!(changed.cache_content_id.is_none());
        assert_eq!(calls, 2);
    });
}

#[test]
fn redaction_during_inference_rejects_result_and_prevents_cache_reuse() {
    tauri::async_runtime::block_on(async {
        let storage = storage::test_storage(PathBuf::new());
        let chunk = seed(&storage);
        let mut input = occurrence(&chunk, 0, "pixels");
        let result = infer_or_reuse_frame(&storage, &chunk, &mut input, || async {
            storage
                .with_connection(|connection| {
                    storage::create_redaction(
                        connection,
                        &storage::CreateRedactionInput {
                            created_at_ms: 21_000,
                            end_offset_ms: 1000,
                            id: Uuid::now_v7().to_string(),
                            reason: None,
                            recording_id: chunk.recording_id.clone(),
                            start_offset_ms: 0,
                            track_ids_json: "[\"video-0\"]".into(),
                        },
                    )
                })
                .unwrap();
            Ok(OcrContentInput {
                text: "synthetic hidden result".into(),
                blocks: vec![],
            })
        })
        .await
        .unwrap_err();
        assert_eq!(result.code, "ai_processing_cancelled");
        let result = infer_or_reuse_frame(&storage, &chunk, &mut input, || async {
            panic!("redacted media reached provider")
        })
        .await
        .unwrap_err();
        assert_eq!(result.code, "ai_processing_cancelled");
    });
}

#[test]
fn normal_visual_query_skips_the_raw_word_table_when_disabled() {
    let storage = storage::test_storage(PathBuf::new());
    storage
        .with_connection(|connection| connection.execute_batch("DROP TABLE ocr_regions;"))
        .unwrap();
    assert!(
        visual_observations_for_recording_with_regions(&storage, "synthetic", false)
            .unwrap()
            .ocr_regions
            .is_empty()
    );
    assert!(visual_observations_for_recording_with_regions(&storage, "synthetic", true).is_err());
}

#[test]
fn review_explicit_reprocess_pins_current_engine_and_safe_source_and_queues_only_ocr() {
    tauri::async_runtime::block_on(async {
        let app = tauri::test::mock_app();
        let storage = storage::test_storage(PathBuf::new());
        let path = std::env::temp_dir().join(format!("daydream-ocr-plan-{}.mkv", Uuid::now_v7()));
        fs::write(&path, b"synthetic source metadata").unwrap();
        let hidden_source = Uuid::now_v7().to_string();
        let safe_source = Uuid::now_v7().to_string();
        let recording_id = storage
            .with_connection(|connection| {
                let fixture = test_visual_recording(connection);
                storage::finalize_recording_segment(
                    connection,
                    &storage::FinalizeRecordingSegmentInput {
                        desktop_audio_stream_index: Some(1),
                        duration_ms: 10_000,
                        ended_at_ms: 10_001,
                        id: fixture.segment_id.clone(),
                        microphone_audio_stream_index: None,
                        probe_json: None,
                        updated_at_ms: 10_001,
                        video_stream_index: Some(0),
                    },
                )?;
                connection.execute(
                    "UPDATE recording_segments SET file_path=?2 WHERE id=?1",
                    rusqlite::params![fixture.segment_id, path.to_string_lossy()],
                )?;
                storage::mark_recording_completed(connection, &fixture.recording_id, 10_001)?;
                let sources =
                    [(&hidden_source, "hidden"), (&safe_source, "safe")].map(|(track, source)| {
                        storage::RecordingVideoSource {
                            track_id: track.clone(),
                            source_id: source.into(),
                            label: source.into(),
                            width: 4,
                            height: 2,
                            config_json: "{}".into(),
                        }
                    });
                storage::save_recording_video_sources(
                    connection,
                    &fixture.recording_id,
                    &sources,
                    1,
                )?;
                storage::save_segment_video_streams(
                    connection,
                    &fixture.segment_id,
                    &sources,
                    1.0,
                )?;
                storage::create_redaction(
                    connection,
                    &storage::CreateRedactionInput {
                        id: Uuid::now_v7().to_string(),
                        recording_id: fixture.recording_id.clone(),
                        start_offset_ms: 0,
                        end_offset_ms: 10_000,
                        reason: None,
                        created_at_ms: 10_001,
                        track_ids_json: serde_json::to_string(std::slice::from_ref(&hidden_source))
                            .unwrap(),
                    },
                )?;
                for (key, value) in [
                    ("ai.ocr.engine", serde_json::json!("vision")),
                    ("ai.vision.provider", serde_json::json!("openai-compatible")),
                    ("ai.vision.baseUrl", serde_json::json!("http://127.0.0.1:9")),
                    ("ai.vision.model", serde_json::json!("synthetic-model")),
                    (
                        "ai.vision.apiKeyRef",
                        serde_json::json!("ai.secret.vision.synthetic"),
                    ),
                    ("ai.vision.observationsEnabled", serde_json::json!(false)),
                ] {
                    storage::save_app_setting(
                        connection,
                        &SaveAppSettingInput {
                            key: key.into(),
                            value_json: value.to_string(),
                            is_sensitive: false,
                            timestamp_ms: 20_000,
                        },
                    )?;
                }
                Ok(fixture.recording_id)
            })
            .unwrap();
        app.manage(storage);
        let storage = app.state::<AppStorage>();
        assert!(
            reprocess_recording_ocr(app.handle(), &storage, &recording_id)
                .await
                .is_err(),
            "Active processing must finish before admitting a replacement pass"
        );
        storage
            .with_connection(|connection| {
                for job in storage::list_processing_jobs(connection, Some(&recording_id))? {
                    storage::update_processing_job_status(
                        connection,
                        &job.id,
                        "succeeded",
                        None,
                        20000,
                    )?;
                }
                Ok(())
            })
            .unwrap();
        let job = reprocess_recording_ocr(app.handle(), &storage, &recording_id)
            .await
            .unwrap();
        assert_eq!(job.kind, "ocr");
        let chunks = storage
            .with_connection(|connection| storage::list_processing_chunks(connection, &job.id))
            .unwrap();
        assert!(!chunks.is_empty());
        assert!(chunks.iter().all(|chunk| chunk.source_kind == "ocr"));
        for chunk in &chunks {
            assert_eq!(
                chunk.track_id.as_deref(),
                Some(safe_source.as_str()),
                "OCR reprocessing must retain the unredacted source UUID"
            );
            let selected = storage
                .with_connection(|connection| {
                    let segment =
                        storage::list_recording_segments(connection, &recording_id)?.remove(0);
                    processing::source_segment(connection, chunk, segment)
                })
                .unwrap();
            assert_eq!(
                selected.video_stream_index,
                Some(1),
                "Never substitute the redacted first stream"
            );
            let mut old_invalid_chunk = chunk.clone();
            old_invalid_chunk.track_id = None;
            assert!(
                storage
                    .with_connection(|connection| {
                        let segment =
                            storage::list_recording_segments(connection, &recording_id)?.remove(0);
                        processing::source_segment(connection, &old_invalid_chunk, segment)
                    })
                    .is_err(),
                "Already-queued independent-source work without an ID must fail before extraction"
            );
        }
        let pinned = storage
            .with_connection(|connection| {
                storage::get_processing_job_ocr_config(connection, &job.id)
            })
            .unwrap()
            .unwrap();
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&pinned).unwrap()["engine"],
            "vision"
        );
        assert!(
            reprocess_recording_ocr(app.handle(), &storage, &recording_id)
                .await
                .is_err(),
            "duplicate active OCR jobs must not race"
        );
        fs::remove_file(path).unwrap();
    });
}

#[test]
fn resumed_and_retried_ocr_reuses_paid_success_calls_with_fresh_provenance() {
    tauri::async_runtime::block_on(async {
        for restart in ["paused", "failed"] {
            let storage = storage::test_storage(PathBuf::new());
            let chunk = seed(&storage);
            let mut calls = 0;
            let mut first = occurrence(&chunk, 0, "paid-success-pixels");
            first.content = Some(
                infer_or_reuse_frame(&storage, &chunk, &mut first, || async {
                    calls += 1;
                    Ok(OcrContentInput {
                        text: "Synthetic stable meeting".into(),
                        blocks: vec![],
                    })
                })
                .await
                .unwrap(),
            );
            storage
                .with_connection_mut(|connection| {
                    storage::save_ocr_frame_result(connection, &chunk, &first)
                })
                .unwrap();
            let mut second = occurrence(&chunk, 1, "different-frame-pixels");
            assert!(
                infer_or_reuse_frame(&storage, &chunk, &mut second, || async {
                    calls += 1;
                    Err(ai_error(
                        "ai_ocr_vision_request_failed",
                        "Synthetic interrupted provider.",
                    ))
                })
                .await
                .is_err()
            );
            second.status = "failed".into();
            second.error_code = Some("ai_ocr_vision_request_failed".into());
            storage
                .with_connection_mut(|connection| {
                    storage::save_ocr_frame_result(connection, &chunk, &second)
                })
                .unwrap();
            storage
                .with_connection(|connection| {
                    storage::update_processing_job_status(
                        connection,
                        &chunk.job_id,
                        restart,
                        None,
                        30_000,
                    )?;
                    storage::update_processing_chunk_status(
                        connection,
                        &chunk.id,
                        if restart == "paused" {
                            "queued"
                        } else {
                            "failed"
                        },
                        0.0,
                        None,
                        30_000,
                    )?;
                    Ok(())
                })
                .unwrap();
            if restart == "paused" {
                update_processing_job_for_action(&storage, &chunk.job_id, "queued").unwrap();
            } else {
                retry_processing_job_for_action(&storage, &chunk.job_id).unwrap();
            }
            let current = storage
                .with_connection(|connection| {
                    storage::update_processing_job_status(
                        connection,
                        &chunk.job_id,
                        "running",
                        None,
                        31_000,
                    )?;
                    storage::claim_processing_chunk(connection, &chunk.id, 0.0, 31_000)
                })
                .unwrap()
                .unwrap();
            assert!(current.attempt_count > chunk.attempt_count);
            let selection = GetOcrDocumentInput {
                recording_id: chunk.recording_id.clone(),
                source_track_id: "video-0".into(),
                timestamp_ms: 0,
                document_id: None,
            };
            assert!(
                storage
                    .with_connection(|connection| storage::get_ocr_document(connection, &selection))
                    .unwrap()
                    .document
                    .is_none(),
                "old attempt must not become current by a cache hit"
            );
            let mut resumed = occurrence(&current, 0, "paid-success-pixels");
            let content = infer_or_reuse_frame(&storage, &current, &mut resumed, || async {
                calls += 1;
                Ok(OcrContentInput {
                    text: "Synthetic stable meeting".into(),
                    blocks: vec![],
                })
            })
            .await
            .unwrap();
            assert_eq!(
                calls, 2,
                "{restart} repeated an already paid successful provider call"
            );
            assert!(resumed.cache_content_id.is_some());
            assert!(storage
                .with_connection(|connection| storage::get_ocr_document(connection, &selection))
                .unwrap()
                .document
                .is_none());
            resumed.content = Some(content);
            assert!(storage
                .with_connection_mut(|connection| storage::save_ocr_frame_result(
                    connection, &current, &resumed
                ))
                .unwrap());
            let visible = storage
                .with_connection(|connection| storage::get_ocr_document(connection, &selection))
                .unwrap()
                .document
                .unwrap();
            assert_eq!(visible.entry.id, resumed.id);
            assert!(visible.entry.cache_hit);
            let mut retried_failure = occurrence(&current, 1, "different-frame-pixels");
            let content =
                infer_or_reuse_frame(&storage, &current, &mut retried_failure, || async {
                    calls += 1;
                    Ok(OcrContentInput {
                        text: "Synthetic recovered chat".into(),
                        blocks: vec![],
                    })
                })
                .await
                .unwrap();
            assert!(
                retried_failure.cache_content_id.is_none(),
                "failed recognition must not be reused"
            );
            assert_eq!(calls, 3);
            retried_failure.content = Some(content);
            assert!(storage
                .with_connection_mut(|connection| storage::save_ocr_frame_result(
                    connection,
                    &current,
                    &retried_failure
                ))
                .unwrap());
        }
    });
}
