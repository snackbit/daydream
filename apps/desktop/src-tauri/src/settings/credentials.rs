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

pub(super) fn reset(storage: &AppStorage) -> Result<(), AppSettingsError> {
    storage
        .with_connection_mut(|connection| {
            let transaction = connection.transaction()?;
            transaction.execute("DELETE FROM app_settings WHERE is_sensitive=0", [])?;
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
    inputs.extend(settings_to_storage_inputs(&settings, timestamp)?);
    let rows = storage
        .with_connection_mut(|connection| {
            let transaction = connection.transaction()?;
            for input in &inputs {
                storage::save_app_setting(&transaction, input)?;
            }
            storage::maintenance::authorize_retention_on_settings_save(&transaction)?;
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
mod tests {
    use super::*;
    #[test]
    fn explicit_settings_save_resumes_retention_only_after_a_successful_commit() {
        let storage = storage::test_storage("/tmp/synthetic-reset-retention-authorization".into());
        reset(&storage).unwrap();
        assert!(storage
            .with_connection(storage::maintenance::retention_cleanup_paused)
            .unwrap());
        storage.with_connection(|connection| connection.execute_batch("CREATE TRIGGER reject_settings AFTER INSERT ON app_settings WHEN NEW.key='ai.recall' BEGIN SELECT RAISE(FAIL,'synthetic settings failure'); END;")).unwrap();
        let input = SaveAppSettingsInput {
            settings: crate::settings::default_settings(),
            credentials: SettingsCredentialDrafts::default(),
        };
        assert!(save_with_encryptor(&storage, input, |_| panic!("No credential drafts")).is_err());
        assert!(storage
            .with_connection(storage::maintenance::retention_cleanup_paused)
            .unwrap());
        storage
            .with_connection(|connection| connection.execute_batch("DROP TRIGGER reject_settings;"))
            .unwrap();
        save_with_encryptor(
            &storage,
            SaveAppSettingsInput {
                settings: crate::settings::default_settings(),
                credentials: SettingsCredentialDrafts::default(),
            },
            |_| panic!("No credential drafts"),
        )
        .unwrap();
        assert!(!storage
            .with_connection(storage::maintenance::retention_cleanup_paused)
            .unwrap());
    }
    #[test]
    fn failed_reset_preserves_profiles_and_credentials() {
        let storage =
            storage::test_storage(std::path::PathBuf::from("/tmp/synthetic-recall-reset"));
        let saved = save_with_encryptor(
            &storage,
            SaveAppSettingsInput {
                settings: crate::settings::default_settings(),
                credentials: SettingsCredentialDrafts {
                    recall_generation_key: Some("synthetic-key".into()),
                    ..Default::default()
                },
            },
            |_| Ok("{\"ciphertext\":\"synthetic-envelope\"}".into()),
        )
        .unwrap();
        let rows_before = storage.with_connection(storage::list_app_settings).unwrap();
        storage.with_connection(|connection| connection.execute_batch("CREATE TRIGGER reject_reset BEFORE DELETE ON app_settings WHEN OLD.key='ai.recall' BEGIN SELECT RAISE(FAIL,'synthetic reset failure'); END;")).unwrap();
        assert!(reset(&storage).is_err());
        assert_eq!(crate::settings::load_app_settings(&storage).unwrap(), saved);
        assert_eq!(
            storage.with_connection(storage::list_app_settings).unwrap(),
            rows_before
        );
    }
    #[test]
    fn reset_withdraws_profiles_but_keeps_only_credentials_pinned_by_jobs() {
        let storage =
            storage::test_storage(std::path::PathBuf::from("/tmp/synthetic-recall-reset"));
        let mut settings = crate::settings::default_settings();
        settings.ai.vision.provider = "openai-compatible".into();
        let saved = save_with_encryptor(
            &storage,
            SaveAppSettingsInput {
                settings,
                credentials: SettingsCredentialDrafts {
                    recall_generation_key: Some("synthetic-pinned-key".into()),
                    vision_key: Some("synthetic-unused-key".into()),
                    ..Default::default()
                },
            },
            |_| Ok("{\"ciphertext\":\"synthetic-envelope\"}".into()),
        )
        .unwrap();
        let reference = saved.ai.recall.generation.api_key_ref.clone().unwrap();
        let config = serde_json::to_string(&saved.ai.recall).unwrap();
        storage.with_connection(|connection| connection.execute("INSERT INTO recall_jobs(id,day_key,kind,status,input_fingerprint,config_fingerprint,config_json,created_at_ms,updated_at_ms) VALUES('synthetic-job','2026-01-01','generation','paused','synthetic-input','synthetic-profile',?1,1,1)", [config])).unwrap();
        reset(&storage).unwrap();
        assert_eq!(
            crate::settings::load_app_settings(&storage).unwrap(),
            crate::settings::default_settings()
        );
        let rows = storage.with_connection(storage::list_app_settings).unwrap();
        let secrets = rows
            .iter()
            .filter(|row| row.is_sensitive)
            .collect::<Vec<_>>();
        assert_eq!(secrets.len(), 1);
        assert_eq!(secrets[0].key, reference);
        assert!(storage
            .with_connection(storage::maintenance::retention_cleanup_paused)
            .unwrap());
        let deleted = storage
            .with_connection(|connection| prune_unreferenced_secrets(connection, Some(&reference)))
            .unwrap();
        assert_eq!(
            deleted, 0,
            "Explicit deletion shares the same pinned reference guard"
        );
    }
    #[test]
    fn active_recall_job_retains_its_pinned_key_until_cancelled() {
        let storage =
            storage::test_storage(std::path::PathBuf::from("/tmp/synthetic-recall-settings"));
        let settings = crate::settings::default_settings();
        let first = save_with_encryptor(
            &storage,
            SaveAppSettingsInput {
                settings,
                credentials: SettingsCredentialDrafts {
                    recall_generation_key: Some("synthetic-first-key".into()),
                    ..Default::default()
                },
            },
            |_| Ok("{\"ciphertext\":\"synthetic-envelope\"}".into()),
        )
        .unwrap();
        let old_ref = first.ai.recall.generation.api_key_ref.clone().unwrap();
        let config = serde_json::to_string(&first.ai.recall).unwrap();
        storage.with_connection(|connection| connection.execute("INSERT INTO recall_jobs(id,day_key,kind,status,input_fingerprint,config_fingerprint,config_json,created_at_ms,updated_at_ms) VALUES('synthetic-job','2026-01-01','generation','paused','synthetic-input','synthetic-profile',?1,1,1)", [config])).unwrap();
        let second = save_with_encryptor(
            &storage,
            SaveAppSettingsInput {
                settings: first,
                credentials: SettingsCredentialDrafts {
                    recall_generation_key: Some("synthetic-second-key".into()),
                    ..Default::default()
                },
            },
            |_| Ok("{\"ciphertext\":\"synthetic-envelope\"}".into()),
        )
        .unwrap();
        assert!(storage
            .with_connection(storage::list_app_settings)
            .unwrap()
            .iter()
            .any(|row| row.key == old_ref));
        storage
            .with_connection(|connection| {
                connection.execute(
                    "UPDATE recall_jobs SET status='cancelled' WHERE id='synthetic-job'",
                    [],
                )
            })
            .unwrap();
        save_with_encryptor(
            &storage,
            SaveAppSettingsInput {
                settings: second,
                credentials: SettingsCredentialDrafts::default(),
            },
            |_| unreachable!(),
        )
        .unwrap();
        assert!(!storage
            .with_connection(storage::list_app_settings)
            .unwrap()
            .iter()
            .any(|row| row.key == old_ref));
    }
    #[test]
    fn recall_keys_and_profiles_commit_together_or_roll_back_together() {
        let storage =
            storage::test_storage(std::path::PathBuf::from("/tmp/synthetic-recall-settings"));
        storage.with_connection(|connection| connection.execute_batch("CREATE TRIGGER reject_recall BEFORE INSERT ON app_settings WHEN NEW.key='ai.recall' BEGIN SELECT RAISE(FAIL,'synthetic write failure'); END;")).unwrap();
        let mut settings = crate::settings::default_settings();
        settings.ai.recall.embedding.provider = "openai-compatible".into();
        settings.ai.recall.embedding.enabled = true;
        settings.ai.recall.generation.enabled = true;
        let result = save_with_encryptor(
            &storage,
            SaveAppSettingsInput {
                settings,
                credentials: SettingsCredentialDrafts {
                    recall_embedding_key: Some("synthetic-embedding-key".into()),
                    recall_generation_key: Some("synthetic-generation-key".into()),
                    ..Default::default()
                },
            },
            |_| Ok("{\"ciphertext\":\"synthetic-envelope\"}".into()),
        );
        assert!(result.is_err());
        assert!(storage
            .with_connection(storage::list_app_settings)
            .unwrap()
            .is_empty());
    }
    #[test]
    fn anonymous_recall_profiles_drop_keys_without_encryption() {
        let storage =
            storage::test_storage(std::path::PathBuf::from("/tmp/synthetic-recall-settings"));
        let mut settings = crate::settings::default_settings();
        settings.ai.recall.embedding.provider = "openai-compatible".into();
        settings.ai.recall.embedding.auth_mode = "none".into();
        settings.ai.recall.generation.auth_mode = "none".into();
        let saved = save_with_encryptor(
            &storage,
            SaveAppSettingsInput {
                settings,
                credentials: SettingsCredentialDrafts {
                    recall_embedding_key: Some("synthetic-embedding-key".into()),
                    recall_generation_key: Some("synthetic-generation-key".into()),
                    ..Default::default()
                },
            },
            |_| panic!("Anonymous profiles must not encrypt key drafts"),
        )
        .unwrap();
        assert!(saved.ai.recall.embedding.api_key_ref.is_none());
        assert!(saved.ai.recall.generation.api_key_ref.is_none());
    }
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
                    ..Default::default()
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
                    ..Default::default()
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
