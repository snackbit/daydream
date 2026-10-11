//! Unlink only a disposable copy, never the Cargo executable or operator's app.
use std::{
    process::{Command, Output},
    sync::atomic::AtomicBool,
    time::Duration,
};

const FIXTURE: &str =
    "recorder::media_worker::deleted_executable_tests::deleted_executable_fixture";

#[test]
fn deleted_executable_fixture() {
    if std::env::var_os("DAYDREAM_DELETED_EXECUTABLE_FIXTURE").is_none() {
        return;
    }
    if std::env::var_os("DAYDREAM_CACHED_EXECUTABLE_CHILD").is_some() {
        assert!(std::env::current_exe()
            .unwrap()
            .to_string_lossy()
            .ends_with(" (deleted)"));
        return;
    }
    // Whisper keeps this path from construction until a later worker restart.
    let cached = crate::subprocess::worker_executable().unwrap();
    let executable = std::env::current_exe().unwrap();
    assert!(executable.starts_with(std::env::temp_dir()));
    assert_eq!(executable.file_name().unwrap(), "synthetic-worker-parent");
    std::fs::remove_file(&executable).unwrap();
    assert!(!executable.exists());
    super::tests::same_executable_worker_finalizes_video_and_two_real_gstreamer_audio_branches();
    let mut command = Command::new(cached);
    command
        .args(["--exact", FIXTURE, "--nocapture"])
        .env("DAYDREAM_CACHED_EXECUTABLE_CHILD", "1");
    assert_fixture_output(
        crate::subprocess::run(
            &mut command,
            &AtomicBool::new(false),
            Duration::from_secs(5),
            16 * 1024,
        )
        .unwrap(),
    );
}

#[test]
fn replaced_executable_still_starts_finalizes_and_recovers_media() {
    let root = std::env::temp_dir().join(format!(
        "daydream-deleted-executable-{}",
        uuid::Uuid::now_v7()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let executable = root.join("synthetic-worker-parent");
    std::fs::copy(std::env::current_exe().unwrap(), &executable).unwrap();
    let mut command = Command::new(&executable);
    command
        .args(["--exact", FIXTURE, "--nocapture"])
        .env("DAYDREAM_DELETED_EXECUTABLE_FIXTURE", "1")
        .env_remove("DAYDREAM_MEDIA_WORKER_BINARY");
    let result = crate::subprocess::run(
        &mut command,
        &AtomicBool::new(false),
        Duration::from_secs(30),
        16 * 1024,
    );
    std::fs::remove_dir_all(&root).unwrap();
    assert_fixture_output(result.unwrap());
}

fn assert_fixture_output(output: Output) {
    assert!(
        output.status.success(),
        "Synthetic deleted-executable fixture failed:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
