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
