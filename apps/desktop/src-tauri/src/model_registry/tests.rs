use super::{types::*, *};

#[test]
fn legacy_import_preserves_consent_and_keys_without_duplicate_imports() {
    let mut settings = crate::settings::default_settings();
    settings.ai.recall.generation.base_url = Some("https://api.openai.com/v1".into());
    settings.ai.recall.generation.model = Some("synthetic-text".into());
    settings.ai.recall.generation.api_key_ref = Some("ai.secret.synthetic".into());
    import_legacy(&mut settings);
    let registry = settings.ai.model_registry.clone().unwrap();
    assert_eq!(registry.models.len(), 1);
    assert!(!settings.ai.recall.generation.enabled);
    assert_eq!(
        registry.connections[0].api_key_ref.as_deref(),
        Some("ai.secret.synthetic")
    );
    import_legacy(&mut settings);
    assert_eq!(settings.ai.model_registry.as_ref(), Some(&registry));
    project(&mut settings).unwrap();
    assert!(!settings.ai.recall.generation.enabled);
    assert!(!settings.ai.vision.observations_enabled);
}

#[test]
fn bindings_require_correct_capabilities_and_subscription_embeddings_are_rejected() {
    let mut settings = crate::settings::default_settings();
    settings.ai.recall.generation.model = Some("synthetic".into());
    import_legacy(&mut settings);
    let mut registry = settings.ai.model_registry.unwrap();
    registry.bindings.vision = registry.bindings.summary.clone();
    assert!(validate(&registry).is_err());
    registry.models[0].capabilities.push(Capability::Vision);
    assert!(validate(&registry).is_ok());
    registry.models[0].capabilities.push(Capability::Embedding);
    registry.connections[0].auth_mode = "chatgpt".into();
    registry.connections[0].provider = "openai".into();
    registry.connections[0].base_url = "https://api.openai.com/v1".into();
    registry.connections[0].protocol = Protocol::Responses;
    assert!(validate(&registry).is_err());
}
#[test]
fn legacy_import_preserves_effective_endpoints_and_new_connections_use_their_exact_base() {
    let mut settings = crate::settings::default_settings();
    settings.ai.recall.generation.base_url = Some("https://example.invalid/custom".into());
    settings.ai.recall.generation.model = Some("synthetic".into());
    settings.ai.vision.base_url = Some("https://example.invalid/special/chat/completions".into());
    settings.ai.vision.model = Some("synthetic".into());
    import_legacy(&mut settings);
    let registry = settings.ai.model_registry.as_mut().unwrap();
    assert_eq!(
        registry.connections[0].base_url,
        "https://example.invalid/custom/v1"
    );
    assert_eq!(
        registry.connections[1].base_url,
        "https://example.invalid/special"
    );
    registry.connections[0].base_url = "https://example.invalid/custom-api".into();
    project(&mut settings).unwrap();
    assert_eq!(
        settings.ai.recall.generation.base_url.as_deref(),
        Some("https://example.invalid/custom-api")
    );
}
