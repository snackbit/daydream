use super::*;
use std::{fs, sync::atomic::AtomicBool};

struct Fixture {
    root: PathBuf,
    storage: AppStorage,
    id: String,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
fn fixture() -> Fixture {
    let root = std::env::temp_dir().join(format!("daydream-prepared-{}", uuid::Uuid::now_v7()));
    fs::create_dir_all(&root).unwrap();
    let storage = storage::test_file_storage(root.join("db.sqlite3"));
    let id = uuid::Uuid::now_v7().to_string();
    let directory = root.join(&id);
    fs::create_dir(&directory).unwrap();
    let source = crate::ffmpeg::audio_timeline_tests::gap_fixture();
    let path = directory.join("source.mkv");
    fs::rename(&source.file_path, &path).unwrap();
    storage
        .with_connection(|c| {
            storage::create_recording(
                c,
                &storage::CreateRecordingInput {
                    id: id.clone(),
                    active_scene_id: None,
                    created_at_ms: 1,
                    default_fps: 1.,
                    notes: None,
                    output_root: None,
                    quality_label: "balanced-ocr".into(),
                    storage_dir: directory.to_string_lossy().into_owned(),
                },
            )?;
            let segment = uuid::Uuid::now_v7().to_string();
            storage::create_recording_segment(
                c,
                &storage::CreateRecordingSegmentInput {
                    id: segment.clone(),
                    recording_id: id.clone(),
                    file_path: path.to_string_lossy().into_owned(),
                    segment_index: 1,
                    started_at_ms: 1,
                },
            )?;
            storage::finalize_recording_segment(
                c,
                &storage::FinalizeRecordingSegmentInput {
                    id: segment.clone(),
                    duration_ms: 12_000,
                    ended_at_ms: 12_001,
                    updated_at_ms: 12_001,
                    video_stream_index: Some(0),
                    desktop_audio_stream_index: Some(1),
                    microphone_audio_stream_index: Some(2),
                    probe_json: None,
                },
            )?;
            storage::mark_recording_completed(c, &id, 12_001)?;
            Ok(())
        })
        .unwrap();
    Fixture { root, storage, id }
}

#[test]
fn prepared_playback_persists_across_sessions_serves_ranges_and_revokes_stale_privacy() {
    for include_runs in [false, true] {
        verify_prepared_playback(include_runs);
    }
}

fn verify_prepared_playback(include_runs: bool) {
    let fixture = fixture();
    generation::recording(&fixture.storage, &fixture.id, &AtomicBool::new(false)).unwrap();
    let (recording, segments, sources) = fixture
        .storage
        .with_read_connection(|c| {
            Ok((
                storage::get_recording(c, &fixture.id)?,
                storage::list_recording_segments(c, &fixture.id)?,
                storage::recording_video_streams(c, &fixture.id)?,
            ))
        })
        .unwrap();
    let before = fixture
        .storage
        .with_read_connection(|c| {
            let mut q = c.prepare(
                "SELECT path FROM studio_media_assets WHERE recording_id=?1 ORDER BY asset_key",
            )?;
            let rows = q
                .query_map([&fixture.id], |r| r.get::<_, String>(0))?
                .collect::<rusqlite::Result<Vec<_>>>();
            rows
        })
        .unwrap()
        .iter()
        .map(|p| (p.clone(), fs::metadata(p).unwrap().modified().unwrap()))
        .collect::<Vec<_>>();
    assert_eq!(before.len(), 6);
    generation::recording(&fixture.storage, &fixture.id, &AtomicBool::new(false)).unwrap();
    assert!(before
        .iter()
        .all(|(p, mtime)| fs::metadata(p).unwrap().modified().unwrap() == *mtime));
    let server = crate::playback_server::PlaybackServer::start().unwrap();
    let assets = server
        .create_recording_source_session(
            &recording,
            &segments,
            &sources,
            fixture.storage.clone(),
            false,
            include_runs,
        )
        .unwrap();
    assert!(assets.segments[0].can_seek_in_stream);
    assert_eq!(assets.runs.is_empty(), !include_runs);
    assert!(assets.runs.iter().all(|run| run.assets.can_seek_in_stream));
    assert!(assets.segments[0].video_url.contains("/file/"));
    let video_path = server.sessions.lock().unwrap()[&assets.session_id]
        .prepared_files
        .get(&format!("{}/video-0", segments[0].id))
        .unwrap()
        .clone();
    let request = {
        let sessions = server.sessions.lock().unwrap();
        let session = &sessions[&assets.session_id];
        session.request_snapshot(&session.token, &segments[0].id, Some("video-0"))
    };
    let probe = crate::recorder::media_writer::probe_segment_best_effort(&video_path);
    assert!(
        probe
            .duration_ms
            .is_some_and(|duration| (11_900..=12_100).contains(&duration)),
        "prepared sparse video must retain the declared source duration, got {:?}",
        probe.duration_ms
    );
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        let client = reqwest::Client::new();
        let warmup = client
            .get(server.decoder_warmup_url())
            .send()
            .await
            .unwrap();
        assert_eq!(warmup.status(), 200);
        assert_eq!(warmup.headers()["content-type"], "video/mp4");
        let warmup = warmup.bytes().await.unwrap();
        assert_eq!(&warmup[4..8], b"ftyp");
        let head = client
            .head(server.decoder_warmup_url())
            .send()
            .await
            .unwrap();
        assert_eq!(head.status(), 200);
        assert_eq!(head.headers()["content-length"], warmup.len().to_string());
        assert!(head.bytes().await.unwrap().is_empty());
        let response = client
            .get(&assets.segments[0].video_url)
            .header("Range", "bytes=0-15")
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 206);
        assert_eq!(response.headers()["accept-ranges"], "bytes");
        let bytes = response.bytes().await.unwrap();
        assert_eq!(bytes.len(), 16);
        assert_eq!(&bytes[4..8], b"ftyp");
        let head = client
            .head(&assets.segments[0].video_url)
            .send()
            .await
            .unwrap();
        assert!(head.status().is_success());
        assert!(head.bytes().await.unwrap().is_empty());
        let invalid = client
            .get(&assets.segments[0].video_url)
            .header("Range", "bytes=9999999999-")
            .send()
            .await
            .unwrap();
        assert_eq!(invalid.status(), 416);
        fixture
            .storage
            .with_connection(|c| {
                storage::create_redaction(
                    c,
                    &storage::CreateRedactionInput {
                        id: uuid::Uuid::now_v7().to_string(),
                        recording_id: fixture.id.clone(),
                        start_offset_ms: 0,
                        end_offset_ms: 1000,
                        created_at_ms: 20_000,
                        track_ids_json: "[]".into(),
                        reason: None,
                    },
                )
                .map(|_| ())
            })
            .unwrap();
        assert_eq!(
            client
                .get(&assets.segments[0].video_url)
                .send()
                .await
                .unwrap()
                .status(),
            403
        );
        generation::recording(&fixture.storage, &fixture.id, &AtomicBool::new(false)).unwrap();
        assert!(
            video_path.is_file(),
            "an open session still owns the retired file"
        );
        let fresh = server
            .create_recording_source_session(
                &recording,
                &segments,
                &sources,
                fixture.storage.clone(),
                false,
                include_runs,
            )
            .unwrap();
        for (offset, status) in [(0, 403), (3000, 200)] {
            let url = &fresh.segments[0]
                .thumbnails
                .iter()
                .find(|t| t.offset_ms == offset)
                .unwrap()
                .url;
            assert_eq!(
                client.get(url).send().await.unwrap().status().as_u16(),
                status
            );
        }
        let privacy = fixture
            .storage
            .with_read_connection(|c| {
                crate::library::audio_cleanup::privacy_fingerprint(c, &fixture.id)
            })
            .unwrap();
        let source_hash = fingerprint(&[(segments[0].file_path.clone(), 12_000)]).unwrap();
        let cached = Path::new(&segments[0].file_path)
            .parent()
            .unwrap()
            .join(".daydream-thumbnails-v1")
            .join(source_hash)
            .join(privacy)
            .join("stream-0");
        assert!(
            !cached.join("0.jpg").exists(),
            "hidden timestamps must not be republished in the current privacy namespace"
        );
        assert!(cached.join("3000.jpg").is_file());
        let service = super::super::thumbnails::ThumbnailService::new_with_resources(
            server.sessions.clone(),
            0,
            Default::default(),
        );
        let segment =
            server.sessions.lock().unwrap()[&fresh.session_id].segments[&segments[0].id].clone();
        let namespace = fixture
            .storage
            .with_read_connection(|c| {
                crate::library::audio_cleanup::privacy_fingerprint(c, &fixture.id)
            })
            .unwrap();
        let key = super::super::thumbnails::ThumbnailKey {
            cache_namespace: namespace,
            video_stream_index: 0,
            offset_ms: 3000,
            segment_id: segments[0].id.clone(),
            session_id: fresh.session_id.clone(),
        };
        let (done, completed) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = done.send(service.request_thumbnail(key, segment, 0, 3000));
        });
        assert!(completed
            .recv_timeout(std::time::Duration::from_millis(500))
            .expect("disk thumbnails bypass the decode queue even with no workers")
            .is_ok());
        server.close_session(&fresh.session_id).unwrap();
    });
    server.close_session(&assets.session_id).unwrap();
    assert!(
        video_path.exists(),
        "an in-flight request still owns the retired media"
    );
    drop(request);
    assert!(
        !video_path.exists(),
        "retired media is reclaimed after its final owner releases it"
    );
}

#[test]
fn track_redactions_exclude_only_affected_durable_media_and_recover_after_unhide() {
    let fixture = fixture();
    let cut = uuid::Uuid::now_v7().to_string();
    fixture
        .storage
        .with_connection(|c| {
            storage::create_redaction(
                c,
                &storage::CreateRedactionInput {
                    id: cut.clone(),
                    recording_id: fixture.id.clone(),
                    start_offset_ms: 250,
                    end_offset_ms: 500,
                    created_at_ms: 20_000,
                    track_ids_json: "[\"microphone\"]".into(),
                    reason: None,
                },
            )
            .map(|_| ())
        })
        .unwrap();
    generation::recording(&fixture.storage, &fixture.id, &AtomicBool::new(false)).unwrap();
    let keys = fixture
        .storage
        .with_read_connection(|c| {
            let mut q =
                c.prepare("SELECT asset_key FROM studio_media_assets WHERE recording_id=?1")?;
            let rows = q
                .query_map([&fixture.id], |r| r.get::<_, String>(0))?
                .collect::<rusqlite::Result<Vec<_>>>();
            rows
        })
        .unwrap();
    assert_eq!(keys.len(), 4);
    assert!(keys.iter().all(|key| !key.contains("microphone")));
    fixture
        .storage
        .with_connection(|c| storage::delete_redaction(c, &fixture.id, &cut))
        .unwrap();
    generation::recording(&fixture.storage, &fixture.id, &AtomicBool::new(false)).unwrap();
    assert_eq!(
        fixture
            .storage
            .with_read_connection(|c| c.query_row(
                "SELECT COUNT(*) FROM studio_media_assets WHERE recording_id=?1",
                [&fixture.id],
                |r| r.get::<_, i64>(0)
            ))
            .unwrap(),
        6
    );
}

#[test]
fn interrupted_preparation_preserves_sources_and_has_no_published_assets() {
    let fixture = fixture();
    assert!(generation::recording(&fixture.storage, &fixture.id, &AtomicBool::new(true)).is_err());
    assert_eq!(
        fixture
            .storage
            .with_read_connection(|c| c.query_row(
                "SELECT COUNT(*) FROM studio_media_assets",
                [],
                |r| r.get::<_, i64>(0)
            ))
            .unwrap(),
        0
    );
    assert!(fixture
        .storage
        .with_read_connection(|c| storage::list_recording_segments(c, &fixture.id))
        .unwrap()
        .iter()
        .all(|s| Path::new(&s.file_path).is_file()));
}

#[test]
fn reclamation_preserves_shared_references_and_recovers_unindexed_files() {
    let fixture = fixture();
    let recording = fixture
        .storage
        .with_read_connection(|c| storage::get_recording(c, &fixture.id))
        .unwrap();
    let old = destination(&recording, "segment", "video", 0, "old");
    let next = destination(&recording, "segment", "video", 0, "next");
    fs::create_dir_all(old.parent().unwrap()).unwrap();
    fs::write(&old, b"synthetic old media").unwrap();
    fs::write(&next, b"synthetic replacement media").unwrap();
    save(
        &fixture.storage,
        &recording,
        "segment",
        "video",
        0,
        "old",
        &old,
    )
    .unwrap();
    save(
        &fixture.storage,
        &recording,
        "run-segment",
        "video",
        0,
        "old",
        &old,
    )
    .unwrap();
    save(
        &fixture.storage,
        &recording,
        "segment",
        "video",
        0,
        "next",
        &next,
    )
    .unwrap();
    lifecycle::reclaim(&fixture.storage, &recording).unwrap();
    assert!(old.is_file(), "the run still references this generation");
    let session = lifecycle::lock().unwrap().lease(&fixture.storage, &old);
    save(
        &fixture.storage,
        &recording,
        "run-segment",
        "video",
        0,
        "next",
        &next,
    )
    .unwrap();
    let orphan = old.with_file_name("interrupted.partial.mp4");
    fs::write(&orphan, b"synthetic interrupted output").unwrap();
    lifecycle::reclaim(&fixture.storage, &recording).unwrap();
    assert!(old.exists(), "a playback owner retains a superseded file");
    assert!(!orphan.exists());
    assert!(next.is_file());

    // Reusing a retired filename must survive the last playback reader leaving
    // before the preparation owner has saved its metadata.
    let output = lifecycle::lock().unwrap().lease(&fixture.storage, &old);
    drop(session);
    assert!(old.exists());
    save(
        &fixture.storage,
        &recording,
        "segment",
        "video",
        0,
        "old",
        &old,
    )
    .unwrap();
    drop(output);
    assert!(old.exists(), "the republished generation is still indexed");
    let last_reader = lifecycle::lock().unwrap().lease(&fixture.storage, &old);
    save(
        &fixture.storage,
        &recording,
        "segment",
        "video",
        0,
        "next",
        &next,
    )
    .unwrap();
    drop(last_reader);
    assert!(
        !old.exists(),
        "the final release reclaims unreferenced media"
    );
}
