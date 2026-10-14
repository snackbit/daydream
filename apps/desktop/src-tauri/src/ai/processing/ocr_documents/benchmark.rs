//! Opt-in production-adapter benchmark. Only generated screens enter processing.
use super::*;
use crate::recall::{embedding::ManagedEmbeddingProvider, index, search};

#[test]
#[ignore = "45-minute synthetic OCR/index benchmark; requires ImageMagick, Tesseract and installed E5 weights"]
fn synthetic_45_minute_local_processing_benchmark() {
    let root = std::env::temp_dir().join(format!("daydream-visual-benchmark-{}", Uuid::now_v7()));
    fs::create_dir_all(&root).unwrap();
    let models = std::env::var_os("DAYDREAM_BENCHMARK_MODELS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let data = std::env::var_os("XDG_DATA_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| {
                    PathBuf::from(
                        std::env::var_os("HOME").expect("home or benchmark model directory"),
                    )
                    .join(".local/share")
                });
            data.join("com.snackbit.daydream/models")
        });
    assert!(
        models
            .join("embeddings/multilingual-e5-small-q8_0.gguf")
            .is_file(),
        "Install the pinned E5 model or set DAYDREAM_BENCHMARK_MODELS_DIR"
    );
    let mut manifest = String::new();
    for minute in 0..45 {
        let image = root.join(format!("screen-{minute}.png"));
        let text = format!(
            "Synthetic Ticket {}\nTermin Montag 09:30\nBudget EUR 123.45\n{}",
            100 + minute,
            (0..20)
                .map(|line| format!(
                    "Synthetic project {line:02} Status ready Meeting notes and review"
                ))
                .collect::<Vec<_>>()
                .join("\n")
        );
        let status = Command::new("magick")
            .args([
                "-size",
                "5120x1440",
                "xc:white",
                "-font",
                "DejaVu-Sans",
                "-pointsize",
                "28",
                "-fill",
                "#222222",
                "-gravity",
                "NorthWest",
                "-annotate",
                "+40+40",
            ])
            .arg(text)
            .arg(format!("PNG24:{}", image.display()))
            .status()
            .unwrap();
        assert!(status.success(), "Synthetic screen generation failed");
        manifest.push_str(&format!("file '{}'\nduration 60\n", image.display()));
    }
    manifest.push_str(&format!(
        "file '{}'\n",
        root.join("screen-44.png").display()
    ));
    let manifest_path = root.join("screens.txt");
    fs::write(&manifest_path, manifest).unwrap();
    let video = root.join("synthetic.mkv");
    let mut command = Command::new(ffmpeg::binary_path());
    command
        .args([
            "-hide_banner",
            "-v",
            "error",
            "-f",
            "concat",
            "-safe",
            "0",
            "-i",
        ])
        .arg(&manifest_path)
        .args([
            "-vf", "fps=0.2", "-t", "2700", "-c:v", "ffv1", "-threads", "1",
        ])
        .arg(&video);
    assert!(
        crate::subprocess::run(
            &mut command,
            &AtomicBool::new(false),
            Duration::from_secs(600),
            1_000_000
        )
        .unwrap()
        .status
        .success(),
        "Synthetic source encoding failed"
    );
    let storage = storage::test_storage(root.clone());
    let original=storage.with_connection(|connection| {
        let (chunk,_)=crate::ai::tests::test_transcription_chunk(connection);
        connection.execute("UPDATE recordings SET storage_dir=?2,default_fps=0.2,status='completed',ended_at_ms=2700001 WHERE id=?1",rusqlite::params![chunk.recording_id,root.to_string_lossy()])?;
        connection.execute("UPDATE recording_segments SET file_path=?2,state='finalized',video_stream_index=0,duration_ms=2700000,ended_at_ms=2700001,timeline_base_ms=0,timeline_basis_version=1 WHERE id=?1",rusqlite::params![chunk.segment_id,video.to_string_lossy()])?;
        connection.execute("DELETE FROM processing_chunks WHERE id=?1",[&chunk.id])?;
        Ok(chunk)
    }).unwrap();
    let runtime = ResolvedTesseractRuntime {
        binary: "tesseract".into(),
        data_source: "system".into(),
        languages: "eng+deu".into(),
        runtime_path: None,
        runtime_source: "system".into(),
        runtime_version: None,
        tessdata_dir: None,
    };
    let profile = settings::RecallSettings::default();
    storage
        .with_connection(|connection| {
            storage::save_app_setting(
                connection,
                &SaveAppSettingInput {
                    key: "ai.recall".into(),
                    value_json: serde_json::to_string(&profile).unwrap(),
                    is_sensitive: false,
                    timestamp_ms: now_ms(),
                },
            )
        })
        .unwrap();
    let cancelled = Arc::new(AtomicBool::new(false));
    let started = Instant::now();
    let calls = AtomicUsize::new(0);
    let occurrences = AtomicUsize::new(0);
    tauri::async_runtime::block_on(async {
        for section in 0..90 {
            if section % 15 == 0 {
                println!(
                    "synthetic_visual_progress section={section} elapsed_ms={}",
                    elapsed_ms(started)
                );
            }
            let chunk = storage
                .with_connection(|connection| {
                    storage::create_processing_chunk(
                        connection,
                        &CreateProcessingChunkInput {
                            id: Uuid::now_v7().to_string(),
                            job_id: original.job_id.clone(),
                            recording_id: original.recording_id.clone(),
                            source_kind: "ocr".into(),
                            segment_id: original.segment_id.clone(),
                            track_id: None,
                            start_offset_ms: Some(section * 30_000),
                            end_offset_ms: Some((section + 1) * 30_000),
                            status: "running".into(),
                            attempt_count: 1,
                            progress: 0.,
                            error_code: None,
                            result_code: None,
                            created_at_ms: now_ms(),
                            updated_at_ms: now_ms(),
                        },
                    )
                })
                .unwrap();
            let directory = root.join(format!("frames-{section}"));
            fs::create_dir_all(&directory).unwrap();
            extraction::extract_to_directory(
                &directory,
                video.to_str().unwrap(),
                0,
                section * 30_000,
                30_000,
                Some(0.2),
                &cancelled,
            )
            .unwrap();
            let mut paths = fs::read_dir(&directory)
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .collect::<Vec<_>>();
            paths.sort();
            let frames = extraction::synthetic_frames(
                directory,
                paths,
                section * 30_000,
                (section + 1) * 30_000,
                0.2,
            );
            let samples = sampling::select_frames(
                &chunk,
                &frames,
                "synthetic-local-adaptive-v1",
                "eligible",
                false,
                &cancelled,
            )
            .unwrap();
            let results = futures_util::stream::iter(samples)
                .map(|sample| {
                    let storage = &storage;
                    let chunk = &chunk;
                    let runtime = runtime.clone();
                    let cancel = cancelled.clone();
                    let calls = &calls;
                    let occurrences = &occurrences;
                    async move {
                        let frame = sample.frame;
                        let _privacy =
                            crate::recording_privacy::processing_lease(&chunk.recording_id).await;
                        let _recognition = recognition_lease::acquire(
                            &chunk.recording_id,
                            "video-0",
                            &frame.pixel_hash,
                            "synthetic-local-adaptive-v1",
                            cancel.clone(),
                        )
                        .await
                        .unwrap();
                        let mut input = OcrFrameInput {
                            id: Uuid::now_v7().to_string(),
                            frame_timestamp_ms: frame.timestamp_ms,
                            sample_end_ms: frame.end_ms,
                            source_frame_ref: format!(
                                "segment:{}:offset:{}:frame:{}",
                                chunk.segment_id.as_deref().unwrap(),
                                frame.timestamp_ms,
                                (frame.timestamp_ms - chunk.start_offset_ms.unwrap()) / 5000
                            ),
                            source_track_id: "video-0".into(),
                            width: frame.width,
                            height: frame.height,
                            config_fingerprint: "synthetic-local-adaptive-v1".into(),
                            engine: "tesseract".into(),
                            provider: "tesseract".into(),
                            model: Some("tesseract".into()),
                            pixel_hash: frame.pixel_hash.clone(),
                            status: if sample.selected { "succeeded" } else { "gap" }.into(),
                            error_code: None,
                            content: None,
                            cache_content_id: None,
                            raw_regions: vec![],
                        };
                        if sample.selected {
                            let mut content =
                                infer_or_reuse_frame(storage, chunk, &mut input, || async {
                                    calls.fetch_add(1, Ordering::SeqCst);
                                    tauri::async_runtime::spawn_blocking(move || {
                                        tesseract::run_tesseract_document(&runtime, &frame, &cancel)
                                    })
                                    .await
                                    .unwrap()
                                })
                                .await
                                .unwrap();
                            assert!(
                                content.text.contains("123.45"),
                                "Synthetic amount lost by OCR"
                            );
                            input.raw_regions = raw_word_inputs(
                                chunk,
                                &FullFrame {
                                    path: PathBuf::new(),
                                    timestamp_ms: input.frame_timestamp_ms,
                                    end_ms: input.sample_end_ms,
                                    width: input.width,
                                    height: input.height,
                                    pixel_hash: input.pixel_hash.clone(),
                                    source_ref: input.source_frame_ref.clone(),
                                },
                                &mut content,
                            );
                            input.content = Some(content);
                        }
                        assert!(storage
                            .with_connection_mut(|connection| storage::save_ocr_frame_result(
                                connection, chunk, &input
                            ))
                            .unwrap());
                        occurrences.fetch_add(1, Ordering::SeqCst);
                    }
                })
                .buffer_unordered(4)
                .collect::<Vec<_>>()
                .await;
            assert_eq!(results.len(), 6);
            storage
                .with_connection(|connection| {
                    storage::update_processing_chunk_status(
                        connection,
                        &chunk.id,
                        "succeeded",
                        1.,
                        None,
                        now_ms(),
                    )
                })
                .unwrap();
        }
        let provider = ManagedEmbeddingProvider::new(
            &profile.embedding,
            models,
            PathBuf::from(env!("DAYDREAM_EMBEDDING_WORKER")),
            None,
        )
        .unwrap();
        let progress = index::index_pending(&storage, &provider, &cancelled, |_| {})
            .await
            .unwrap();
        assert_eq!(progress.pending, 0);
        let result = search::search(
            &storage,
            Some(&provider),
            "123.45",
            &search::SearchScope {
                lexical_only: true,
                ..Default::default()
            },
            20,
            &cancelled,
        )
        .await
        .unwrap();
        assert!(!result.hits.is_empty(), "Published OCR must be searchable");
        println!("synthetic_visual_benchmark duration_ms=2700000 width=5120 height=1440 frames={} ocr_calls={} indexed={} elapsed_ms={}",occurrences.load(Ordering::SeqCst),calls.load(Ordering::SeqCst),progress.indexed,elapsed_ms(started));
    });
    assert_eq!(occurrences.load(Ordering::SeqCst), 540);
    assert!(
        started.elapsed() < Duration::from_secs(900),
        "Synthetic OCR/index processing exceeded 15 minutes"
    );
    sampling::discard(std::slice::from_ref(&original.job_id));
    fs::remove_dir_all(root).unwrap();
}
