use super::super::super::*;
use serde::{Deserialize, Serialize};
use sha2::Sha256;

pub(super) const OCR_PROMPT_VERSION: &str = "literal-ocr-v1";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(super) enum OcrEngine {
    Tesseract,
    Vision,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct OcrJobConfig {
    pub(super) engine: OcrEngine,
    pub(super) vision: Option<settings::AiVisionProviderSettings>,
    pub(super) recognition_fps: f64,
    pub(super) sync_with_recording: bool,
    pub(super) prompt_version: String,
    #[serde(default = "full_sampling", skip_serializing_if = "is_full_sampling")]
    pub(super) sampling_mode: String,
}
fn full_sampling() -> String {
    "full".into()
}
fn is_full_sampling(value: &String) -> bool {
    value == "full"
}
impl OcrJobConfig {
    pub(super) fn capture(settings: &settings::AppSettings) -> Result<Self, AiError> {
        let engine = match settings.ai.ocr.engine.as_str() {
            "tesseract" => OcrEngine::Tesseract,
            "vision" => OcrEngine::Vision,
            _ => return Err(ai_error("ai_ocr_engine_invalid", "OCR engine is invalid.")),
        };
        Ok(Self {
            vision: (engine == OcrEngine::Vision).then(|| {
                let mut vision = settings.ai.vision.clone();
                vision.observations_enabled = false;
                vision
            }),
            engine,
            recognition_fps: settings.ai.processing.frame_recognition_fps,
            sync_with_recording: settings.ai.processing.frame_recognition_sync_with_recording,
            prompt_version: OCR_PROMPT_VERSION.into(),
            sampling_mode: settings.ai.processing.visual_sampling_mode.clone(),
        })
    }
    pub(super) fn legacy() -> Self {
        let mut config = Self::capture(&settings::default_settings()).expect("default OCR config");
        config.sampling_mode = "full".into();
        config
    }
    pub(super) fn fps(&self, recorded_fps: f64) -> f64 {
        (if self.sync_with_recording {
            recorded_fps
        } else {
            self.recognition_fps
        })
        .clamp(
            settings::MIN_AI_FRAME_RECOGNITION_FPS,
            settings::MAX_AI_FRAME_RECOGNITION_FPS,
        )
    }
    pub(super) fn fingerprint(&self, runtime_identity: &str) -> Result<String, AiError> {
        let bytes = serde_json::to_vec(self).map_err(|_| {
            ai_error(
                "ai_ocr_config_invalid",
                "OCR configuration could not be encoded.",
            )
        })?;
        let mut hash = Sha256::new();
        hash.update(bytes);
        hash.update(runtime_identity.as_bytes());
        Ok(format!("sha256:{}", hex::encode(hash.finalize())))
    }
}

pub(crate) fn capture_ocr_job_config(settings: &settings::AppSettings) -> Result<String, AiError> {
    serde_json::to_string(&OcrJobConfig::capture(settings)?).map_err(|_| {
        ai_error(
            "ai_ocr_config_invalid",
            "OCR configuration could not be encoded.",
        )
    })
}

pub(super) fn load_ocr_job_config(
    storage: &AppStorage,
    job_id: &str,
) -> Result<OcrJobConfig, AiError> {
    let json = storage
        .with_connection(|connection| storage::get_processing_job_ocr_config(connection, job_id))
        .map_err(storage_error)?;
    match json {
        Some(json) => {
            let config: OcrJobConfig = serde_json::from_str(&json).map_err(|_| {
                ai_error(
                    "ai_ocr_config_invalid",
                    "Stored OCR configuration is invalid.",
                )
            })?;
            if !config.recognition_fps.is_finite()
                || config.recognition_fps <= 0.0
                || config.prompt_version != OCR_PROMPT_VERSION
                || !matches!(config.sampling_mode.as_str(), "full" | "adaptive-v1")
            {
                return Err(ai_error(
                    "ai_ocr_config_invalid",
                    "Stored OCR configuration is not supported.",
                ));
            }
            Ok(config)
        }
        None => Ok(OcrJobConfig::legacy()), // Existing work never gains implicit remote consent.
    }
}

pub(super) fn load_chunk_ocr_config(
    storage: &AppStorage,
    chunk: &ProcessingChunkRow,
) -> Result<OcrJobConfig, AiError> {
    let original = load_ocr_job_config(storage, &chunk.job_id)?;
    let override_json = storage
        .with_connection(|connection| storage::processing_chunk_config(connection, &chunk.id))
        .map_err(storage_error)?;
    let Some(json) = override_json else {
        return Ok(original);
    };
    let settings: settings::AppSettings = serde_json::from_str(&json).map_err(|_| {
        ai_error(
            "ai_processing_config_invalid",
            "The selected source configuration is invalid.",
        )
    })?;
    let mut updated = OcrJobConfig::capture(&settings)?;
    // Retrying existing samples changes their provider, not their source-time
    // identity. Full OCR reprocessing remains the explicit cadence-change path.
    updated.recognition_fps = original.recognition_fps;
    updated.sync_with_recording = original.sync_with_recording;
    updated.sampling_mode = original.sampling_mode;
    Ok(updated)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn historical_config_retains_full_sampling_and_its_serialized_cache_identity() {
        let json = r#"{"engine":"tesseract","vision":null,"recognitionFps":0.2,"syncWithRecording":true,"promptVersion":"literal-ocr-v1"}"#;
        let old: OcrJobConfig = serde_json::from_str(json).unwrap();
        assert_eq!(old.sampling_mode, "full");
        assert_eq!(serde_json::to_string(&old).unwrap(), json);
        let new = OcrJobConfig::capture(&settings::default_settings()).unwrap();
        assert_eq!(new.sampling_mode, "adaptive-v1");
        assert_ne!(
            old.fingerprint("native").unwrap(),
            new.fingerprint("native").unwrap()
        );
    }
    #[test]
    fn legacy_jobs_keep_local_ocr_even_after_settings_choose_vision() {
        let mut settings = settings::default_settings();
        settings.ai.ocr.engine = "vision".into();
        let pinned = OcrJobConfig::legacy();
        assert_eq!(pinned.engine, OcrEngine::Tesseract);
        assert!(pinned.vision.is_none());
        assert_ne!(
            pinned.fingerprint("native").unwrap(),
            OcrJobConfig::capture(&settings)
                .unwrap()
                .fingerprint("native")
                .unwrap()
        );
    }
    #[test]
    fn pinned_config_cannot_change_when_live_settings_change() {
        let settings = settings::default_settings();
        let json = capture_ocr_job_config(&settings).unwrap();
        let mut changed = settings;
        changed.ai.ocr.engine = "vision".into();
        let pinned: OcrJobConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(pinned.engine, OcrEngine::Tesseract);
        assert!(!json.contains("plaintext"));
    }
}
