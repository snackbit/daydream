use super::AiVisionProviderSettings;

pub(crate) fn vision_ready(settings: &AiVisionProviderSettings) -> bool {
    settings.provider == "openai-compatible"
        && settings
            .base_url
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        && settings
            .model
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        && (settings.auth_mode == "none"
            || settings
                .api_key_ref
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn anonymous_profile_and_legacy_key_profiles_have_distinct_readiness() {
        let mut profile = crate::settings::default_settings().ai.vision;
        profile.provider = "openai-compatible".into();
        profile.base_url = Some("http://127.0.0.1/v1".into());
        profile.model = Some("synthetic".into());
        assert!(!vision_ready(&profile));
        profile.auth_mode = "none".into();
        assert!(vision_ready(&profile));
        profile.model = None;
        assert!(!vision_ready(&profile));
    }
}
