use super::super::super::*;
use super::{
    image::{
        decode_png_rgba, file_sha1, source_frame_ref_for_region, visual_changed_pixel_ratio,
        visual_delta_regions, write_visual_crop_png,
    },
    types::*,
};

pub(super) fn extract_visual_frames(
    app: &AppHandle,
    segment: &RecordingSegmentRow,
    segment_playback_offset_ms: i64,
    chunk: &ProcessingChunkRow,
    recording_fps: f64,
    app_settings: &settings::AppSettings,
    cancel: &AtomicBool,
) -> Result<ExtractedVisualFrameSet, AiError> {
    let video_stream_index = segment.video_stream_index.ok_or_else(|| {
        ai_error(
            "ai_processing_video_stream_missing",
            "Video stream is missing.",
        )
    })?;
    let chunk_start = chunk.start_offset_ms.ok_or_else(|| {
        ai_error(
            "ai_processing_chunk_offset_missing",
            "Processing chunk start offset is missing.",
        )
    })?;
    let chunk_end = chunk.end_offset_ms.ok_or_else(|| {
        ai_error(
            "ai_processing_chunk_offset_missing",
            "Processing chunk end offset is missing.",
        )
    })?;
    let slice_start_ms = chunk_start
        .checked_sub(segment_playback_offset_ms)
        .ok_or_else(|| {
            ai_error(
                "ai_processing_chunk_offset_invalid",
                "Processing chunk offset is outside its segment.",
            )
        })?;
    let slice_duration_ms = chunk_end.saturating_sub(chunk_start);
    let temp_dir = TempProcessingDirectory::new(processing_temp_visual_directory(
        app,
        &chunk.job_id,
        &chunk.id,
    )?);
    let fps = effective_frame_recognition_fps(app_settings, recording_fps);
    let output_pattern = temp_dir.path().join("frame-%06d.png");
    let output = Command::new(ffmpeg::binary_path())
        .args([
            "-hide_banner",
            "-v",
            "error",
            "-ss",
            &format_seconds(slice_start_ms),
            "-t",
            &format_seconds(slice_duration_ms),
            "-i",
            &segment.file_path,
            "-map",
            &format!("0:{video_stream_index}"),
            "-vf",
            &format!("fps={fps:.6}"),
            "-vsync",
            "0",
        ])
        .arg(&output_pattern)
        .output()
        .map_err(file_error)?;
    check_processing_cancel(cancel)?;
    if !output.status.success() {
        return Err(ai_error(
            "ai_processing_visual_extract_failed",
            "Visual frame extraction failed.",
        ));
    }

    let mut paths = fs::read_dir(temp_dir.path())
        .map_err(file_error)?
        .map(|entry| entry.map(|entry| entry.path()).map_err(file_error))
        .collect::<Result<Vec<_>, _>>()?;
    paths.sort();
    let sample_interval_ms = frame_sample_interval_ms(fps);
    let tile_size = app_settings.ai.processing.visual_delta_tile_size_px.clamp(
        settings::MIN_AI_VISUAL_DELTA_TILE_SIZE_PX,
        settings::MAX_AI_VISUAL_DELTA_TILE_SIZE_PX,
    ) as usize;
    let segment_id = segment.id.clone();
    let context_for = |path: &PathBuf, index: usize| -> VisualContextFrame {
        let timestamp = (chunk_start + (index as i64 * sample_interval_ms)).min(chunk_end);
        VisualContextFrame {
            frame_timestamp_ms: timestamp,
            path: path.clone(),
            source_frame_ref: format!(
                "segment:{}:offset:{}:frame:{}",
                segment_id, timestamp, index
            ),
        }
    };
    let mut previous_signature = None::<String>;
    let mut previous_image = None::<VisualImage>;
    let mut frames = Vec::new();
    for (index, path) in paths.iter().enumerate() {
        check_processing_cancel(cancel)?;
        let timestamp = (chunk_start + (index as i64 * sample_interval_ms)).min(chunk_end);
        let full_checkpoint = full_checkpoint_for_frame(
            timestamp,
            chunk_start,
            app_settings
                .ai
                .processing
                .periodic_full_frame_interval_ms
                .into(),
            app_settings
                .ai
                .processing
                .max_full_frame_checkpoints_per_minute,
        );
        let signature = file_sha1(path)?;
        if app_settings.ai.processing.frame_similarity_enabled
            && previous_signature.as_deref() == Some(signature.as_str())
            && !full_checkpoint
        {
            previous_signature = Some(signature);
            continue;
        }
        let image = decode_png_rgba(path)?;
        let width = image.width as i64;
        let height = image.height as i64;
        let full_region = VisualRegion {
            height,
            width,
            x: 0,
            y: 0,
        };
        let regions = if full_checkpoint {
            vec![full_region]
        } else if let Some(previous) = previous_image.as_ref() {
            visual_delta_regions(previous, &image, tile_size)
        } else {
            vec![full_region]
        };
        if regions.is_empty() {
            previous_signature = Some(signature);
            previous_image = Some(image);
            continue;
        }
        let abrupt_transition = previous_image.as_ref().is_some_and(|previous| {
            visual_changed_pixel_ratio(previous, &image) >= VISUAL_ABRUPT_TRANSITION_RATIO
        });
        let before_context = abrupt_transition
            .then(|| {
                index
                    .checked_sub(1)
                    .map(|previous_index| context_for(&paths[previous_index], previous_index))
            })
            .flatten();
        let after_context = abrupt_transition
            .then(|| {
                paths
                    .get(index + 1)
                    .map(|next_path| context_for(next_path, index + 1))
            })
            .flatten();
        for (region_index, region) in regions.into_iter().enumerate() {
            let is_full_region = region == full_region;
            let crop_path = if is_full_region {
                path.clone()
            } else {
                let crop_path = temp_dir
                    .path()
                    .join(format!("frame-{index:06}-region-{region_index:03}.png"));
                write_visual_crop_png(&image, &region, &crop_path)?;
                crop_path
            };
            let region_signature = if is_full_region {
                signature.clone()
            } else {
                file_sha1(&crop_path)?
            };
            let source_frame_ref =
                source_frame_ref_for_region(&segment.id, timestamp, index, &region);
            frames.push(ExtractedVisualFrame {
                abrupt_transition,
                after_context: after_context.clone(),
                before_context: before_context.clone(),
                frame_timestamp_ms: timestamp,
                full_checkpoint,
                height,
                path: crop_path,
                region,
                signature: region_signature,
                source_frame_ref,
                width,
            });
        }
        previous_signature = Some(signature);
        previous_image = Some(image);
    }

    Ok(ExtractedVisualFrameSet {
        frames,
        _temp_dir: temp_dir,
    })
}

pub(in crate::ai) fn effective_frame_recognition_fps(
    app_settings: &settings::AppSettings,
    recording_fps: f64,
) -> f64 {
    let fps = if app_settings
        .ai
        .processing
        .frame_recognition_sync_with_recording
    {
        recording_fps
    } else {
        app_settings.ai.processing.frame_recognition_fps
    };
    fps.clamp(
        settings::MIN_AI_FRAME_RECOGNITION_FPS,
        settings::MAX_AI_FRAME_RECOGNITION_FPS,
    )
}

pub(in crate::ai) fn frame_sample_interval_ms(fps: f64) -> i64 {
    ((1_000.0 / fps).round() as i64).max(1)
}

pub(in crate::ai) fn full_checkpoint_for_frame(
    timestamp_ms: i64,
    chunk_start_ms: i64,
    interval_ms: i64,
    max_per_minute: u32,
) -> bool {
    if timestamp_ms <= chunk_start_ms {
        return true;
    }
    if interval_ms <= 0 || timestamp_ms.saturating_sub(chunk_start_ms) % interval_ms != 0 {
        return false;
    }
    let minute_index = timestamp_ms.saturating_sub(chunk_start_ms) / 60_000;
    let minute_start = chunk_start_ms + (minute_index * 60_000);
    let checkpoints_before = ((timestamp_ms - minute_start) / interval_ms).max(0) as u32;
    checkpoints_before < max_per_minute
}
