use super::*;

fn replacement(
    storage: &AppStorage,
    old: &ProcessingChunkRow,
    start: i64,
    end: i64,
    created: i64,
) -> ProcessingChunkRow {
    storage.with_connection(|c| {
        let job = storage::create_processing_job(c, &CreateProcessingJobInput { id: Uuid::now_v7().to_string(), recording_id: old.recording_id.clone(), kind: AI_PROCESSING_JOB_KIND.into(), status: "running".into(), created_at_ms: created, updated_at_ms: created, error: None })?;
        let chunk = storage::create_processing_chunk(c, &CreateProcessingChunkInput { id: Uuid::now_v7().to_string(), job_id: job.id, recording_id: old.recording_id.clone(), source_kind: "ocr".into(), segment_id: old.segment_id.clone(), track_id: None, start_offset_ms: Some(start), end_offset_ms: Some(end), attempt_count: 0, created_at_ms: created, updated_at_ms: created, status: "running".into(), progress: 0.0, error_code: None, result_code: None })?;
        let group = Uuid::now_v7().to_string();
        c.execute("INSERT INTO processing_replacement_groups(id,recording_id,job_id,source_track_id,source_kind,start_ms,end_ms) VALUES(?1,?2,?3,'video-0','ocr',?4,?5)",rusqlite::params![group,chunk.recording_id,chunk.job_id,start,end])?;
        c.execute("INSERT INTO processing_replacement_chunks(chunk_id,group_id) VALUES(?1,?2)",rusqlite::params![chunk.id,group])?;
        Ok(chunk)
    }).unwrap()
}

fn read(storage: &AppStorage, chunk: &ProcessingChunkRow, timestamp_ms: i64) -> OcrDocument {
    storage
        .with_connection(|c| {
            storage::get_ocr_document(
                c,
                &GetOcrDocumentInput {
                    recording_id: chunk.recording_id.clone(),
                    source_track_id: "video-0".into(),
                    timestamp_ms,
                    document_id: None,
                },
            )
        })
        .unwrap()
        .document
        .expect("retained OCR coverage")
}

#[test]
fn reprocessing_unchanged_ocr_reuses_the_last_cached_occurrence() {
    tauri::async_runtime::block_on(async {
        let storage = storage::test_storage(PathBuf::new());
        let old = seed(&storage);
        let mut first = occurrence(&old, 0, "unchanged pixels");
        first.content = Some(OcrContentInput {
            text: "cached text".into(),
            blocks: vec![],
        });
        storage
            .with_connection_mut(|c| storage::save_ocr_frame_result(c, &old, &first))
            .unwrap();
        let child = replacement(&storage, &old, 0, 1000, 30000);
        let mut next = occurrence(&child, 0, "unchanged pixels");
        next.content = Some(
            infer_or_reuse_frame(&storage, &child, &mut next, || async {
                panic!("unchanged content must reuse its cache")
            })
            .await
            .unwrap(),
        );
        assert!(next.cache_content_id.is_some());
        assert!(storage
            .with_connection_mut(|c| storage::save_ocr_frame_result(c, &child, &next))
            .unwrap());
        let visible = read(&storage, &child, 500);
        assert_eq!(visible.entry.id, next.id);
        assert_eq!(visible.entry.text, "cached text");
        assert!(visible.entry.cache_hit);
    });
}

#[test]
fn reprocessing_ocr_retains_both_sides_of_shorter_samples_and_their_provenance() {
    for (start, end) in [(0, 500), (250, 750), (500, 1000)] {
        let storage = storage::test_storage(PathBuf::new());
        let old = seed(&storage);
        // Published output from an earlier cancelled pass must stay visible too.
        let prior = replacement(&storage, &old, 0, 1000, 30000);
        let mut first = occurrence(&prior, 0, "prior pixels");
        let raw_id = Uuid::now_v7().to_string();
        first.content = Some(OcrContentInput {
            text: "prior text".into(),
            blocks: vec![storage::OcrBlockInput {
                id: "block".into(),
                order: 0,
                text: "prior text".into(),
                confidence: None,
                region: None,
                raw_region_ids: vec![],
                words: vec![storage::OcrWordInput {
                    id: "word".into(),
                    hierarchy: None,
                    text: "prior text".into(),
                    confidence: None,
                    region: Some(VisualRegion {
                        x: 0,
                        y: 0,
                        width: 10,
                        height: 10,
                    }),
                    raw_region_id: None,
                }],
            }],
        });
        first.raw_regions.push(CreateVisualRegionInput {
            id: raw_id.clone(),
            recording_id: prior.recording_id.clone(),
            frame_timestamp_ms: 0,
            source_frame_ref: first.source_frame_ref.clone(),
            region_x: 0,
            region_y: 0,
            region_width: 10,
            region_height: 10,
            text: "prior text".into(),
            confidence: None,
            provider: "tesseract".into(),
            model: None,
            job_id: prior.job_id.clone(),
            chunk_id: prior.id.clone(),
            observation_kind: None,
            prompt_version: None,
        });
        storage
            .with_connection_mut(|c| storage::save_ocr_frame_result(c, &prior, &first))
            .unwrap();
        storage
            .with_connection(|c| {
                storage::update_processing_job_status(c, &prior.job_id, "cancelled", None, 31000)?;
                storage::update_processing_chunk_status(
                    c,
                    &prior.id,
                    "cancelled",
                    0.5,
                    None,
                    31000,
                )?;
                Ok(())
            })
            .unwrap();
        let child = replacement(
            &storage,
            &old,
            start,
            if start == 0 { 1000 } else { end },
            40000,
        );
        let mut next = occurrence(&child, 0, "new pixels");
        next.frame_timestamp_ms = start;
        next.sample_end_ms = end;
        next.content = Some(OcrContentInput {
            text: "replacement text".into(),
            blocks: vec![],
        });
        storage
            .with_connection_mut(|c| storage::save_ocr_frame_result(c, &child, &next))
            .unwrap();
        for timestamp in [0, 249, 250, 499, 500, 749, 750, 999] {
            let document = read(&storage, &child, timestamp);
            if (start..end).contains(&timestamp) {
                assert_eq!(document.entry.text, "replacement text");
            } else {
                assert_eq!(document.entry.text, "prior text");
                assert_eq!(document.entry.source_frame_ref, first.source_frame_ref);
                assert_eq!(
                    document.blocks[0].words[0].raw_region_id.as_deref(),
                    Some(raw_id.as_str())
                );
            }
        }
        let retained_raw = storage
            .with_connection(|c| {
                c.query_row(
                    "SELECT COUNT(*) FROM ocr_regions WHERE id=?1",
                    [&raw_id],
                    |r| r.get::<_, i64>(0),
                )
            })
            .unwrap();
        assert_eq!(
            retained_raw, 1,
            "retained intervals must retain their source links"
        );
        if end == 500 {
            let mut failed = occurrence(&child, 0, "failed pixels");
            failed.frame_timestamp_ms = 500;
            failed.sample_end_ms = 1000;
            failed.status = "failed".into();
            // The next cadence sample belongs to the same full-scope pass.
            storage
                .with_connection_mut(|c| storage::save_ocr_frame_result(c, &child, &failed))
                .unwrap();
            assert_eq!(read(&storage, &child, 750).entry.text, "prior text");
        }
    }
}

#[test]
fn reprocessing_ocr_retry_ignores_cancelled_unpublished_passes_but_protects_published_frames() {
    for published in [false, true] {
        let storage = storage::test_storage(PathBuf::new());
        let old = seed(&storage);
        let earlier = replacement(&storage, &old, 0, 1000, 30000);
        storage
            .with_connection(|c| {
                storage::update_processing_chunk_status(
                    c,
                    &earlier.id,
                    "failed",
                    0.0,
                    Some("synthetic_failure"),
                    31000,
                )?;
                storage::update_processing_job_status(c, &earlier.job_id, "failed", None, 31000)?;
                Ok(())
            })
            .unwrap();
        let newer = replacement(&storage, &old, 0, 1000, 40000);
        if published {
            let mut frame = occurrence(&newer, 0, "newer pixels");
            frame.content = Some(OcrContentInput {
                text: "newer published text".into(),
                blocks: vec![],
            });
            storage
                .with_connection_mut(|c| storage::save_ocr_frame_result(c, &newer, &frame))
                .unwrap();
        }
        storage
            .with_connection(|c| {
                storage::update_processing_chunk_status(
                    c,
                    &newer.id,
                    "cancelled",
                    0.0,
                    None,
                    41000,
                )?;
                storage::update_processing_job_status(c, &newer.job_id, "cancelled", None, 41000)?;
                Ok(())
            })
            .unwrap();
        retry_processing_job_for_action(&storage, &earlier.job_id).unwrap();
        let current = storage
            .with_connection(|c| {
                storage::update_processing_job_status(c, &earlier.job_id, "running", None, 42000)?;
                storage::claim_processing_chunk(c, &earlier.id, 0.0, 42000)
            })
            .unwrap()
            .unwrap();
        let mut frame = occurrence(&current, 0, "recovered pixels");
        frame.content = Some(OcrContentInput {
            text: "recovered text".into(),
            blocks: vec![],
        });
        let result =
            storage.with_connection_mut(|c| storage::save_ocr_frame_result(c, &current, &frame));
        if published {
            assert!(result.is_err());
            assert_eq!(
                read(&storage, &newer, 500).entry.text,
                "newer published text"
            );
        } else {
            assert!(result.unwrap());
            assert_eq!(read(&storage, &earlier, 500).entry.text, "recovered text");
        }
    }
}

#[test]
fn reprocessing_ocr_interval_retention_does_not_expose_redacted_prior_frames() {
    let storage = storage::test_storage(PathBuf::new());
    let old = seed(&storage);
    let mut first = occurrence(&old, 0, "prior pixels");
    first.content = Some(OcrContentInput {
        text: "redacted source text".into(),
        blocks: vec![],
    });
    storage
        .with_connection_mut(|c| storage::save_ocr_frame_result(c, &old, &first))
        .unwrap();
    storage
        .with_connection(|c| {
            storage::create_redaction(
                c,
                &storage::CreateRedactionInput {
                    id: Uuid::now_v7().to_string(),
                    recording_id: old.recording_id.clone(),
                    start_offset_ms: 0,
                    end_offset_ms: 250,
                    track_ids_json: "[\"video-0\"]".into(),
                    reason: None,
                    created_at_ms: 30000,
                },
            )?;
            Ok(())
        })
        .unwrap();
    let child = replacement(&storage, &old, 500, 750, 40000);
    let mut next = occurrence(&child, 0, "new pixels");
    next.frame_timestamp_ms = 500;
    next.sample_end_ms = 750;
    next.content = Some(OcrContentInput {
        text: "eligible replacement text".into(),
        blocks: vec![],
    });
    storage
        .with_connection_mut(|c| storage::save_ocr_frame_result(c, &child, &next))
        .unwrap();
    assert_eq!(
        read(&storage, &child, 600).entry.text,
        "eligible replacement text"
    );
    for timestamp_ms in [300, 800] {
        assert!(
            storage
                .with_connection(|c| storage::get_ocr_document(
                    c,
                    &GetOcrDocumentInput {
                        recording_id: old.recording_id.clone(),
                        source_track_id: "video-0".into(),
                        timestamp_ms,
                        document_id: None
                    }
                ))
                .unwrap()
                .document
                .is_none(),
            "retaining an interval must not change original privacy eligibility"
        );
    }
}
