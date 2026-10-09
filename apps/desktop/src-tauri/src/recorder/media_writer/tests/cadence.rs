//! Live input reproduces queue backpressure; prerecorded FFmpeg tones cannot.
use super::*;
use gstreamer::prelude::*;
use gstreamer_app as gst_app;
use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Instant,
};

#[test]
fn sparse_live_writer_keeps_both_audio_sources_continuous_through_rollover() {
    gst::init().unwrap();
    let root = std::env::temp_dir().join(format!(
        "daydream-synthetic-cadence-{}",
        uuid::Uuid::now_v7()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let input = MediaSegmentInput {
        video_sources: vec![crate::storage::RecordingVideoSource {
            track_id: "synthetic-monitor".into(),
            source_id: "synthetic-display".into(),
            label: "Synthetic display".into(),
            width: 64,
            height: 36,
            config_json: "{}".into(),
        }],
        benchmarks_dir: root.clone(),
        canvas_width: 64,
        canvas_height: 36,
        desktop_audio_source_id: Some("__synthetic_audio__".into()),
        microphone_audio_source_id: Some("__synthetic_audio__".into()),
        fps: 0.2,
        output_path: root.join("0001.mkv"),
        recording_id: "synthetic".into(),
        segment_id: "synthetic".into(),
        segment_index: 1,
        started_at_ms: 0,
        video_bitrate_kbps: 32,
    };
    // Replace only ambient-device boundaries with clocked synthetic appsrcs.
    // The production encoder, queues, splitmux configuration and muxer stay real.
    let mut description = pipeline::build_pipeline_description(&input);
    for name in ["desktop_audio_src", "microphone_audio_src"] {
        description = description.replace(&format!("audiotestsrc name={name} is-live=true"), &format!("appsrc name={name} is-live=true format=time block=true max-buffers=5 max-bytes=0 caps=\"audio/x-raw,format=F32LE,rate=48000,channels=2,layout=interleaved\""));
    }
    let pipeline = gst::parse::launch(&description)
        .unwrap()
        .downcast::<gst::Pipeline>()
        .unwrap();
    let video = pipeline
        .by_name("video_src")
        .unwrap()
        .downcast::<gst_app::AppSrc>()
        .unwrap();
    video.set_caps(Some(
        &gst::Caps::builder("video/x-raw")
            .field("format", "I420")
            .field("width", 64i32)
            .field("height", 36i32)
            .field("framerate", gst::Fraction::new(1, 5))
            .build(),
    ));
    pipeline.set_state(gst::State::Playing).unwrap();
    let start = Instant::now();
    // splitmux evaluates a completed GOP at the next reference frame. At 0.2
    // FPS include the 35-second frame so the nominal 30-second boundary rolls.
    let stop = Duration::from_secs(37);
    let blocked = Arc::new(std::sync::Mutex::new(vec![Duration::ZERO; 2]));
    let ended = Arc::new(AtomicUsize::new(0));
    let video_ended = ended.clone();
    let mut threads = vec![std::thread::spawn(move || {
        for index in 0..8 {
            std::thread::sleep(Duration::from_secs(index * 5).saturating_sub(start.elapsed()));
            let mut buffer = gst::Buffer::from_mut_slice(vec![0u8; 64 * 36 * 3 / 2]);
            let writable = buffer.get_mut().unwrap();
            writable.set_pts(gst::ClockTime::from_seconds(index * 5));
            writable.set_duration(gst::ClockTime::from_seconds(5));
            video.push_buffer(buffer).unwrap();
        }
        std::thread::sleep(stop.saturating_sub(start.elapsed()));
        video.end_of_stream().unwrap();
        video_ended.fetch_add(1, Ordering::SeqCst);
    })];
    for (index, name) in ["desktop_audio_src", "microphone_audio_src"]
        .iter()
        .enumerate()
    {
        let source = pipeline
            .by_name(name)
            .unwrap()
            .downcast::<gst_app::AppSrc>()
            .unwrap();
        let blocked = blocked.clone();
        let ended = ended.clone();
        threads.push(std::thread::spawn(move || {
            let mut sample = 0u64;
            while start.elapsed() < stop {
                let mut data = Vec::with_capacity(7680);
                for _ in 0..960 {
                    let value =
                        ((sample as f64 * (440. + index as f64 * 440.) * std::f64::consts::TAU
                            / 48000.)
                            .sin()
                            * 0.25) as f32;
                    data.extend_from_slice(&value.to_le_bytes());
                    data.extend_from_slice(&value.to_le_bytes());
                    sample += 1;
                }
                let mut buffer = gst::Buffer::from_mut_slice(data);
                let writable = buffer.get_mut().unwrap();
                writable.set_pts(duration_to_clock_time(start.elapsed()));
                writable.set_duration(gst::ClockTime::from_mseconds(20));
                let push = Instant::now();
                source.push_buffer(buffer).unwrap();
                let mut maximum = blocked.lock().unwrap();
                maximum[index] = maximum[index].max(push.elapsed());
                drop(maximum);
                std::thread::sleep(Duration::from_millis(20));
            }
            source.end_of_stream().unwrap();
            ended.fetch_add(1, Ordering::SeqCst);
        }));
    }
    let bus = pipeline.bus().unwrap();
    let deadline = start + stop + Duration::from_secs(12);
    let mut fragments = std::collections::BTreeMap::<String, (Duration, bool)>::new();
    let mut complete = false;
    let mut error = None;
    while Instant::now() < deadline {
        // Async splitmux consumes sink EOS. Like the production stop owner,
        // require source EOS plus durable closure of all opened fragments.
        if ended.load(Ordering::SeqCst) == 3
            && fragments.len() == 2
            && fragments.values().all(|(_, closed)| *closed)
        {
            complete = true;
            break;
        }
        let Some(message) = bus.timed_pop(gst::ClockTime::from_mseconds(100)) else {
            continue;
        };
        match message.view() {
            gst::MessageView::Error(failure) => {
                error = Some(failure.error());
                break;
            }
            gst::MessageView::Element(element) => {
                if let Some(s) = element.structure() {
                    if let (Ok(path), Ok(time)) = (
                        s.get::<String>("location"),
                        s.get::<gst::ClockTime>("running-time"),
                    ) {
                        match s.name().as_str() {
                            "splitmuxsink-fragment-opened" => {
                                fragments
                                    .insert(path, (Duration::from_nanos(time.nseconds()), false));
                            }
                            "splitmuxsink-fragment-closed" => {
                                if let Some((_, closed)) = fragments.get_mut(&path) {
                                    *closed = true;
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }
    }
    pipeline.set_state(gst::State::Null).unwrap();
    for thread in threads {
        thread.join().unwrap();
    }
    assert!(
        complete,
        "writer did not durably finalize: {error:?}, {} source EOS, {fragments:?}",
        ended.load(Ordering::SeqCst)
    );
    let mut paths = std::fs::read_dir(&root)
        .unwrap()
        .map(|item| item.unwrap().path())
        .collect::<Vec<_>>();
    paths.sort();
    assert_eq!(paths.len(), 2, "storage rollover must remain active");
    for audio in 0..2 {
        let mut times = Vec::new();
        for path in &paths {
            let origin = fragments[path.to_str().unwrap()].0;
            times.extend(
                packet_times(path, audio)
                    .into_iter()
                    .map(|pts| pts + origin),
            );
        }
        assert!(times.len() > 1500, "live audio stopped producing");
        assert!(
            blocked.lock().unwrap()[audio] < Duration::from_millis(100),
            "audio input was blocked by sparse video"
        );
        assert!(
            times
                .windows(2)
                .all(|pair| pair[1].saturating_sub(pair[0]) < Duration::from_millis(45)),
            "captured audio has missing packets, including at rollover"
        );
    }
    std::fs::remove_dir_all(root).unwrap();
}

fn packet_times(path: &std::path::Path, index: usize) -> Vec<Duration> {
    let description = format!(
        "filesrc location={} ! matroskademux name=d d.audio_{index} ! appsink name=s sync=false",
        launch_value(path.as_os_str())
    );
    let pipeline = gst::parse::launch(&description)
        .unwrap()
        .downcast::<gst::Pipeline>()
        .unwrap();
    let sink = pipeline
        .by_name("s")
        .unwrap()
        .downcast::<gst_app::AppSink>()
        .unwrap();
    pipeline.set_state(gst::State::Playing).unwrap();
    let mut times = Vec::new();
    while let Some(sample) = sink.try_pull_sample(gst::ClockTime::from_seconds(3)) {
        if let Some(pts) = sample.buffer().and_then(|buffer| buffer.pts()) {
            times.push(Duration::from_nanos(pts.nseconds()));
        }
    }
    let eos = sink.is_eos();
    pipeline.set_state(gst::State::Null).unwrap();
    assert!(eos, "synthetic demux did not reach EOS");
    times
}
