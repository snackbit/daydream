use super::{run_tests::fragment, PlaybackServer};
use crate::storage::{RecordingRow, SegmentVideoStream};
use std::collections::BTreeMap;

fn recording() -> RecordingRow {
    RecordingRow {
        created_at_ms: 0,
        default_fps: 0.2,
        id: "synthetic-recording".into(),
        active_scene_id: None,
        ended_at_ms: Some(102_882),
        output_root: None,
        quality_label: "balanced-ocr".into(),
        started_at_ms: Some(0),
        status: "completed".into(),
        storage_dir: "/synthetic".into(),
        title: None,
    }
}
fn source() -> SegmentVideoStream {
    SegmentVideoStream {
        track_id: "synthetic-screen".into(),
        source_id: Some("synthetic-source".into()),
        label: "Synthetic screen".into(),
        stream_index: 0,
        width: Some(32),
        height: Some(18),
        fps: Some(0.2),
    }
}

#[test]
fn playback_session_resolves_missing_fragment_bases_without_truncating_the_run() {
    let path = std::env::temp_dir().join(format!("daydream-run-duration-{}", uuid::Uuid::now_v7()));
    std::fs::write(&path, b"synthetic").unwrap();
    let mut base = 0;
    let mut fragments = [30_279, 30_000, 30_000, 12_603]
        .into_iter()
        .enumerate()
        .map(|(index, duration)| {
            let mut row = fragment(index as i64, path.to_str().unwrap());
            row.started_at_ms = base;
            row.ended_at_ms = Some(base + duration);
            row.duration_ms = Some(duration);
            row.timeline_base_ms = Some(base);
            base += duration;
            row
        })
        .collect::<Vec<_>>();
    let sources = fragments
        .iter()
        .map(|row| (row.id.clone(), vec![source()]))
        .collect::<BTreeMap<_, _>>();
    let server = PlaybackServer::start().unwrap();
    for persisted in [
        vec![true, false, false, false],
        vec![false; 4],
        vec![true, false, true, false],
        vec![true; 4],
    ] {
        for (row, saved) in fragments.iter_mut().zip(persisted) {
            row.timeline_base_ms = saved.then_some(row.started_at_ms);
            row.timeline_basis_version = saved.then_some(1);
        }
        let assets = server
            .create_source_session(&recording(), &fragments, &sources)
            .unwrap();
        assert_eq!(assets.runs.len(), 1);
        assert_eq!(assets.runs[0].duration_ms, 102_882);
        assert_eq!(assets.runs[0].playback_offset_ms, 0);
        assert_eq!(assets.runs[0].segment_ids.len(), 4);
        let sessions = server.sessions.lock().unwrap();
        let run = &sessions[&assets.session_id].segments[&assets.runs[0].run_id];
        assert_eq!(
            run.duration_ms, 102_882,
            "HTTP output limits must use the same run duration"
        );
    }
    // A resumed run starts after a real pause; it must not include that pause.
    fragments[2].capture_run_id = Some("resumed".into());
    fragments[3].capture_run_id = Some("resumed".into());
    for row in &mut fragments[2..] {
        row.started_at_ms += 5_000;
        row.ended_at_ms = row.ended_at_ms.map(|end| end + 5_000);
        row.timeline_base_ms = None;
    }
    let assets = server
        .create_source_session(&recording(), &fragments, &sources)
        .unwrap();
    assert_eq!(
        assets
            .runs
            .iter()
            .map(|run| (run.playback_offset_ms, run.duration_ms))
            .collect::<Vec<_>>(),
        vec![(0, 60_279), (65_279, 42_603)]
    );
    std::fs::remove_file(path).unwrap();
}

#[test]
fn full_run_http_audio_keeps_beeps_at_their_timestamps_and_seeks_past_ninety_seconds() {
    use std::io::{Read, Write};
    use std::process::Command;
    let root =
        std::env::temp_dir().join(format!("daydream-full-run-http-{}", uuid::Uuid::now_v7()));
    std::fs::create_dir(&root).unwrap();
    let download = |url: &str| {
        let (address, path) = url
            .strip_prefix("http://")
            .unwrap()
            .split_once('/')
            .unwrap();
        let mut connection = std::net::TcpStream::connect(address).unwrap();
        connection
            .set_read_timeout(Some(std::time::Duration::from_secs(30)))
            .unwrap();
        write!(
            connection,
            "GET /{path} HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n"
        )
        .unwrap();
        let mut response = Vec::new();
        connection.read_to_end(&mut response).unwrap();
        assert!(response.starts_with(b"HTTP/1.1 200 OK"));
        let body = response
            .windows(4)
            .position(|bytes| bytes == b"\r\n\r\n")
            .unwrap()
            + 4;
        let file = root.join("download.mp4");
        std::fs::write(&file, &response[body..]).unwrap();
        file
    };
    let mut base = 0;
    let mut fragments = Vec::new();
    for (index, duration) in [30_279, 30_000, 30_000, 12_603].into_iter().enumerate() {
        let path = root.join(format!("{index}.mkv"));
        let seconds = format!("{:.3}", duration as f64 / 1000.);
        let tone = |frequency| {
            format!("aevalsrc=if(between(t\\,4\\,4.2)\\,0.2*sin(2*PI*{frequency}*t)\\,0):s=48000:d={seconds}")
        };
        let mux = Command::new(crate::ffmpeg::binary_path())
            .args([
                "-hide_banner",
                "-v",
                "error",
                "-f",
                "lavfi",
                "-i",
                &format!("color=white:size=32x18:rate=1/5:duration={seconds}"),
                "-f",
                "lavfi",
                "-i",
                &tone(440),
                "-f",
                "lavfi",
                "-i",
                &tone(880),
                "-map",
                "0:v",
                "-map",
                "1:a",
                "-map",
                "2:a",
                "-c:v",
                "libx264",
                "-threads",
                "1",
                "-c:a",
                "libopus",
                "-t",
                &seconds,
            ])
            .arg(&path)
            .output()
            .unwrap();
        assert!(
            mux.status.success(),
            "synthetic mux failed: {}",
            String::from_utf8_lossy(&mux.stderr)
        );
        let mut row = fragment(index as i64, path.to_str().unwrap());
        row.duration_ms = Some(duration);
        row.started_at_ms = base;
        row.ended_at_ms = Some(base + duration);
        row.timeline_base_ms = (index == 0).then_some(0);
        row.timeline_basis_version = (index == 0).then_some(1);
        fragments.push(row);
        base += duration;
    }
    let sources = fragments
        .iter()
        .map(|row| (row.id.clone(), vec![source()]))
        .collect();
    let server = PlaybackServer::start().unwrap();
    let assets = server
        .create_source_session(&recording(), &fragments, &sources)
        .unwrap();
    let run = &assets.runs[0];
    assert_eq!(run.duration_ms, 102_882);
    for track in &run.assets.audio {
        for start_ms in [0, 90_279] {
            let file = download(&format!("{}?startMs={start_ms}", track.url));
            let output = Command::new(crate::ffmpeg::binary_path())
                .args([
                    "-hide_banner",
                    "-v",
                    "error",
                    "-i",
                    file.to_str().unwrap(),
                    "-ac",
                    "1",
                    "-ar",
                    "48000",
                    "-f",
                    "f32le",
                    "pipe:1",
                ])
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "HTTP audio decode failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(
                output.stderr.is_empty(),
                "served AAC contained corrupt packets: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            let samples = output
                .stdout
                .chunks_exact(4)
                .map(|bytes| f32::from_le_bytes(bytes.try_into().unwrap()))
                .collect::<Vec<_>>();
            let expected = (102_882 - start_ms) * 48;
            assert!(
                (samples.len() as i64 - expected).abs() <= 2_400,
                "full HTTP stream duration: {} samples, expected {expected}",
                samples.len()
            );
            let peak = |time_ms: i64| {
                let start = ((time_ms - start_ms) * 48) as usize;
                samples[start..start + 480]
                    .iter()
                    .copied()
                    .map(f32::abs)
                    .fold(0., f32::max)
            };
            for beep in [4_000, 34_279, 64_279, 94_279]
                .into_iter()
                .filter(|beep| *beep >= start_ms)
            {
                assert!(peak(beep + 100) > 0.05, "beep missing at {beep} ms");
                assert!(
                    peak(beep - 200) < 0.005,
                    "beep moved earlier than {beep} ms"
                );
                assert!(peak(beep + 500) < 0.005, "beep moved later than {beep} ms");
            }
        }
    }
    // The late video URL must also reach the last fragment and its audio tail.
    let file = download(&format!("{}?startMs=95000", run.assets.video_url));
    let video = Command::new(crate::ffmpeg::binary_path())
        .args([
            "-hide_banner",
            "-v",
            "error",
            "-i",
            file.to_str().unwrap(),
            "-an",
            "-f",
            "framemd5",
            "pipe:1",
        ])
        .output()
        .unwrap();
    assert!(video.status.success(), "late HTTP video decode failed");
    let decoded = String::from_utf8(video.stdout).unwrap();
    assert!(
        decoded
            .lines()
            .filter(|line| !line.starts_with('#') && !line.is_empty())
            .count()
            >= 7,
        "video did not hold through the final tail"
    );
    server.close_session(&assets.session_id).unwrap();
    std::fs::remove_dir_all(root).unwrap();
}
