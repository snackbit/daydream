use super::*;

const OCR_LANGUAGE_CATALOG: [(&str, &str); 2] = [("tessdata-eng", "eng"), ("tessdata-deu", "deu")];

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::ai) struct OcrLanguageData {
    pub(in crate::ai) code: &'static str,
    pub(in crate::ai) path: PathBuf,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(in crate::ai) struct SystemTesseractProbe {
    pub(in crate::ai) available: bool,
    pub(in crate::ai) languages: Vec<String>,
    pub(in crate::ai) path: Option<PathBuf>,
    pub(in crate::ai) tessdata_dir: Option<PathBuf>,
    pub(in crate::ai) version: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ResolvedTesseractRuntime {
    pub(crate) binary: OsString,
    pub(crate) data_source: String,
    pub(crate) languages: String,
    pub(crate) runtime_path: Option<PathBuf>,
    pub(crate) runtime_source: String,
    pub(crate) runtime_version: Option<String>,
    pub(crate) tessdata_dir: Option<PathBuf>,
}

pub(crate) fn resolve_tesseract_runtime(
    app: &AppHandle,
) -> Result<ResolvedTesseractRuntime, AiError> {
    let managed_runtime = managed_tesseract_runtime_path(app)?;
    let system = probe_system_tesseract();
    let managed_tessdata = verified_managed_tessdata(app)?;
    let complete_managed_tessdata = managed_tessdata.len() == OCR_LANGUAGE_CATALOG.len();
    let system_languages = usable_system_languages(&system);

    let (data_source, languages, tessdata_dir) = if complete_managed_tessdata {
        (
            "managed-best".to_string(),
            language_codes(&managed_tessdata),
            managed_tessdata
                .first()
                .and_then(|language| language.path.parent())
                .map(Path::to_path_buf),
        )
    } else if !system_languages.is_empty() {
        (
            "system".to_string(),
            system_languages,
            system.tessdata_dir.clone(),
        )
    } else if !managed_tessdata.is_empty() {
        (
            "partial".to_string(),
            language_codes(&managed_tessdata),
            managed_tessdata
                .first()
                .and_then(|language| language.path.parent())
                .map(Path::to_path_buf),
        )
    } else {
        return Err(ai_error(
            "ai_ocr_tessdata_missing",
            "Download Tesseract OCR data or install system tessdata before running OCR.",
        ));
    };

    let (binary, runtime_path, runtime_source, runtime_version) = if managed_runtime.is_file() {
        (
            managed_runtime.clone().into_os_string(),
            Some(managed_runtime),
            "managed".to_string(),
            None,
        )
    } else if system.available {
        (
            OsString::from("tesseract"),
            system.path.clone(),
            "system".to_string(),
            system.version.clone(),
        )
    } else {
        return Err(ai_error(
            "ai_ocr_tesseract_unavailable",
            "Tesseract OCR is not available on this system.",
        ));
    };

    Ok(ResolvedTesseractRuntime {
        binary,
        data_source,
        languages: languages.join("+"),
        runtime_path,
        runtime_source,
        runtime_version,
        tessdata_dir,
    })
}

pub(in crate::ai) fn probe_system_tesseract() -> SystemTesseractProbe {
    let path = which_tesseract();
    let Some(path) = path else {
        return SystemTesseractProbe::default();
    };

    let version_output = Command::new(&path).arg("--version").output();
    let Ok(version_output) = version_output else {
        return SystemTesseractProbe::default();
    };
    if !version_output.status.success() {
        return SystemTesseractProbe::default();
    }

    let version_text = command_text(&version_output);
    let version = parse_tesseract_version(&version_text);
    let lang_output = Command::new(&path).arg("--list-langs").output();
    let (tessdata_dir, languages) = lang_output
        .ok()
        .filter(|output| output.status.success())
        .map(|output| parse_tesseract_languages(&command_text(&output)))
        .unwrap_or((None, Vec::new()));

    SystemTesseractProbe {
        available: true,
        languages,
        path: Some(path),
        tessdata_dir,
        version,
    }
}

pub(in crate::ai) fn verified_managed_tessdata(
    app: &AppHandle,
) -> Result<Vec<OcrLanguageData>, AiError> {
    let mut available = Vec::new();
    for (id, code) in OCR_LANGUAGE_CATALOG {
        let catalog = catalog_model(id).ok_or_else(|| {
            ai_error(
                "ai_ocr_tessdata_missing",
                "Tesseract OCR data is not configured.",
            )
        })?;
        let path = model_path(app, catalog)?;
        if is_verified_model_file(&path, catalog)? {
            available.push(OcrLanguageData { code, path });
        }
    }

    Ok(available)
}

pub(in crate::ai) fn managed_tesseract_runtime_path(app: &AppHandle) -> Result<PathBuf, AiError> {
    let runtime_catalog = catalog_model("tesseract-runtime").ok_or_else(|| {
        ai_error(
            "ai_ocr_tesseract_runtime_unavailable",
            "Tesseract OCR runtime is not configured.",
        )
    })?;

    model_path(app, runtime_catalog)
}

pub(in crate::ai) fn usable_system_languages(system: &SystemTesseractProbe) -> Vec<&'static str> {
    OCR_LANGUAGE_CATALOG
        .iter()
        .filter_map(|(_id, code)| {
            system
                .languages
                .iter()
                .any(|language| language == code)
                .then_some(*code)
        })
        .collect()
}

pub(in crate::ai) fn language_codes(languages: &[OcrLanguageData]) -> Vec<&'static str> {
    languages.iter().map(|language| language.code).collect()
}

pub(in crate::ai) fn parse_tesseract_version(output: &str) -> Option<String> {
    output
        .lines()
        .find_map(|line| line.strip_prefix("tesseract "))
        .and_then(|version| version.split_whitespace().next())
        .map(ToOwned::to_owned)
}

pub(in crate::ai) fn parse_tesseract_languages(output: &str) -> (Option<PathBuf>, Vec<String>) {
    let mut lines = output.lines();
    let first_line = lines.next().unwrap_or_default();
    let tessdata_dir = first_line
        .split('"')
        .nth(1)
        .filter(|path| !path.trim().is_empty())
        .map(PathBuf::from);
    let languages = lines
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect();

    (tessdata_dir, languages)
}

fn command_text(output: &std::process::Output) -> String {
    let mut bytes = output.stdout.clone();
    bytes.extend_from_slice(&output.stderr);
    String::from_utf8_lossy(&bytes).to_string()
}

fn which_tesseract() -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|directory| directory.join(tesseract_binary_name()))
        .find(|candidate| candidate.is_file())
}

fn tesseract_binary_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "tesseract.exe"
    } else {
        "tesseract"
    }
}
