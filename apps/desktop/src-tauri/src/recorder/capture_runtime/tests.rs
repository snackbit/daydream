use std::{
    sync::{Arc, Barrier},
    time::Duration,
};

use super::{
    bounded_frame_channel, capture_timing_snapshot, new_capture_timing, record_capture_frame,
    record_compose_duration, record_composed_frame, record_dmabuf_frame_read_back_to_cpu,
    record_frame_sent_to_writer, record_scheduler_tick, CapturedFrame, CapturedFrameFormat,
    CapturedFrameStorage, RuntimeWorker, WriterFrameHandoffPlan,
};
use crate::recorder::types::CaptureFrameStorageKind;

#[test]
fn supported_formats_are_accepted_and_plan_writer_handoff() {
    let yuv420 = CapturedFrame::new(
        "monitor-1",
        1,
        2,
        2,
        2,
        CapturedFrameFormat::Yuv420p,
        vec![0; 6],
    )
    .expect("valid yuv frame");
    let rgba = CapturedFrame::new(
        "monitor-1",
        1,
        2,
        2,
        8,
        CapturedFrameFormat::Rgba,
        vec![0; 16],
    )
    .expect("valid rgba frame");

    assert_eq!(
        yuv420.writer_handoff_plan().expect("yuv handoff plan"),
        WriterFrameHandoffPlan::PassThroughYuv420p,
    );
    assert_eq!(
        rgba.writer_handoff_plan().expect("rgba handoff plan"),
        WriterFrameHandoffPlan::ConvertToYuv420p {
            source_format: CapturedFrameFormat::Rgba,
        },
    );
}

#[test]
fn unsupported_formats_and_invalid_dimensions_fail() {
    let dimension_error = CapturedFrame::new(
        "monitor-1",
        1,
        0,
        2,
        8,
        CapturedFrameFormat::Rgba,
        vec![0; 16],
    )
    .expect_err("zero width should fail");

    assert_eq!(dimension_error.code, "recorder_capture_format_unsupported",);
}

#[test]
fn dmabuf_storage_gets_specific_gate_h2_error() {
    let dmabuf = CapturedFrameStorage::DmaBuf;
    let unknown = CapturedFrameStorage::Unknown("Other".to_string());

    let dmabuf_error = dmabuf
        .ensure_cpu_readable("monitor-1")
        .expect_err("dmabuf should require gpu path");
    let unknown_error = unknown
        .ensure_cpu_readable("monitor-1")
        .expect_err("unknown storage should fail");

    assert_eq!(dmabuf_error.code, "recorder_capture_dmabuf_required");
    assert_eq!(unknown_error.code, "recorder_capture_format_unsupported");
}

#[test]
fn frame_debug_output_does_not_include_frame_bytes() {
    let frame = CapturedFrame::new(
        "monitor-1",
        1,
        1,
        1,
        4,
        CapturedFrameFormat::Rgbx,
        vec![1, 2, 3, 4],
    )
    .expect("valid frame");
    let debug = format!("{frame:?}");

    assert!(debug.contains("data_len"));
    assert!(!debug.contains("[1, 2, 3, 4]"));
}

#[test]
fn bounded_frame_channel_sends_receives_and_shuts_down() {
    let (sender, receiver) = bounded_frame_channel(1);
    let barrier = Arc::new(Barrier::new(2));
    let worker_barrier = Arc::clone(&barrier);
    let worker = RuntimeWorker::spawn("test-capture-worker", move || {
        sender.send(test_frame(1))?;
        worker_barrier.wait();
        sender.send(test_frame(2))
    });

    let first = receiver.recv().expect("first frame");
    assert_eq!(first.timestamp_ms, 1);
    barrier.wait();
    let second = receiver.recv().expect("second frame");
    assert_eq!(second.timestamp_ms, 2);

    worker.join().expect("worker joins");
    assert!(receiver.recv_timeout(Duration::from_millis(10)).is_err());
}

#[test]
fn capture_timing_counters_track_live_frame_flow() {
    let timing = new_capture_timing();

    record_capture_frame(
        &timing,
        "monitor-1",
        CaptureFrameStorageKind::CpuReadable,
        100,
    );
    record_capture_frame(&timing, "monitor-2", CaptureFrameStorageKind::DmaBuf, 90);
    record_capture_frame(
        &timing,
        "monitor-1",
        CaptureFrameStorageKind::CpuReadable,
        130,
    );
    record_dmabuf_frame_read_back_to_cpu(&timing);
    record_composed_frame(&timing, 150, true);
    record_scheduler_tick(&timing, 2);
    record_compose_duration(&timing, 7);
    record_compose_duration(&timing, 3);
    record_frame_sent_to_writer(&timing);

    let snapshot = capture_timing_snapshot(&timing);

    assert_eq!(snapshot.composed_frames, 1);
    assert_eq!(snapshot.cpu_readable_frames_received, 2);
    assert_eq!(snapshot.dmabuf_frames_received, 1);
    assert_eq!(snapshot.dmabuf_frames_read_back_to_cpu, 1);
    assert_eq!(
        snapshot.frames_received_by_source.get("monitor-1"),
        Some(&2)
    );
    assert_eq!(
        snapshot.frames_received_by_source.get("monitor-2"),
        Some(&1)
    );
    assert_eq!(
        snapshot.latest_frame_storage_by_source.get("monitor-1"),
        Some(&CaptureFrameStorageKind::CpuReadable)
    );
    assert_eq!(
        snapshot.latest_frame_storage_by_source.get("monitor-2"),
        Some(&CaptureFrameStorageKind::DmaBuf)
    );
    assert_eq!(snapshot.frames_sent_to_writer, 1);
    assert_eq!(snapshot.last_composed_timestamp_ms, Some(150));
    assert_eq!(snapshot.max_compose_duration_ms, 7);
    assert_eq!(snapshot.total_compose_duration_ms, 10);
    assert_eq!(snapshot.reused_frame_compositions, 1);
    assert_eq!(snapshot.scheduler_missed_deadlines, 2);
    assert_eq!(snapshot.scheduler_ticks, 1);
    assert_eq!(snapshot.source_timestamp_min_ms, Some(90));
    assert_eq!(snapshot.source_timestamp_max_ms, Some(130));
}

fn test_frame(timestamp_ms: u64) -> CapturedFrame {
    CapturedFrame::new(
        "monitor-1",
        timestamp_ms,
        1,
        1,
        4,
        CapturedFrameFormat::Rgba,
        vec![0, 0, 0, 255],
    )
    .expect("valid test frame")
}
