use super::{
    super::{capture_runtime::bounded_frame_channel, errors::dependency_error, media_writer},
    protocol::{self, Request, Response},
};
use std::{
    os::{fd::FromRawFd, unix::net::UnixStream},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    thread,
    time::Duration,
};

// This entrypoint never initializes Tauri, storage, capture or model workers.
pub(crate) fn run_worker_process(control_fd: i32, frames_fd: i32) -> i32 {
    if control_fd != 198 || frames_fd != 199 {
        return 2;
    }
    let control = unsafe { UnixStream::from_raw_fd(control_fd) };
    let frames = unsafe { UnixStream::from_raw_fd(frames_fd) };
    if run(control, frames).is_ok() {
        0
    } else {
        1
    }
}
fn run(mut control: UnixStream, mut frames: UnixStream) -> std::io::Result<()> {
    let request: Request = protocol::receive(&mut control)?;
    #[cfg(test)]
    if matches!(request, Request::TestDescriptorHandoff) {
        use std::io::Read;
        let mut marker = [0; 4];
        frames.read_exact(&mut marker)?;
        if marker != [1, 2, 3, 4] {
            return Err(std::io::Error::other(
                "Synthetic frame descriptor mismatch.",
            ));
        }
        protocol::send(&mut control, &Response::Ready { generation: 0 })?;
        loop {
            thread::park();
        }
    }
    #[cfg(test)]
    if matches!(request, Request::TestParentDeathWait) {
        protocol::send(&mut control, &Response::Ready { generation: 0 })?;
        loop {
            std::thread::park();
        }
    }
    if let Request::Recover {
        version,
        input,
        output,
    } = request
    {
        if version != protocol::VERSION {
            return Err(std::io::Error::other("Invalid media worker version."));
        }
        protocol::send(
            &mut control,
            &Response::Recovered {
                result: super::recovery::remux(input, output),
            },
        )?;
        return Ok(());
    }
    let Request::Start {
        version,
        generation,
        input,
    } = request
    else {
        return Err(std::io::Error::other("Invalid media worker request."));
    };
    if version != protocol::VERSION {
        return Err(std::io::Error::other("Invalid media worker version."));
    }
    protocol::validate_input(&input)?;
    let (sender, receiver) = bounded_frame_channel(1);
    let frame_input = input.clone();
    let invalid = Arc::new(AtomicBool::new(false));
    let failed = invalid.clone();
    let (frame_done, frame_complete) = mpsc::channel();
    thread::spawn(move || {
        let mut sequences = vec![0; frame_input.video_sources.len().max(1)];
        loop {
            match protocol::read_frame(&mut frames, generation, &frame_input, &mut sequences) {
                Ok(frame) => {
                    if sender.send(frame).is_err() {
                        break;
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(_) => {
                    failed.store(true, Ordering::SeqCst);
                    break;
                }
            }
        }
        let _ = frame_done.send(());
        drop(sender);
    });
    let mut writer = match media_writer::start_in_process_segment(&input, receiver, None) {
        Ok(writer) => writer,
        Err(error) => {
            protocol::send(&mut control, &Response::Failed { generation, error })?;
            return Ok(());
        }
    };
    let flush = writer.flush_handle();
    protocol::send(&mut control, &Response::Ready { generation })?;
    let (requests, commands) = mpsc::channel();
    let mut reader = control.try_clone()?;
    thread::spawn(move || {
        while let Ok(request) = protocol::receive(&mut reader) {
            if requests.send(request).is_err() {
                return;
            }
        }
    });
    let mut frames_closed = false;
    loop {
        frames_closed |= frame_complete.try_recv().is_ok();
        let events = writer.drain_fragments();
        if !events.is_empty() {
            protocol::send(&mut control, &Response::Fragments { generation, events })?;
        }
        if invalid.load(Ordering::SeqCst) {
            protocol::send(
                &mut control,
                &Response::Failed {
                    generation,
                    error: dependency_error(
                        "recorder_media_transport_failed",
                        "Invalid media frame transport.",
                    ),
                },
            )?;
            return Ok(());
        }
        if let Some(error) = writer.poll_error() {
            frames_closed |= frame_complete.try_recv().is_ok();
            // A drained frame channel is expected during the parent's stop barrier.
            if error.code != "recorder_media_writer_frame_write_failed" || !frames_closed {
                protocol::send(&mut control, &Response::Failed { generation, error })?;
                return Ok(());
            }
        }
        match commands.recv_timeout(Duration::from_millis(25)) {
            #[cfg(test)]
            Ok(Request::TestHoldAudioPad { generation: g }) if g == generation => {
                writer.hold_audio_pad_for_test().map_err(|_| {
                    std::io::Error::other("Synthetic audio stream lock could not be installed.")
                })?;
                protocol::send(&mut control, &Response::TestAudioPadHeld { generation })?;
            }
            #[cfg(test)]
            Ok(Request::TestNull { generation: g }) if g == generation => {
                protocol::send(&mut control, &Response::TestNullStarted { generation })?;
                writer.set_null_for_test();
                protocol::send(&mut control, &Response::TestNullReturned { generation })?;
            }
            #[cfg(test)]
            Ok(Request::TestLegacyEos {
                generation: g,
                timestamp_ms,
            }) if g == generation => {
                let result = writer.legacy_audio_eos_for_test(timestamp_ms);
                protocol::send(
                    &mut control,
                    &Response::TestLegacyEos { generation, result },
                )?;
            }
            Ok(Request::Cutoff {
                generation: g,
                timestamp_ms,
            }) if g == generation => {
                if let Err(error) = writer.apply_cutoff(timestamp_ms) {
                    protocol::send(&mut control, &Response::Failed { generation, error })?;
                    return Ok(());
                }
            }
            #[cfg(test)]
            Ok(Request::TestStall { generation: g }) if g == generation => loop {
                match commands.recv() {
                    Ok(Request::Abort { generation: g }) if g == generation => return Ok(()),
                    Err(_) => return Ok(()),
                    _ => {}
                }
            },
            Ok(Request::Stop {
                generation: g,
                timestamp_ms,
            }) if g == generation => {
                let _ = frame_complete.recv_timeout(Duration::from_secs(2));
                let (result, finished) = mpsc::channel();
                thread::spawn(move || {
                    let _ = result.send(writer.stop(timestamp_ms));
                });
                loop {
                    match finished.recv_timeout(Duration::from_millis(25)) {
                        Ok(result) => {
                            let probe = result
                                .as_ref()
                                .ok()
                                .and_then(|events| {
                                    events
                                        .iter()
                                        .filter(|event| !event.opened)
                                        .max_by_key(|event| event.timestamp_ms)
                                })
                                .map(|event| {
                                    (
                                        event.path.clone(),
                                        media_writer::probe_segment_best_effort(&event.path),
                                    )
                                });
                            protocol::send(
                                &mut control,
                                &Response::Finished {
                                    generation,
                                    result,
                                    probe,
                                },
                            )?;
                            return Ok(());
                        }
                        Err(mpsc::RecvTimeoutError::Disconnected) => {
                            return Err(std::io::Error::other("Media finalizer exited."))
                        }
                        Err(mpsc::RecvTimeoutError::Timeout) => {}
                    }
                    match commands.try_recv() {
                        Ok(Request::Flush { generation: g }) if g == generation => {
                            use gstreamer::prelude::*;
                            let _ = flush.send_event(gstreamer::event::FlushStart::new());
                        }
                        Ok(Request::Abort { generation: g }) if g == generation => return Ok(()),
                        Err(mpsc::TryRecvError::Disconnected) => return Ok(()),
                        _ => {}
                    }
                }
            }
            Ok(Request::Abort { generation: g }) if g == generation => return Ok(()),
            Err(mpsc::RecvTimeoutError::Disconnected) => return Ok(()),
            _ => {}
        }
    }
}
