use super::*;

fn fixture() -> (AppStorage, ProcessingChunkRow) {
    let storage = storage::test_storage(PathBuf::new());
    let chunk = storage
        .with_connection(|connection| {
            let (chunk, _) = crate::ai::tests::test_transcription_chunk(connection);
            storage::mark_recording_completed(connection, &chunk.recording_id, 30_001)?;
            connection.execute(
                "UPDATE recording_tracks SET kind='screen' WHERE id=?1",
                [&chunk.track_id],
            )?;
            connection.execute(
                "UPDATE processing_chunks SET source_kind='vlm',end_offset_ms=30000 WHERE id=?1",
                [&chunk.id],
            )?;
            storage::get_processing_chunk(connection, &chunk.id)
        })
        .unwrap()
        .unwrap();
    (storage, chunk)
}

fn frame(time: i64) -> FullFrame {
    FullFrame {
        path: PathBuf::new(),
        timestamp_ms: time,
        end_ms: time + 5000,
        width: 64,
        height: 64,
        pixel_hash: format!("synthetic-{time}"),
        source_ref: format!("synthetic-frame-{time}"),
    }
}

// Authored synthetic PNGs; only image content and five-second cadence matter.
fn images(start: i64, colors: &[u8]) -> FullFrameSet {
    let root =
        std::env::temp_dir().join(format!("daydream-observation-findings-{}", Uuid::now_v7()));
    fs::create_dir_all(&root).unwrap();
    let paths = colors
        .iter()
        .enumerate()
        .map(|(index, color)| {
            let path = root.join(format!("{index}.png"));
            let mut encoder = png::Encoder::new(fs::File::create(&path).unwrap(), 64, 64);
            encoder.set_color(png::ColorType::Rgb);
            encoder.set_depth(png::BitDepth::Eight);
            encoder
                .write_header()
                .unwrap()
                .write_image_data(&vec![*color; 64 * 64 * 3])
                .unwrap();
            path
        })
        .collect();
    super::super::super::ocr_documents::extraction::synthetic_frames(
        root,
        paths,
        start,
        start + colors.len() as i64 * 5000,
        0.2,
    )
}

#[test]
fn finding_explicit_observation_retry_overrides_reset_sampling_history() {
    let (storage, mut chunk) = fixture();
    let app = tauri::test::mock_app();
    app.manage(storage.clone());
    sampling::select(
        app.handle(),
        &chunk,
        &images(0, &[255; 6]),
        "synthetic-profile",
        true,
        &AtomicBool::new(false),
    )
    .unwrap();
    chunk.start_offset_ms = Some(30_000);
    chunk.end_offset_ms = Some(60_000);
    storage.with_connection(|connection| connection.execute("UPDATE processing_chunks SET start_offset_ms=30000,end_offset_ms=60000 WHERE id=?1", [&chunk.id]).map(|_| ())).unwrap();
    let frames = images(30_000, &[255, 0, 0, 0, 0, 0]);
    let original = sampling::select(
        app.handle(),
        &chunk,
        &frames,
        "synthetic-profile",
        true,
        &AtomicBool::new(false),
    )
    .unwrap();
    assert!(original[1].selected);
    sampling::discard(std::slice::from_ref(&chunk.job_id));
    let samples = sampling::select(
        app.handle(),
        &chunk,
        &frames,
        "synthetic-profile",
        true,
        &AtomicBool::new(false),
    )
    .unwrap();
    assert!(
        !samples[1].selected,
        "Restart selects 30 seconds and omits 35 seconds"
    );
    let state = queue_visual_frame_states(
        app.handle(),
        &storage,
        &chunk,
        &[visual_frame(&original[1].frame)],
    )
    .unwrap()
    .remove(0);
    storage
        .with_connection(|connection| {
            storage::update_visual_frame_processing_state_status(
                connection,
                &state.id,
                "failed",
                1.0,
                Some("ai_vision_remote_request_failed"),
                0,
                now_ms(),
            )?;
            connection.execute(
                "UPDATE processing_chunks SET status='failed' WHERE id=?1",
                [&chunk.id],
            )?;
            Ok(())
        })
        .unwrap();
    super::super::super::worker::retry_processing_units_for_action(
        &storage,
        &super::super::super::worker::RetryProcessingUnitsInput {
            job_id: chunk.job_id.clone(),
            chunk_id: Some(chunk.id.clone()),
            unit_id: Some(state.id),
            source_kind: Some("vlm".into()),
            use_current_settings: false,
        },
    )
    .unwrap();
    let selection = super::super::super::worker::load_unit_selection(&storage, &chunk.id).unwrap();
    assert_eq!(
        selection.frame_timestamps.as_deref(),
        Some([35_000].as_slice())
    );
    assert!(should_observe(
        Some(&samples),
        1,
        selection.frame_timestamps.as_deref()
    ));
    assert!(!should_observe(Some(&samples), 1, None));
    assert!(!should_observe(Some(&samples), 1, Some(&[40_000])));
    assert!(should_observe(Some(&samples), 0, None));
    assert!(should_observe(None, 0, None));
    sampling::discard(std::slice::from_ref(&chunk.job_id));
}

#[test]
fn observation_retry_with_current_settings_keeps_the_original_sampling_clock() {
    let (storage, chunk) = fixture();
    let app = tauri::test::mock_app();
    app.manage(storage.clone());
    let mut original = settings::default_settings();
    original.ai.processing.visual_sampling_mode = "full".into();
    original.ai.processing.frame_recognition_fps = 1.0;
    original.ai.processing.frame_recognition_sync_with_recording = false;
    storage
        .with_connection(|connection| {
            storage::set_processing_job_runtime(
                connection,
                &chunk.job_id,
                false,
                &serde_json::to_string(&original).unwrap(),
            )?;
            for (key, value) in [
                (
                    "ai.processing.visualSamplingMode",
                    serde_json::json!("adaptive-v1"),
                ),
                ("ai.processing.frameRecognitionFps", serde_json::json!(0.2)),
                (
                    "ai.processing.frameRecognitionSyncWithRecording",
                    serde_json::json!(true),
                ),
                ("ai.vision.provider", serde_json::json!("openai-compatible")),
                (
                    "ai.vision.baseUrl",
                    serde_json::json!("http://synthetic.invalid/v1"),
                ),
                ("ai.vision.model", serde_json::json!("new-synthetic-model")),
                ("ai.vision.authMode", serde_json::json!("none")),
                ("ai.vision.observationsEnabled", serde_json::json!(true)),
            ] {
                storage::save_app_setting(
                    connection,
                    &SaveAppSettingInput {
                        key: key.into(),
                        value_json: value.to_string(),
                        is_sensitive: false,
                        timestamp_ms: 1,
                    },
                )?;
            }
            Ok(())
        })
        .unwrap();
    let state = queue_visual_frame_states(
        app.handle(),
        &storage,
        &chunk,
        &[visual_frame(&frame(1000))],
    )
    .unwrap()
    .remove(0);
    storage
        .with_connection(|connection| {
            storage::update_visual_frame_processing_state_status(
                connection,
                &state.id,
                "failed",
                1.0,
                Some("ai_vision_remote_request_failed"),
                0,
                now_ms(),
            )?;
            storage::update_processing_chunk_status(
                connection,
                &chunk.id,
                "failed",
                1.0,
                None,
                now_ms(),
            )?;
            Ok(())
        })
        .unwrap();
    super::super::super::worker::retry_processing_units_for_action(
        &storage,
        &super::super::super::worker::RetryProcessingUnitsInput {
            job_id: chunk.job_id.clone(),
            chunk_id: Some(chunk.id.clone()),
            unit_id: Some(state.id.clone()),
            source_kind: Some("vlm".into()),
            use_current_settings: true,
        },
    )
    .unwrap();
    let chunk = super::super::super::worker::claim_next_processing_chunk(&storage, &chunk.job_id)
        .unwrap()
        .expect("the retried observation is ready for its next worker");
    let loaded = configuration::load_visual_settings(&storage, &chunk).unwrap();
    assert_eq!(
        loaded.ai.vision.model.as_deref(),
        Some("new-synthetic-model")
    );
    assert_eq!(loaded.ai.processing.visual_sampling_mode, "full");
    assert_eq!(loaded.ai.processing.frame_recognition_fps, 1.0);
    assert!(!loaded.ai.processing.frame_recognition_sync_with_recording);
    let selection = super::super::super::worker::load_unit_selection(&storage, &chunk.id).unwrap();
    assert_eq!(
        selection.frame_timestamps.as_deref(),
        Some([1000].as_slice())
    );
    // Use the loaded cadence to model the production extraction clock.
    let fps = loaded.ai.processing.frame_recognition_fps;
    let candidates = ObservationFrames::Cached(
        (0..30)
            .map(|index| frame((index as f64 * 1000.0 / fps).round() as i64))
            .collect(),
    );
    let pending = prepare_observation_frames(
        app.handle(),
        &storage,
        &chunk,
        &candidates,
        &selection.frame_timestamps,
        &BTreeSet::new(),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].1.timestamp_ms, 1000);
    assert_eq!(pending[0].2.id, state.id);
    let rows = storage
        .with_connection(|connection| {
            connection.query_row(
                "SELECT status,output_count FROM visual_frame_processing_states WHERE id=?1",
                [&state.id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
            )
        })
        .unwrap();
    assert_eq!(rows, ("queued".into(), 0));
}

#[test]
fn finding_shared_cached_observations_still_require_adaptive_sampling() {
    let (storage, chunk) = fixture();
    let config = VisionRequestConfig {
        route: None,
        storage: None,
        api_key: String::new(),
        base_url: "http://synthetic.invalid".into(),
        model: "synthetic".into(),
        provider: "openai-compatible".into(),
    };
    storage.with_connection(|connection| {
        for time in (0..30_000).step_by(5000) {
            connection.execute("INSERT INTO studio_visual_samples(job_id,recording_id,source_id,frame_ms,end_ms,fingerprint,pixel_hash,chunk_id,observation_json,context_start_ms,width,height,source_frame_ref) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?4,64,64,?10)", rusqlite::params![chunk.job_id,chunk.recording_id,chunk.track_id,time,time+5000,shared_inference::fingerprint(&config),format!("synthetic-{time}"),chunk.id,r#"{"status":"changed","text":"Synthetic screen change"}"#,format!("synthetic-frame-{time}")])?;
        }
        Ok(())
    }).unwrap();
    assert_eq!(
        cached_frames(&storage, &chunk, &config, 0.2, false)
            .unwrap()
            .unwrap()
            .len(),
        6
    );
    // Adaptive selection needs pixels, even when every model result already exists.
    assert!(cached_frames(&storage, &chunk, &config, 0.2, true)
        .unwrap()
        .is_none());
    let app = tauri::test::mock_app();
    app.manage(storage.clone());
    let samples = sampling::select(
        app.handle(),
        &chunk,
        &images(0, &[255, 0, 255, 0, 255, 0]),
        &shared_inference::fingerprint(&config),
        true,
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(
        samples
            .iter()
            .filter(|sample| sample.selected)
            .map(|sample| sample.frame.timestamp_ms)
            .collect::<Vec<_>>(),
        vec![0]
    );
    let mut next = chunk.clone();
    next.start_offset_ms = Some(30_000);
    next.end_offset_ms = Some(60_000);
    let samples = sampling::select(
        app.handle(),
        &next,
        &images(30_000, &[0, 255, 0, 255, 0, 255]),
        &shared_inference::fingerprint(&config),
        true,
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(
        samples
            .iter()
            .filter(|sample| sample.selected)
            .map(|sample| sample.frame.timestamp_ms)
            .collect::<Vec<_>>(),
        vec![30_000]
    );
    sampling::discard(std::slice::from_ref(&chunk.job_id));
}

#[test]
fn finding_slower_observation_retry_counts_retained_checkpoints() {
    let (storage, chunk) = fixture();
    let app = tauri::test::mock_app();
    app.manage(storage.clone());
    let mut succeeded = BTreeSet::new();
    for time in (0..30_000).step_by(1000) {
        let state = queue_visual_frame_states(
            app.handle(),
            &storage,
            &chunk,
            &[visual_frame(&frame(time))],
        )
        .unwrap()
        .remove(0);
        let success = time != 25_000;
        storage
            .with_connection(|connection| {
                storage::update_visual_frame_processing_state_status(
                    connection,
                    &state.id,
                    if success { "succeeded" } else { "queued" },
                    if success { 1.0 } else { 0.0 },
                    None,
                    0,
                    now_ms(),
                )
                .map(|_| ())
            })
            .unwrap();
        if success {
            succeeded.insert(time);
        }
    }
    let candidates = ObservationFrames::Cached((0..30_000).step_by(5000).map(frame).collect());
    let pending = prepare_observation_frames(
        app.handle(),
        &storage,
        &chunk,
        &candidates,
        &Some(vec![25_000]),
        &succeeded,
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(pending.len(), 1);
    storage.with_connection(|connection| {
        let counts: (i64, i64) = connection.query_row("SELECT total_units,(SELECT COUNT(*) FROM visual_frame_processing_states WHERE chunk_id=?1 AND status IN('succeeded','failed')) FROM processing_chunk_runtime WHERE chunk_id=?1", [&chunk.id], |row| Ok((row.get(0)?, row.get(1)?)))?;
        assert_eq!(counts, (30, 29));
        storage::update_visual_frame_processing_state_status(connection, &pending[0].2.id, "succeeded", 1.0, None, 1, now_ms())?;
        let completed: i64 = connection.query_row("SELECT COUNT(*) FROM visual_frame_processing_states WHERE chunk_id=?1 AND status='succeeded'", [&chunk.id], |row| row.get(0))?;
        assert_eq!(completed, 30);
        Ok(())
    }).unwrap();
}

#[test]
fn finding_observation_preparation_preserves_successes_and_cancellation() {
    let (storage, chunk) = fixture();
    let app = tauri::test::mock_app();
    app.manage(storage.clone());
    let frames = ObservationFrames::Cached((0..30_000).step_by(5000).map(frame).collect());
    let error = prepare_observation_frames(
        app.handle(),
        &storage,
        &chunk,
        &frames,
        &None,
        &BTreeSet::new(),
        &AtomicBool::new(true),
    )
    .err()
    .unwrap();
    assert_eq!(error.code, "ai_processing_cancelled");
    let pending = prepare_observation_frames(
        app.handle(),
        &storage,
        &chunk,
        &frames,
        &None,
        &BTreeSet::new(),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(pending.len(), 6);
    let mut succeeded = BTreeSet::new();
    for (_, frame, state) in pending {
        storage
            .with_connection(|connection| {
                storage::update_visual_frame_processing_state_status(
                    connection,
                    &state.id,
                    "succeeded",
                    1.0,
                    None,
                    1,
                    now_ms(),
                )
                .map(|_| ())
            })
            .unwrap();
        succeeded.insert(frame.timestamp_ms);
    }
    assert!(prepare_observation_frames(
        app.handle(),
        &storage,
        &chunk,
        &frames,
        &None,
        &succeeded,
        &AtomicBool::new(false)
    )
    .unwrap()
    .is_empty());
    storage.with_connection(|connection| {
        let counts: (i64, i64) = connection.query_row("SELECT total_units,(SELECT COUNT(*) FROM visual_frame_processing_states WHERE chunk_id=?1 AND status='succeeded' AND output_count=1) FROM processing_chunk_runtime WHERE chunk_id=?1", [&chunk.id], |row| Ok((row.get(0)?, row.get(1)?)))?;
        assert_eq!(counts, (6, 6));
        Ok(())
    }).unwrap();
}
