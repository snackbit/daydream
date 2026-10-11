use super::resources::PlaybackResources;
use std::{
    io::Read,
    net::{TcpListener, TcpStream},
    process::{Command, Stdio},
    sync::mpsc,
    time::{Duration, Instant},
};

#[test]
fn closing_playback_cancels_blocked_http_reader_and_waits_for_owner_release() {
    let resources = PlaybackResources::default();
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let _client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
    let (mut server, _) = listener.accept().unwrap();
    let lease = resources
        .begin(Some(&server), Some("synthetic-session".into()))
        .unwrap();
    let (done, finished) = mpsc::channel();
    std::thread::spawn(move || {
        let mut byte = [0];
        let _ = server.read(&mut byte);
        drop(lease);
        let _ = done.send(());
    });
    resources.cancel_and_drain(Duration::from_secs(2)).unwrap();
    finished.recv_timeout(Duration::from_secs(1)).unwrap();
}
#[test]
fn closing_playback_kills_and_reaps_child_before_drain_success() {
    let resources = PlaybackResources::default();
    let lease = resources
        .begin(None, Some("synthetic-session".into()))
        .unwrap();
    let mut command = Command::new("sleep");
    command
        .arg("60")
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let mut stdout = lease.spawn(&mut command).unwrap();
    let (done, finished) = mpsc::channel();
    std::thread::spawn(move || {
        let mut byte = [0];
        let _ = stdout.read(&mut byte);
        let status = lease.wait().unwrap();
        assert!(!status.success());
        drop(lease);
        let _ = done.send(());
    });
    let start = Instant::now();
    resources.cancel_and_drain(Duration::from_secs(3)).unwrap();
    assert!(start.elapsed() < Duration::from_secs(3));
    finished.recv_timeout(Duration::from_secs(1)).unwrap();
}
#[test]
fn playback_admission_tracks_maintenance_lease_without_manual_resume() {
    let resources = PlaybackResources::default();
    let registry = crate::maintenance::RuntimeActivityRegistry::default();
    resources.attach_registry(registry.clone());
    let maintenance = registry.begin_maintenance().unwrap();
    assert!(resources.begin(None, None).is_err());
    drop(maintenance);
    assert!(resources.begin(None, None).is_ok());
}

#[test]
fn video_stream_delivers_a_playable_fragment_before_the_recording_finishes() {
    let resources = PlaybackResources::default();
    let lease = resources
        .begin(None, Some("synthetic-startup".into()))
        .unwrap();
    let mut command = Command::new(crate::ffmpeg::binary_path());
    command.args([
        "-hide_banner",
        "-v",
        "error",
        "-re",
        "-f",
        "lavfi",
        "-i",
        "testsrc2=size=64x48:rate=4:duration=6",
        "-an",
    ]);
    super::encoders::append_video_encoder_args_with_tail(
        &mut command,
        &super::encoders::VideoEncoder::Libx264,
        4.,
        6000,
    );
    command
        .args(["-t", "6"])
        .args(super::ffmpeg_args::video_muxer_ffmpeg_args())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let mut output = lease.spawn(&mut command).unwrap();
    let (done, finished) = mpsc::channel();
    let reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let mut buffer = [0; 4096];
        while let Ok(size) = output.read(&mut buffer) {
            if size == 0 {
                break;
            }
            bytes.extend_from_slice(&buffer[..size]);
            if bytes.windows(4).any(|part| part == b"moof") {
                done.send(true).unwrap();
                return;
            }
        }
        let _ = done.send(false);
        drop(lease);
    });
    // Input is paced for six seconds. A two-second first-fragment deadline
    // detects waiting for EOF or the encoder's much later keyframe.
    let arrived = finished.recv_timeout(Duration::from_secs(2));
    resources.cancel_and_drain(Duration::from_secs(3)).unwrap();
    reader.join().unwrap();
    assert!(
        arrived.unwrap_or(false),
        "video startup waited for the complete recording instead of emitting a fragment"
    );
}
