use super::{
    process,
    protocol::{self, Request, Response},
};
use std::{
    io::Write,
    os::{
        fd::AsRawFd,
        unix::{net::UnixStream, process::CommandExt},
    },
    process::{Command, Stdio},
    time::{Duration, Instant},
};

#[test]
fn descriptor_collision_fixture() {
    if std::env::var_os("DAYDREAM_FD_FIXTURE").is_none() {
        return;
    }
    unsafe extern "C" {
        fn dup2(old: i32, new: i32) -> i32;
        fn fcntl(fd: i32, command: i32, ...) -> i32;
        fn close(fd: i32) -> i32;
    }
    for swapped in [false, true] {
        let (mut control, child_control) = UnixStream::pair().unwrap();
        let (mut frames, child_frames) = UnixStream::pair().unwrap();
        let (control_fd, frames_fd) = if swapped { (199, 198) } else { (198, 199) };
        unsafe {
            assert_eq!(dup2(child_control.as_raw_fd(), control_fd), control_fd);
            assert_eq!(dup2(child_frames.as_raw_fd(), frames_fd), frames_fd);
            assert_eq!(fcntl(control_fd, 2, 1), 0);
            assert_eq!(fcntl(frames_fd, 2, 1), 0);
        }
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args([
                "--exact",
                "recorder::media_worker::tests::worker_entry",
                "--nocapture",
            ])
            .env("DAYDREAM_TEST_MEDIA_WORKER", "1")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::inherit());
        let parent = std::process::id();
        unsafe {
            command.pre_exec(move || process::prepare_child_fds(control_fd, frames_fd, parent));
        }
        let mut child = command.spawn().unwrap();
        unsafe {
            close(control_fd);
            close(frames_fd);
        }
        control
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        protocol::send(&mut control, &Request::TestDescriptorHandoff).unwrap();
        frames.write_all(&[1, 2, 3, 4]).unwrap();
        let response: Response = protocol::receive(&mut control).unwrap();
        assert!(matches!(response, Response::Ready { generation: 0 }));
        child.kill().unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            if child.try_wait().unwrap().is_some() {
                break;
            }
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    std::process::exit(0);
}
#[test]
fn private_worker_descriptor_handoff_handles_same_and_swapped_cloexec_fds() {
    let mut fixture = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "recorder::media_worker::fd_tests::descriptor_collision_fixture",
            "--nocapture",
        ])
        .env("DAYDREAM_FD_FIXTURE", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(6);
    loop {
        if let Some(status) = fixture.try_wait().unwrap() {
            assert!(status.success());
            return;
        }
        if Instant::now() >= deadline {
            let _ = fixture.kill();
            panic!("Descriptor handoff fixture exceeded its deadline.");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}
