//! Isolate subreaper policy in a fixture process, never in the application/test host.
use super::{
    process,
    protocol::{self, Request, Response},
};
use std::{
    io::{BufRead, Write},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

#[test]
fn dying_parent_fixture() {
    if std::env::var_os("DAYDREAM_DYING_PARENT_FIXTURE").is_none() {
        return;
    }
    let (child, mut control, frames, _, _) = process::spawn().unwrap();
    protocol::send(&mut control, &Request::TestParentDeathWait).unwrap();
    let response: Response = protocol::receive(&mut control).unwrap();
    assert!(matches!(response, Response::Ready { generation: 0 }));
    println!("synthetic_child_pid={}", child.id());
    std::io::stdout().flush().unwrap();
    std::mem::forget((child, control, frames));
    std::process::exit(0);
}
#[test]
fn observing_parent_fixture() {
    if std::env::var_os("DAYDREAM_OBSERVING_PARENT_FIXTURE").is_none() {
        return;
    }
    unsafe extern "C" {
        fn prctl(option: i32, ...) -> i32;
        fn waitpid(pid: i32, status: *mut i32, options: i32) -> i32;
        fn kill(pid: i32, signal: i32) -> i32;
    }
    assert_eq!(unsafe { prctl(36, 1, 0usize, 0usize, 0usize) }, 0);
    let mut parent = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "recorder::media_worker::parent_death_tests::dying_parent_fixture",
            "--nocapture",
        ])
        .env("DAYDREAM_DYING_PARENT_FIXTURE", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let output = parent.stdout.take().unwrap();
    let pid = std::io::BufReader::new(output)
        .lines()
        .map(|line| line.unwrap())
        .find_map(|line| {
            line.strip_prefix("synthetic_child_pid=")
                .and_then(|pid| pid.parse::<i32>().ok())
        })
        .expect("fixture child pid");
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut status = 0;
    loop {
        let reaped = unsafe { waitpid(pid, &mut status, 1) };
        if reaped == pid {
            assert_eq!(
                status & 0x7f,
                9,
                "Child exited without the Linux parent-death signal."
            );
            break;
        }
        if Instant::now() >= deadline {
            unsafe { kill(pid, 9) };
            panic!("Media worker survived its parent's death.");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(parent
        .try_wait()
        .unwrap()
        .is_some_and(|status| status.success()));
    std::process::exit(0);
}
#[test]
fn linux_parent_death_kills_and_reaps_synthetic_media_worker() {
    let mut fixture = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "recorder::media_worker::parent_death_tests::observing_parent_fixture",
            "--nocapture",
        ])
        .env("DAYDREAM_OBSERVING_PARENT_FIXTURE", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(status) = fixture.try_wait().unwrap() {
            assert!(status.success());
            return;
        }
        if Instant::now() >= deadline {
            let _ = fixture.kill();
            panic!("Parent-death fixture exceeded its deadline.");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}
