use super::*;

pub(super) fn playback_asset_is_fresh(source_path: &str, output_path: &Path) -> bool {
    let Some((source_len, source_modified_ms)) = file_fingerprint(source_path) else {
        return false;
    };
    let Some((output_len, output_modified_ms)) = file_fingerprint(&path_to_string(output_path))
    else {
        return false;
    };

    source_len > 0 && output_len > 0 && output_modified_ms >= source_modified_ms
}

pub(super) fn run_ffmpeg(
    ffmpeg_path: &str,
    input_args: &[&str],
    output_path: &Path,
    output_format: &str,
) -> Result<(), String> {
    let temp_path = temp_output_path(output_path);
    let _ = fs::remove_file(&temp_path);
    let mut command = Command::new(ffmpeg_path);
    command
        .args(input_args)
        .args(["-f", output_format])
        .arg(&temp_path);
    let output = command.output().map_err(|error| {
        format!(
            "Unable to run FFmpeg at {ffmpeg_path}. Set DAYDREAM_FFMPEG_PATH or install ffmpeg: {error}"
        )
    })?;

    if !output.status.success() {
        let _ = fs::remove_file(&temp_path);
        return Err(format!(
            "FFmpeg failed while preparing playback assets: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    if !temp_path.is_file() || fs::metadata(&temp_path).map_or(0, |metadata| metadata.len()) == 0 {
        let _ = fs::remove_file(&temp_path);
        return Err(format!(
            "FFmpeg completed without creating playback asset {}: {}",
            path_to_string(output_path),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    fs::rename(&temp_path, output_path).map_err(|error| {
        format!(
            "Unable to move playback asset {} into place: {error}",
            path_to_string(output_path)
        )
    })
}

pub(super) fn temp_output_path(output_path: &Path) -> PathBuf {
    let Some(extension) = output_path
        .extension()
        .and_then(|extension| extension.to_str())
    else {
        return output_path.with_extension("tmp");
    };

    output_path.with_extension(format!("tmp.{extension}"))
}
