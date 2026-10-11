//! Settings and encrypted credentials share a single SQLite commit.
use super::{
    persistence::{settings_from_storage_rows, settings_to_storage_inputs},
    service::{now_ms, storage_error},
    types::*,
    validation::validate_settings,
};
use crate::{
    secret_crypto,
    storage::{self, AppStorage, SaveAppSettingInput},
};

/// Import once under the same SQLite transaction as the legacy configuration read.
pub(crate) fn import_model_registry(storage: &AppStorage) -> Result<(), AppSettingsError> {
    storage
        .with_connection_mut(|connection| {
            let transaction = connection.transaction()?;
            let mut settings =
                settings_from_storage_rows(storage::list_app_settings(&transaction)?)
                    .map_err(|_| rusqlite::Error::InvalidQuery)?;
            if settings.ai.model_registry.is_none() {
                crate::model_registry::import_legacy(&mut settings);
                let value = serde_json::to_string(&settings.ai.model_registry)
                    .map_err(|_| rusqlite::Error::InvalidQuery)?;
                storage::save_app_setting(
                    &transaction,
                    &SaveAppSettingInput {
                        key: "ai.modelRegistry".into(),
                        value_json: value,
                        is_sensitive: false,
                        timestamp_ms: now_ms() as i64,
                    },
                )?;
            }
            transaction.commit()
        })
        .map_err(storage_error)
}

pub(super) fn save(
    storage: &AppStorage,
    input: SaveAppSettingsInput,
) -> Result<AppSettings, AppSettingsError> {
    save_with_encryptor(storage, input, |key| {
        secret_crypto::encrypt_sensitive_setting_with_platform_key(key).map_err(|_| ())
    })
}

pub(super) fn reset(storage: &AppStorage) -> Result<(), AppSettingsError> {
    crate::model_registry::subscription::cancel_all();
    storage
        .with_connection_mut(|connection| {
            let transaction = connection.transaction()?;
            transaction.execute("DELETE FROM app_settings WHERE is_sensitive=0", [])?;
            transaction.execute(
                "DELETE FROM app_settings WHERE key LIKE 'chatgpt.tokens.%'",
                [],
            )?;
            prune_unreferenced_secrets(&transaction, None)?;
            storage::maintenance::pause_retention_after_reset(&transaction)?;
            transaction.commit()
        })
        .map_err(storage_error)
}

fn save_with_encryptor(
    storage: &AppStorage,
    input: SaveAppSettingsInput,
    mut encrypt: impl FnMut(&str) -> Result<String, ()>,
) -> Result<AppSettings, AppSettingsError> {
    let mut settings = validate_settings(input.settings)?;
    let timestamp = now_ms() as i64;
    let mut inputs = Vec::new();
    if settings.ai.transcription.provider != "openai-compatible" {
        settings.ai.transcription.remote_api_key_ref = None;
    } else if let Some(key) = input
        .credentials
        .transcription_key
        .as_deref()
        .filter(|key| !key.trim().is_empty())
    {
        settings.ai.transcription.remote_api_key_ref = Some(encrypt_draft(
            &mut inputs,
            "transcription",
            key,
            timestamp,
            &mut encrypt,
        )?);
    }
    if settings.ai.vision.provider != "openai-compatible" || settings.ai.vision.auth_mode == "none"
    {
        settings.ai.vision.api_key_ref = None;
    } else if let Some(key) = input
        .credentials
        .vision_key
        .as_deref()
        .filter(|key| !key.trim().is_empty())
    {
        settings.ai.vision.api_key_ref = Some(encrypt_draft(
            &mut inputs,
            "vision",
            key,
            timestamp,
            &mut encrypt,
        )?);
    }
    let recall = &mut settings.ai.recall;
    if recall.embedding.provider != "openai-compatible" || recall.embedding.auth_mode == "none" {
        recall.embedding.api_key_ref = None;
    } else if let Some(key) = input
        .credentials
        .recall_embedding_key
        .as_deref()
        .filter(|key| !key.trim().is_empty())
    {
        recall.embedding.api_key_ref = Some(encrypt_draft(
            &mut inputs,
            "recall.embedding",
            key,
            timestamp,
            &mut encrypt,
        )?);
    }
    if recall.generation.auth_mode == "none" {
        recall.generation.api_key_ref = None;
    } else if let Some(key) = input
        .credentials
        .recall_generation_key
        .as_deref()
        .filter(|key| !key.trim().is_empty())
    {
        recall.generation.api_key_ref = Some(encrypt_draft(
            &mut inputs,
            "recall.generation",
            key,
            timestamp,
            &mut encrypt,
        )?);
    }
    if let Some(registry) = &mut settings.ai.model_registry {
        for (id, key) in &input.credentials.model_keys {
            let connection = registry
                .connections
                .iter_mut()
                .find(|connection| &connection.id == id)
                .ok_or_else(|| AppSettingsError {
                    code: "settings_model_invalid".into(),
                    field: Some("ai.modelRegistry".into()),
                    message: "The credential's connection no longer exists.".into(),
                })?;
            if connection.auth_mode != "api-key" || key.trim().is_empty() {
                continue;
            }
            connection.api_key_ref = Some(encrypt_draft(
                &mut inputs,
                "model",
                key,
                timestamp,
                &mut encrypt,
            )?);
        }
        for connection in &mut registry.connections {
            if connection.auth_mode != "api-key" {
                connection.api_key_ref = None;
            }
        }
        crate::model_registry::project(&mut settings)?;
    }
    inputs.extend(settings_to_storage_inputs(&settings, timestamp)?);
    let rows = storage
        .with_connection_mut(|connection| {
            let transaction = connection.transaction()?;
            for input in &inputs {
                storage::save_app_setting(&transaction, input)?;
            }
            if input.apply_storage_cleanup.unwrap_or(true) {
                storage::maintenance::authorize_retention_on_settings_save(&transaction)?;
            }
            prune_unreferenced_secrets(&transaction, None)?;
            let rows = storage::list_app_settings(&transaction)?;
            transaction.commit()?;
            Ok(rows)
        })
        .map_err(storage_error)?;
    settings_from_storage_rows(rows)
}

fn encrypt_draft(
    inputs: &mut Vec<SaveAppSettingInput>,
    role: &str,
    key: &str,
    timestamp: i64,
    encrypt: &mut impl FnMut(&str) -> Result<String, ()>,
) -> Result<String, AppSettingsError> {
    let envelope = encrypt(key.trim()).map_err(|_| AppSettingsError {
        code: "settings_credential_unavailable".into(),
        field: Some(format!("ai.{role}.apiKey")),
        message: "The API key could not be encrypted. Your saved settings were not changed.".into(),
    })?;
    let reference = format!("ai.secret.{role}.{}", uuid::Uuid::now_v7());
    inputs.push(SaveAppSettingInput {
        key: reference.clone(),
        value_json: envelope,
        is_sensitive: true,
        timestamp_ms: timestamp,
    });
    Ok(reference)
}

/// Shared authority for bulk pruning and explicit deletion. Referenced secrets
/// remain encrypted at rest until their settings/job snapshots release them.
pub(crate) fn prune_unreferenced_secrets(
    connection: &rusqlite::Connection,
    secret_ref: Option<&str>,
) -> rusqlite::Result<usize> {
    // Pinned OCR/job/chunk configurations retain their credential until those
    // records are deleted or explicitly replaced. This preserves saved retries.
    connection.execute("DELETE FROM app_settings WHERE key LIKE 'ai.secret.%' AND (?1 IS NULL OR key=?1) AND NOT EXISTS (
      SELECT 1 FROM app_settings s,json_tree(s.value_json) t WHERE s.is_sensitive=0 AND t.atom=app_settings.key
      UNION ALL SELECT 1 FROM processing_job_runtime r,json_tree(r.config_json) t WHERE t.atom=app_settings.key
      UNION ALL SELECT 1 FROM processing_chunk_runtime r,json_tree(r.config_json) t WHERE t.atom=app_settings.key
      UNION ALL SELECT 1 FROM processing_job_ocr_configs r,json_tree(r.config_json) t WHERE t.atom=app_settings.key
      UNION ALL SELECT 1 FROM recall_jobs r,json_tree(r.config_json) t WHERE r.status IN ('pending','running','paused') AND t.atom=app_settings.key
    );", [secret_ref])
}

#[cfg(test)]
mod model_registry_tests;
#[cfg(test)]
mod tests;
