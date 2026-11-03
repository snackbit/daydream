use super::*;
use crate::settings::RecallSettings;

async fn boundary_worker(script: &str) -> LocalWorker {
    let worker = LocalWorker {
        models_dir: PathBuf::new(),
        worker_path: PathBuf::new(),
        state: Arc::new(Mutex::new(None)),
    };
    let mut child = Command::new("sh")
        .args(["-c", script])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let input = child.stdin.take().unwrap();
    let output = BufReader::new(child.stdout.take().unwrap());
    *worker.state.lock().await = Some(Running {
        child,
        input,
        output,
        identity: "multilingual-e5-small-q8_0:cpu::".into(),
        last_used: Instant::now(),
    });
    worker
}

#[test]
fn cancellation_kills_busy_worker_and_crashes_discard_transport() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let settings = RecallSettings::default().embedding;
            let worker = boundary_worker("read request; while :; do :; done").await;
            let cancelled = Arc::new(AtomicBool::new(false));
            let signal = cancelled.clone();
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(50)).await;
                signal.store(true, Ordering::Release);
            });
            let result = worker
                .embed(
                    &settings,
                    EmbeddingPurpose::Query,
                    &["synthetic fixture".into()],
                    &cancelled,
                )
                .await;
            assert_eq!(result.unwrap_err(), "recall_cancelled");
            assert!(worker.state.lock().await.is_none());

            let worker = boundary_worker("read request; exit 1").await;
            let result = worker
                .embed(
                    &settings,
                    EmbeddingPurpose::Query,
                    &["synthetic fixture".into()],
                    &AtomicBool::new(false),
                )
                .await;
            assert_eq!(result.unwrap_err(), "recall_embedding_worker_crashed");
            assert!(worker.state.lock().await.is_none());
        });
}
#[test]
fn dropping_inflight_future_discards_unread_worker_reply() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let settings = RecallSettings::default().embedding;
            let marker = std::env::temp_dir().join(format!(
                "daydream-embedding-request-{}",
                uuid::Uuid::now_v7()
            ));
            let marker_argument = marker.to_string_lossy().replace('\'', "'\\''");
            let worker = boundary_worker(&format!(
                "read request; printf ready > '{marker_argument}'; while :; do :; done"
            ))
            .await;
            let cancelled = AtomicBool::new(false);
            let texts = ["synthetic fixture".into()];
            {
                let request = worker.embed(
                    &settings,
                    EmbeddingPurpose::Query,
                    &texts,
                    &cancelled,
                );
                tokio::pin!(request);
                // Another test can own the shared inference slot. Wait for the
                // child to receive this request before cancelling its future.
                tokio::select! {
                    biased;
                    result = &mut request => panic!("Synthetic worker should remain in flight: {result:?}"),
                    result = tokio::time::timeout(Duration::from_secs(10), async {
                        while !marker.exists() {
                            tokio::time::sleep(Duration::from_millis(10)).await;
                        }
                    }) => result.expect("Synthetic worker did not receive request"),
                }
            }
            std::fs::remove_file(marker).unwrap();
            assert!(
                worker.state.lock().await.is_none(),
                "An unread reply must not survive cancellation by future drop"
            );
        });
}

#[test]
fn process_reply_is_bounded_and_success_preserves_worker() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let settings = RecallSettings::default().embedding;
            let worker = boundary_worker(
                "read request; printf '%s\\n' '{\"vectors\":[[1,2]]}'; read request",
            )
            .await;
            assert_eq!(
                worker
                    .embed(
                        &settings,
                        EmbeddingPurpose::Document,
                        &["synthetic fixture".into()],
                        &AtomicBool::new(false)
                    )
                    .await
                    .unwrap(),
                vec![vec![1.0, 2.0]]
            );
            assert!(worker.state.lock().await.is_some());
            let worker = boundary_worker("read request; head -c 1100000 /dev/zero").await;
            let result = worker
                .embed(
                    &settings,
                    EmbeddingPurpose::Query,
                    &["synthetic fixture".into()],
                    &AtomicBool::new(false),
                )
                .await;
            assert_eq!(
                result.unwrap_err(),
                "recall_embedding_worker_response_limit"
            );
            assert!(worker.state.lock().await.is_none());
        });
}
#[test]
fn auto_retries_cpu_when_optional_gpu_process_cannot_start() {
    use std::os::unix::fs::PermissionsExt;
    let root = std::env::temp_dir().join(format!(
        "daydream-embedding-backend-{}",
        uuid::Uuid::now_v7()
    ));
    std::fs::create_dir(&root).unwrap();
    let cpu = root.join("daydream-embedding-worker");
    let gpu = root.join("daydream-embedding-worker-vulkan");
    std::fs::write(
        &cpu,
        b"#!/bin/sh\nprintf '%s\\n' '{\"ready\":true}'\nread request\n",
    )
    .unwrap();
    // Simulate missing Vulkan loader/driver startup: the executable exits
    // before the ready handshake. This is a real process boundary.
    std::fs::write(&gpu, b"#!/bin/sh\nexit 1\n").unwrap();
    for file in [&cpu, &gpu] {
        std::fs::set_permissions(file, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let mut settings = RecallSettings::default().embedding;
            settings.local_backend = "auto".into();
            let mut running =
                start_configured_worker(&cpu, &root.join("synthetic.gguf"), &settings, "fixture")
                    .await
                    .unwrap();
            assert!(running.child.try_wait().unwrap().is_none());
            running.child.kill().await.unwrap();
            settings.local_backend = "vulkan".into();
            assert!(start_configured_worker(
                &cpu,
                &root.join("synthetic.gguf"),
                &settings,
                "fixture"
            )
            .await
            .is_err());
            std::fs::remove_file(&gpu).unwrap();
            assert_eq!(
                start_configured_worker(&cpu, &root.join("synthetic.gguf"), &settings, "fixture")
                    .await
                    .err()
                    .unwrap(),
                "recall_embedding_vulkan_unavailable"
            );
        });
    std::fs::remove_dir_all(root).unwrap();
}
