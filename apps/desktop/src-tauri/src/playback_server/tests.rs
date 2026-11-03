use std::{
    process::Command,
    sync::{mpsc, Arc},
};

use super::{
    encoders::{
        append_video_encoder_args_with_fps, detect_video_encoder, playback_video_fps, VideoEncoder,
    },
    ffmpeg_args::{input_seek_ffmpeg_args, thumbnail_ffmpeg_args},
    http::{parse_request, PlaybackRequest},
    thumbnails::{
        thumbnail_offsets, ThumbnailEnqueueOutcome, ThumbnailJob, ThumbnailKey, ThumbnailState,
    },
    PlaybackSegment,
};

#[test]
fn parser_rejects_requests_without_session_token() {
    assert!(parse_request("GET /playback/session/segments/one/video HTTP/1.1\r\n").is_none());
}

#[test]
fn parser_reads_video_start_time() {
    let parsed = parse_request(
        "GET /playback/session/token/segments/segment-1/video?startMs=1200 HTTP/1.1\r\n",
    )
    .expect("parse request");

    assert_eq!(parsed.0, "session");
    assert_eq!(parsed.1, "token");
    match parsed.2 {
        PlaybackRequest::Video {
            segment_id,
            start_ms,
            ..
        } => {
            assert_eq!(segment_id, "segment-1");
            assert_eq!(start_ms, 1200);
        }
        _ => panic!("expected video request"),
    }
}

#[test]
fn calendar_slice_requests_bound_both_video_and_audio_and_reject_invalid_limits() {
    use super::http::output_duration_ms;
    for suffix in ["video", "audio/desktop-audio-1"] {
        let request=format!("GET /playback/session/token/segments/segment/{suffix}?startMs=1000&durationMs=500 HTTP/1.1\r\n");
        match parse_request(&request).unwrap().2 {
            PlaybackRequest::Video {
                start_ms,
                duration_ms,
                ..
            }
            | PlaybackRequest::Audio {
                start_ms,
                duration_ms,
                ..
            } => {
                assert_eq!(start_ms, 1000);
                assert_eq!(duration_ms, Some(500));
            }
            _ => panic!("expected media slice"),
        }
        for invalid in ["0", "-1", "1.5", "invalid"] {
            assert!(parse_request(&format!("GET /playback/session/token/segments/segment/{suffix}?durationMs={invalid} HTTP/1.1\r\n")).is_none());
        }
    }
    assert_eq!(output_duration_ms(5000, 1000, None), 4000);
    assert_eq!(output_duration_ms(5000, 1000, Some(500)), 500);
    assert_eq!(output_duration_ms(5000, 1000, Some(9000)), 4000);
    assert_eq!(output_duration_ms(5000, 6000, Some(1000)), 1);
}

#[test]
fn thumbnails_cover_segment_from_start_at_fixed_interval() {
    assert_eq!(thumbnail_offsets(9_500), vec![0, 3_000, 6_000, 9_000]);
}

#[test]
fn thumbnail_state_deduplicates_matching_queued_requests() {
    let mut state = ThumbnailState::new();
    let key = thumbnail_key("session-1", "segment-1", 3_000);
    let (first_sender, _first_receiver) = mpsc::channel();
    let (second_sender, _second_receiver) = mpsc::channel();

    assert_eq!(
        state
            .enqueue(key.clone(), thumbnail_job(key.clone()), first_sender)
            .expect("enqueue first"),
        ThumbnailEnqueueOutcome::Enqueued,
    );
    assert_eq!(
        state
            .enqueue(key.clone(), thumbnail_job(key.clone()), second_sender)
            .expect("enqueue second"),
        ThumbnailEnqueueOutcome::Deduplicated,
    );

    assert_eq!(state.queue.len(), 1);
    assert_eq!(state.in_progress.get(&key).map(Vec::len), Some(2));
}

#[test]
fn thumbnail_state_pops_jobs_in_fifo_order() {
    let mut state = ThumbnailState::new();
    let first = thumbnail_key("session-1", "segment-1", 0);
    let second = thumbnail_key("session-1", "segment-1", 3_000);
    let (first_sender, _first_receiver) = mpsc::channel();
    let (second_sender, _second_receiver) = mpsc::channel();

    state
        .enqueue(first.clone(), thumbnail_job(first.clone()), first_sender)
        .expect("enqueue first");
    state
        .enqueue(second.clone(), thumbnail_job(second.clone()), second_sender)
        .expect("enqueue second");

    assert_eq!(state.pop_next_job().map(|job| job.key), Some(first));
    assert_eq!(state.pop_next_job().map(|job| job.key), Some(second));
}

#[test]
fn thumbnail_state_returns_cache_hits_without_queueing() {
    let mut state = ThumbnailState::new();
    let key = thumbnail_key("session-1", "segment-1", 0);
    let bytes = Arc::new(vec![1, 2, 3]);
    let (sender, receiver) = mpsc::channel();
    state.cache.insert(key.clone(), Arc::clone(&bytes));

    assert_eq!(
        state
            .enqueue(key.clone(), thumbnail_job(key), sender)
            .expect("cache hit"),
        ThumbnailEnqueueOutcome::Cached,
    );

    assert_eq!(state.queue.len(), 0);
    assert_eq!(
        receiver.recv().expect("cached result").expect("thumbnail"),
        bytes,
    );
}

#[test]
fn thumbnail_state_close_session_removes_queued_jobs_and_cache() {
    let mut state = ThumbnailState::new();
    let closed_cached = thumbnail_key("session-1", "segment-1", 0);
    let closed_queued = thumbnail_key("session-1", "segment-1", 3_000);
    let retained_cached = thumbnail_key("session-2", "segment-1", 0);
    let retained_queued = thumbnail_key("session-2", "segment-1", 3_000);
    let (closed_sender, _closed_receiver) = mpsc::channel();
    let (retained_sender, _retained_receiver) = mpsc::channel();
    state.cache.insert(closed_cached.clone(), Arc::new(vec![1]));
    state
        .cache
        .insert(retained_cached.clone(), Arc::new(vec![2]));

    state
        .enqueue(
            closed_queued.clone(),
            thumbnail_job(closed_queued.clone()),
            closed_sender,
        )
        .expect("enqueue closed");
    state
        .enqueue(
            retained_queued.clone(),
            thumbnail_job(retained_queued.clone()),
            retained_sender,
        )
        .expect("enqueue retained");

    let waiters = state.close_session("session-1");

    assert_eq!(waiters.len(), 1);
    assert!(!state.cache.contains_key(&closed_cached));
    assert!(state.cache.contains_key(&retained_cached));
    assert_eq!(state.queue.len(), 1);
    assert_eq!(
        state.queue.front().map(|job| &job.key),
        Some(&retained_queued),
    );
}

#[test]
fn stream_args_seek_after_input_for_non_zero_start_time() {
    assert_eq!(
        input_seek_ffmpeg_args(1_200, "/tmp/daydream/segment.mkv"),
        vec![
            "-i".to_string(),
            "/tmp/daydream/segment.mkv".to_string(),
            "-ss".to_string(),
            "1.200".to_string(),
        ],
    );
}

#[test]
fn playback_video_fps_clamps_to_browser_safe_range() {
    assert_eq!(playback_video_fps(0.2), 1.0);
    assert_eq!(playback_video_fps(30.0), 30.0);
    assert_eq!(playback_video_fps(1_000.0), 60.0);
    assert_eq!(playback_video_fps(f64::NAN), 1.0);
}

#[test]
fn video_encoder_args_force_clamped_output_fps() {
    let mut command = Command::new("ffmpeg");
    append_video_encoder_args_with_fps(&mut command, &VideoEncoder::Libx264, 1_000.0);
    let args = command
        .get_args()
        .map(|arg| arg.to_string_lossy().to_string())
        .collect::<Vec<_>>();

    assert!(args.contains(&"-vf".to_string()));
    assert!(args
        .iter()
        .any(|arg| arg.starts_with("fps=60.000:start_time=0,tpad=")));
}

#[test]
fn thumbnail_args_select_visible_frame_and_write_one_bounded_jpeg() {
    let segment = PlaybackSegment {
        microphone_fragments: vec![],
        fragments: vec![],
        duration_ms: 10_000,
        video_streams: Vec::new(),
        desktop_audio_stream_index: None,
        file_path: "/tmp/daydream/segment.mkv".to_string(),
        microphone_audio_stream_index: None,
        playback_fps: 1.0,
        video_stream_index: Some(0),
    };
    let args = thumbnail_ffmpeg_args(&segment, 0, 3_000);

    assert!(!args.iter().any(|arg| arg == "-ss"));
    assert!(args
        .iter()
        .any(|arg| arg.contains("fps=1000:start_time=3.000:round=up")));
    assert!(args
        .iter()
        .any(|arg| arg.contains("tpad=stop_mode=clone:stop_duration=4.000")));
    assert!(args.contains(&"image2pipe".to_string()));
    assert_eq!(args.last(), Some(&"pipe:1".to_string()));
}

#[test]
fn software_encoder_is_available_as_fallback() {
    let encoder = detect_video_encoder();

    assert!(matches!(
        encoder,
        VideoEncoder::Amf
            | VideoEncoder::Libx264
            | VideoEncoder::Nvenc
            | VideoEncoder::Qsv
            | VideoEncoder::Vaapi
    ));
}

fn thumbnail_key(session_id: &str, segment_id: &str, offset_ms: i64) -> ThumbnailKey {
    ThumbnailKey {
        cache_namespace: "synthetic".into(),
        video_stream_index: 0,
        offset_ms,
        segment_id: segment_id.to_string(),
        session_id: session_id.to_string(),
    }
}

fn thumbnail_job(key: ThumbnailKey) -> ThumbnailJob {
    ThumbnailJob {
        offset_ms: key.offset_ms,
        segment: PlaybackSegment {
            microphone_fragments: vec![],
            fragments: vec![],
            duration_ms: 10_000,
            video_streams: Vec::new(),
            desktop_audio_stream_index: None,
            file_path: "/tmp/daydream/segment.mkv".to_string(),
            microphone_audio_stream_index: None,
            playback_fps: 1.0,
            video_stream_index: Some(0),
        },
        video_stream_index: 0,
        key,
    }
}
