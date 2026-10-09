use super::*;

pub(super) fn extract_audio_stream(
    input_path: &str,
    stream_index: i64,
    slice_start_ms: i64,
    slice_duration_ms: i64,
    output_path: &Path,
    cancel: &AtomicBool,
) -> Result<(), AiError> {
    check_atomic_cancel(cancel)?;
    let _ = fs::remove_file(output_path);
    let mut command = Command::new(ffmpeg::binary_path());
    command
        .args(ffmpeg_audio_extract_args(
            input_path,
            stream_index,
            slice_start_ms,
            slice_duration_ms,
        ))
        .arg(output_path);
    let output = crate::subprocess::run(&mut command, cancel, Duration::from_secs(60), 1_000_000)
        .map_err(|error| {
        ai_error(
            if error.kind() == std::io::ErrorKind::Interrupted {
                "ai_processing_cancelled"
            } else if error.kind() == std::io::ErrorKind::TimedOut {
                "ai_processing_audio_extract_timeout"
            } else {
                "ai_processing_audio_extract_failed"
            },
            "Audio extraction did not finish.",
        )
    })?;
    check_atomic_cancel(cancel)?;
    if !output.status.success() {
        return Err(ai_error(
            "ai_processing_audio_extract_failed",
            "Audio extraction failed.",
        ));
    }

    Ok(())
}

pub(in crate::ai) fn ffmpeg_audio_extract_args(
    input_path: &str,
    stream_index: i64,
    slice_start_ms: i64,
    slice_duration_ms: i64,
) -> Vec<String> {
    vec![
        "-hide_banner".to_string(),
        "-v".to_string(),
        "error".to_string(),
        "-y".to_string(),
        "-i".to_string(),
        input_path.to_string(),
        "-ss".to_string(),
        format_seconds(slice_start_ms),
        "-t".to_string(),
        format_seconds(slice_duration_ms),
        "-map".to_string(),
        format!("0:{stream_index}"),
        "-vn".to_string(),
        "-af".to_string(),
        ffmpeg::audio_timeline_filter(16_000, slice_start_ms.saturating_add(slice_duration_ms)),
        "-ac".to_string(),
        "1".to_string(),
        "-ar".to_string(),
        "16000".to_string(),
        "-c:a".to_string(),
        "pcm_s16le".to_string(),
        "-f".to_string(),
        "wav".to_string(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn extracted_audio_preserves_timestamp_gaps_and_slice_origin() {
        let segment = crate::ffmpeg::audio_timeline_tests::gap_fixture();
        for stream in [1, 2] {
            for (start, duration) in [(0, 12_000), (4500, 1000)] {
                let output = std::env::temp_dir()
                    .join(format!("daydream-synthetic-extract-{}.wav", Uuid::now_v7()));
                extract_audio_stream(
                    &segment.file_path,
                    stream,
                    start,
                    duration,
                    &output,
                    &AtomicBool::new(false),
                )
                .unwrap();
                let pcm = Command::new(ffmpeg::binary_path())
                    .args(["-v", "error", "-i"])
                    .arg(&output)
                    .args(["-f", "s16le", "pipe:1"])
                    .output()
                    .unwrap();
                assert!(pcm.status.success());
                let samples = pcm
                    .stdout
                    .chunks_exact(2)
                    .map(|b| i16::from_le_bytes([b[0], b[1]]) as i32)
                    .collect::<Vec<_>>();
                assert_eq!(
                    samples.len(),
                    duration as usize * 16,
                    "PCM duration must retain missing time"
                );
                let gap = if start == 0 {
                    4600 * 16..5100 * 16
                } else {
                    100 * 16..600 * 16
                };
                assert!(
                    samples[gap].iter().all(|s| s.abs() < 2),
                    "gap must stay silent at the original offset"
                );
                assert!(
                    samples[samples.len() - 1600..]
                        .iter()
                        .any(|s| s.abs() > 100),
                    "post-gap speech shifted or lost"
                );
                fs::remove_file(output).unwrap();
            }
        }
        fs::remove_file(segment.file_path).unwrap();
    }
}
