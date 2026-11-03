use super::*;
use crate::storage::{self, AppStorage};
use rusqlite::params;
use std::{fs, path::PathBuf, sync::atomic::AtomicBool};

struct Fixture {
    storage: AppStorage,
    id: String,
    segment: storage::RecordingSegmentRow,
    root: PathBuf,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
fn fixture() -> Fixture {
    let root = std::env::temp_dir().join(format!(
        "daydream-cleanup-synthetic-{}",
        uuid::Uuid::now_v7()
    ));
    let id = uuid::Uuid::now_v7().to_string();
    let dir = root.join("recordings").join(&id);
    fs::create_dir_all(&dir).unwrap();
    let mut segment = crate::ffmpeg::audio_timeline_tests::gap_fixture();
    let path = dir.join("source.mkv");
    fs::rename(&segment.file_path, &path).unwrap();
    segment.file_path = path.to_string_lossy().into_owned();
    segment.id = uuid::Uuid::now_v7().to_string();
    segment.recording_id = id.clone();
    let storage = storage::test_storage(root.join("database"));
    storage
        .with_connection(|c| {
            storage::create_recording(
                c,
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
            storage::create_recording_segment(
                c,
                &storage::CreateRecordingSegmentInput {
                    id: segment.id.clone(),
                    recording_id: id.clone(),
                    file_path: segment.file_path.clone(),
                    segment_index: 1,
                    started_at_ms: 0,
                },
            )?;
            storage::finalize_recording_segment(
                c,
                &storage::FinalizeRecordingSegmentInput {
                    id: segment.id.clone(),
                    duration_ms: 12_000,
                    ended_at_ms: 12_000,
                    updated_at_ms: 12_000,
                    desktop_audio_stream_index: Some(1),
                    microphone_audio_stream_index: Some(2),
                    video_stream_index: Some(0),
                    probe_json: Some(
                        "{\"audio\":[{\"index\":1,\"channels\":1},{\"index\":2,\"channels\":1}]}"
                            .into(),
                    ),
                },
            )?;
            storage::mark_recording_completed(c, &id, 12_000)?;
            Ok(())
        })
        .unwrap();
    Fixture {
        storage,
        id,
        segment,
        root,
    }
}
fn admit(f: &Fixture) -> (String, String) {
    let revision = uuid::Uuid::now_v7().to_string();
    let fingerprint = f
        .storage
        .with_connection(|c| persistence::fingerprint(c, &f.id))
        .unwrap();
    f.storage.with_connection(|c|{
        c.execute("INSERT INTO audio_cleanup_generations(id,recording_id,config_json,source_fingerprint,status,created_at_ms) VALUES(?1,?2,?3,?4,'running',1)",params![revision,f.id,serde_json::to_string(&AudioCleanupConfig{mode:CleanupMode::Auto,..Default::default()}).unwrap(),fingerprint])?;
        c.execute("INSERT INTO recording_audio_cleanup(recording_id,pending_revision) VALUES(?1,?2) ON CONFLICT(recording_id) DO UPDATE SET pending_revision=excluded.pending_revision",params![f.id,revision])?;Ok(())
    }).unwrap();
    (revision, fingerprint)
}
fn generate(f: &Fixture, revision: &str, fingerprint: &str) {
    service::generate(
        &f.storage,
        &f.id,
        revision,
        fingerprint,
        &AudioCleanupConfig {
            mode: CleanupMode::Auto,
            ..Default::default()
        },
        vec![],
        &AtomicBool::new(false),
    )
    .unwrap();
}
fn job(f: &Fixture) -> String {
    let id = uuid::Uuid::now_v7().to_string();
    f.storage
        .with_connection(|c| {
            storage::create_processing_job(
                c,
                &storage::CreateProcessingJobInput {
                    id: id.clone(),
                    recording_id: f.id.clone(),
                    kind: "transcription".into(),
                    status: "queued".into(),
                    created_at_ms: 1,
                    updated_at_ms: 1,
                    error: None,
                },
            )
        })
        .unwrap();
    id
}
#[test]
fn saved_audio_roundtrip_preserves_gaps_originals_job_pins_and_deletion() {
    let f = fixture();
    let original = fs::read(&f.segment.file_path).unwrap();
    let legacy = job(&f);
    let (revision, fingerprint) = admit(&f);
    generate(&f, &revision, &fingerprint);
    let state = read_state(&f.storage, &f.id).unwrap();
    assert_eq!(state.phase, "ready");
    assert_eq!(state.progress, 1.0);
    let manifest = f
        .storage
        .with_connection(|c| active_manifest(c, &f.id))
        .unwrap()
        .unwrap();
    assert_eq!(manifest.blocks.len(), 1);
    assert_eq!(manifest.blocks[0].duration_ms, 12_000);
    assert_eq!(fs::read(&f.segment.file_path).unwrap(), original);
    let output = f.root.join("microphone.wav");
    media::extract_cleaned_window(
        &manifest,
        &f.segment,
        4500,
        1000,
        &output,
        &AtomicBool::new(false),
    )
    .unwrap();
    let mut reader = hound::WavReader::open(&output).unwrap();
    let values: Vec<_> = reader.samples::<i16>().map(Result::unwrap).collect();
    assert_eq!(values.len(), 16_000);
    assert!(values[1600..9600].iter().all(|sample| sample.abs() < 2));
    let pinned = job(&f);
    f.storage
        .with_connection(|c| pin_job(c, &pinned, &f.id))
        .unwrap();
    f.storage
        .with_connection(|c| {
            c.execute(
                "UPDATE recording_audio_cleanup SET active_revision=NULL WHERE recording_id=?1",
                [&f.id],
            )
        })
        .unwrap();
    assert!(f
        .storage
        .with_connection(|c| pinned_manifest(c, &legacy, &f.id))
        .unwrap()
        .is_none());
    assert_eq!(
        f.storage
            .with_connection(|c| pinned_manifest(c, &pinned, &f.id))
            .unwrap()
            .unwrap()
            .revision,
        revision
    );
    let recording = f
        .storage
        .with_connection(|c| storage::get_recording(c, &f.id))
        .unwrap();
    crate::library::delete_recording_row(&f.storage, &f.root.join("recordings"), &recording)
        .unwrap();
    assert!(!std::path::Path::new(&recording.storage_dir).exists());
    assert_eq!(
        f.storage
            .with_connection(|c| c.query_row(
                "SELECT count(*) FROM audio_cleanup_generations WHERE recording_id=?1",
                [&f.id],
                |r| r.get::<_, i64>(0)
            ))
            .unwrap(),
        0
    );
}
#[test]
fn redacted_reference_passes_microphone_through_and_invalidates_ready_audio() {
    let f = fixture();
    f.storage
        .with_connection(|c| {
            storage::create_redaction(
                c,
                &storage::CreateRedactionInput {
                    id: uuid::Uuid::now_v7().to_string(),
                    recording_id: f.id.clone(),
                    start_offset_ms: 0,
                    end_offset_ms: 12_000,
                    track_ids_json: "[\"desktop-audio-1\"]".into(),
                    reason: None,
                    created_at_ms: 1,
                },
            )
        })
        .unwrap();
    let (revision, fingerprint) = admit(&f);
    generate(&f, &revision, &fingerprint);
    assert_eq!(read_state(&f.storage, &f.id).unwrap().matched_ms, 0);
    let manifest = f
        .storage
        .with_connection(|c| active_manifest(c, &f.id))
        .unwrap()
        .unwrap();
    let (raw, channels) = media::decode(
        &f.segment.file_path,
        2,
        0,
        4000,
        &f.root,
        &AtomicBool::new(false),
    )
    .unwrap();
    let (cleaned, cleaned_channels) = media::decode(
        &manifest.blocks[0].path,
        0,
        0,
        4000,
        &f.root,
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(channels, cleaned_channels);
    assert!(raw.iter().zip(&cleaned).all(|(a, b)| (a - b).abs() < 2e-6));
    f.storage
        .with_connection(|c| {
            storage::create_redaction(
                c,
                &storage::CreateRedactionInput {
                    id: uuid::Uuid::now_v7().to_string(),
                    recording_id: f.id.clone(),
                    start_offset_ms: 2000,
                    end_offset_ms: 3000,
                    track_ids_json: "[\"microphone-2\"]".into(),
                    reason: None,
                    created_at_ms: 2,
                },
            )
        })
        .unwrap();
    assert!(f
        .storage
        .with_connection(|c| active_manifest(c, &f.id))
        .is_err());
    assert_eq!(read_state(&f.storage, &f.id).unwrap().phase, "invalidated");
}
#[test]
fn interrupted_generation_does_not_replace_applied_audio_and_can_resume() {
    let f = fixture();
    let (first, fingerprint) = admit(&f);
    generate(&f, &first, &fingerprint);
    let (second, fingerprint) = admit(&f);
    assert!(service::generate(
        &f.storage,
        &f.id,
        &second,
        &fingerprint,
        &AudioCleanupConfig {
            mode: CleanupMode::Auto,
            ..Default::default()
        },
        vec![],
        &AtomicBool::new(true)
    )
    .is_err());
    recover_interrupted(&f.storage).unwrap();
    assert_eq!(read_state(&f.storage, &f.id).unwrap().phase, "cancelled");
    assert_eq!(
        f.storage
            .with_connection(|c| active_manifest(c, &f.id))
            .unwrap()
            .unwrap()
            .revision,
        first
    );
    f.storage
        .with_connection(|c| {
            c.execute(
                "UPDATE audio_cleanup_generations SET status='running' WHERE id=?1",
                [&second],
            )
        })
        .unwrap();
    generate(&f, &second, &fingerprint);
    assert_eq!(
        f.storage
            .with_connection(|c| active_manifest(c, &f.id))
            .unwrap()
            .unwrap()
            .revision,
        second
    );
}

#[test]
fn real_playback_serves_cleaned_microphone_and_rejects_a_new_redaction() {
    use std::io::{Read, Write};
    let f = fixture();
    let (revision, fingerprint) = admit(&f);
    generate(&f, &revision, &fingerprint);
    let recording = f
        .storage
        .with_connection(|c| storage::get_recording(c, &f.id))
        .unwrap();
    let server = crate::playback_server::PlaybackServer::start().unwrap();
    let segments = vec![f.segment.clone()];
    let assets = server
        .create_recording_source_session(
            &recording,
            &segments,
            &Default::default(),
            f.storage.clone(),
            false,
            true,
        )
        .unwrap();
    let url = assets.segments[0]
        .audio
        .iter()
        .find(|audio| audio.kind == crate::library::LibraryTrackKind::Microphone)
        .unwrap()
        .url
        .clone();
    let request = |url: &str| {
        let url = reqwest::Url::parse(url).unwrap();
        let mut socket = std::net::TcpStream::connect(("127.0.0.1", url.port().unwrap())).unwrap();
        socket
            .set_read_timeout(Some(std::time::Duration::from_secs(10)))
            .unwrap();
        write!(
            socket,
            "GET {}?durationMs=4000 HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
            url.path()
        )
        .unwrap();
        let mut bytes = Vec::new();
        socket.read_to_end(&mut bytes).unwrap();
        bytes
    };
    let bytes = request(&url);
    let boundary = bytes
        .windows(4)
        .position(|bytes| bytes == b"\r\n\r\n")
        .unwrap();
    assert!(std::str::from_utf8(&bytes[..boundary])
        .unwrap()
        .starts_with("HTTP/1.1 200"));
    let file = f.root.join("playback.m4a");
    fs::write(&file, &bytes[boundary + 4..]).unwrap();
    let decoded = std::process::Command::new(crate::ffmpeg::binary_path())
        .args(["-v", "error", "-i"])
        .arg(file)
        .args(["-ac", "1", "-ar", "16000", "-f", "f32le", "pipe:1"])
        .output()
        .unwrap();
    assert!(
        decoded.status.success(),
        "The synthetic cleaned stream was not valid playable audio: {} bytes; {}",
        bytes.len() - boundary - 4,
        String::from_utf8_lossy(&decoded.stderr)
    );
    let samples: Vec<_> = decoded
        .stdout
        .chunks_exact(4)
        .map(|bytes| f32::from_le_bytes(bytes.try_into().unwrap()))
        .collect();
    assert!(samples.len() >= 60_000);
    let crossings = samples[8000..24000]
        .windows(2)
        .filter(|pair| pair[0] < 0.0 && pair[1] >= 0.0)
        .count();
    assert!(
        (850..=910).contains(&crossings),
        "Playback did not serve the 880 Hz microphone fixture"
    );
    f.storage
        .with_connection(|c| {
            storage::create_redaction(
                c,
                &storage::CreateRedactionInput {
                    id: uuid::Uuid::now_v7().to_string(),
                    recording_id: f.id.clone(),
                    start_offset_ms: 1000,
                    end_offset_ms: 2000,
                    track_ids_json: "[\"desktop-audio-1\"]".into(),
                    reason: None,
                    created_at_ms: 1,
                },
            )
        })
        .unwrap();
    assert!(std::str::from_utf8(&request(&url))
        .unwrap()
        .starts_with("HTTP/1.1 403"));
    server.close_session(&assets.session_id).unwrap();
}

#[test]
fn rejected_cleanup_playback_retries_do_not_retain_sessions() {
    let f = fixture();
    let (revision, fingerprint) = admit(&f);
    generate(&f, &revision, &fingerprint);
    let recording = f
        .storage
        .with_connection(|c| storage::get_recording(c, &f.id))
        .unwrap();
    let manifest = f
        .storage
        .with_connection(|c| active_manifest(c, &f.id))
        .unwrap()
        .unwrap();
    let server = crate::playback_server::PlaybackServer::start().unwrap();
    let segments = vec![f.segment.clone()];
    let create = || {
        server.create_recording_source_session(
            &recording,
            &segments,
            &Default::default(),
            f.storage.clone(),
            false,
            true,
        )
    };
    let existing = create().unwrap();
    assert_eq!(server.session_count(), 1);
    // A derived file removed outside the app must fail before session allocation.
    let block = &manifest.blocks[0];
    let moved = format!("{}.held", block.path);
    fs::rename(&block.path, &moved).unwrap();
    for _ in 0..3 {
        assert!(create().unwrap_err().contains("Cleanup is unavailable"));
        assert_eq!(server.session_count(), 1);
    }
    fs::rename(&moved, &block.path).unwrap();
    // A valid manifest with incomplete duration fails after allocation and retires that session.
    let mut incomplete = manifest.blocks.clone();
    incomplete[0].duration_ms -= 1;
    f.storage
        .with_connection(|c| {
            c.execute(
                "UPDATE audio_cleanup_generations SET manifest_json=?2 WHERE id=?1",
                params![revision, serde_json::to_string(&incomplete).unwrap()],
            )
        })
        .unwrap();
    for _ in 0..3 {
        assert!(create().unwrap_err().contains("incomplete"));
        assert_eq!(server.session_count(), 1);
    }
    f.storage
        .with_connection(|c| {
            storage::create_redaction(
                c,
                &storage::CreateRedactionInput {
                    id: uuid::Uuid::now_v7().to_string(),
                    recording_id: f.id.clone(),
                    start_offset_ms: 1000,
                    end_offset_ms: 2000,
                    track_ids_json: "[\"desktop-audio-1\"]".into(),
                    reason: None,
                    created_at_ms: 1,
                },
            )
        })
        .unwrap();
    for _ in 0..3 {
        assert!(create().unwrap_err().contains("Cleanup is unavailable"));
        assert_eq!(server.session_count(), 1);
    }
    f.storage
        .with_connection(|c| {
            c.execute(
                "UPDATE recording_audio_cleanup SET active_revision=NULL WHERE recording_id=?1",
                [&f.id],
            )
        })
        .unwrap();
    let original = create().unwrap();
    assert_eq!(server.session_count(), 2);
    server.close_session(&original.session_id).unwrap();
    server.close_session(&existing.session_id).unwrap();
    assert_eq!(server.session_count(), 0);
}

#[test]
fn legacy_microphone_identity_is_hidden_before_cleanup_decodes_audio() {
    let f = fixture();
    let track = uuid::Uuid::now_v7().to_string();
    f.storage
        .with_connection(|c| {
            storage::upsert_recording_track(
                c,
                &storage::UpsertRecordingTrackInput {
                    id: track.clone(),
                    recording_id: f.id.clone(),
                    kind: "microphone".into(),
                    label: "Synthetic microphone".into(),
                    source_id: None,
                    path: None,
                    codec: None,
                    created_at_ms: 0,
                },
            )?;
            storage::create_redaction(
                c,
                &storage::CreateRedactionInput {
                    id: uuid::Uuid::now_v7().to_string(),
                    recording_id: f.id.clone(),
                    start_offset_ms: 0,
                    end_offset_ms: 12_000,
                    track_ids_json: serde_json::to_string(&vec![track]).unwrap(),
                    reason: None,
                    created_at_ms: 1,
                },
            )?;
            Ok(())
        })
        .unwrap();
    let (revision, fingerprint) = admit(&f);
    generate(&f, &revision, &fingerprint);
    let manifest = f
        .storage
        .with_connection(|c| active_manifest(c, &f.id))
        .unwrap()
        .unwrap();
    let (samples, _) = media::decode(
        &manifest.blocks[0].path,
        0,
        0,
        1000,
        &f.root,
        &AtomicBool::new(false),
    )
    .unwrap();
    assert!(samples.iter().all(|sample| *sample == 0.0));
    assert_eq!(manifest.blocks[0].matched_ms, 0);
}
