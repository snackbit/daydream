use super::super::super::*;
use super::extraction::FullFrame;
use sha2::Sha256;

pub(super) fn runtime_identity(runtime: &ResolvedTesseractRuntime) -> Result<String, AiError> {
    let mut hash = Sha256::new();
    hash.update(
        format!(
            "{:?}:{:?}:{:?}:{}",
            runtime.binary, runtime.runtime_version, runtime.tessdata_dir, runtime.languages
        )
        .as_bytes(),
    );
    if let Some(path) = &runtime.runtime_path {
        hash_file(&mut hash, path)?;
    }
    if let Some(directory) = &runtime.tessdata_dir {
        for language in runtime.languages.split('+') {
            hash_file(
                &mut hash,
                &directory.join(format!("{language}.traineddata")),
            )?;
        }
    }
    Ok(format!("sha256:{}", hex::encode(hash.finalize())))
}
fn hash_file(hash: &mut Sha256, path: &Path) -> Result<(), AiError> {
    let mut file = File::open(path).map_err(file_error)?;
    let mut buffer = [0_u8; 8192];
    loop {
        let read = file.read(&mut buffer).map_err(file_error)?;
        if read == 0 {
            return Ok(());
        }
        hash.update(&buffer[..read]);
    }
}
use crate::storage::{OcrBlockInput, OcrContentInput, OcrWordHierarchy, OcrWordInput};

pub(super) fn run_tesseract_document(
    runtime: &ResolvedTesseractRuntime,
    frame: &FullFrame,
    cancel: &AtomicBool,
) -> Result<OcrContentInput, AiError> {
    let mut command = Command::new(&runtime.binary);
    command.env("OMP_THREAD_LIMIT", "1");
    if let Some(directory) = &runtime.tessdata_dir {
        command.arg("--tessdata-dir").arg(directory);
    }
    command
        .args(["-l", &runtime.languages])
        .arg(&frame.path)
        .args(["stdout", "-c", "tessedit_create_tsv=1"]);
    let output = crate::subprocess::run(&mut command, cancel, Duration::from_secs(30), 8_000_000)
        .map_err(|error| {
        ai_error(
            if error.kind() == std::io::ErrorKind::Interrupted {
                "ai_processing_cancelled"
            } else if error.kind() == std::io::ErrorKind::TimedOut {
                "ai_ocr_tesseract_timeout"
            } else {
                "ai_ocr_tesseract_unavailable"
            },
            "Tesseract did not finish this frame.",
        )
    })?;
    if !output.status.success() {
        return Err(ai_error(
            "ai_ocr_tesseract_failed",
            "Tesseract failed to read the full frame.",
        ));
    }
    let tsv = std::str::from_utf8(&output.stdout).map_err(|_| {
        ai_error(
            "ai_ocr_tesseract_output_invalid",
            "Tesseract output is invalid.",
        )
    })?;
    parse_document(tsv, frame.width, frame.height)
}

pub(super) fn parse_document(
    tsv: &str,
    frame_width: i64,
    frame_height: i64,
) -> Result<OcrContentInput, AiError> {
    let mut lines = tsv.lines();
    let header = lines.next().ok_or_else(invalid_tsv)?;
    let columns = header
        .split('\t')
        .enumerate()
        .map(|(index, name)| (name, index))
        .collect::<BTreeMap<_, _>>();
    let required = [
        "level",
        "page_num",
        "block_num",
        "par_num",
        "line_num",
        "word_num",
        "left",
        "top",
        "width",
        "height",
        "conf",
        "text",
    ];
    let indexes = required
        .iter()
        .map(|name| columns.get(name).copied().ok_or_else(invalid_tsv))
        .collect::<Result<Vec<_>, _>>()?;
    let mut blocks: Vec<OcrBlockInput> = Vec::new();
    let mut previous_key = None;
    for line in lines {
        let fields = line.split('\t').collect::<Vec<_>>();
        let field = |index: usize| fields.get(indexes[index]).copied().ok_or_else(invalid_tsv);
        let number = |index: usize| -> Result<i64, AiError> {
            field(index)?.parse().map_err(|_| invalid_tsv())
        };
        if number(0)? != 5 {
            continue;
        }
        let text = field(11)?.trim();
        if text.is_empty() {
            continue;
        }
        let hierarchy = OcrWordHierarchy {
            page_num: number(1)?,
            block_num: number(2)?,
            paragraph_num: number(3)?,
            line_num: number(4)?,
            word_num: number(5)?,
        };
        let key = (
            hierarchy.page_num,
            hierarchy.block_num,
            hierarchy.paragraph_num,
            hierarchy.line_num,
        );
        let (x, y, width, height) = (number(6)?, number(7)?, number(8)?, number(9)?);
        if x < 0 || y < 0 || width <= 0 || height <= 0 || x >= frame_width || y >= frame_height {
            continue;
        }
        let confidence = field(10)?.parse::<f64>().map_err(|_| invalid_tsv())?;
        let confidence = (confidence.is_finite() && confidence >= 0.0)
            .then_some((confidence / 100.0).clamp(0.0, 1.0));
        let region = VisualRegion {
            x,
            y,
            width: width.min(frame_width - x),
            height: height.min(frame_height - y),
        };
        if previous_key != Some(key) {
            blocks.push(OcrBlockInput {
                id: Uuid::now_v7().to_string(),
                order: blocks.len() as i64,
                text: String::new(),
                confidence: None,
                region: None,
                raw_region_ids: Vec::new(),
                words: Vec::new(),
            });
            previous_key = Some(key);
        }
        let block = blocks.last_mut().expect("line block exists");
        if !block.text.is_empty() {
            block.text.push(' ');
        }
        block.text.push_str(text);
        block.region = Some(union_region(block.region.as_ref(), &region));
        block.words.push(OcrWordInput {
            id: Uuid::now_v7().to_string(),
            text: text.into(),
            confidence,
            region: Some(region),
            raw_region_id: None,
            hierarchy: Some(hierarchy),
        });
    }
    for block in &mut blocks {
        let values = block
            .words
            .iter()
            .filter_map(|word| word.confidence)
            .collect::<Vec<_>>();
        block.confidence =
            (!values.is_empty()).then(|| values.iter().sum::<f64>() / values.len() as f64);
    }
    Ok(OcrContentInput {
        text: blocks
            .iter()
            .map(|block| block.text.as_str())
            .collect::<Vec<_>>()
            .join("\n"),
        blocks,
    })
}
fn invalid_tsv() -> AiError {
    ai_error(
        "ai_ocr_tesseract_output_invalid",
        "Tesseract returned invalid structured text.",
    )
}
fn union_region(previous: Option<&VisualRegion>, next: &VisualRegion) -> VisualRegion {
    let Some(previous) = previous else {
        return *next;
    };
    let x = previous.x.min(next.x);
    let y = previous.y.min(next.y);
    VisualRegion {
        x,
        y,
        width: (previous.x + previous.width).max(next.x + next.width) - x,
        height: (previous.y + previous.height).max(next.y + next.height) - y,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tsv_words_keep_native_reading_order_and_line_hierarchy() {
        let tsv="level\tpage_num\tblock_num\tpar_num\tline_num\tword_num\tleft\ttop\twidth\theight\tconf\ttext\n5\t1\t1\t1\t1\t1\t10\t12\t20\t10\t99\tSynthetic\n5\t1\t1\t1\t1\t2\t40\t10\t20\t10\t95\tmeeting\n5\t1\t1\t1\t2\t1\t10\t30\t20\t10\t80\tNotes";
        let document = parse_document(tsv, 100, 100).unwrap();
        assert_eq!(document.text, "Synthetic meeting\nNotes");
        assert_eq!(
            document.blocks[0].words[1]
                .hierarchy
                .as_ref()
                .unwrap()
                .word_num,
            2
        );
    }
    #[test]
    fn empty_tsv_is_successful_empty_document() {
        let header="level\tpage_num\tblock_num\tpar_num\tline_num\tword_num\tleft\ttop\twidth\theight\tconf\ttext";
        assert_eq!(parse_document(header, 100, 100).unwrap().text, "");
        assert!(parse_document("invalid", 100, 100).is_err());
    }
}
