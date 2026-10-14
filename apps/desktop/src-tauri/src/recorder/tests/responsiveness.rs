//! Uses the production Tauri dispatch and synthetic resource barriers. MockRuntime
//! substitutes the OS window only; no portal, devices or private data are opened.
use super::*;
use std::sync::mpsc;
use tauri::{
    ipc::{InvokeBody, InvokeResponse},
    test::MockRuntime,
    webview::InvokeRequest,
    WebviewWindow,
};

// Deliberate old behavior verifies that the fixture detects UI-thread waits.
#[tauri::command]
fn synchronous_database_control(
    storage: tauri::State<'_, storage::AppStorage>,
) -> Result<(), String> {
    storage
        .with_connection(|_| Ok(()))
        .map_err(|_| "Synthetic read failed".into())
}

fn app() -> (tauri::App<MockRuntime>, WebviewWindow<MockRuntime>) {
    let app = tauri::test::mock_builder()
        .invoke_handler(tauri::generate_handler![
            crate::recorder::get_recorder_scene,
            crate::recorder::source_recording::get_capture_setup,
            crate::recorder::get_recorder_recording_stats,
            crate::recorder::get_recorder_preview_status,
            crate::recorder::stop_recorder_preview,
            crate::recorder::save_recorder_scene,
            crate::recorder::source_recording::save_capture_setup,
            synchronous_database_control,
        ])
        .build(tauri::test::mock_context(tauri::test::noop_assets()))
        .unwrap();
    app.manage(test_storage());
    app.manage(RecorderManager::new(PathBuf::from(
        "/tmp/daydream-synthetic-responsive",
    )));
    let window = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();
    (app, window)
}

fn dispatch(window: &WebviewWindow<MockRuntime>, command: &str) -> mpsc::Receiver<InvokeResponse> {
    dispatch_body(window, command, InvokeBody::default())
}

fn dispatch_body(
    window: &WebviewWindow<MockRuntime>,
    command: &str,
    body: InvokeBody,
) -> mpsc::Receiver<InvokeResponse> {
    let (sender, receiver) = mpsc::channel();
    window.as_ref().clone().on_message(
        InvokeRequest {
            cmd: command.into(),
            callback: tauri::ipc::CallbackFn(0),
            error: tauri::ipc::CallbackFn(1),
            url: window.url().unwrap(),
            body,
            headers: Default::default(),
            invoke_key: tauri::test::INVOKE_KEY.into(),
        },
        Box::new(move |_, _, response, _, _| {
            sender.send(response).unwrap();
        }),
    );
    receiver
}

#[test]
fn database_contention_does_not_block_capture_command_dispatch() {
    let mut scene = test_scene(&Uuid::now_v7().to_string());
    scene.independent_video = true;
    for (command, responsive, body) in [
        ("getRecorderScene", true, json!({})),
        ("getCaptureSetup", true, json!({})),
        ("getRecorderRecordingStats", true, json!({})),
        (
            "saveRecorderScene",
            true,
            json!({ "input": { "scene": scene } }),
        ),
        (
            "saveCaptureSetup",
            true,
            json!({ "input": { "scene": scene, "fps": 0.2, "videoBitrateKbps": 2000 } }),
        ),
        ("synchronous_database_control", false, json!({})),
    ] {
        let (app, window) = app();
        let storage = app.state::<storage::AppStorage>().inner().clone();
        let (locked_tx, locked_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let holder = thread::spawn(move || {
            storage
                .with_connection(|_| {
                    locked_tx.send(()).unwrap();
                    release_rx.recv().unwrap();
                    Ok(())
                })
                .unwrap()
        });
        locked_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        let (returned_tx, returned_rx) = mpsc::channel();
        let dispatcher = thread::spawn(move || {
            let pending = dispatch_body(&window, command, InvokeBody::Json(body));
            let status = dispatch(&window, "getRecorderPreviewStatus");
            returned_tx.send((pending, status)).unwrap();
        });
        let returned = returned_rx.recv_timeout(Duration::from_secs(2));
        // Always release the synthetic lock before asserting, including red runs.
        release_tx.send(()).unwrap();
        holder.join().unwrap();
        dispatcher.join().unwrap();
        assert_eq!(
            returned.is_ok(),
            responsive,
            "Capture command dispatch while SQLite is locked: {command}"
        );
        let (pending, status) = match returned {
            Ok(responses) => responses,
            Err(_) => returned_rx.recv_timeout(Duration::from_secs(2)).unwrap(),
        };
        let status = status.recv_timeout(Duration::from_secs(2)).unwrap();
        assert!(
            matches!(status, InvokeResponse::Ok(_)),
            "Unexpected IPC response: {status:?}"
        );
        assert!(matches!(
            pending.recv_timeout(Duration::from_secs(2)).unwrap(),
            InvokeResponse::Ok(_)
        ));
        if command.starts_with("save") {
            let saved = get_recorder_scene(app.state::<storage::AppStorage>())
                .unwrap()
                .unwrap();
            assert_eq!(saved, scene);
        }
    }
}

#[derive(Debug)]
struct DelayedPreview {
    entered: mpsc::Sender<()>,
    release: mpsc::Receiver<()>,
}
impl CapturePreviewSession for DelayedPreview {
    fn stop(self: Box<Self>) -> Result<(), super::super::types::RecorderError> {
        self.entered.send(()).unwrap();
        self.release.recv().unwrap();
        Ok(())
    }
}

#[derive(Debug)]
struct FailedPreview;
impl CapturePreviewSession for FailedPreview {
    fn stop(self: Box<Self>) -> Result<(), super::super::types::RecorderError> {
        Err(super::super::errors::capture_adapter_failed_error(
            "Synthetic cleanup failure",
        ))
    }
}

#[test]
fn failed_preview_cleanup_does_not_publish_idle() {
    let manager = RecorderManager::new(PathBuf::from("/tmp/daydream-synthetic-responsive"));
    let PreviewStartDecision::Pending { generation, .. } =
        manager.begin_preview_start("synthetic", 1).unwrap()
    else {
        panic!("pending start");
    };
    manager
        .activate_preview(
            "synthetic".into(),
            generation,
            vec!["monitor".into()],
            Box::new(FailedPreview),
            2,
        )
        .unwrap();
    let error = manager.stop_preview(3).unwrap_err();
    let status = manager.preview_status().unwrap();
    assert_eq!(
        status.state,
        super::super::types::RecorderPreviewState::Error
    );
    assert_eq!(status.error, Some(error));
}

#[test]
fn obsolete_empty_scene_start_cannot_cancel_a_newer_preview() {
    let manager = RecorderManager::new(PathBuf::from("/tmp/daydream-synthetic-responsive"));
    let PreviewStartDecision::Pending { generation, .. } =
        manager.begin_preview_start("empty", 1).unwrap()
    else {
        panic!("pending start");
    };
    manager.stop_preview(2).unwrap();
    let PreviewStartDecision::Pending {
        generation: replacement_generation,
        ..
    } = manager.begin_preview_start("replacement", 3).unwrap()
    else {
        panic!("replacement start");
    };
    let status =
        stop_empty_preview_start(&manager, &empty_test_scene("empty"), "empty", generation)
            .unwrap()
            .unwrap();
    assert_eq!(status.scene_id.as_deref(), Some("replacement"));
    assert!(manager.preview_start_is_current("replacement", replacement_generation));
}

#[test]
fn capture_blocking_worker_failure_returns_a_safe_native_error() {
    let error = tauri::async_runtime::block_on(super::super::operation_timing::run_blocking(
        "synthetic_failure",
        || -> Result<(), super::super::types::RecorderError> {
            panic!("Synthetic worker panic");
        },
    ))
    .unwrap_err();
    assert_eq!(error.code, "recorder_capture_adapter_failed");
    assert!(!error.message.contains("Synthetic worker panic"));
}

#[test]
fn preview_cleanup_does_not_block_dispatch_or_acknowledge_early() {
    let (app, window) = app();
    let manager = app.state::<RecorderManager>();
    let PreviewStartDecision::Pending { generation, .. } =
        manager.begin_preview_start("synthetic", 1).unwrap()
    else {
        panic!("pending start");
    };
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    manager
        .activate_preview(
            "synthetic".into(),
            generation,
            vec!["monitor".into()],
            Box::new(DelayedPreview {
                entered: entered_tx,
                release: release_rx,
            }),
            2,
        )
        .unwrap();
    let (returned_tx, returned_rx) = mpsc::channel();
    let dispatcher = thread::spawn(move || {
        let pending = dispatch(&window, "stopRecorderPreview");
        let status = dispatch(&window, "getRecorderPreviewStatus");
        returned_tx.send((pending, status)).unwrap();
    });
    entered_rx.recv_timeout(Duration::from_secs(2)).unwrap();
    let returned = returned_rx.recv_timeout(Duration::from_secs(2));
    let early = returned.as_ref().ok().map(|(pending, status)| {
        assert!(
            pending.try_recv().is_err(),
            "Stop cannot acknowledge before cleanup finishes"
        );
        let InvokeResponse::Ok(body) = status.recv_timeout(Duration::from_secs(2)).unwrap() else {
            panic!("status response");
        };
        body.deserialize::<super::super::types::RecorderPreviewStatus>()
            .unwrap()
            .state
    });
    let replacement_app = app.handle().clone();
    let (replacement_tx, replacement_rx) = mpsc::channel();
    let replacement = thread::spawn(move || {
        replacement_tx
            .send(
                replacement_app
                    .state::<RecorderManager>()
                    .begin_preview_start("replacement", 4),
            )
            .unwrap();
    });
    assert!(replacement_rx.try_recv().is_err());
    release_tx.send(()).unwrap();
    dispatcher.join().unwrap();
    let (pending, _) = returned.expect("Cleanup must not block IPC dispatch");
    assert_eq!(
        early.unwrap(),
        super::super::types::RecorderPreviewState::Live
    );
    let InvokeResponse::Ok(body) = pending.recv_timeout(Duration::from_secs(2)).unwrap() else {
        panic!("stop response");
    };
    assert_eq!(
        body.deserialize::<super::super::types::RecorderPreviewStatus>()
            .unwrap()
            .state,
        super::super::types::RecorderPreviewState::Idle
    );
    assert!(matches!(
        replacement_rx
            .recv_timeout(Duration::from_secs(2))
            .unwrap()
            .unwrap(),
        PreviewStartDecision::Pending { .. }
    ));
    replacement.join().unwrap();
    assert_eq!(
        manager.preview_status().unwrap().scene_id.as_deref(),
        Some("replacement")
    );
}
