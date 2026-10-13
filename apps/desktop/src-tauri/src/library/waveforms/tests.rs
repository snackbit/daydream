use super::*;
use crate::library::LibraryWaveformTileRequest;
use crate::storage::{self, AppStorage};
use std::{fs, path::PathBuf, sync::atomic::AtomicBool};

struct Fixture {
    storage: AppStorage,
    id: String,
    root: PathBuf,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
fn fixture() -> Fixture {
    let root =
        std::env::temp_dir().join(format!("daydream-studio-waveform-{}", uuid::Uuid::now_v7()));
    let id = uuid::Uuid::now_v7().to_string();
    let dir = root.join("recordings").join(&id);
    fs::create_dir_all(&dir).unwrap();
    let segment = crate::ffmpeg::audio_timeline_tests::gap_fixture();
    let path = dir.join("source.mkv");
    fs::rename(&segment.file_path, &path).unwrap();
    let storage = storage::test_storage(root.join("database"));
    storage
        .with_connection(|connection| {
            storage::create_recording(
                connection,
                &storage::CreateRecordingInput {
                    id: id.clone(),
                    active_scene_id: None,
                    created_at_ms: 0,
                    default_fps: 1.0,
                    notes: None,
                    output_root: Some(root.join("recordings").to_string_lossy().into_owned()),
                    quality_label: "balanced-ocr".into(),
                    storage_dir: dir.to_string_lossy().into_owned(),
                },
            )?;
            let segment_id = uuid::Uuid::now_v7().to_string();
            storage::create_recording_segment(
                connection,
                &storage::CreateRecordingSegmentInput {
                    id: segment_id.clone(),
                    recording_id: id.clone(),
                    file_path: path.to_string_lossy().into_owned(),
                    segment_index: 1,
                    started_at_ms: 0,
                },
            )?;
            storage::finalize_recording_segment(
                connection,
                &storage::FinalizeRecordingSegmentInput {
                    id: segment_id,
                    duration_ms: 12_000,
                    ended_at_ms: 12_000,
                    updated_at_ms: 12_000,
                    desktop_audio_stream_index: Some(1),
                    microphone_audio_stream_index: Some(2),
                    video_stream_index: Some(0),
                    probe_json: None,
                },
            )?;
            storage::mark_recording_completed(connection, &id, 12_000)?;
            Ok(())
        })
        .unwrap();
    Fixture { storage, id, root }
}
fn query(fixture: &Fixture, width: f64) -> LibraryWaveformTileRequest {
    LibraryWaveformTileRequest {
        recording_id: fixture.id.clone(),
        start_timeline_ms: 0,
        end_timeline_ms: 12_000,
        viewport_width_px: width,
    }
}

#[test]
fn studio_waveforms_are_durable_local_work_and_reads_never_extract_audio() {
    let fixture = fixture();
    let first = viewport::read(&fixture.storage, &query(&fixture, 1200.0)).unwrap();
    assert!(first.pending);
    assert!(first.tracks.iter().all(|track| track.tiles.is_empty()));
    let recall_revision = fixture
        .storage
        .with_connection(|connection| {
            connection.query_row("SELECT revision FROM recall_meta WHERE id=1", [], |row| {
                row.get::<_, i64>(0)
            })
        })
        .unwrap();
    jobs::admit_batch(&fixture.storage).unwrap();
    assert_eq!(
        fixture
            .storage
            .with_connection(|connection| connection.query_row(
                "SELECT revision FROM recall_meta WHERE id=1",
                [],
                |row| row.get::<_, i64>(0)
            ))
            .unwrap(),
        recall_revision,
        "waveform admission cannot invalidate searchable evidence"
    );
    let jobs = fixture
        .storage
        .with_connection(|connection| storage::list_processing_jobs(connection, Some(&fixture.id)))
        .unwrap();
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].kind, "waveform");
    jobs::admit_recording(&fixture.storage, &fixture.id).unwrap();
    assert_eq!(
        fixture
            .storage
            .with_connection(|connection| storage::list_processing_jobs(
                connection,
                Some(&fixture.id)
            ))
            .unwrap()
            .len(),
        1
    );
    let (_, sources) = jobs::sources(&fixture.storage, &fixture.id).unwrap();
    assert_eq!(sources.len(), 2);
    for source in &sources {
        source.prepare(&AtomicBool::new(false)).unwrap();
    }
    let fine = viewport::read(&fixture.storage, &query(&fixture, 1200.0)).unwrap();
    let overview = viewport::read(&fixture.storage, &query(&fixture, 12.0)).unwrap();
    assert!(!fine.pending);
    assert!(!overview.pending);
    assert!(overview.bin_ms > fine.bin_ms);
    assert_eq!(fine.tracks.len(), 2);
    for source in &sources {
        let before = fs::read(&source.path).unwrap();
        source.prepare(&AtomicBool::new(true)).unwrap();
        assert_eq!(
            fs::read(&source.path).unwrap(),
            before,
            "warm preparation must not decode or rewrite"
        );
    }
    let tile = &fine.tracks[0].tiles[0];
    assert!(tile.peak_samples.iter().any(|peak| *peak > 0.8));
    assert!(
        tile.peak_samples
            .iter()
            .skip((4600 / tile.bin_ms) as usize)
            .take((400 / tile.bin_ms) as usize)
            .all(|peak| *peak < 0.01),
        "the fixture's 4.4–5.2 s timestamp gap remains silent"
    );
}

#[test]
fn studio_waveform_cancellation_corruption_redaction_and_deletion_are_recoverable() {
    let fixture = fixture();
    let (_, sources) = jobs::sources(&fixture.storage, &fixture.id).unwrap();
    assert_eq!(
        sources[0].prepare(&AtomicBool::new(true)).unwrap_err(),
        "ai_processing_cancelled"
    );
    assert!(!sources[0].path.exists());
    sources[0].prepare(&AtomicBool::new(false)).unwrap();
    fs::write(&sources[0].path, b"interrupted cache").unwrap();
    assert!(sources[0].maximum().is_err());
    assert!(
        viewport::read(&fixture.storage, &query(&fixture, 1200.0))
            .unwrap()
            .pending
    );
    sources[0].prepare(&AtomicBool::new(false)).unwrap();
    assert!(sources[0].maximum().is_ok());
    fixture
        .storage
        .with_connection(|connection| {
            storage::create_redaction(
                connection,
                &storage::CreateRedactionInput {
                    id: uuid::Uuid::now_v7().to_string(),
                    recording_id: fixture.id.clone(),
                    start_offset_ms: 0,
                    end_offset_ms: 1000,
                    track_ids_json: serde_json::to_string(&vec![sources[0].track_id.clone()])
                        .unwrap(),
                    reason: None,
                    created_at_ms: 12_001,
                },
            )?;
            Ok(())
        })
        .unwrap();
    let (_, visible) = jobs::sources(&fixture.storage, &fixture.id).unwrap();
    assert!(visible
        .iter()
        .filter(|source| source.track_id == sources[0].track_id)
        .all(|source| source.start_ms >= 1000));
    assert!(visible
        .iter()
        .all(|source| source.path != sources[0].path || source.track_id != sources[0].track_id));
    let recording = fixture
        .storage
        .with_connection(|connection| storage::get_recording(connection, &fixture.id))
        .unwrap();
    crate::library::delete_recording_row(
        &fixture.storage,
        &fixture.root.join("recordings"),
        &recording,
    )
    .unwrap();
    assert!(!sources[0].path.exists());
    assert!(fixture
        .storage
        .with_connection(|connection| storage::list_processing_jobs(connection, Some(&fixture.id)))
        .unwrap()
        .is_empty());
}

#[test]
fn studio_waveform_admission_preserves_pause_cancel_and_live_input_ownership() {
    let fixture = fixture();
    fixture
        .storage
        .with_connection(|connection| {
            connection.execute("INSERT INTO recording_live_state(recording_id,classifier_required,classified_through_ms) VALUES(?1,0,12000)", [&fixture.id])?;
            connection
                .execute(
                    "UPDATE recordings SET status='paused' WHERE id=?1",
                    [&fixture.id],
                )
                .map(|_| ())
        })
        .unwrap();
    jobs::admit_batch(&fixture.storage).unwrap();
    let job = fixture
        .storage
        .with_connection(|connection| storage::list_processing_jobs(connection, Some(&fixture.id)))
        .unwrap()
        .remove(0);
    assert!(fixture
        .storage
        .with_connection(|connection| storage::processing_job_input_open(connection, &job.id))
        .unwrap());
    fixture
        .storage
        .with_connection(|connection| {
            storage::update_processing_job_status(
                connection,
                &job.id,
                "paused",
                None,
                job.updated_at_ms + 1,
            )
        })
        .unwrap();
    jobs::admit_recording(&fixture.storage, &fixture.id).unwrap();
    assert_eq!(
        fixture
            .storage
            .with_connection(|connection| storage::get_processing_job(connection, &job.id))
            .unwrap()
            .unwrap()
            .status,
        "paused"
    );
    fixture
        .storage
        .with_connection(|connection| {
            storage::update_processing_job_status(
                connection,
                &job.id,
                "cancelled",
                None,
                job.updated_at_ms + 2,
            )?;
            storage::mark_recording_completed(connection, &fixture.id, 12_004)?;
            Ok(())
        })
        .unwrap();
    jobs::admit_batch(&fixture.storage).unwrap();
    assert!(!fixture
        .storage
        .with_connection(|connection| storage::processing_job_input_open(connection, &job.id))
        .unwrap());
    assert_eq!(
        fixture
            .storage
            .with_connection(|connection| storage::list_processing_jobs(
                connection,
                Some(&fixture.id)
            ))
            .unwrap()
            .len(),
        1,
        "manual cancellation must not create replacement work"
    );
    assert_eq!(
        fixture
            .storage
            .with_connection(|connection| connection.query_row(
                "SELECT COUNT(*) FROM waveform_admission",
                [],
                |row| row.get::<_, i64>(0)
            ))
            .unwrap(),
        0
    );
}
