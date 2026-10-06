use super::super::super::*;
use super::super::visual::decode_png_rgba_for_ocr;
use sha2::Sha256;

pub(super) struct FullFrame {
    pub(super) path: PathBuf,
    pub(super) timestamp_ms: i64,
    pub(super) end_ms: i64,
    pub(super) width: i64,
    pub(super) height: i64,
    pub(super) pixel_hash: String,
    pub(super) source_ref: String,
}

pub(super) struct FullFrameSet {
    _directory: TempProcessingDirectory,
    paths: Vec<PathBuf>,
    segment_id: String,
    track_id: Option<String>,
    chunk_start: i64,
    chunk_end: i64,
    fps: f64,
}
impl FullFrameSet {
    pub(super) fn len(&self) -> usize {
        self.paths.len()
    }
    pub(super) fn timestamp(&self, index: usize) -> i64 {
        self.chunk_start + sample_offset_ms(index, self.fps)
    }
    pub(super) fn frame(&self, index: usize) -> Result<FullFrame, AiError> {
        let path = self.paths[index].clone();
        let image = decode_png_rgba_for_ocr(&path)?;
        let timestamp_ms = self.chunk_start + sample_offset_ms(index, self.fps);
        Ok(FullFrame {
            path,
            timestamp_ms,
            end_ms: (self.chunk_start + sample_offset_ms(index + 1, self.fps)).min(self.chunk_end),
            width: image.width as i64,
            height: image.height as i64,
            pixel_hash: pixel_hash(&image),
            source_ref: format!(
                "segment:{}:offset:{}:frame:{}",
                if let Some(track) = &self.track_id {
                    format!("{}:stream:{track}", self.segment_id)
                } else {
                    self.segment_id.clone()
                },
                timestamp_ms,
                index
            ),
        })
    }
}

pub(super) fn extract_full_frames(
    app: &AppHandle,
    chunk: &ProcessingChunkRow,
    segment: &RecordingSegmentRow,
    playback_offset: i64,
    fps: f64,
    cancel: &AtomicBool,
) -> Result<FullFrameSet, AiError> {
    check_processing_cancel(cancel)?;
    let start = chunk.start_offset_ms.ok_or_else(|| {
        ai_error(
            "ai_ocr_source_invalid",
            "OCR source has no start timestamp.",
        )
    })?;
    let end = chunk
        .end_offset_ms
        .ok_or_else(|| ai_error("ai_ocr_source_invalid", "OCR source has no end timestamp."))?;
    let stream = segment
        .video_stream_index
        .ok_or_else(|| ai_error("ai_ocr_source_invalid", "OCR source has no video stream."))?;
    let directory = TempProcessingDirectory::new(processing_temp_visual_directory(
        app,
        &chunk.job_id,
        &chunk.id,
    )?);
    extract_to_directory(
        directory.path(),
        &segment.file_path,
        stream,
        start - playback_offset,
        end - start,
        fps,
        cancel,
    )?;
    check_processing_cancel(cancel)?;
    let mut paths = fs::read_dir(directory.path())
        .map_err(file_error)?
        .map(|entry| entry.map(|entry| entry.path()).map_err(file_error))
        .collect::<Result<Vec<_>, _>>()?;
    paths.sort();
    paths.truncate(
        paths
            .len()
            .min(((end - start) as f64 * fps / 1000.0).ceil() as usize),
    );
    Ok(FullFrameSet {
        _directory: directory,
        paths,
        segment_id: segment.id.clone(),
        track_id: chunk.track_id.clone(),
        chunk_start: start,
        chunk_end: end,
        fps,
    })
}

fn extract_to_directory(
    directory: &Path,
    source: &str,
    stream: i64,
    start: i64,
    duration: i64,
    fps: f64,
    cancel: &AtomicBool,
) -> Result<(), AiError> {
    let mut command = Command::new(ffmpeg::binary_path());
    command
        .args([
            "-hide_banner",
            "-v",
            "error",
            "-ss",
            &format_seconds(start),
            "-t",
            &format_seconds(duration),
            "-threads",
            "2",
            "-i",
            source,
            "-map",
            &format!("0:{stream}"),
            "-vf",
            &format!("fps={fps:.6}"),
            "-fps_mode",
            "passthrough",
            "-threads",
            "1",
        ])
        .arg(directory.join("frame-%06d.png"));
    let output = crate::subprocess::run(&mut command, cancel, Duration::from_secs(60), 1_000_000)
        .map_err(|error| {
        ai_error(
            if error.kind() == std::io::ErrorKind::Interrupted {
                "ai_processing_cancelled"
            } else if error.kind() == std::io::ErrorKind::TimedOut {
                "ai_ocr_frame_extract_timeout"
            } else {
                "ai_ocr_frame_extract_failed"
            },
            "OCR frame extraction did not finish.",
        )
    })?;
    if !output.status.success() {
        return Err(ai_error(
            "ai_ocr_frame_extract_failed",
            "Full-frame OCR extraction failed.",
        ));
    }
    Ok(())
}

fn sample_offset_ms(index: usize, fps: f64) -> i64 {
    ((index as f64 / fps) * 1000.0).round() as i64
}
fn pixel_hash(image: &super::super::visual::VisualImage) -> String {
    let mut hash = Sha256::new();
    hash.update((image.width as u64).to_le_bytes());
    hash.update((image.height as u64).to_le_bytes());
    hash.update(&image.pixels);
    format!("sha256:{}", hex::encode(hash.finalize()))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_sample_has_original_canvas_and_exact_pixel_identity() {
        let a = super::super::super::visual::VisualImage {
            width: 2,
            height: 1,
            pixels: vec![1; 8],
        };
        let mut b = a.clone();
        b.pixels[0] = 2;
        assert_eq!(pixel_hash(&a), pixel_hash(&a.clone()));
        assert_ne!(pixel_hash(&a), pixel_hash(&b));
        assert_eq!(sample_offset_ms(1800, 3.0), 600_000);
        assert_eq!(sample_offset_ms(2, 3.0), 667);
    }
    #[test]
    fn unchanged_video_still_yields_every_full_resolution_sample() {
        let root = std::env::temp_dir().join(format!("daydream-ocr-samples-{}", Uuid::now_v7()));
        fs::create_dir_all(&root).unwrap();
        let video = root.join("synthetic.mkv");
        let output = Command::new(ffmpeg::binary_path())
            .args([
                "-hide_banner",
                "-v",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                "color=c=white:s=64x32:r=2:d=2",
                "-c:v",
                "ffv1",
            ])
            .arg(&video)
            .output()
            .unwrap();
        assert!(output.status.success(), "synthetic video generation failed");
        let frames = root.join("frames");
        fs::create_dir_all(&frames).unwrap();
        extract_to_directory(
            &frames,
            video.to_str().unwrap(),
            0,
            0,
            2000,
            2.0,
            &AtomicBool::new(false),
        )
        .unwrap();
        let mut paths = fs::read_dir(&frames)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect::<Vec<_>>();
        paths.sort();
        let sampled = FullFrameSet {
            track_id: None,
            _directory: TempProcessingDirectory::new(frames),
            paths,
            segment_id: "synthetic".into(),
            chunk_start: 0,
            chunk_end: 2000,
            fps: 2.0,
        };
        assert_eq!(sampled.len(), 4);
        for index in 0..4 {
            let frame = sampled.frame(index).unwrap();
            assert_eq!((frame.width, frame.height), (64, 32));
            assert_eq!(frame.timestamp_ms, index as i64 * 500);
            assert_eq!(frame.pixel_hash, sampled.frame(0).unwrap().pixel_hash);
        }
        drop(sampled);
        fs::remove_dir_all(root).unwrap();
    }
}
