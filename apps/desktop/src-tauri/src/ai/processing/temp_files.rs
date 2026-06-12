use super::super::*;

pub(in crate::ai) fn processing_temp_audio_path(
    app: &AppHandle,
    job_id: &str,
    chunk_id: &str,
) -> Result<PathBuf, AiError> {
    let directory = processing_temp_audio_directory(app, job_id)?;
    Ok(directory.join(format!("{chunk_id}.wav")))
}

pub(in crate::ai) fn processing_temp_audio_range_path(
    app: &AppHandle,
    job_id: &str,
    chunk_id: &str,
    range_index: usize,
) -> Result<PathBuf, AiError> {
    let directory = processing_temp_audio_directory(app, job_id)?;
    Ok(directory.join(format!("{chunk_id}-{range_index}.wav")))
}

fn processing_temp_audio_directory(app: &AppHandle, job_id: &str) -> Result<PathBuf, AiError> {
    let directory = app
        .path()
        .app_data_dir()
        .map_err(|error| {
            ai_error(
                "ai_processing_app_data_unavailable",
                &format!("App data directory is unavailable: {error}"),
            )
        })?
        .join("processing")
        .join("transcription")
        .join(job_id);
    fs::create_dir_all(&directory).map_err(file_error)?;
    Ok(directory)
}

pub(in crate::ai) fn processing_temp_visual_directory(
    app: &AppHandle,
    job_id: &str,
    chunk_id: &str,
) -> Result<PathBuf, AiError> {
    let directory = app
        .path()
        .app_data_dir()
        .map_err(|error| {
            ai_error(
                "ai_processing_app_data_unavailable",
                &format!("App data directory is unavailable: {error}"),
            )
        })?
        .join("processing")
        .join("visual")
        .join(job_id)
        .join(chunk_id);
    if directory.exists() {
        fs::remove_dir_all(&directory).map_err(file_error)?;
    }
    fs::create_dir_all(&directory).map_err(file_error)?;
    Ok(directory)
}

pub(in crate::ai) struct TempAudioFile {
    path: PathBuf,
}

impl TempAudioFile {
    pub(in crate::ai) fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub(in crate::ai) fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempAudioFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

pub(in crate::ai) struct TempProcessingDirectory {
    path: PathBuf,
}

impl TempProcessingDirectory {
    pub(in crate::ai) fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub(in crate::ai) fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempProcessingDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
