use std::fs;

use uuid::Uuid;

use super::debug::{debug_data_for_recording, ensure_debug_tools_enabled};
use super::waveforms::{
    parse_volumedetect_max_amplitude, waveform_bins, waveform_peak_rms_bins, waveform_tile_bin_ms,
    waveform_tile_ranges_for_segment, waveform_tile_sample_count, waveforms_for_recording,
};
use super::*;

#[test]
fn summary_marks_missing_segment_files() {
    let temp_dir = test_dir("missing-segment-files");
    fs::create_dir_all(&temp_dir).expect("create temp recording dir");
    let recording = recording_row(&temp_dir);
    let segments = vec![segment_row(temp_dir.join("segments/0001.mkv"), "finalized")];

    let summary = summary_from_rows(&recording, &segments);

    let _ = fs::remove_dir_all(temp_dir);
    assert_eq!(
        summary.availability,
        LibraryRecordingAvailability::MissingFiles
    );
    assert_eq!(summary.playable_segment_count, 1);
    assert!(summary
        .missing_reasons
        .iter()
        .any(|reason| reason.contains("video file is missing")));
}

#[test]
fn summary_marks_recordings_without_finalized_segments_incomplete() {
    let temp_dir = test_dir("incomplete-recording");
    fs::create_dir_all(&temp_dir).expect("create temp recording dir");
    let recording = recording_row(&temp_dir);
    let segments = vec![segment_row(temp_dir.join("segments/0001.mkv"), "recording")];

    let summary = summary_from_rows(&recording, &segments);

    let _ = fs::remove_dir_all(temp_dir);
    assert_eq!(
        summary.availability,
        LibraryRecordingAvailability::Incomplete
    );
    assert_eq!(summary.playable_segment_count, 0);
}

#[test]
fn summary_marks_active_recordings_locked() {
    let temp_dir = test_dir("locked-recording");
    fs::create_dir_all(&temp_dir).expect("create temp recording dir");
    let mut recording = recording_row(&temp_dir);
    recording.status = "recording".to_string();
    let segments = vec![segment_row(temp_dir.join("segments/0001.mkv"), "recording")];

    let summary = summary_from_rows(&recording, &segments);

    let _ = fs::remove_dir_all(temp_dir);
    assert_eq!(summary.availability, LibraryRecordingAvailability::Locked);
    assert!(summary
        .missing_reasons
        .iter()
        .any(|reason| reason.contains("in progress")));
}

#[test]
fn summary_and_detail_include_recording_title() {
    let temp_dir = test_dir("recording-title");
    fs::create_dir_all(&temp_dir).expect("create temp recording dir");
    let mut recording = recording_row(&temp_dir);
    recording.title = Some("Workday review".to_string());
    let segment_path = temp_dir.join("segments/0001.mkv");
    fs::create_dir_all(segment_path.parent().expect("segment parent")).expect("create segment dir");
    fs::write(&segment_path, [1_u8]).expect("write segment");
    let segments = vec![segment_row(segment_path, "finalized")];

    let summary = summary_from_rows(&recording, &segments);
    let detail = detail_from_rows(&recording, &segments, &[]);

    let _ = fs::remove_dir_all(temp_dir);
    assert_eq!(summary.title.as_deref(), Some("Workday review"));
    assert_eq!(detail.title.as_deref(), Some("Workday review"));
}

#[test]
fn normalize_recording_title_trims_clears_and_rejects_long_values() {
    assert_eq!(
        normalize_recording_title(Some("  Workday review  ".to_string())).expect("trim"),
        Some("Workday review".to_string()),
    );
    assert_eq!(
        normalize_recording_title(Some("   ".to_string())).expect("clear"),
        None,
    );
    assert!(
        normalize_recording_title(Some("x".repeat(RECORDING_TITLE_MAX_CHARS + 1)))
            .expect_err("long title should fail")
            .contains("characters or fewer")
    );
}

#[test]
fn new_redactions_merge_with_existing_redactions_per_track() {
    let tracks = vec![
        library_track("video-0", LibraryTrackKind::Video),
        library_track("desktop-audio-1", LibraryTrackKind::DesktopAudio),
        library_track("microphone-2", LibraryTrackKind::Microphone),
    ];
    let existing_redactions = vec![
        redaction_row("existing-video", 24_000, 26_000, &["video-0"]),
        redaction_row(
            "existing-desktop-audio",
            23_000,
            24_000,
            &["desktop-audio-1"],
        ),
    ];

    let normalized = build_normalized_redaction_inputs(
        "recording-1",
        &[CreateLibraryRedactionRangeInput {
            end_offset_ms: 25_000,
            start_offset_ms: 23_500,
            track_ids: vec![
                "desktop-audio-1".to_string(),
                "microphone-2".to_string(),
                "video-0".to_string(),
            ],
        }],
        &existing_redactions,
        &tracks,
        30_000,
    )
    .expect("normalize redactions");

    let normalized_ranges = normalized
        .iter()
        .map(|redaction| {
            (
                redaction.start_offset_ms,
                redaction.end_offset_ms,
                parse_redaction_track_ids(&redaction.track_ids_json).expect("track ids"),
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(
        normalized_ranges,
        vec![
            (23_000, 25_000, vec!["desktop-audio-1".to_string()],),
            (23_500, 25_000, vec!["microphone-2".to_string()]),
            (23_500, 26_000, vec!["video-0".to_string()]),
        ]
    );
}

#[test]
fn unredaction_range_splits_existing_redactions_per_track() {
    let tracks = vec![
        library_track("video-0", LibraryTrackKind::Video),
        library_track("desktop-audio-1", LibraryTrackKind::DesktopAudio),
    ];
    let existing_redactions = vec![redaction_row(
        "existing",
        20_000,
        26_000,
        &["desktop-audio-1", "video-0"],
    )];

    let normalized = build_unredacted_redaction_inputs(
        "recording-1",
        &[CreateLibraryRedactionRangeInput {
            end_offset_ms: 24_000,
            start_offset_ms: 22_000,
            track_ids: vec!["desktop-audio-1".to_string(), "video-0".to_string()],
        }],
        &existing_redactions,
        &tracks,
        30_000,
    )
    .expect("unredact range");

    let normalized_ranges = normalized
        .iter()
        .map(|redaction| {
            (
                redaction.start_offset_ms,
                redaction.end_offset_ms,
                parse_redaction_track_ids(&redaction.track_ids_json).expect("track ids"),
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(
        normalized_ranges,
        vec![
            (
                20_000,
                22_000,
                vec!["desktop-audio-1".to_string(), "video-0".to_string()],
            ),
            (
                20_000,
                26_000,
                vec![
                    "activity:focus".to_string(),
                    "activity:keyboard".to_string(),
                    "activity:mouse".to_string(),
                ],
            ),
            (
                24_000,
                26_000,
                vec!["desktop-audio-1".to_string(), "video-0".to_string()],
            ),
        ]
    );
}

#[test]
fn debug_tools_are_disabled_by_default() {
    let storage = storage::test_storage(PathBuf::new());

    let error = ensure_debug_tools_enabled(&storage).expect_err("debug should be disabled");

    assert_eq!(error, "Debug tools are disabled.");
}

#[test]
fn debug_tools_are_enabled_by_persisted_setting() {
    let storage = storage::test_storage(PathBuf::new());
    storage
        .with_connection(|connection| {
            storage::save_app_setting(
                connection,
                &storage::SaveAppSettingInput {
                    is_sensitive: false,
                    key: "debug.enabled".to_string(),
                    timestamp_ms: 1,
                    value_json: "true".to_string(),
                },
            )
        })
        .expect("save debug setting");

    ensure_debug_tools_enabled(&storage).expect("debug should be enabled");
}

#[test]
fn debug_raw_visual_rows_hide_redacted_visual_text() {
    let storage = storage::test_storage(PathBuf::new());
    let recording_id = storage
        .with_connection_mut(|connection| {
            storage::save_app_setting(
                connection,
                &storage::SaveAppSettingInput {
                    is_sensitive: false,
                    key: "debug.enabled".to_string(),
                    timestamp_ms: 1,
                    value_json: "true".to_string(),
                },
            )?;

            let recording_id = Uuid::now_v7().to_string();
            let segment_id = Uuid::now_v7().to_string();
            let video_track_id = Uuid::now_v7().to_string();
            let audio_track_id = Uuid::now_v7().to_string();
            let job_id = Uuid::now_v7().to_string();
            let ocr_chunk_id = Uuid::now_v7().to_string();
            let vlm_chunk_id = Uuid::now_v7().to_string();

            storage::create_recording(
                connection,
                &storage::CreateRecordingInput {
                    active_scene_id: None,
                    created_at_ms: 1,
                    default_fps: 1.0,
                    id: recording_id.clone(),
                    notes: None,
                    output_root: None,
                    quality_label: "balanced-ocr".to_string(),
                    storage_dir: "/tmp/daydream-debug-redaction-test".to_string(),
                },
            )?;
            storage::create_recording_segment(
                connection,
                &storage::CreateRecordingSegmentInput {
                    file_path: "/tmp/daydream-debug-redaction-test/segments/0001.mkv".to_string(),
                    id: segment_id.clone(),
                    recording_id: recording_id.clone(),
                    segment_index: 1,
                    started_at_ms: 1,
                },
            )?;
            storage::upsert_recording_track(
                connection,
                &storage::UpsertRecordingTrackInput {
                    codec: Some("h264".to_string()),
                    created_at_ms: 1,
                    id: video_track_id.clone(),
                    kind: "screen".to_string(),
                    label: "Display".to_string(),
                    path: None,
                    recording_id: recording_id.clone(),
                    source_id: None,
                },
            )?;
            storage::upsert_recording_track(
                connection,
                &storage::UpsertRecordingTrackInput {
                    codec: Some("pcm_s16le".to_string()),
                    created_at_ms: 1,
                    id: audio_track_id.clone(),
                    kind: "microphone".to_string(),
                    label: "Microphone".to_string(),
                    path: None,
                    recording_id: recording_id.clone(),
                    source_id: None,
                },
            )?;
            storage::create_processing_job(
                connection,
                &storage::CreateProcessingJobInput {
                    created_at_ms: 1,
                    error: None,
                    id: job_id.clone(),
                    kind: "ai-processing".to_string(),
                    recording_id: recording_id.clone(),
                    status: "running".to_string(),
                    updated_at_ms: 1,
                },
            )?;

            for (chunk_id, source_kind) in [(&ocr_chunk_id, "ocr"), (&vlm_chunk_id, "vlm")] {
                storage::create_processing_chunk(
                    connection,
                    &storage::CreateProcessingChunkInput {
                        attempt_count: 1,
                        created_at_ms: 1,
                        end_offset_ms: Some(10_000),
                        error_code: None,
                        id: chunk_id.clone(),
                        job_id: job_id.clone(),
                        progress: 1.0,
                        recording_id: recording_id.clone(),
                        result_code: None,
                        segment_id: Some(segment_id.clone()),
                        source_kind: source_kind.to_string(),
                        start_offset_ms: Some(0),
                        status: "succeeded".to_string(),
                        track_id: None,
                        updated_at_ms: 1,
                    },
                )?;
            }

            storage::create_redaction(
                connection,
                &storage::CreateRedactionInput {
                    created_at_ms: 2,
                    end_offset_ms: 2_000,
                    id: Uuid::now_v7().to_string(),
                    reason: None,
                    recording_id: recording_id.clone(),
                    start_offset_ms: 1_000,
                    track_ids_json: serde_json::to_string(&vec![video_track_id])
                        .expect("track ids json"),
                },
            )?;

            for (chunk_id, timestamp, text) in [
                (&ocr_chunk_id, 1_500, "hidden debug OCR text"),
                (&ocr_chunk_id, 2_500, "visible debug OCR text"),
            ] {
                storage::create_ocr_region(
                    connection,
                    &storage::CreateVisualRegionInput {
                        chunk_id: chunk_id.clone(),
                        confidence: Some(0.9),
                        frame_timestamp_ms: timestamp,
                        id: Uuid::now_v7().to_string(),
                        job_id: job_id.clone(),
                        model: Some("tesseract".to_string()),
                        observation_kind: None,
                        prompt_version: None,
                        provider: "tesseract".to_string(),
                        recording_id: recording_id.clone(),
                        region_height: 20,
                        region_width: 30,
                        region_x: 1,
                        region_y: 2,
                        source_frame_ref: format!("segment:test:offset:{timestamp}:frame:0"),
                        text: text.to_string(),
                    },
                )?;
            }

            for (chunk_id, timestamp, text) in [
                (&vlm_chunk_id, 1_500, "hidden debug VLM observation"),
                (&vlm_chunk_id, 2_500, "visible debug VLM observation"),
            ] {
                storage::create_vlm_observation(
                    connection,
                    &storage::CreateVisualRegionInput {
                        chunk_id: chunk_id.clone(),
                        confidence: Some(0.8),
                        frame_timestamp_ms: timestamp,
                        id: Uuid::now_v7().to_string(),
                        job_id: job_id.clone(),
                        model: Some("vision".to_string()),
                        observation_kind: Some("screen-observation".to_string()),
                        prompt_version: Some("m2-test".to_string()),
                        provider: "openai-compatible".to_string(),
                        recording_id: recording_id.clone(),
                        region_height: 20,
                        region_width: 30,
                        region_x: 1,
                        region_y: 2,
                        source_frame_ref: format!("segment:test:offset:{timestamp}:frame:0"),
                        text: text.to_string(),
                    },
                )?;
            }

            Ok(recording_id)
        })
        .expect("seed debug visual rows");

    let debug = debug_data_for_recording(&recording_id, &storage).expect("debug data");
    let raw_json = serde_json::to_string(&debug.raw).expect("serialize raw debug data");

    assert!(!raw_json.contains("hidden debug OCR text"));
    assert!(!raw_json.contains("hidden debug VLM observation"));
    assert!(raw_json.contains("visible debug OCR text"));
    assert!(raw_json.contains("visible debug VLM observation"));
}

#[test]
fn delete_guard_rejects_active_recordings() {
    let temp_dir = test_dir("delete-guard-active-recording");
    let mut recording = recording_row(&temp_dir);
    recording.status = "paused".to_string();

    let error =
        ensure_recording_is_deletable(&recording).expect_err("active recordings cannot be deleted");

    assert!(error.contains("cannot be deleted"));
}

#[test]
fn summary_reports_recursive_recording_directory_size() {
    let temp_dir = test_dir("recording-size");
    fs::create_dir_all(temp_dir.join("segments")).expect("create segments dir");
    fs::write(temp_dir.join("segments/0001.mkv"), [1_u8; 10]).expect("write segment");
    fs::write(temp_dir.join("metadata.json"), [1_u8; 5]).expect("write metadata");
    let recording = recording_row(&temp_dir);
    let segments = vec![segment_row(temp_dir.join("segments/0001.mkv"), "finalized")];

    let summary = summary_from_rows(&recording, &segments);

    let _ = fs::remove_dir_all(temp_dir);
    assert_eq!(summary.size_bytes, 15);
}

#[test]
fn library_segments_preserve_pause_gaps_in_timeline() {
    let temp_dir = test_dir("pause-gaps");
    fs::create_dir_all(temp_dir.join("segments")).expect("create segments dir");
    let first_path = temp_dir.join("segments/0001.mkv");
    let second_path = temp_dir.join("segments/0002.mkv");
    fs::write(&first_path, "one").expect("write first segment");
    fs::write(&second_path, "two").expect("write second segment");
    let mut first = segment_row(first_path, "finalized");
    first.duration_ms = Some(2_000);
    first.ended_at_ms = Some(3_000);
    let mut second = segment_row(second_path, "finalized");
    second.id = "segment-2".to_string();
    second.segment_index = 2;
    second.started_at_ms = 6_000;
    second.ended_at_ms = Some(8_000);
    second.duration_ms = Some(2_000);

    let segments = library_segments(&[first, second]);

    let _ = fs::remove_dir_all(temp_dir);
    assert_eq!(segments[0].pause_after_ms, 3_000);
    assert_eq!(segments[1].playback_offset_ms, 5_000);
}

#[test]
fn waveforms_do_not_write_cache_files() {
    let temp_dir = test_dir("waveform-no-cache");
    fs::create_dir_all(temp_dir.join("segments")).expect("create segments dir");
    let segment_path = temp_dir.join("segments/0001.mkv");
    fs::write(&segment_path, "media").expect("write segment");
    let recording = recording_row(&temp_dir);
    let segment = segment_row(segment_path, "finalized");

    let _waveforms = waveforms_for_recording(&recording, &[segment]);

    assert!(!temp_dir.join("waveforms/library-waveforms.json").exists());
    let _ = fs::remove_dir_all(temp_dir);
}

#[test]
fn waveform_bins_return_peak_buckets() {
    let bins = waveform_bins(&[0.1, 0.8, 0.2, 0.4], 2);

    assert_eq!(bins, vec![0.8, 0.4]);
}

#[test]
fn waveform_peak_rms_bins_return_native_amplitude_buckets() {
    let (peak_samples, rms_samples) = waveform_peak_rms_bins(&[0.3, -0.4, 0.6, -0.8], 2);

    assert_eq!(peak_samples, vec![0.4, 0.8]);
    assert!((rms_samples[0] - 0.353_553_390_593_273_8).abs() < f64::EPSILON);
    assert!((rms_samples[1] - std::f64::consts::FRAC_1_SQRT_2).abs() < f64::EPSILON);
}

#[test]
fn waveform_tile_sample_count_keeps_short_non_empty_tiles_visible() {
    assert_eq!(waveform_tile_sample_count(10, 250, false), 2);
    assert_eq!(waveform_tile_sample_count(10, 250, true), 1);
    assert_eq!(waveform_tile_sample_count(1_000, 250, false), 4);
}

#[test]
fn waveform_tile_bin_ms_tracks_visible_ms_per_pixel() {
    assert_eq!(
        waveform_tile_bin_ms(&LibraryWaveformTileRequest {
            end_timeline_ms: 10_000,
            recording_id: "recording-1".to_string(),
            start_timeline_ms: 0,
            viewport_width_px: 1_000.0,
        }),
        10
    );
    assert_eq!(
        waveform_tile_bin_ms(&LibraryWaveformTileRequest {
            end_timeline_ms: 3_000,
            recording_id: "recording-1".to_string(),
            start_timeline_ms: 0,
            viewport_width_px: 1_000.0,
        }),
        5
    );
    assert_eq!(
        waveform_tile_bin_ms(&LibraryWaveformTileRequest {
            end_timeline_ms: 260_000,
            recording_id: "recording-1".to_string(),
            start_timeline_ms: 0,
            viewport_width_px: 1_000.0,
        }),
        250
    );
}

#[test]
fn waveform_tile_ranges_follow_segment_timeline_offsets() {
    let segment = LibraryRecordingSegment {
        desktop_audio_stream_index: Some(1),
        duration_ms: Some(10_000),
        ended_at_ms: Some(11_000),
        file_exists: true,
        file_path: "/tmp/daydream/segment.mkv".to_string(),
        id: "segment-1".to_string(),
        microphone_audio_stream_index: Some(2),
        pause_after_ms: 0,
        playback_offset_ms: 5_000,
        segment_index: 1,
        started_at_ms: 1_000,
        state: "finalized".to_string(),
        video_stream_index: Some(0),
    };

    let ranges = waveform_tile_ranges_for_segment(&segment, 7_500, 7_900);
    let outside_ranges = waveform_tile_ranges_for_segment(&segment, 1_000, 2_000);

    assert_eq!(ranges, vec![(0, 10_000)]);
    assert!(outside_ranges.is_empty());
}

#[test]
fn parse_volumedetect_max_amplitude_reads_db_and_silence() {
    assert_eq!(
        parse_volumedetect_max_amplitude("[Parsed_volumedetect_0] max_volume: -6.0 dB\n"),
        Some(0.501_187_233_627_272_2)
    );
    assert_eq!(
        parse_volumedetect_max_amplitude("[Parsed_volumedetect_0] max_volume: -inf dB\n"),
        Some(0.0)
    );
}

#[test]
fn playback_asset_cache_requires_existing_non_empty_output() {
    let temp_dir = test_dir("playback-cache");
    fs::create_dir_all(&temp_dir).expect("create temp dir");
    let source_path = temp_dir.join("segment.mkv");
    let output_path = temp_dir.join("video.mp4");
    fs::write(&source_path, "source").expect("write source");

    assert!(!playback_asset_is_fresh(
        &path_to_string(&source_path),
        &output_path
    ));

    fs::write(&output_path, "output").expect("write output");

    assert!(playback_asset_is_fresh(
        &path_to_string(&source_path),
        &output_path
    ));
    let _ = fs::remove_dir_all(temp_dir);
}

#[test]
fn recording_thumbnail_returns_none_without_playable_segments() {
    let temp_dir = test_dir("recording-thumbnail-empty");
    fs::create_dir_all(&temp_dir).expect("create temp dir");
    let recording = recording_row(&temp_dir);
    let segment = segment_row(temp_dir.join("segments/0001.mkv"), "recording");

    let thumbnail = recording_thumbnail_for_recording(&recording, &[segment])
        .expect("thumbnail lookup should not fail");

    let _ = fs::remove_dir_all(temp_dir);
    assert_eq!(thumbnail, None);
}

#[test]
fn temp_output_path_preserves_container_extension() {
    assert_eq!(
        temp_output_path(Path::new("/tmp/daydream/video.mp4")),
        PathBuf::from("/tmp/daydream/video.tmp.mp4")
    );
    assert_eq!(
        temp_output_path(Path::new("/tmp/daydream/audio.m4a")),
        PathBuf::from("/tmp/daydream/audio.tmp.m4a")
    );
    assert_eq!(
        temp_output_path(Path::new("/tmp/daydream/thumb.jpg")),
        PathBuf::from("/tmp/daydream/thumb.tmp.jpg")
    );
}

#[cfg(unix)]
#[test]
fn ffmpeg_success_without_temp_output_is_rejected() {
    use std::os::unix::fs::PermissionsExt;

    let temp_dir = test_dir("missing-ffmpeg-output");
    fs::create_dir_all(&temp_dir).expect("create temp dir");
    let fake_ffmpeg_path = temp_dir.join("fake-ffmpeg");
    let output_path = temp_dir.join("video.mp4");
    fs::write(&fake_ffmpeg_path, "#!/bin/sh\nexit 0\n").expect("write fake ffmpeg");
    fs::set_permissions(&fake_ffmpeg_path, fs::Permissions::from_mode(0o755))
        .expect("make fake ffmpeg executable");

    let error = run_ffmpeg(&path_to_string(&fake_ffmpeg_path), &[], &output_path, "mp4")
        .expect_err("missing temp output should fail");

    let _ = fs::remove_dir_all(temp_dir);
    assert!(error.contains("completed without creating playback asset"));
}

fn recording_row(storage_dir: &Path) -> RecordingRow {
    RecordingRow {
        active_scene_id: None,
        created_at_ms: 1_000,
        default_fps: 5.0,
        ended_at_ms: Some(5_000),
        id: "recording-1".to_string(),
        output_root: None,
        quality_label: "balanced-ocr".to_string(),
        started_at_ms: Some(1_000),
        status: "completed".to_string(),
        storage_dir: path_to_string(storage_dir),
        title: None,
    }
}

fn segment_row(file_path: PathBuf, state: &str) -> RecordingSegmentRow {
    RecordingSegmentRow {
        desktop_audio_stream_index: Some(1),
        duration_ms: Some(4_000),
        ended_at_ms: Some(5_000),
        error_code: None,
        ffmpeg_exit_code: Some(0),
        file_path: path_to_string(&file_path),
        id: "segment-1".to_string(),
        microphone_audio_stream_index: Some(2),
        probe_json: Some(
            serde_json::json!({
                "audio": [
                    {"codecName": "opus", "index": 1},
                    {"codecName": "opus", "index": 2}
                ],
                "video": {"codecName": "h264", "index": 0}
            })
            .to_string(),
        ),
        recording_id: "recording-1".to_string(),
        segment_index: 1,
        started_at_ms: 1_000,
        state: state.to_string(),
        video_stream_index: Some(0),
    }
}

fn library_track(id: &str, kind: LibraryTrackKind) -> LibraryRecordingTrack {
    LibraryRecordingTrack {
        codec: None,
        description: id.to_string(),
        id: id.to_string(),
        kind,
        label: id.to_string(),
        stream_index: None,
    }
}

fn redaction_row(
    id: &str,
    start_offset_ms: i64,
    end_offset_ms: i64,
    track_ids: &[&str],
) -> RedactionRow {
    RedactionRow {
        created_at_ms: 1,
        end_offset_ms,
        id: id.to_string(),
        reason: None,
        recording_id: "recording-1".to_string(),
        start_offset_ms,
        track_ids_json: serde_json::to_string(track_ids).expect("track ids json"),
    }
}

fn test_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("daydream-library-{name}-{}", std::process::id()))
}
