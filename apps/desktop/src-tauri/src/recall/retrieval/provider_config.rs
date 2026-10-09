//! Resolve explicitly enabled recall profiles and private credentials at the edge.
use super::embedding::ManagedEmbeddingProvider;
use crate::{
    settings,
    storage::{self, AppStorage},
};
use std::path::PathBuf;
use tauri::Manager;

pub fn credential(
    storage: &AppStorage,
    reference: Option<&str>,
    auth_mode: &str,
) -> Result<Option<String>, String> {
    if auth_mode == "none" {
        return Ok(None);
    }
    if auth_mode != "api-key" {
        return Err("recall_auth_mode_invalid".into());
    }
    let reference = reference.ok_or("recall_credential_missing")?;
    let envelope = storage
        .with_connection(|connection| storage::get_app_setting(connection, reference))
        .map_err(|_| "recall_storage_failed")?
        .ok_or("recall_credential_missing")?
        .value_json;
    crate::secret_crypto::decrypt_sensitive_setting_with_platform_key(&envelope)
        .map(Some)
        .map_err(|_| "recall_credential_unavailable".into())
}

pub fn worker_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    if cfg!(debug_assertions) {
        return Ok(PathBuf::from(env!("DAYDREAM_EMBEDDING_WORKER")));
    }
    app.path()
        .resolve(
            if cfg!(windows) {
                "embedding/daydream-embedding-worker.exe"
            } else {
                "embedding/daydream-embedding-worker"
            },
            tauri::path::BaseDirectory::Resource,
        )
        .map_err(|_| "recall_embedding_worker_unavailable".into())
}

pub fn build_embedding_provider(
    app: &tauri::AppHandle,
) -> Result<ManagedEmbeddingProvider, String> {
    let storage = app.state::<AppStorage>();
    let settings = settings::load_app_settings(&storage).map_err(|_| "recall_settings_failed")?;
    let profile = settings.ai.recall.embedding;
    let key = if profile.provider == "local" {
        None
    } else {
        credential(&storage, profile.api_key_ref.as_deref(), &profile.auth_mode)?
    };
    let models = app
        .path()
        .app_data_dir()
        .map_err(|_| "recall_storage_failed")?
        .join("models");
    ManagedEmbeddingProvider::new(&profile, models, worker_path(app)?, key)
}
