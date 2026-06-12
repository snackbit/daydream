use super::*;

pub(super) fn recording_thumbnail_for_recording(
    recording: &RecordingRow,
    segments: &[RecordingSegmentRow],
) -> Result<Option<String>, String> {
    let Some(segment) = playable_segments(segments)
        .into_iter()
        .find(|segment| Path::new(&segment.file_path).is_file())
    else {
        return Ok(None);
    };
    let Some(video_stream_index) = segment.video_stream_index else {
        return Ok(None);
    };

    let ffmpeg_path = ffmpeg::binary_path();
    let segment_dir = Path::new(&recording.storage_dir)
        .join("playback")
        .join(&segment.id);
    fs::create_dir_all(&segment_dir).map_err(|error| {
        format!(
            "Unable to create playback cache directory {}: {error}",
            path_to_string(&segment_dir)
        )
    })?;

    let thumbnail_path = segment_dir.join("overview.jpg");
    ensure_timeline_thumbnail(
        &ffmpeg_path,
        segment,
        video_stream_index,
        0,
        &thumbnail_path,
    )?;

    Ok(Some(path_to_string(&thumbnail_path)))
}

fn ensure_timeline_thumbnail(
    ffmpeg_path: &str,
    segment: &RecordingSegmentRow,
    video_stream_index: i64,
    offset_ms: i64,
    output_path: &Path,
) -> Result<(), String> {
    if playback_asset_is_fresh(&segment.file_path, output_path) {
        return Ok(());
    }

    run_ffmpeg(
        ffmpeg_path,
        &[
            "-hide_banner",
            "-y",
            "-i",
            &segment.file_path,
            "-ss",
            &format!("{:.3}", offset_ms as f64 / 1000.0),
            "-map",
            &format!("0:{video_stream_index}"),
            "-frames:v",
            "1",
            "-an",
            "-vf",
            "scale=320:-1",
            "-q:v",
            "4",
            "-update",
            "1",
        ],
        output_path,
        "image2",
    )
}
