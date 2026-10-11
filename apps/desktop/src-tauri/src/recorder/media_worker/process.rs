//! Same executable launch and bounded reap, shared by capture and recovery.
use std::{
    io,
    os::{
        fd::AsRawFd,
        unix::{net::UnixStream, process::CommandExt},
    },
    process::{Child, Command, Stdio},
    sync::mpsc,
    thread,
    time::Duration,
};
#[derive(Debug)]
pub(super) struct WorkerChild {
    child: Option<Child>,
    supervisor: mpsc::Sender<Child>,
}
impl std::ops::Deref for WorkerChild {
    type Target = Child;
    fn deref(&self) -> &Child {
        self.child.as_ref().expect("owned media child")
    }
}
impl std::ops::DerefMut for WorkerChild {
    fn deref_mut(&mut self) -> &mut Child {
        self.child.as_mut().expect("owned media child")
    }
}
impl Drop for WorkerChild {
    fn drop(&mut self) {
        if let Some(child) = self.child.take() {
            let _ = self.supervisor.send(child);
        }
    }
}

pub(super) fn spawn() -> io::Result<(WorkerChild, UnixStream, UnixStream, UnixStream, UnixStream)> {
    let (control, child_control) = UnixStream::pair()?;
    let (frames, child_frames) = UnixStream::pair()?;
    control.set_write_timeout(Some(Duration::from_millis(250)))?;
    // Backpressure belongs to the dedicated transport thread. Capture replaces
    // its pending frame; stop/abort shutdown the socket to interrupt this write.
    let control_fd = child_control.as_raw_fd();
    let frames_fd = child_frames.as_raw_fd();
    #[cfg(test)]
    let executable = std::env::var_os("DAYDREAM_MEDIA_WORKER_BINARY")
        .map(std::path::PathBuf::from)
        .map_or_else(crate::subprocess::worker_executable, Ok)?;
    #[cfg(not(test))]
    let executable = crate::subprocess::worker_executable()?;
    let mut command = Command::new(executable);
    #[cfg(test)]
    if std::env::var_os("DAYDREAM_MEDIA_WORKER_BINARY").is_none() {
        command
            .args([
                "--exact",
                "recorder::media_worker::tests::worker_entry",
                "--nocapture",
            ])
            .env("DAYDREAM_TEST_MEDIA_WORKER", "1");
    }
    #[cfg(test)]
    if std::env::var_os("DAYDREAM_MEDIA_WORKER_BINARY").is_some() {
        command.args(["--daydream-media-worker", "198", "199"]);
    }
    #[cfg(not(test))]
    command.args(["--daydream-media-worker", "198", "199"]);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(test)]
    command.stderr(Stdio::inherit());
    let parent = std::process::id();
    unsafe {
        command.pre_exec(move || prepare_child_fds(control_fd, frames_fd, parent));
    }
    let reader = control.try_clone()?;
    let transport = frames.try_clone()?;
    let (result, started) = mpsc::channel();
    let (supervisor, child_returned) = mpsc::channel::<Child>();
    thread::Builder::new()
        .name("daydream-media-supervisor".into())
        .spawn(move || {
            // PR_SET_PDEATHSIG follows the creating thread. A temporary Tokio
            // startup task must not become the child's lifetime owner.
            let spawned = command.spawn().map(|child| WorkerChild {
                child: Some(child),
                supervisor,
            });
            drop(child_control);
            drop(child_frames);
            let _ = result.send(spawned);
            if let Ok(mut child) = child_returned.recv() {
                let _ = child.kill();
                // The caller already applied its absolute reap deadline. An
                // uninterruptible child remains quarantined; retain this owner
                // until actual exit without joining it in any user operation.
                loop {
                    match child.try_wait() {
                        Ok(Some(_)) => break,
                        _ => thread::sleep(Duration::from_millis(50)),
                    }
                }
            }
        })?;
    let child = started
        .recv_timeout(Duration::from_secs(10))
        .map_err(|_| {
            io::Error::new(
                io::ErrorKind::TimedOut,
                "Media supervisor did not finish startup.",
            )
        })??;
    Ok((child, control, frames, reader, transport))
}

// Only async-signal-safe libc operations may run here after fork. Duplicate
// both inputs before replacing reserved descriptors: input descriptors can
// themselves be198/199, or swapped, and dup2(fd,fd) preserves CLOEXEC.
pub(super) unsafe fn prepare_child_fds(
    control_fd: i32,
    frames_fd: i32,
    parent: u32,
) -> io::Result<()> {
    unsafe extern "C" {
        fn fcntl(fd: i32, command: i32, ...) -> i32;
        fn dup2(old: i32, new: i32) -> i32;
        fn close(fd: i32) -> i32;
        fn prctl(option: i32, ...) -> i32;
        fn getppid() -> i32;
    }
    if prctl(1, 9, 0usize, 0usize, 0usize) != 0 {
        return Err(io::Error::last_os_error());
    }
    if getppid() != parent as i32 {
        return Err(io::Error::from_raw_os_error(10));
    }
    const DUPFD_CLOEXEC: i32 = 1030;
    let control_copy = fcntl(control_fd, DUPFD_CLOEXEC, 200);
    if control_copy < 0 {
        return Err(io::Error::last_os_error());
    }
    let frames_copy = fcntl(frames_fd, DUPFD_CLOEXEC, 200);
    if frames_copy < 0 {
        let error = io::Error::last_os_error();
        close(control_copy);
        return Err(error);
    }
    let result = if dup2(control_copy, 198) < 0 || dup2(frames_copy, 199) < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    };
    close(control_copy);
    close(frames_copy);
    result
}
