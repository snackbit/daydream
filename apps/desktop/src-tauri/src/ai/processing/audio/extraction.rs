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
