use super::{file_error, transcription_error, TranscriptionError};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone)]
pub(super) struct LocalScratchOwner {
    pub(super) job_id: Option<String>,
    pub(super) request_id: String,
}
impl LocalScratchOwner {
    pub(super) fn validated(
        job_id: Option<&str>,
        request_id: &str,
    ) -> Result<Self, TranscriptionError> {
        uuid::Uuid::parse_str(request_id).map_err(|_| {
            transcription_error(
                "ai_transcription_request_invalid",
                "Local scratch identity is invalid.",
            )
        })?;
        if let Some(job_id) = job_id {
            uuid::Uuid::parse_str(job_id).map_err(|_| {
                transcription_error(
                    "ai_transcription_request_invalid",
                    "Local scratch job identity is invalid.",
                )
            })?;
        }
        Ok(Self {
            job_id: job_id.map(str::to_string),
            request_id: request_id.to_string(),
        })
    }
    pub(super) fn from_audio_path(path: &Path) -> Option<Self> {
        let request = path.parent()?;
        let parent = request.parent()?;
        let job_id = if parent.file_name()?.to_str()? == "local-transcription" {
            None
        } else {
            Some(parent.file_name()?.to_str()?)
        };
        Self::validated(job_id, request.file_name()?.to_str()?).ok()
    }
}

pub(super) struct LocalAudioCopy {
    path: PathBuf,
    directory: PathBuf,
}
impl LocalAudioCopy {
    pub(super) fn write(
        app_data_dir: &Path,
        owner: &LocalScratchOwner,
        bytes: &[u8],
    ) -> Result<Self, TranscriptionError> {
        let mut directory = app_data_dir.join("processing/local-transcription");
        if let Some(job_id) = &owner.job_id {
            directory = directory.join(job_id);
        }
        directory = directory.join(&owner.request_id);
        fs::create_dir_all(&directory).map_err(file_error)?;
        let copy = Self {
            path: directory.join(format!("{}.wav", uuid::Uuid::now_v7())),
            directory,
        };
        fs::write(&copy.path, bytes).map_err(file_error)?;
        Ok(copy)
    }
    pub(super) fn path(&self) -> &Path {
        &self.path
    }
}
impl Drop for LocalAudioCopy {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.directory);
    }
}
