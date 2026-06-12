use super::super::super::*;
use super::types::*;

pub(super) fn run_tesseract_ocr(
    path: &Path,
    frame: &ExtractedVisualFrame,
) -> Result<Vec<OcrCandidate>, AiError> {
    run_tesseract_ocr_with_binary("tesseract", path, frame)
}

pub(in crate::ai) fn run_tesseract_ocr_with_binary(
    binary: &str,
    path: &Path,
    frame: &ExtractedVisualFrame,
) -> Result<Vec<OcrCandidate>, AiError> {
    let output = Command::new(binary)
        .arg(path)
        .args(["stdout", "tsv"])
        .output()
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                ai_error(
                    "ai_ocr_tesseract_unavailable",
                    "Tesseract OCR is not available on this system.",
                )
            } else {
                file_error(error)
            }
        })?;
    if !output.status.success() {
        return Err(ai_error(
            "ai_ocr_tesseract_failed",
            "Tesseract OCR failed for the selected frame.",
        ));
    }
    let tsv = String::from_utf8(output.stdout).map_err(|error| {
        ai_error(
            "ai_ocr_tesseract_output_invalid",
            &format!("Tesseract OCR output was not valid UTF-8: {error}"),
        )
    })?;
    parse_tesseract_tsv(&tsv, frame)
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
