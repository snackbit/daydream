use super::super::local_requests::transcribe_local_via_rest;
use super::*;

#[derive(Default)]
struct DelayedCleanupRunner {
    started: AtomicBool,
    finished: AtomicBool,
    copy: StdMutex<Option<PathBuf>>,
}
impl LocalTranscriptionRunner for DelayedCleanupRunner {
    fn transcribe(
        &self,
        input: LocalTranscriptionInput,
    ) -> Result<VerboseTranscription, TranscriptionError> {
        *self.copy.lock().unwrap() = Some(input.audio_path.clone());
        self.started.store(true, Ordering::SeqCst);
        for _ in 0..300 {
            if input.cancel.load(Ordering::SeqCst) {
                thread::sleep(Duration::from_millis(100));
                self.finished.store(true, Ordering::SeqCst);
                return Err(transcription_error(
                    "ai_transcription_cancelled",
                    "cancelled",
                ));
            }
            thread::sleep(Duration::from_millis(10));
        }
        panic!("request cancellation did not reach local handler");
    }
}

#[test]
fn local_cancel_acknowledges_handler_cleanup_before_returning() {
    tauri::async_runtime::block_on(async {
        let root = temp_test_dir();
        create_model_file(&root, "tiny");
        let runner = Arc::new(DelayedCleanupRunner::default());
        let server = LocalTranscriptionServer::start(root.clone(), runner.clone()).unwrap();
        let endpoint = server.endpoint();
        let audio = root.join("input.wav");
        fs::write(&audio, b"private audio").unwrap();
        let cancel = Arc::new(AtomicBool::new(false));
        let operation_cancel = cancel.clone();
        let task = tauri::async_runtime::spawn(async move {
            transcribe_local_via_rest(
                RestTranscriptionRequest {
                    local_request_id: None,
                    local_scratch_owner: None,
                    audio_path: &audio,
                    bearer_token: Some(&endpoint.token),
                    cancel: Some(operation_cancel),
                    endpoint: &endpoint.url,
                    language: "auto",
                    local_backend: Some("cpu"),
                    max_initial_timestamp_seconds: None,
                    model: "tiny",
                    timeout: Duration::from_secs(5),
                    word_timestamps: false,
                },
                None,
            )
            .await
        });
        wait_for_atomic(&runner.started, "handler started").await;
        cancel.store(true, Ordering::SeqCst);
        assert_eq!(
            task.await.unwrap().unwrap_err().code,
            "ai_transcription_cancelled"
        );
        let drained = runner.finished.load(Ordering::SeqCst);
        let copy_gone = !runner.copy.lock().unwrap().as_ref().unwrap().exists();
        drop(server);
        fs::remove_dir_all(root).unwrap();
        assert!(
            drained,
            "client returned before local handler acknowledged cancellation"
        );
        assert!(
            copy_gone,
            "private WAV copy outlived successful request drain"
        );
    });
}

fn recording_fixture(root: &Path) -> (storage::AppStorage, storage::RecordingRow, String, PathBuf) {
    let storage = storage::test_storage(PathBuf::new());
    let id = Uuid::now_v7().to_string();
    let job_id = Uuid::now_v7().to_string();
    let directory = root.join("recordings").join(&id);
    fs::create_dir_all(&directory).unwrap();
    let recording = storage
        .with_connection(|connection| {
            storage::create_recording(
                connection,
                &storage::CreateRecordingInput {
                    active_scene_id: None,
                    created_at_ms: 1,
                    default_fps: 1.0,
                    id: id.clone(),
                    notes: None,
                    output_root: None,
                    quality_label: "balanced-ocr".into(),
                    storage_dir: directory.to_string_lossy().into(),
                },
            )?;
            storage::create_processing_job(
                connection,
                &storage::CreateProcessingJobInput {
                    created_at_ms: 1,
                    error: None,
                    id: job_id.clone(),
                    kind: "ai-processing".into(),
                    recording_id: id.clone(),
                    status: "running".into(),
                    updated_at_ms: 1,
                },
            )?;
            storage::mark_recording_completed(connection, &id, 2)
        })
        .unwrap();
    let audio = root
        .join("processing/transcription")
        .join(&job_id)
        .join("input.wav");
    fs::create_dir_all(audio.parent().unwrap()).unwrap();
    fs::write(&audio, b"private audio").unwrap();
    (storage, recording, job_id, audio)
}

async fn deletion_drains_handler(abort_client: bool) {
    let root = temp_test_dir();
    create_model_file(&root, "tiny");
    let (storage, recording, job_id, audio) = recording_fixture(&root);
    let runner = Arc::new(DelayedCleanupRunner::default());
    let server = LocalTranscriptionServer::start(root.clone(), runner.clone()).unwrap();
    let endpoint = server.endpoint();
    let lease = crate::recording_privacy::processing_lease(&recording.id).await;
    let cancel = Arc::new(AtomicBool::new(false));
    let operation_cancel = cancel.clone();
    let task = tauri::async_runtime::spawn(async move {
        transcribe_local_via_rest(
            RestTranscriptionRequest {
                local_request_id: None,
                local_scratch_owner: None,
                audio_path: &audio,
                bearer_token: Some(&endpoint.token),
                cancel: Some(operation_cancel),
                endpoint: &endpoint.url,
                language: "auto",
                local_backend: Some("cpu"),
                max_initial_timestamp_seconds: None,
                model: "tiny",
                timeout: Duration::from_secs(5),
                word_timestamps: false,
            },
            Some(lease),
        )
        .await
    });
    wait_for_atomic(&runner.started, "handler start").await;
    if abort_client {
        task.abort();
    } else {
        cancel.store(true, Ordering::SeqCst);
    }
    let mutation = crate::recording_privacy::mutation_lease(&recording.id).await;
    assert!(
        runner.finished.load(Ordering::SeqCst),
        "exclusive deletion bypassed active local handler"
    );
    crate::recording_deletion::delete_recording(&storage, &root.join("recordings"), &recording)
        .unwrap();
    assert!(!runner.copy.lock().unwrap().as_ref().unwrap().exists());
    assert!(!root
        .join("processing/local-transcription")
        .join(job_id)
        .exists());
    if abort_client {
        assert!(task.await.is_err());
    } else {
        assert_eq!(
            task.await.unwrap().unwrap_err().code,
            "ai_transcription_cancelled"
        );
    }
    drop(mutation);
    drop(server);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn successful_deletion_waits_for_local_wav_cleanup() {
    tauri::async_runtime::block_on(deletion_drains_handler(false));
}

#[test]
fn aborted_client_cannot_release_handler_lease_or_recreate_deleted_wav() {
    tauri::async_runtime::block_on(deletion_drains_handler(true));
}

#[test]
fn rejected_unconsumed_local_request_releases_its_lease() {
    tauri::async_runtime::block_on(async {
        let root = temp_test_dir();
        create_model_file(&root, "tiny");
        let (_storage, recording, _, audio) = recording_fixture(&root);
        let runner = Arc::new(MockRunner::default());
        let server = LocalTranscriptionServer::start(root.clone(), runner.clone()).unwrap();
        let endpoint = server.endpoint();
        let lease = crate::recording_privacy::processing_lease(&recording.id).await;
        let result = transcribe_local_via_rest(
            RestTranscriptionRequest {
                local_request_id: None,
                local_scratch_owner: None,
                audio_path: &audio,
                bearer_token: Some("wrong"),
                cancel: None,
                endpoint: &endpoint.url,
                language: "auto",
                local_backend: Some("cpu"),
                max_initial_timestamp_seconds: None,
                model: "tiny",
                timeout: Duration::from_secs(5),
                word_timestamps: false,
            },
            Some(lease),
        )
        .await
        .unwrap_err();
        assert_eq!(result.code, "ai_transcription_remote_auth_failed");
        assert!(crate::recording_privacy::try_mutation_lease(&recording.id).is_some());
        assert!(runner.calls.lock().unwrap().is_empty());
        assert!(!root.join("processing/local-transcription").exists());
        drop(server);
        fs::remove_dir_all(root).unwrap();
    });
}

#[test]
fn revoked_pending_request_cannot_create_a_delayed_wav_copy() {
    tauri::async_runtime::block_on(async {
        let root = temp_test_dir();
        create_model_file(&root, "tiny");
        let (_storage, recording, job_id, audio) = recording_fixture(&root);
        let runner = Arc::new(MockRunner::default());
        let server = LocalTranscriptionServer::start(root.clone(), runner.clone()).unwrap();
        let endpoint = server.endpoint();
        let registration = super::super::local_receipts::LocalRequestRegistration::new(
            Arc::new(AtomicBool::new(false)),
            Some(crate::recording_privacy::processing_lease(&recording.id).await),
            Some(job_id),
        );
        let id = registration.id().to_string();
        registration.drain(true).await;
        drop(registration);
        assert!(crate::recording_privacy::try_mutation_lease(&recording.id).is_some());
        let response = transcribe_via_rest(RestTranscriptionRequest {
            local_request_id: Some(&id),
            local_scratch_owner: None,
            audio_path: &audio,
            bearer_token: Some(&endpoint.token),
            cancel: None,
            endpoint: &endpoint.url,
            language: "auto",
            local_backend: Some("cpu"),
            max_initial_timestamp_seconds: None,
            model: "tiny",
            timeout: Duration::from_secs(5),
            word_timestamps: false,
        })
        .await
        .unwrap_err();
        assert_eq!(response.code, "ai_transcription_cancelled");
        assert!(runner.calls.lock().unwrap().is_empty());
        assert!(!root.join("processing/local-transcription").exists());
        drop(server);
        fs::remove_dir_all(root).unwrap();
    });
}

#[test]
fn failed_parent_cancels_active_model_and_releases_processing_lease() {
    tauri::async_runtime::block_on(async {
        let root = temp_test_dir();
        create_model_file(&root, "tiny");
        let (storage, recording, job_id, audio) = recording_fixture(&root);
        let runner = Arc::new(DelayedCleanupRunner::default());
        let server = LocalTranscriptionServer::start(root.clone(), runner.clone()).unwrap();
        let endpoint = server.endpoint();
        let lease = crate::recording_privacy::processing_lease(&recording.id).await;
        let cancel = Arc::new(AtomicBool::new(false));
        let operation_cancel = cancel.clone();
        let mut task = tauri::async_runtime::spawn(async move {
            transcribe_local_via_rest(
                RestTranscriptionRequest {
                    local_request_id: None,
                    local_scratch_owner: None,
                    audio_path: &audio,
                    bearer_token: Some(&endpoint.token),
                    cancel: Some(operation_cancel),
                    endpoint: &endpoint.url,
                    language: "auto",
                    local_backend: Some("cpu"),
                    max_initial_timestamp_seconds: None,
                    model: "tiny",
                    timeout: Duration::from_secs(5),
                    word_timestamps: false,
                },
                Some(lease),
            )
            .await
        });
        wait_for_atomic(&runner.started, "handler start").await;
        storage
            .with_connection(|connection| {
                storage::update_processing_job_status(
                    connection,
                    &job_id,
                    "failed",
                    Some("other_chunk_failed"),
                    3,
                )
            })
            .unwrap();
        crate::ai::cancel_job_if_inactive(&storage, &job_id, &cancel);
        let result = tokio::time::timeout(Duration::from_millis(600), &mut task).await;
        let drained_promptly = result.is_ok();
        if let Ok(result) = result {
            assert_eq!(
                result.unwrap().unwrap_err().code,
                "ai_transcription_cancelled"
            );
        } else {
            cancel.store(true, Ordering::SeqCst);
            let _ = task.await;
        }
        assert!(crate::recording_privacy::try_mutation_lease(&recording.id).is_some());
        assert!(!runner.copy.lock().unwrap().as_ref().unwrap().exists());
        drop(server);
        fs::remove_dir_all(root).unwrap();
        assert!(
            drained_promptly,
            "failed parent left active inference holding its processing lease"
        );
    });
}
