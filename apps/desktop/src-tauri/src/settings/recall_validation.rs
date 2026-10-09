use super::types::{AppSettingsError, RecallSettings};
pub(super) fn validate_recall_settings(settings: &RecallSettings) -> Result<(), AppSettingsError> {
    let invalid = |field: &str| AppSettingsError {
        code: "settings_invalid_value".into(),
        field: Some(format!("ai.recall.{field}")),
        message: "Recall provider setting is invalid.".into(),
    };
    let embedding = &settings.embedding;
    if !(1..=8192).contains(&embedding.dimensions) {
        return Err(invalid("embedding.dimensions"));
    }
    if !["e5", "bge", "plain"].contains(&embedding.convention.as_str()) {
        return Err(invalid("embedding.convention"));
    }
    if !["local", "openai-compatible"].contains(&embedding.provider.as_str()) {
        return Err(invalid("embedding.provider"));
    }
    if !["cpu", "auto", "vulkan"].contains(&embedding.local_backend.as_str()) {
        return Err(invalid("embedding.localBackend"));
    }
    if !["multilingual-e5-small-q8_0", "bge-small-en-v1.5-q8_0"]
        .contains(&embedding.local_model_id.as_str())
    {
        return Err(invalid("embedding.localModelId"));
    }
    for (role, auth, url, model, key) in [
        (
            "embedding",
            &embedding.auth_mode,
            &embedding.base_url,
            &embedding.model,
            &embedding.api_key_ref,
        ),
        (
            "generation",
            &settings.generation.auth_mode,
            &settings.generation.base_url,
            &settings.generation.model,
            &settings.generation.api_key_ref,
        ),
    ] {
        if !["api-key", "none"].contains(&auth.as_str()) {
            return Err(invalid(&format!("{role}.authMode")));
        }
        // Incomplete profiles are saveable; readiness owns required values.
        if let Some(url) = url.as_ref().filter(|url| !url.trim().is_empty()) {
            let parsed =
                reqwest::Url::parse(url).map_err(|_| invalid(&format!("{role}.baseUrl")))?;
            if !["http", "https"].contains(&parsed.scheme())
                || parsed.host_str().is_none()
                || !parsed.username().is_empty()
                || parsed.password().is_some()
                || parsed.query().is_some()
                || parsed.fragment().is_some()
            {
                return Err(invalid(&format!("{role}.baseUrl")));
            }
        }
        if model
            .as_ref()
            .is_some_and(|model| model.len() > 256 || model.chars().any(char::is_control))
        {
            return Err(invalid(&format!("{role}.model")));
        }
        if key
            .as_ref()
            .is_some_and(|key| !key.starts_with("ai.secret.") || key.len() > 256)
        {
            return Err(invalid(&format!("{role}.apiKeyRef")));
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn incomplete_profiles_remain_saveable_and_defaults_are_local() {
        let mut settings = RecallSettings::default();
        assert_eq!(settings.embedding.provider, "local");
        assert!(!settings.embedding.enabled && !settings.generation.enabled);
        settings.generation.enabled = true;
        assert!(validate_recall_settings(&settings).is_ok());
        settings.generation.base_url = Some("https://user:password@example.invalid/v1".into());
        assert!(validate_recall_settings(&settings).is_err());
    }
}
