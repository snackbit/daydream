use super::super::super::*;
use super::super::visual::decode_png_rgba_for_ocr;
use sha2::Sha256;

#[derive(Clone)]
pub(in crate::ai) struct FullFrame {
    pub(in crate::ai) path: PathBuf,
    pub(in crate::ai) timestamp_ms: i64,
    pub(in crate::ai) end_ms: i64,
    pub(in crate::ai) width: i64,
    pub(in crate::ai) height: i64,
    pub(in crate::ai) pixel_hash: String,
    pub(in crate::ai) source_ref: String,
}

pub(in crate::ai) struct FullFrameSet {
    _directory: TempProcessingDirectory,
    paths: Vec<PathBuf>,
    segment_id: String,
    track_id: Option<String>,
    chunk_start: i64,
    chunk_end: i64,
    fps: f64,
    metadata: Mutex<BTreeMap<usize, FullFrame>>,
    decoded: Mutex<Option<DecodedFrame>>,
}
struct DecodedFrame {
    encoded_hash: String,
    pixel_hash: String,
    image: Arc<super::super::visual::VisualImage>,
}
impl FullFrameSet {
    pub(in crate::ai) fn len(&self) -> usize {
        self.paths.len()
    }
    pub(in crate::ai) fn timestamp(&self, index: usize) -> i64 {
        self.chunk_start + sample_offset_ms(index, self.fps)
    }
    pub(in crate::ai) fn frame(&self, index: usize) -> Result<FullFrame, AiError> {
        if let Some(frame) = self
            .metadata
            .lock()
            .map_err(|_| file_error(std::io::Error::other("Frame metadata unavailable")))?
            .get(&index)
            .cloned()
        {
            return Ok(frame);
        }
        self.decode(index).map(|(frame, _)| frame)
    }
    pub(in crate::ai) fn decode(
        &self,
        index: usize,
    ) -> Result<(FullFrame, Arc<super::super::visual::VisualImage>), AiError> {
        let path = self.paths[index].clone();
        // Repeated PNGs are common on static screens. Compare their encoded
        // identity before decoding and hashing millions of identical pixels.
        let encoded_hash = format!("{:x}", Sha256::digest(fs::read(&path).map_err(file_error)?));
        let cached = self
            .decoded
            .lock()
            .map_err(|_| file_error(std::io::Error::other("Decoded frame cache unavailable")))?
            .as_ref()
            .filter(|entry| entry.encoded_hash == encoded_hash)
            .map(|entry| (entry.image.clone(), entry.pixel_hash.clone()));
        let (image, hash) = if let Some(cached) = cached {
            cached
        } else {
            let image = Arc::new(decode_png_rgba_for_ocr(&path)?);
            let hash = pixel_hash(&image);
            if image.pixels.len() <= 32 * 1024 * 1024 {
                *self.decoded.lock().map_err(|_| {
                    file_error(std::io::Error::other("Decoded frame cache unavailable"))
                })? = Some(DecodedFrame {
                    encoded_hash,
                    pixel_hash: hash.clone(),
                    image: image.clone(),
                });
            }
            (image, hash)
        };
        let timestamp_ms = self.chunk_start + sample_offset_ms(index, self.fps);
        let frame = FullFrame {
            path,
            timestamp_ms,
            end_ms: (self.chunk_start + sample_offset_ms(index + 1, self.fps)).min(self.chunk_end),
            width: image.width as i64,
            height: image.height as i64,
            pixel_hash: hash,
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
        };
        self.metadata
            .lock()
            .map_err(|_| file_error(std::io::Error::other("Frame metadata unavailable")))?
            .insert(index, frame.clone());
        Ok((frame, image))
    }
    pub(in crate::ai) fn file_bytes(&self) -> u64 {
        self.paths
            .iter()
            .filter_map(|path| fs::metadata(path).ok())
            .map(|metadata| metadata.len())
            .sum()
    }
}

pub(in crate::ai) fn extract_full_frames(
    app: &AppHandle,
    chunk: &ProcessingChunkRow,
    segment: &RecordingSegmentRow,
    playback_offset: i64,
    fps: f64,
    cancel: &AtomicBool,
) -> Result<FullFrameSet, AiError> {
    extract_frames(app, chunk, segment, playback_offset, Some(fps), cancel)
}

pub(in crate::ai) fn extract_context_frame(
    app: &AppHandle,
    chunk: &ProcessingChunkRow,
    segment: &RecordingSegmentRow,
    playback_offset: i64,
    cancel: &AtomicBool,
) -> Result<FullFrameSet, AiError> {
    extract_frames(app, chunk, segment, playback_offset, None, cancel)
}

fn extract_frames(
    app: &AppHandle,
    chunk: &ProcessingChunkRow,
    segment: &RecordingSegmentRow,
    playback_offset: i64,
    fps: Option<f64>,
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
    paths.truncate(fps.map_or(1, |fps| {
        ((end - start) as f64 * fps / 1000.0).ceil() as usize
    }));
    Ok(FullFrameSet {
        _directory: directory,
        paths,
        segment_id: segment.id.clone(),
        track_id: chunk.track_id.clone(),
        chunk_start: start,
        chunk_end: end,
        fps: fps.unwrap_or(1.0),
        metadata: Mutex::new(BTreeMap::new()),
        decoded: Mutex::new(None),
    })
}

pub(super) fn extract_to_directory(
    directory: &Path,
    source: &str,
    stream: i64,
    start: i64,
    duration: i64,
    fps: Option<f64>,
    cancel: &AtomicBool,
) -> Result<(), AiError> {
    let mut command = Command::new(ffmpeg::binary_path());
    command.args(["-hide_banner", "-v", "error", "-ss", &format_seconds(start)]);
    if fps.is_some() {
        command.args(["-t", &format_seconds(duration)]);
    }
    command.args([
        "-threads",
        "2",
        "-i",
        source,
        "-map",
        &format!("0:{stream}"),
    ]);
    if let Some(fps) = fps {
        command.args(["-vf", &format!("fps={fps:.6}")]);
    } else {
        command.args(["-frames:v", "1"]);
    }
    command
        .args(["-fps_mode", "passthrough", "-threads", "1"])
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
#[cfg(test)]
pub(in crate::ai) fn synthetic_frames(
    directory: PathBuf,
    paths: Vec<PathBuf>,
    start: i64,
    end: i64,
    fps: f64,
) -> FullFrameSet {
    FullFrameSet {
        _directory: TempProcessingDirectory::new(directory),
        paths,
        segment_id: "synthetic".into(),
        track_id: None,
        chunk_start: start,
        chunk_end: end,
        fps,
        metadata: Mutex::new(BTreeMap::new()),
        decoded: Mutex::new(None),
    }
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
    fn restored_context_uses_one_source_frame_at_fractional_and_whole_second_times() {
        let root =
            std::env::temp_dir().join(format!("daydream-context-restore-{}", Uuid::now_v7()));
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
                "color=c=white:s=64x32:r=3:d=3",
                "-f",
                "lavfi",
                "-i",
                "color=c=blue:s=64x32:r=3:d=3",
                "-map",
                "0:v",
                "-map",
                "1:v",
                "-c:v",
                "ffv1",
                "-threads",
                "1",
            ])
            .arg(&video)
            .output()
            .unwrap();
        assert!(output.status.success(), "synthetic video generation failed");
        for time in [333, 667, 1000, 1667] {
            let directory = root.join(time.to_string());
            fs::create_dir_all(&directory).unwrap();
            extract_to_directory(
                &directory,
                video.to_str().unwrap(),
                1,
                time,
                1,
                None,
                &AtomicBool::new(false),
            )
            .unwrap();
            let paths = fs::read_dir(&directory)
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .collect::<Vec<_>>();
            assert_eq!(paths.len(), 1, "one context frame at {time} ms");
            let image = decode_png_rgba_for_ocr(&paths[0]).unwrap();
            assert_eq!((image.width, image.height), (64, 32));
            assert!(
                image.pixels[2] > 240 && image.pixels[0] < 10,
                "selected blue stream restored"
            );
        }
        let cancelled = root.join("cancelled");
        fs::create_dir_all(&cancelled).unwrap();
        let error = extract_to_directory(
            &cancelled,
            video.to_str().unwrap(),
            1,
            333,
            1,
            None,
            &AtomicBool::new(true),
        )
        .unwrap_err();
        assert_eq!(error.code, "ai_processing_cancelled");
        fs::remove_dir_all(root).unwrap();
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
            Some(2.0),
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
            metadata: Mutex::new(BTreeMap::new()),
            decoded: Mutex::new(None),
        };
        assert_eq!(sampled.len(), 4);
        let (_, first_image) = sampled.decode(0).unwrap();
        for index in 0..4 {
            let (frame, image) = sampled.decode(index).unwrap();
            assert!(Arc::ptr_eq(&first_image, &image));
            assert_eq!((frame.width, frame.height), (64, 32));
            assert_eq!(frame.timestamp_ms, index as i64 * 500);
            assert_eq!(frame.pixel_hash, sampled.frame(0).unwrap().pixel_hash);
        }
        drop(sampled);
        fs::remove_dir_all(root).unwrap();
    }
}
