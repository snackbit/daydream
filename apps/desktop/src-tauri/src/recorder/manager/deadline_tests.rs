use super::*;
use std::{
    process::{Command, Stdio},
    time::{Duration, Instant},
};

#[test]
fn forced_terminal_fixture() {
    if std::env::var_os("DAYDREAM_TERMINAL_FIXTURE").is_none() {
        return;
    }
    let manager = RecorderManager::new(std::env::temp_dir().join("daydream-synthetic-terminal"));
    let guard = manager.inner.lock().unwrap();
    let original = guard.status.clone();
    let error = lifecycle_error(
        "recorder_stop_deadline_exceeded",
        "Synthetic blocked cleanup.",
    );
    let terminal =
        manager.force_lifecycle_failure("synthetic", original.backend_health, error, 100);
    assert_eq!(terminal.state, RecorderState::Error);
    assert!(terminal.finalization.is_none());
    assert!(terminal.status_revision > original.status_revision);
    assert_eq!(
        terminal.cleanup_disposition,
        super::super::types::RecorderCleanupDisposition::Quarantined
    );
    // The storage/capture owner can still hold the runtime lock. Public status
    // must publish the terminal failure without waiting for that owner.
    assert_eq!(manager.status().unwrap(), terminal);
    assert!(manager.forced_lifecycle_error("synthetic").is_some());
    assert!(!super::super::media_worker::is_quiescent());
    std::process::exit(0);
}
#[test]
fn terminal_status_does_not_wait_for_blocked_runtime_owner() {
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "recorder::manager::deadline_tests::forced_terminal_fixture",
            "--nocapture",
        ])
        .env("DAYDREAM_TERMINAL_FIXTURE", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            assert!(status.success());
            break;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            panic!("Public terminal status waited for the blocked runtime owner.");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}
