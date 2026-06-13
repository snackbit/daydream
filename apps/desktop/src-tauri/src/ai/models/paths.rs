use super::super::*;

pub(in crate::ai) fn model_cache_dir(app: &AppHandle) -> Result<PathBuf, AiError> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|error| AiError {
            code: "ai_model_cache_unavailable".to_string(),
            message: format!("Model cache directory is unavailable: {error}"),
        })?
        .join("models")
        .join("whisper.cpp"))
}

pub(in crate::ai) fn catalog_cache_dir(
    app: &AppHandle,
    catalog: &CatalogModel,
) -> Result<PathBuf, AiError> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|error| AiError {
            code: "ai_model_cache_unavailable".to_string(),
            message: format!("Model cache directory is unavailable: {error}"),
        })?
        .join("models")
        .join(catalog.cache_dir))
}

pub(in crate::ai) fn model_path(
    app: &AppHandle,
    catalog: &CatalogModel,
) -> Result<PathBuf, AiError> {
    if catalog.provider == "local-whisper-cpp" {
        return Ok(model_cache_dir(app)?.join(catalog.filename));
    }

    Ok(catalog_cache_dir(app, catalog)?.join(catalog.filename))
}

pub(in crate::ai) fn partial_model_path(
    app: &AppHandle,
    catalog: &CatalogModel,
) -> Result<PathBuf, AiError> {
    Ok(model_path(app, catalog)?.with_extension(format!(
        "{}part",
        model_path(app, catalog)?
            .extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| format!("{extension}."))
            .unwrap_or_default()
    )))
}

pub(in crate::ai) fn rollback_finalized_model_file(
    partial_path: &Path,
    final_path: &Path,
) -> Result<(), AiError> {
    if !final_path.exists() {
        return Ok(());
    }
    if partial_path.exists() {
        delete_file_if_exists(final_path)?;
        return Ok(());
    }
    fs::rename(final_path, partial_path).map_err(file_error)
}

pub(in crate::ai) fn downloaded_bytes(partial_path: &Path, final_path: &Path) -> i64 {
    fs::metadata(partial_path)
        .or_else(|_error| fs::metadata(final_path))
        .map(|metadata| metadata.len() as i64)
        .unwrap_or(0)
}

pub(in crate::ai) fn delete_file_if_exists(path: &Path) -> Result<(), AiError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(file_error(error)),
    }
}

pub(in crate::ai) fn is_verified_model_file(
    path: &Path,
    catalog: &CatalogModel,
) -> Result<bool, AiError> {
    Ok(path.is_file() && sha1_file(path)? == catalog.sha1)
}

pub(in crate::ai) fn sha1_file(path: &Path) -> Result<String, AiError> {
    let mut file = File::open(path).map_err(file_error)?;
    let mut hasher = Sha1::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(file_error)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    Ok(hex::encode(hasher.finalize()))
}
