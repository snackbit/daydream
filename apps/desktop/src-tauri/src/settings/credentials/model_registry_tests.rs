use super::*;

fn configured() -> AppSettings {
    let mut settings = crate::settings::default_settings();
    settings.ai.recall.generation.model = Some("synthetic-model".into());
    settings.ai.recall.generation.base_url = Some("https://api.openai.com/v1".into());
    crate::model_registry::import_legacy(&mut settings);
    settings
}

#[test]
fn model_keys_and_bindings_commit_atomically_and_import_is_idempotent() {
    let storage = storage::test_storage("/tmp/synthetic-model-settings".into());
    let settings = configured();
    let id = settings.ai.model_registry.as_ref().unwrap().connections[0]
        .id
        .clone();
    let input = SaveAppSettingsInput {
        apply_storage_cleanup: None,
        settings,
        credentials: SettingsCredentialDrafts {
            model_keys: [(id, "synthetic-key".into())].into(),
            ..Default::default()
        },
    };
    storage.with_connection(|c| c.execute_batch("CREATE TRIGGER reject_registry BEFORE INSERT ON app_settings WHEN NEW.key='ai.modelRegistry' BEGIN SELECT RAISE(FAIL,'synthetic write failure'); END;")).unwrap();
    assert!(save_with_encryptor(&storage, input.clone(), |_| Ok(
        "{\"ciphertext\":\"synthetic-encrypted\"}".into()
    ))
    .is_err());
    assert!(storage
        .with_connection(storage::list_app_settings)
        .unwrap()
        .is_empty());
    storage
        .with_connection(|c| c.execute_batch("DROP TRIGGER reject_registry"))
        .unwrap();
    let saved = save_with_encryptor(&storage, input, |_| {
        Ok("{\"ciphertext\":\"synthetic-encrypted\"}".into())
    })
    .unwrap();
    let connection = &saved.ai.model_registry.as_ref().unwrap().connections[0];
    assert_eq!(
        saved.ai.recall.generation.api_key_ref,
        connection.api_key_ref
    );
    assert!(!saved.ai.recall.generation.enabled);
    let before = storage.with_connection(storage::list_app_settings).unwrap();
    import_model_registry(&storage).unwrap();
    import_model_registry(&storage).unwrap();
    assert_eq!(
        storage.with_connection(storage::list_app_settings).unwrap(),
        before
    );
}

#[test]
fn failed_model_key_encryption_preserves_all_saved_settings() {
    let storage = storage::test_storage("/tmp/synthetic-model-encryption".into());
    let saved = save_with_encryptor(
        &storage,
        SaveAppSettingsInput {
            apply_storage_cleanup: None,
            settings: configured(),
            credentials: Default::default(),
        },
        |_| unreachable!(),
    )
    .unwrap();
    let id = saved.ai.model_registry.as_ref().unwrap().connections[0]
        .id
        .clone();
    let before = storage.with_connection(storage::list_app_settings).unwrap();
    let result = save_with_encryptor(
        &storage,
        SaveAppSettingsInput {
            apply_storage_cleanup: None,
            settings: saved,
            credentials: SettingsCredentialDrafts {
                model_keys: [(id, "synthetic-key".into())].into(),
                ..Default::default()
            },
        },
        |_| Err(()),
    );
    assert!(result.is_err());
    assert_eq!(
        storage.with_connection(storage::list_app_settings).unwrap(),
        before
    );
}
