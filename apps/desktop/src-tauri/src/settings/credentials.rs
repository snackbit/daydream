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

pub(super) fn save(
    storage: &AppStorage,
    input: SaveAppSettingsInput,
) -> Result<AppSettings, AppSettingsError> {
    save_with_encryptor(storage, input, |key| {
        secret_crypto::encrypt_sensitive_setting_with_platform_key(key).map_err(|_| ())
    })
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
    inputs.extend(settings_to_storage_inputs(&settings, timestamp)?);
    let rows = storage
        .with_connection_mut(|connection| {
            let transaction = connection.transaction()?;
            for input in &inputs {
                storage::save_app_setting(&transaction, input)?;
            }
            prune_unreferenced(&transaction)?;
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

fn prune_unreferenced(connection: &rusqlite::Connection) -> rusqlite::Result<()> {
    // Pinned OCR/job/chunk configurations retain their credential until those
    // records are deleted or explicitly replaced. This preserves saved retries.
    connection.execute_batch("DELETE FROM app_settings WHERE key LIKE 'ai.secret.%' AND NOT EXISTS (
      SELECT 1 FROM app_settings s,json_tree(s.value_json) t WHERE s.is_sensitive=0 AND t.atom=app_settings.key
      UNION ALL SELECT 1 FROM processing_job_runtime r,json_tree(r.config_json) t WHERE t.atom=app_settings.key
      UNION ALL SELECT 1 FROM processing_chunk_runtime r,json_tree(r.config_json) t WHERE t.atom=app_settings.key
      UNION ALL SELECT 1 FROM processing_job_ocr_configs r,json_tree(r.config_json) t WHERE t.atom=app_settings.key
    );")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn failed_settings_write_rolls_back_the_new_encrypted_credential() {
        let storage = storage::test_storage(std::path::PathBuf::from("/tmp/synthetic-settings"));
        storage.with_connection(|connection|connection.execute_batch("CREATE TRIGGER reject_vision BEFORE INSERT ON app_settings WHEN NEW.key='ai.vision.model' BEGIN SELECT RAISE(FAIL,'synthetic write failure'); END;")).unwrap();
        let mut settings = crate::settings::default_settings();
        settings.ai.vision.provider = "openai-compatible".into();
        settings.ai.vision.model = Some("synthetic".into());
        let result = save_with_encryptor(
            &storage,
            SaveAppSettingsInput {
                settings,
                credentials: SettingsCredentialDrafts {
                    vision_key: Some("synthetic-key".into()),
                    transcription_key: None,
                },
            },
            |_| Ok("{\"ciphertext\":\"synthetic-envelope\"}".into()),
        );
        assert!(result.is_err());
        assert!(
            storage
                .with_connection(storage::list_app_settings)
                .unwrap()
                .is_empty(),
            "Both the settings and prepared ciphertext must roll back."
        );
    }
    #[test]
    fn anonymous_provider_never_encrypts_or_retains_a_key_draft() {
        let storage = storage::test_storage(std::path::PathBuf::from("/tmp/synthetic-settings"));
        let mut settings = crate::settings::default_settings();
        settings.ai.vision.provider = "openai-compatible".into();
        settings.ai.vision.auth_mode = "none".into();
        settings.ai.vision.api_key_ref = Some("ai.secret.vision.old".into());
        let saved = save_with_encryptor(
            &storage,
            SaveAppSettingsInput {
                settings,
                credentials: SettingsCredentialDrafts {
                    vision_key: Some("synthetic-key".into()),
                    transcription_key: None,
                },
            },
            |_| panic!("Anonymous endpoints must not prepare authorization credentials"),
        )
        .unwrap();
        assert!(saved.ai.vision.api_key_ref.is_none());
        assert!(!storage
            .with_connection(storage::list_app_settings)
            .unwrap()
            .iter()
            .any(|row| row.is_sensitive));
    }
}
