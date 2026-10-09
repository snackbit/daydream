//! Private, bounded embedding adapters. No prompt/provider body is logged.
#[path = "embedding/api.rs"]
mod api;
#[path = "embedding/local.rs"]
mod local;

pub(crate) async fn shutdown_for_maintenance() -> Result<(), String> {
    local::shutdown_for_maintenance().await
}

use crate::settings::RecallEmbeddingSettings;
use sha2::{Digest, Sha256};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
};

#[derive(Clone, Debug)]
pub struct EmbeddingSpace {
    pub fingerprint: String,
    pub dimensions: usize,
}

#[derive(Clone, Copy)]
pub enum EmbeddingPurpose {
    Query,
    Document,
}

pub trait EmbeddingProvider: Send + Sync {
    fn space(&self) -> EmbeddingSpace;
    fn configuration_fingerprint(&self) -> String {
        self.space().fingerprint
    }
    fn embed(
        &self,
        purpose: EmbeddingPurpose,
        texts: &[String],
        cancelled: &AtomicBool,
    ) -> impl std::future::Future<Output = Result<Vec<Vec<f32>>, String>> + Send;
}

pub struct ManagedEmbeddingProvider {
    settings: RecallEmbeddingSettings,
    space: EmbeddingSpace,
    local: local::LocalWorker,
    api_key: Option<String>,
}

impl ManagedEmbeddingProvider {
    /// `models_dir` is app_data_dir/models; `worker_path` is the packaged executable.
    pub fn new(
        settings: &RecallEmbeddingSettings,
        models_dir: PathBuf,
        worker_path: PathBuf,
        api_key: Option<String>,
    ) -> Result<Self, String> {
        let space = configured_space(settings)?;
        if settings.provider == "openai-compatible"
            && settings.auth_mode == "api-key"
            && api_key.as_deref().is_none_or(str::is_empty)
        {
            return Err("recall_embedding_key_missing".into());
        }
        Ok(Self {
            settings: settings.clone(),
            space,
            local: local::LocalWorker::new(models_dir, worker_path),
            api_key,
        })
    }
}

/// Installation availability only; this does not claim a driver/model has
/// passed live inference. The provider test and indexing outcomes own that.
pub fn availability(
    settings: &RecallEmbeddingSettings,
    models_dir: &std::path::Path,
    worker_path: &std::path::Path,
) -> Result<(), String> {
    configured_space(settings)?;
    if settings.provider != "local" {
        return Ok(());
    }
    if !worker_path.is_file() {
        return Err("recall_embedding_worker_unavailable".into());
    }
    if !models_dir
        .join("embeddings")
        .join(format!("{}.gguf", settings.local_model_id))
        .is_file()
    {
        return Err("recall_embedding_model_missing".into());
    }
    if settings.local_backend == "vulkan"
        && !worker_path
            .with_file_name(if cfg!(windows) {
                "daydream-embedding-worker-vulkan.exe"
            } else {
                "daydream-embedding-worker-vulkan"
            })
            .is_file()
    {
        return Err("recall_embedding_vulkan_unavailable".into());
    }
    Ok(())
}

pub fn configured_space(settings: &RecallEmbeddingSettings) -> Result<EmbeddingSpace, String> {
    let dimensions = if settings.provider == "local" {
        384
    } else {
        settings.dimensions as usize
    };
    if !(1..=8192).contains(&dimensions) {
        return Err("recall_embedding_dimensions_invalid".into());
    }
    if settings.provider != "local" && settings.provider != "openai-compatible" {
        return Err("recall_embedding_provider_invalid".into());
    }
    if settings.provider == "openai-compatible" && !settings.enabled {
        return Err("recall_embedding_remote_disabled".into());
    }
    if settings.provider == "local"
        && !matches!(settings.local_backend.as_str(), "cpu" | "auto" | "vulkan")
    {
        return Err("recall_embedding_backend_invalid".into());
    }
    if settings.provider == "openai-compatible" {
        let endpoint = settings
            .base_url
            .as_deref()
            .ok_or("recall_embedding_endpoint_missing")?;
        let endpoint =
            reqwest::Url::parse(endpoint).map_err(|_| "recall_embedding_endpoint_invalid")?;
        if !matches!(endpoint.scheme(), "http" | "https")
            || !endpoint.username().is_empty()
            || endpoint.password().is_some()
            || endpoint.query().is_some()
            || endpoint.fragment().is_some()
        {
            return Err("recall_embedding_endpoint_invalid".into());
        }
        if settings
            .model
            .as_deref()
            .is_none_or(|model| model.trim().is_empty())
        {
            return Err("recall_embedding_model_missing".into());
        }
        if !matches!(settings.auth_mode.as_str(), "none" | "api-key")
            || !matches!(settings.convention.as_str(), "plain" | "e5" | "bge")
        {
            return Err("recall_embedding_configuration_invalid".into());
        }
    }
    let identity = if settings.provider == "local" {
        let sha = match settings.local_model_id.as_str() {
            "multilingual-e5-small-q8_0" => {
                "0a34067a40f25d3149b36885faa62bee0e5284d0f9edc102acfc00e115d953e8"
            }
            "bge-small-en-v1.5-q8_0" => {
                "f046db1dc724cf4f6f0a0c5917e922823b73eb1d27b8f9a9c2797f7866974804"
            }
            _ => return Err("recall_embedding_model_unknown".into()),
        };
        format!("llama-abeada335e2e78bd3fe63febafab7e900ce75810:{sha}:{}:pooling-v1:tokenizer-special-v2:e5-unigram-adapter-v2:0942789e0111452f1cc446f70036e26d24458c3c7858c53cc12ca4d87531279b", settings.local_model_id)
    } else {
        format!(
            "api:{}:{}:{}:{}",
            settings.base_url.as_deref().unwrap_or_default(),
            settings.model.as_deref().unwrap_or_default(),
            dimensions,
            settings.convention
        )
    };
    let identity = format!("{identity}:canonical-fragments480bytes-overlap96-fullpassage-v2");
    let space = EmbeddingSpace {
        fingerprint: hex::encode(Sha256::digest(identity.as_bytes())),
        dimensions,
    };
    Ok(space)
}

impl EmbeddingProvider for ManagedEmbeddingProvider {
    fn space(&self) -> EmbeddingSpace {
        self.space.clone()
    }
    fn configuration_fingerprint(&self) -> String {
        configuration_fingerprint(&self.settings)
    }
    async fn embed(
        &self,
        purpose: EmbeddingPurpose,
        texts: &[String],
        cancelled: &AtomicBool,
    ) -> Result<Vec<Vec<f32>>, String> {
        validate_inputs(texts, cancelled)?;
        let vectors = if self.settings.provider == "local" {
            self.local
                .embed(&self.settings, purpose, texts, cancelled)
                .await?
        } else {
            api::embed(
                &self.settings,
                self.api_key.as_deref(),
                purpose,
                texts,
                cancelled,
            )
            .await?
        };
        validate_vectors(vectors, texts.len(), self.space.dimensions, cancelled)
    }
}

pub fn configuration_fingerprint(settings: &RecallEmbeddingSettings) -> String {
    hex::encode(Sha256::digest(
        serde_json::to_vec(settings).unwrap_or_default(),
    ))
}

pub async fn test_embedding_provider(
    settings: &RecallEmbeddingSettings,
    models_dir: PathBuf,
    worker_path: PathBuf,
    key: Option<String>,
) -> Result<EmbeddingSpace, String> {
    let provider = ManagedEmbeddingProvider::new(settings, models_dir, worker_path, key)?;
    provider
        .embed(
            EmbeddingPurpose::Document,
            &[
                "Synthetic travel budget discussion.".into(),
                "Synthetische Besprechung des Reisebudgets.".into(),
            ],
            &AtomicBool::new(false),
        )
        .await?;
    Ok(provider.space())
}

pub(super) fn validate_inputs(texts: &[String], cancelled: &AtomicBool) -> Result<(), String> {
    if cancelled.load(Ordering::Acquire) {
        return Err("recall_cancelled".into());
    }
    if texts.is_empty()
        || texts.len() > 32
        || texts
            .iter()
            .any(|text| text.trim().is_empty() || text.len() > 8192)
    {
        return Err("recall_embedding_input_limit".into());
    }
    Ok(())
}

pub(super) fn validate_vectors(
    vectors: Vec<Vec<f32>>,
    count: usize,
    dimensions: usize,
    cancelled: &AtomicBool,
) -> Result<Vec<Vec<f32>>, String> {
    if cancelled.load(Ordering::Acquire) {
        return Err("recall_cancelled".into());
    }
    if vectors.len() != count
        || vectors.iter().any(|v| {
            v.len() != dimensions || v.iter().any(|x| !x.is_finite()) || v.iter().all(|x| *x == 0.0)
        })
    {
        return Err("recall_embedding_output_invalid".into());
    }
    Ok(vectors)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_wrong_shape_nonfinite_zero_and_cancelled_outputs() {
        let cancelled = AtomicBool::new(false);
        for vectors in [
            vec![],
            vec![vec![1.0]],
            vec![vec![f32::NAN, 1.0]],
            vec![vec![0.0, 0.0]],
        ] {
            assert!(validate_vectors(vectors, 1, 2, &cancelled).is_err());
        }
        cancelled.store(true, Ordering::Release);
        assert!(validate_vectors(vec![vec![1.0, 1.0]], 1, 2, &cancelled).is_err());
    }
    #[test]
    fn remote_consent_and_model_conventions_identify_vector_space() {
        let mut settings = crate::settings::RecallSettings::default().embedding;
        let local = configured_space(&settings).unwrap();
        settings.local_backend = "auto".into();
        assert_eq!(
            local.fingerprint,
            configured_space(&settings).unwrap().fingerprint
        );
        settings.provider = "openai-compatible".into();
        assert!(configured_space(&settings).is_err());
        settings.enabled = true;
        settings.model = Some("synthetic-model".into());
        settings.base_url = Some("http://localhost/v1".into());
        let plain = configured_space(&settings).unwrap();
        settings.convention = "e5".into();
        assert_ne!(
            plain.fingerprint,
            configured_space(&settings).unwrap().fingerprint
        );
        settings.dimensions = 0;
        assert!(configured_space(&settings).is_err());
    }
}
