use super::*;
struct Directory(PathBuf);
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn seed(
    storage: &AppStorage,
    directory: &Path,
    label: &str,
    start: i64,
    missing: bool,
    active: bool,
) -> String {
    let path = directory.join(format!("{label}.mkv"));
    if !missing {
        fs::write(&path, b"synthetic metadata fixture").unwrap();
    }
    let id = Uuid::now_v7().to_string();
    let segment = Uuid::now_v7().to_string();
    storage
        .with_connection(|connection| {
            storage::create_recording(
                connection,
                &storage::CreateRecordingInput {
                    id: id.clone(),
                    active_scene_id: None,
                    created_at_ms: start,
                    default_fps: 1.0,
                    notes: None,
                    output_root: None,
                    quality_label: "balanced-ocr".into(),
                    storage_dir: path_to_string(directory),
                },
            )?;
            storage::create_recording_segment(
                connection,
                &storage::CreateRecordingSegmentInput {
                    id: segment.clone(),
                    recording_id: id.clone(),
                    segment_index: 1,
                    file_path: path_to_string(&path),
                    started_at_ms: start,
                },
            )?;
            storage::finalize_recording_segment(
                connection,
                &storage::FinalizeRecordingSegmentInput {
                    id: segment.clone(),
                    duration_ms: 2000,
                    ended_at_ms: start + 2000,
                    updated_at_ms: start + 2000,
                    video_stream_index: Some(0),
                    desktop_audio_stream_index: Some(1),
                    microphone_audio_stream_index: None,
                    probe_json: None,
                },
            )?;
            if !active {
                storage::mark_recording_completed(connection, &id, start + 2000)?;
            }
            Ok(())
        })
        .unwrap();
    id
}
#[test]
fn day_reader_uses_real_storage_and_keeps_partial_media_without_admitting_missing_or_protected_recordings(
) {
    let directory =
        Directory(std::env::temp_dir().join(format!("daydream-day-test-{}", Uuid::now_v7())));
    fs::create_dir_all(&directory.0).unwrap();
    let storage = storage::test_storage(PathBuf::new());
    let (start, end) = crate::recall::day::day_bounds("2026-10-11", "Europe/Berlin").unwrap();
    let partial = seed(
        &storage,
        &directory.0,
        "partial",
        start - 1000,
        false,
        false,
    );
    seed(
        &storage,
        &directory.0,
        "missing",
        start + 10000,
        true,
        false,
    );
    seed(&storage, &directory.0, "active", start + 20000, false, true);
    let protected = seed(
        &storage,
        &directory.0,
        "protected",
        start + 30000,
        false,
        false,
    );
    seed(&storage, &directory.0, "outside", end + 1000, false, false);
    let missing = Uuid::now_v7().to_string();
    storage
        .with_connection(|connection| {
            storage::create_recording_segment(
                connection,
                &storage::CreateRecordingSegmentInput {
                    id: missing.clone(),
                    recording_id: partial.clone(),
                    segment_index: 2,
                    file_path: path_to_string(&directory.0.join("absent.mkv")),
                    started_at_ms: start + 4000,
                },
            )?;
            storage::finalize_recording_segment(
                connection,
                &storage::FinalizeRecordingSegmentInput {
                    id: missing.clone(),
                    duration_ms: 2000,
                    ended_at_ms: start + 6000,
                    updated_at_ms: start + 6000,
                    video_stream_index: Some(0),
                    desktop_audio_stream_index: None,
                    microphone_audio_stream_index: None,
                    probe_json: None,
                },
            )?;
            storage::create_recording_protection_span(
                connection,
                &storage::CreateRecordingProtectionSpanInput {
                    id: Uuid::now_v7().to_string(),
                    recording_id: protected.clone(),
                    action: "redact".into(),
                    opened_at_ms: start + 30000,
                    rule_id: Uuid::now_v7().to_string(),
                    rule_label: "Synthetic rule".into(),
                    start_offset_ms: 0,
                },
            )?;
            Ok(())
        })
        .unwrap();
    let day = read_day(
        &storage,
        LibraryDayTimelineInput {
            day_id: "2026-10-11".into(),
            timezone: "Europe/Berlin".into(),
        },
    )
    .unwrap();
    assert_eq!(day.recordings.len(), 1);
    assert_eq!(day.recordings[0].id, partial);
    assert_eq!(day.recordings[0].size_bytes, None);
    assert_eq!(day.spans.len(), 1);
    assert_eq!(day.spans[0].start_ms, start);
    assert_eq!(day.spans[0].duration_ms, 1000);
    assert_eq!(day.spans[0].source_start_ms, 1000);
    assert_eq!(day.spans[0].source_offset_ms, 1000);
    assert_eq!(
        day.privacy_revision,
        storage
            .with_connection(storage::recall_privacy_revision)
            .unwrap()
    );
    assert!(read_day(
        &storage,
        LibraryDayTimelineInput {
            day_id: "2026-02-30".into(),
            timezone: "Europe/Berlin".into()
        }
    )
    .is_err());
}

#[test]
fn day_defers_disk_usage_but_recording_details_count_generated_artifacts() {
    let directory =
        Directory(std::env::temp_dir().join(format!("daydream-day-size-{}", Uuid::now_v7())));
    fs::create_dir_all(directory.0.join("generated")).unwrap();
    let storage = storage::test_storage(PathBuf::new());
    let (start, _) = crate::recall::day::day_bounds("2026-10-11", "Europe/Berlin").unwrap();
    let id = seed(&storage, &directory.0, "recording", start, false, false);
    let artifact = b"synthetic generated artifact";
    fs::write(directory.0.join("generated/ocr.json"), artifact).unwrap();
    let day = read_day(
        &storage,
        LibraryDayTimelineInput {
            day_id: "2026-10-11".into(),
            timezone: "Europe/Berlin".into(),
        },
    )
    .unwrap();
    assert_eq!(day.recordings[0].size_bytes, None);
    assert_eq!(
        serde_json::to_value(&day.recordings[0]).unwrap()["sizeBytes"],
        serde_json::Value::Null
    );
    let detail = detail_for_recording(&storage, &id).unwrap();
    assert_eq!(
        detail.size_bytes,
        Some((b"synthetic metadata fixture".len() + artifact.len()) as u64)
    );
    assert_eq!(day.recordings[0].tracks.len(), detail.tracks.len());
    assert_eq!(
        day.recordings[0].audio_cleanup.as_ref().unwrap().phase,
        detail.audio_cleanup.as_ref().unwrap().phase
    );
}

#[test]
fn day_reads_committed_playback_metadata_while_processing_holds_the_writer() {
    let directory =
        Directory(std::env::temp_dir().join(format!("daydream-day-wal-{}", Uuid::now_v7())));
    fs::create_dir_all(&directory.0).unwrap();
    let storage = storage::test_file_storage(directory.0.join("synthetic.sqlite3"));
    let (start, _) = crate::recall::day::day_bounds("2026-10-11", "Europe/Berlin").unwrap();
    seed(&storage, &directory.0, "writer", start, false, false);
    let (entered_tx, entered_rx) = std::sync::mpsc::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let writer_storage = storage.clone();
    let writer = std::thread::spawn(move || {
        writer_storage.with_connection_mut(|connection| {
            let transaction = connection.transaction()?;
            transaction.execute("UPDATE recording_segments SET duration_ms=5000", [])?;
            entered_tx.send(()).unwrap();
            release_rx.recv().unwrap();
            transaction.commit()
        })
    });
    entered_rx.recv().unwrap();
    let (completed_tx, completed_rx) = std::sync::mpsc::channel();
    let reader_storage = storage.clone();
    let reader = std::thread::spawn(move || {
        completed_tx
            .send(read_day(
                &reader_storage,
                LibraryDayTimelineInput {
                    day_id: "2026-10-11".into(),
                    timezone: "Europe/Berlin".into(),
                },
            ))
            .unwrap()
    });
    let read = completed_rx.recv_timeout(std::time::Duration::from_secs(2));
    release_tx.send(()).unwrap();
    writer.join().unwrap().unwrap();
    reader.join().unwrap();
    assert_eq!(
        read.expect("Day opening must not wait for the writer")
            .unwrap()
            .spans[0]
            .duration_ms,
        2000
    );
    assert_eq!(
        read_day(
            &storage,
            LibraryDayTimelineInput {
                day_id: "2026-10-11".into(),
                timezone: "Europe/Berlin".into()
            }
        )
        .unwrap()
        .spans[0]
            .duration_ms,
        5000
    );
}

#[test]
#[ignore = "synthetic disk-usage benchmark; run explicitly with --nocapture"]
fn synthetic_day_opening_disk_usage_benchmark() {
    let directory =
        Directory(std::env::temp_dir().join(format!("daydream-day-bench-{}", Uuid::now_v7())));
    fs::create_dir_all(&directory.0).unwrap();
    let storage = storage::test_storage(PathBuf::new());
    let (start, _) = crate::recall::day::day_bounds("2026-10-11", "Europe/Berlin").unwrap();
    let mut ids = Vec::new();
    for index in 0..8 {
        let recording_dir = directory.0.join(format!("recording-{index}"));
        fs::create_dir_all(recording_dir.join("generated")).unwrap();
        ids.push(seed(
            &storage,
            &recording_dir,
            "media",
            start + index * 10000,
            false,
            false,
        ));
        for artifact in 0..512 {
            fs::write(
                recording_dir.join(format!("generated/{artifact}.json")),
                b"synthetic artifact",
            )
            .unwrap();
        }
    }
    let mut previous = Vec::new();
    let mut current = Vec::new();
    for _ in 0..20 {
        let began = std::time::Instant::now();
        for id in &ids {
            detail_for_recording(&storage, id).unwrap();
        }
        previous.push(began.elapsed().as_secs_f64() * 1000.0);
        let began = std::time::Instant::now();
        let day = read_day(
            &storage,
            LibraryDayTimelineInput {
                day_id: "2026-10-11".into(),
                timezone: "Europe/Berlin".into(),
            },
        )
        .unwrap();
        assert_eq!(day.recordings.len(), ids.len());
        current.push(began.elapsed().as_secs_f64() * 1000.0);
    }
    previous.sort_by(f64::total_cmp);
    current.sort_by(f64::total_cmp);
    eprintln!("synthetic day metadata, 8 recordings + 4096 artifacts, 20 warm runs: previous detail assembly median={:.2}ms p95={:.2}ms; day snapshot median={:.2}ms p95={:.2}ms", previous[10], previous[18], current[10], current[18]);
}
