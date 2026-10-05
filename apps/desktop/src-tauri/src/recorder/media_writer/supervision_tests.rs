use super::session::MediaWriterSegment;
use gstreamer as gst;
use std::sync::mpsc;

#[test]
fn asynchronous_writer_error_is_observable_before_stop_without_private_diagnostics() {
    use gst::prelude::*;
    gst::init().unwrap();
    let pipeline = gst::Pipeline::new();
    let bus = pipeline.bus().unwrap();
    let appsrc = gstreamer_app::AppSrc::builder().build();
    let (_sender, result) = mpsc::channel();
    let mut segment =
        MediaWriterSegment::for_supervision_test(pipeline, bus.clone(), appsrc, result);
    bus.post(
        gst::message::Error::builder(gst::LibraryError::Failed, "private native diagnostics")
            .build(),
    )
    .unwrap();
    let error = segment.poll_error().expect("error before stop");
    assert_eq!(error.code, "recorder_media_writer_failed");
    assert!(!error.message.contains("private native diagnostics"));
    segment.abort();
}

#[test]
fn terminated_frame_writer_is_observable_before_stop() {
    use gst::prelude::*;
    gst::init().unwrap();
    let pipeline = gst::Pipeline::new();
    let bus = pipeline.bus().unwrap();
    let appsrc = gstreamer_app::AppSrc::builder().build();
    let (sender, result) = mpsc::channel();
    sender.send(Err("private frame bytes".into())).unwrap();
    let mut segment = MediaWriterSegment::for_supervision_test(pipeline, bus, appsrc, result);
    let error = segment.poll_error().expect("writer failure before stop");
    assert_eq!(error.code, "recorder_media_writer_frame_write_failed");
    assert!(!error.message.contains("private frame bytes"));
    segment.abort();
}
