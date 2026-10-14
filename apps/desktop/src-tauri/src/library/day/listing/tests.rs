use super::*;
use crate::storage::{
    CreateRecordingInput, CreateRecordingSegmentInput, FinalizeRecordingSegmentInput,
};
use std::path::PathBuf;

fn seed(storage: &AppStorage, label: &str, start: i64, duration: Option<i64>) -> (String, String) {
    let id = uuid::Uuid::now_v7().to_string();
    let segment = uuid::Uuid::now_v7().to_string();
    storage
        .with_connection(|connection| {
            storage::create_recording(
                connection,
                &CreateRecordingInput {
                    id: id.clone(),
                    active_scene_id: None,
                    created_at_ms: start,
                    default_fps: 1.0,
                    notes: None,
                    output_root: None,
                    quality_label: "balanced-ocr".into(),
                    storage_dir: "/tmp/absent-library-listing".into(),
                },
            )?;
            storage::create_recording_segment(
                connection,
                &CreateRecordingSegmentInput {
                    id: segment.clone(),
                    recording_id: id.clone(),
                    segment_index: 1,
                    file_path: format!("/tmp/absent-library-listing/{label}.mkv"),
                    started_at_ms: start,
                },
            )?;
            if let Some(duration) = duration {
                storage::finalize_recording_segment(
                    connection,
                    &FinalizeRecordingSegmentInput {
                        id: segment.clone(),
                        duration_ms: duration,
                        ended_at_ms: start + duration,
                        updated_at_ms: start + duration,
                        video_stream_index: Some(0),
                        desktop_audio_stream_index: None,
                        microphone_audio_stream_index: None,
                        probe_json: None,
                    },
                )?;
                connection.execute(
                    "UPDATE recordings SET status='completed' WHERE id=?1",
                    [&id],
                )?;
            } else {
                connection.execute(
                    "UPDATE recordings SET status='recording' WHERE id=?1",
                    [&id],
                )?;
            }
            Ok(())
        })
        .unwrap();
    (id, segment)
}

fn input(limit: usize, before: Option<&str>) -> ListRecallDaysInput {
    ListRecallDaysInput {
        limit,
        before_day_id: before.map(str::to_string),
        timezone: "Europe/Berlin".into(),
    }
}

#[test]
fn library_reads_committed_metadata_while_background_processing_holds_the_writer() {
    let directory =
        std::env::temp_dir().join(format!("daydream-listing-wal-{}", uuid::Uuid::now_v7()));
    std::fs::create_dir_all(&directory).unwrap();
    let storage = storage::test_file_storage(directory.join("synthetic.sqlite3"));
    let (start, _) = recall::day::day_bounds("2026-10-11", "Europe/Berlin").unwrap();
    seed(&storage, "writer", start, Some(1000));
    let (entered_tx, entered_rx) = std::sync::mpsc::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let writer = storage.clone();
    let writer = std::thread::spawn(move || {
        writer.with_connection_mut(|connection| {
            let transaction = connection.transaction()?;
            transaction.execute("UPDATE recording_segments SET duration_ms=5000", [])?;
            entered_tx.send(()).unwrap();
            release_rx.recv().unwrap();
            transaction.commit()
        })
    });
    entered_rx.recv().unwrap();
    let (completed_tx, completed_rx) = std::sync::mpsc::channel();
    let reader = storage.clone();
    let reader = std::thread::spawn(move || {
        completed_tx
            .send(list_days(&reader, &input(30, None)))
            .unwrap()
    });
    let read = completed_rx.recv_timeout(std::time::Duration::from_secs(2));
    // Always release the writer, including if the read accidentally queues on
    // its mutex. The assertion proves completion before the writer is released.
    release_tx.send(()).unwrap();
    writer.join().unwrap().unwrap();
    reader.join().unwrap();
    assert_eq!(
        read.expect("Library must not wait for the writer")
            .unwrap()
            .days[0]
            .captured_duration_ms,
        1000
    );
    assert_eq!(
        list_days(&storage, &input(30, None)).unwrap().days[0].captured_duration_ms,
        5000
    );
    assert!(storage
        .with_read_connection(|connection| connection.execute("DELETE FROM recordings", []))
        .is_err());
    drop(storage);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn library_days_enumerate_capture_without_refreshing_dirty_evidence() {
    let storage = storage::test_storage(PathBuf::new());
    let (start, _) = recall::day::day_bounds("2026-10-11", "Europe/Berlin").unwrap();
    seed(&storage, "cross-midnight", start - 1000, Some(3000));
    seed(&storage, "overlap", start + 1000, Some(2000));
    storage
        .with_connection(|connection| {
            connection.execute_batch(
                "ALTER TABLE transcript_segments RENAME TO unavailable_transcript_segments;
         ALTER TABLE ocr_document_contents RENAME TO unavailable_ocr_document_contents;
         PRAGMA query_only=ON;",
            )
        })
        .unwrap();
    let changes = storage.with_connection(|c| Ok(c.total_changes())).unwrap();
    let page = list_days(&storage, &input(1, None)).unwrap();
    assert_eq!(page.days.len(), 1);
    assert_eq!(page.days[0].day_id, "2026-10-11");
    assert_eq!(page.days[0].recording_count, 2);
    assert_eq!(page.days[0].captured_duration_ms, 3000);
    assert_eq!(page.next_before_day_id.as_deref(), Some("2026-10-11"));
    assert_eq!(
        storage.with_connection(|c| Ok(c.total_changes())).unwrap(),
        changes
    );
    assert!(storage
        .with_connection(|c| c.query_row(
            "SELECT source_dirty FROM recall_meta WHERE id=1",
            [],
            |r| r.get::<_, bool>(0)
        ))
        .unwrap());
    let older = list_days(&storage, &input(1, page.next_before_day_id.as_deref())).unwrap();
    assert_eq!(older.days[0].day_id, "2026-10-10");
    assert_eq!(older.days[0].captured_duration_ms, 1000);
    assert!(older.next_before_day_id.is_none());
    let serialized = serde_json::to_value(page).unwrap();
    for field in [
        "evidence",
        "summary",
        "inputFingerprint",
        "sessions",
        "tasks",
        "feedback",
    ] {
        assert!(serialized["days"][0].get(field).is_none());
    }
}

#[test]
fn library_days_validate_calendar_and_keep_empty_and_active_days() {
    let storage = storage::test_storage(PathBuf::new());
    let empty = list_days(&storage, &input(30, None)).unwrap();
    assert!(empty.days.is_empty());
    assert!(empty.next_before_day_id.is_none());
    assert!(list_days(&storage, &input(30, Some("2026-02-30"))).is_err());
    let mut invalid = input(30, None);
    invalid.timezone = "not-a-timezone".into();
    assert!(list_days(&storage, &invalid).is_err());
    let now = chrono::Utc::now().timestamp_millis();
    seed(&storage, "active", now - 1000, None);
    let active = list_days(&storage, &input(30, None)).unwrap();
    assert!(active.days[0].recording_active);
    assert_eq!(active.days[0].recording_count, 1);
    assert!(active.days[0].captured_duration_ms >= 1000);
    assert!(active.days[0].thumbnail_recording_id.is_none());
}

#[test]
fn library_review_badges_keep_the_existing_fingerprint_contract() {
    let storage = storage::test_storage(PathBuf::new());
    let (start, _) = recall::day::day_bounds("2026-10-11", "Europe/Berlin").unwrap();
    seed(&storage, "reviewed", start, Some(3000));
    let day_input = RecallDayInput {
        day_id: "2026-10-11".into(),
        timezone: "Europe/Berlin".into(),
    };
    let day = storage
        .with_connection(|connection| {
            let day = recall::day::base_day(connection, &day_input).unwrap();
            connection.execute(
                "INSERT INTO recall_day_reviews VALUES(?1,?2)",
                params![recall::day::day_key(&day_input), day.day.input_fingerprint],
            )?;
            Ok(day)
        })
        .unwrap();
    let page = list_days(&storage, &input(30, None)).unwrap();
    assert!(page.days[0].reviewed);
    assert_eq!(
        page.days[0].captured_duration_ms,
        day.day.captured_duration_ms
    );
    storage.with_connection(|connection| {
        connection.execute("INSERT INTO recall_session_corrections(day_key,session_id,correction,updated_at) VALUES(?1,'synthetic-session','synthetic correction',1)",[recall::day::day_key(&day_input)])?;
        Ok(())
    }).unwrap();
    assert!(!list_days(&storage, &input(30, None)).unwrap().days[0].reviewed);
}

#[test]
fn library_summary_setup_reads_configuration_without_decrypting_credentials() {
    let storage = storage::test_storage(PathBuf::new());
    let (start, _) = recall::day::day_bounds("2026-10-11", "Europe/Berlin").unwrap();
    seed(&storage, "configuration", start, Some(1000));
    let mut settings = crate::settings::RecallSettings::default();
    settings.generation.enabled = true;
    settings.generation.base_url = Some("https://synthetic.invalid/v1".into());
    settings.generation.model = Some("synthetic-model".into());
    settings.generation.api_key_ref = Some("ai.secret.recall.synthetic".into());
    storage
        .with_connection_mut(|connection| {
            storage::save_app_settings(
                connection,
                &[
                    storage::SaveAppSettingInput {
                        key: "ai.recall".into(),
                        value_json: serde_json::to_string(&settings).unwrap(),
                        is_sensitive: false,
                        timestamp_ms: 0,
                    },
                    // Deliberately undecryptable synthetic envelope. Listing needs presence
                    // only; actual Recall actions own credential validation and keyring I/O.
                    storage::SaveAppSettingInput {
                        key: "ai.secret.recall.synthetic".into(),
                        value_json: "\"synthetic-envelope\"".into(),
                        is_sensitive: true,
                        timestamp_ms: 0,
                    },
                ],
            )
        })
        .unwrap();
    let configured = list_days(&storage, &input(30, None)).unwrap();
    assert!(configured.generation_ready);
    assert!(configured.generation_unavailable_reason.is_none());
    storage
        .with_connection(|connection| {
            storage::delete_app_setting(connection, "ai.secret.recall.synthetic")
        })
        .unwrap();
    let missing = list_days(&storage, &input(30, None)).unwrap();
    assert!(!missing.generation_ready);
    assert_eq!(
        missing.generation_unavailable_reason.as_deref(),
        Some("recall_key_missing")
    );
}

#[test]
fn library_preview_eligibility_and_revision_follow_redaction_and_deletion() {
    let directory = std::env::temp_dir().join(format!("daydream-listing-{}", uuid::Uuid::now_v7()));
    std::fs::create_dir_all(&directory).unwrap();
    let media = directory.join("synthetic.mkv");
    std::fs::write(&media, b"synthetic metadata fixture").unwrap();
    let storage = storage::test_storage(PathBuf::new());
    let (start, _) = recall::day::day_bounds("2026-10-11", "Europe/Berlin").unwrap();
    let (recording_id, segment_id) = seed(&storage, "preview", start, Some(3000));
    let preview = directory
        .join("playback")
        .join(segment_id)
        .join("overview.jpg");
    std::fs::create_dir_all(preview.parent().unwrap()).unwrap();
    std::fs::write(&preview, b"synthetic preview fixture").unwrap();
    storage
        .with_connection(|connection| {
            connection.execute(
                "UPDATE recordings SET storage_dir=?1 WHERE id=?2",
                params![directory.to_str().unwrap(), recording_id],
            )?;
            connection.execute(
                "UPDATE recording_segments SET file_path=?1 WHERE recording_id=?2",
                params![media.to_str().unwrap(), recording_id],
            )?;
            Ok(())
        })
        .unwrap();
    let visible = list_days(&storage, &input(30, None)).unwrap();
    assert_eq!(visible.days[0].thumbnail_path.as_deref(), preview.to_str());
    storage
        .with_connection(|connection| {
            storage::create_redaction(
                connection,
                &storage::CreateRedactionInput {
                    id: uuid::Uuid::now_v7().to_string(),
                    recording_id: recording_id.clone(),
                    start_offset_ms: 0,
                    end_offset_ms: 1000,
                    track_ids_json: "[]".into(),
                    reason: None,
                    created_at_ms: start,
                },
            )?;
            Ok(())
        })
        .unwrap();
    let hidden = list_days(&storage, &input(30, None)).unwrap();
    assert!(hidden.privacy_revision > visible.privacy_revision);
    assert!(hidden.days[0].redaction_count > 0);
    assert!(hidden.days[0].thumbnail_path.is_none());
    assert!(hidden.days[0].thumbnail_recording_id.is_none());
    storage
        .with_connection(|connection| storage::delete_recording(connection, &recording_id))
        .unwrap();
    let deleted = list_days(&storage, &input(30, None)).unwrap();
    assert!(deleted.days.is_empty());
    assert!(deleted.privacy_revision > hidden.privacy_revision);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn library_summary_badges_read_lineage_metadata_and_conservatively_handle_dirty_sources() {
    let storage = storage::test_storage(PathBuf::new());
    let (start, end) = recall::day::day_bounds("2026-10-11", "Europe/Berlin").unwrap();
    let (recording_id, segment_id) = seed(&storage, "summary", start, Some(3000));
    storage.with_connection(|connection| {
        let key = "2026-10-11@Europe/Berlin";
        let status = |count,dirty,profile| summary_status(connection,key,(start,end),count,dirty,profile);
        assert_eq!(status(0,false,None)?,"unconfigured");
        assert_eq!(status(0,false,Some("profile"))?,"insufficient");
        assert_eq!(status(1,false,Some("profile"))?,"partial");
        connection.execute("INSERT INTO recall_evidence VALUES('evidence',?1,?2,'ocr','video-0','source',0,1000,?3,?4,'revision','synthetic text','[]')",params![recording_id,segment_id,start,start+1000])?;
        connection.execute("INSERT INTO recall_generations VALUES(?1,'fingerprint','profile','{}','ready',0)",[key])?;
        connection.execute("INSERT INTO recall_generation_inputs VALUES(?1,'evidence','revision')",[key])?;
        assert_eq!(status(1,false,Some("profile"))?,"ready");
        assert_eq!(status(2,false,Some("profile"))?,"partial");
        assert_eq!(status(1,true,Some("profile"))?,"partial");
        assert_eq!(status(1,false,Some("changed-profile"))?,"partial");
        connection.execute("UPDATE recall_generation_inputs SET revision='stale' WHERE day_key=?1",[key])?;
        assert_eq!(status(1,false,Some("profile"))?,"invalidated");
        connection.execute("INSERT INTO recall_jobs(id,kind,day_key,status,progress,input_fingerprint,config_fingerprint,created_at_ms,updated_at_ms) VALUES('job','generation',?1,'pending',0,'input','profile',0,0)",[key])?;
        assert_eq!(status(1,false,Some("profile"))?,"generating");
        Ok(())
    }).unwrap();
}

/// Synthetic published corpus; compares the old full-day path with the card read.
#[test]
#[ignore = "synthetic Library latency comparison"]
fn library_listing_metadata_benchmark() {
    let storage = storage::test_storage(PathBuf::new());
    let (start, _) = recall::day::day_bounds("2026-10-11", "Europe/Berlin").unwrap();
    for index in 0..300 {
        seed(
            &storage,
            &format!("recording-{index}"),
            start - (index / 10) * 86_400_000,
            Some(60_000),
        );
    }
    storage.with_connection(|connection| {
        connection.execute_batch("INSERT INTO recall_source_files SELECT id,file_path,0 FROM recording_segments; DELETE FROM recall_dirty_recordings; UPDATE recall_meta SET source_dirty=0 WHERE id=1;")?;
        connection.execute_batch("WITH RECURSIVE n(value) AS (VALUES(0) UNION ALL SELECT value+1 FROM n WHERE value<199) INSERT INTO recall_evidence SELECT s.id||':'||n.value,s.recording_id,s.id,'ocr','video-0','synthetic-'||n.value,0,1000,s.started_at_ms,s.started_at_ms+1000,'revision',printf('%02000d',n.value),'[]' FROM recording_segments s CROSS JOIN n;")?;
        Ok(())
    }).unwrap();
    let started = std::time::Instant::now();
    let full = recall::day::list_days(&storage, &input(30, None)).unwrap();
    let full_time = started.elapsed();
    let started = std::time::Instant::now();
    let metadata = list_days(&storage, &input(30, None)).unwrap();
    let metadata_time = started.elapsed();
    assert_eq!(metadata.days.len(), full.len());
    for (card, full) in metadata.days.iter().zip(full) {
        assert_eq!(card.day_id, full.day_id);
        assert_eq!(card.recording_count, full.recording_count);
        assert_eq!(card.evidence_count, full.evidence_count);
        assert_eq!(card.captured_duration_ms, full.captured_duration_ms);
    }
    eprintln!("Library synthetic fixture: 300 recordings, 30 days, 60000 evidence rows; full Recall listing {full_time:?}; metadata listing {metadata_time:?}");
}
