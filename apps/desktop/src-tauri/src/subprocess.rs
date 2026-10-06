//! Bounded local tool execution. Call from a blocking worker, never a capture callback.
use std::{
    io::{self, Read},
    process::{Command, Output, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::{Duration, Instant},
};

fn read_bounded(mut stream: impl Read, limit: usize) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 8192];
    let mut exceeded = false;
    loop {
        let read = stream.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        let remaining = limit.saturating_sub(bytes.len());
        bytes.extend_from_slice(&buffer[..read.min(remaining)]);
        exceeded |= read > remaining;
    }
    if exceeded {
        Err(io::Error::other("Local tool output exceeded its limit."))
    } else {
        Ok(bytes)
    }
}

pub(crate) fn run(
    command: &mut Command,
    cancel: &AtomicBool,
    timeout: Duration,
    limit: usize,
) -> io::Result<Output> {
    if cancel.load(Ordering::SeqCst) {
        return Err(io::ErrorKind::Interrupted.into());
    }
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("Local tool stdout unavailable."))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| io::Error::other("Local tool stderr unavailable."))?;
    let out = thread::spawn(move || read_bounded(stdout, limit));
    let err = thread::spawn(move || read_bounded(stderr, limit));
    let started = Instant::now();
    let mut interrupted = None;
    let status = loop {
        if cancel.load(Ordering::SeqCst) || started.elapsed() >= timeout {
            interrupted = Some(if cancel.load(Ordering::SeqCst) {
                io::ErrorKind::Interrupted
            } else {
                io::ErrorKind::TimedOut
            });
            let _ = child.kill();
            break child.wait();
        }
        match child.try_wait() {
            Ok(Some(status)) => break Ok(status),
            Ok(None) => thread::sleep(Duration::from_millis(20)),
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                break Err(error);
            }
        }
    };
    let stdout = out
        .join()
        .map_err(|_| io::Error::other("Local tool output reader failed."))?;
    let stderr = err
        .join()
        .map_err(|_| io::Error::other("Local tool output reader failed."))?;
    if let Some(kind) = interrupted {
        return Err(kind.into());
    }
    Ok(Output {
        status: status?,
        stdout: stdout?,
        stderr: stderr?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cancellation_and_deadlines_reap_tools() {
        let mut command = Command::new("sleep");
        command.arg("5");
        assert_eq!(
            run(
                &mut command,
                &AtomicBool::new(false),
                Duration::from_millis(30),
                1024
            )
            .unwrap_err()
            .kind(),
            io::ErrorKind::TimedOut
        );
        assert_eq!(
            run(
                &mut command,
                &AtomicBool::new(true),
                Duration::from_secs(1),
                1024
            )
            .unwrap_err()
            .kind(),
            io::ErrorKind::Interrupted
        );
    }
}
