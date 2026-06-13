use super::super::super::*;
use super::types::*;

pub(super) fn run_tesseract_ocr(
    app: &AppHandle,
    path: &Path,
    frame: &ExtractedVisualFrame,
) -> Result<Vec<OcrCandidate>, AiError> {
    let runtime = resolve_tesseract_runtime(app)?;
    tracing::info!(
        frame_timestamp_ms = frame.frame_timestamp_ms,
        source_frame_ref = %frame.source_frame_ref,
        runtime_source = %runtime.runtime_source,
        runtime_path = runtime
            .runtime_path
            .as_ref()
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_else(|| runtime.binary.to_string_lossy().to_string()),
        runtime_version = runtime.runtime_version.as_deref().unwrap_or(""),
        data_source = %runtime.data_source,
        tessdata_dir = runtime
            .tessdata_dir
            .as_ref()
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_default(),
        languages = %runtime.languages,
        "ai_ocr_tesseract_runtime_resolved"
    );
    run_tesseract_ocr_command(
        &runtime.binary,
        runtime.tessdata_dir.as_deref(),
        Some(&runtime.languages),
        path,
        frame,
    )
}

#[cfg(test)]
pub(in crate::ai) fn run_tesseract_ocr_with_binary(
    binary: &str,
    path: &Path,
    frame: &ExtractedVisualFrame,
) -> Result<Vec<OcrCandidate>, AiError> {
    run_tesseract_ocr_command(binary, None, None, path, frame)
}

fn run_tesseract_ocr_command(
    binary: impl AsRef<OsStr>,
    tessdata_dir: Option<&Path>,
    languages: Option<&str>,
    path: &Path,
    frame: &ExtractedVisualFrame,
) -> Result<Vec<OcrCandidate>, AiError> {
    let mut command = Command::new(binary);
    configure_tesseract_ocr_command(&mut command, tessdata_dir, languages, path);
    tracing::debug!(
        frame_timestamp_ms = frame.frame_timestamp_ms,
        source_frame_ref = %frame.source_frame_ref,
        frame_width = frame.width,
        frame_height = frame.height,
        region_x = frame.region.x,
        region_y = frame.region.y,
        region_width = frame.region.width,
        region_height = frame.region.height,
        full_checkpoint = frame.full_checkpoint,
        abrupt_transition = frame.abrupt_transition,
        tessdata_dir = tessdata_dir
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_default(),
        languages = languages.unwrap_or(""),
        image_path = %path.display(),
        "ai_ocr_tesseract_started"
    );
    let started = Instant::now();
    let output = command.output().map_err(|error| {
        let elapsed_ms = elapsed_ms(started);
        if error.kind() == std::io::ErrorKind::NotFound {
            tracing::warn!(
                frame_timestamp_ms = frame.frame_timestamp_ms,
                source_frame_ref = %frame.source_frame_ref,
                error = %error,
                elapsed_ms,
                "ai_ocr_tesseract_unavailable"
            );
            ai_error(
                "ai_ocr_tesseract_unavailable",
                "Tesseract OCR is not available on this system.",
            )
        } else {
            tracing::warn!(
                frame_timestamp_ms = frame.frame_timestamp_ms,
                source_frame_ref = %frame.source_frame_ref,
                error = %error,
                elapsed_ms,
                "ai_ocr_tesseract_spawn_failed"
            );
            file_error(error)
        }
    })?;
    let elapsed_ms = elapsed_ms(started);
    if !output.status.success() {
        tracing::warn!(
            frame_timestamp_ms = frame.frame_timestamp_ms,
            source_frame_ref = %frame.source_frame_ref,
            exit_code = ?output.status.code(),
            stdout_bytes = output.stdout.len(),
            stderr_bytes = output.stderr.len(),
            elapsed_ms,
            "ai_ocr_tesseract_failed"
        );
        return Err(ai_error(
            "ai_ocr_tesseract_failed",
            "Tesseract OCR failed for the selected frame.",
        ));
    }
    let tsv = String::from_utf8(output.stdout).map_err(|error| {
        tracing::warn!(
            frame_timestamp_ms = frame.frame_timestamp_ms,
            source_frame_ref = %frame.source_frame_ref,
            stderr_bytes = output.stderr.len(),
            elapsed_ms,
            "ai_ocr_tesseract_output_utf8_invalid"
        );
        ai_error(
            "ai_ocr_tesseract_output_invalid",
            &format!("Tesseract OCR output was not valid UTF-8: {error}"),
        )
    })?;
    let tsv_line_count = tsv.lines().count();
    let tsv_bytes = tsv.len();
    match parse_tesseract_tsv(&tsv, frame) {
        Ok(candidates) => {
            let (confidence_count, average_confidence) = ocr_confidence_summary(&candidates);
            tracing::info!(
                frame_timestamp_ms = frame.frame_timestamp_ms,
                source_frame_ref = %frame.source_frame_ref,
                tsv_bytes,
                tsv_line_count,
                stderr_bytes = output.stderr.len(),
                candidate_count = candidates.len(),
                confidence_count,
                average_confidence = ?average_confidence,
                elapsed_ms,
                "ai_ocr_tesseract_completed"
            );
            Ok(candidates)
        }
        Err(error) => {
            tracing::warn!(
                frame_timestamp_ms = frame.frame_timestamp_ms,
                source_frame_ref = %frame.source_frame_ref,
                error_code = %error.code,
                tsv_bytes,
                tsv_line_count,
                stderr_bytes = output.stderr.len(),
                elapsed_ms,
                "ai_ocr_tesseract_parse_failed"
            );
            Err(error)
        }
    }
}

fn configure_tesseract_ocr_command(
    command: &mut Command,
    tessdata_dir: Option<&Path>,
    languages: Option<&str>,
    path: &Path,
) {
    if let Some(tessdata_dir) = tessdata_dir {
        command.arg("--tessdata-dir").arg(tessdata_dir);
    }
    if let Some(languages) = languages {
        command.args(["-l", languages]);
    }
    command
        .arg(path)
        .args(["stdout", "-c", "tessedit_create_tsv=1"]);
}

#[cfg(test)]
pub(in crate::ai) fn tesseract_ocr_command_args_for_test(
    tessdata_dir: Option<&Path>,
    languages: Option<&str>,
    path: &Path,
) -> Vec<String> {
    let mut command = Command::new("tesseract");
    configure_tesseract_ocr_command(&mut command, tessdata_dir, languages, path);
    command
        .get_args()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect()
}

pub(in crate::ai) fn parse_tesseract_tsv(
    tsv: &str,
    frame: &ExtractedVisualFrame,
) -> Result<Vec<OcrCandidate>, AiError> {
    let mut lines = tsv.lines();
    let header = lines.next().ok_or_else(|| {
        ai_error(
            "ai_ocr_tesseract_output_invalid",
            "Tesseract OCR output was empty.",
        )
    })?;
    let columns = header
        .split('\t')
        .enumerate()
        .map(|(index, column)| (column, index))
        .collect::<BTreeMap<_, _>>();
    let index_for = |name: &str| -> Result<usize, AiError> {
        columns.get(name).copied().ok_or_else(|| {
            ai_error(
                "ai_ocr_tesseract_output_invalid",
                "Tesseract OCR output did not include required TSV columns.",
            )
        })
    };
    let left_index = index_for("left")?;
    let top_index = index_for("top")?;
    let width_index = index_for("width")?;
    let height_index = index_for("height")?;
    let confidence_index = index_for("conf")?;
    let text_index = index_for("text")?;
    let mut candidates = Vec::new();

    for line in lines {
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() <= text_index {
            continue;
        }
        let text = fields[text_index].trim();
        if text.is_empty() {
            continue;
        }
        let left = parse_i64_field(fields.get(left_index), 0);
        let top = parse_i64_field(fields.get(top_index), 0);
        let width = parse_i64_field(fields.get(width_index), frame.region.width);
        let height = parse_i64_field(fields.get(height_index), frame.region.height);
        if width <= 0 || height <= 0 {
            continue;
        }
        let confidence = fields
            .get(confidence_index)
            .and_then(|value| value.parse::<f64>().ok())
            .filter(|value| *value >= 0.0)
            .map(|value| (value / 100.0).clamp(0.0, 1.0));
        candidates.push(OcrCandidate {
            confidence,
            region: ocr_candidate_region(frame, left, top, width, height),
            text: text.to_string(),
        });
    }

    Ok(candidates)
}

fn ocr_confidence_summary(candidates: &[OcrCandidate]) -> (usize, Option<f64>) {
    let confidence_values = candidates
        .iter()
        .filter_map(|candidate| candidate.confidence)
        .collect::<Vec<_>>();
    if confidence_values.is_empty() {
        return (0, None);
    }
    let average = confidence_values.iter().sum::<f64>() / confidence_values.len() as f64;
    (confidence_values.len(), Some(average))
}

fn ocr_candidate_region(
    frame: &ExtractedVisualFrame,
    left: i64,
    top: i64,
    width: i64,
    height: i64,
) -> VisualRegion {
    let crop_x = left.max(0).min(frame.region.width.saturating_sub(1));
    let crop_y = top.max(0).min(frame.region.height.saturating_sub(1));
    let x = frame.region.x + crop_x;
    let y = frame.region.y + crop_y;
    let max_width = frame.region.x + frame.region.width - x;
    let max_height = frame.region.y + frame.region.height - y;

    VisualRegion {
        height: height.min(max_height).min(frame.height - y),
        width: width.min(max_width).min(frame.width - x),
        x,
        y,
    }
}

fn parse_i64_field(value: Option<&&str>, fallback: i64) -> i64 {
    value
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(fallback)
}
