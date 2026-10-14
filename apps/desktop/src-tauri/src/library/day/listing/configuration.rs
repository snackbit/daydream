use crate::{settings, storage};
use sha2::{Digest, Sha256};

/// Whether summary setup is present, without decrypting/testing credentials.
pub(super) fn summary_profile(connection: &rusqlite::Connection) -> Result<String, String> {
    let settings = storage::get_app_setting(connection, "ai.recall")
        .map_err(|_| "recall_settings_unavailable")?
        .map(|row| serde_json::from_str::<settings::RecallSettings>(&row.value_json))
        .transpose()
        .map_err(|_| "recall_settings_unavailable")?
        .unwrap_or_default();
    let profile = settings.generation;
    if !profile.enabled {
        return Err("recall_generation_disabled".into());
    }
    if profile.base_url.is_none() || profile.model.is_none() {
        return Err("recall_profile_invalid".into());
    }
    let subscription = profile
        .model_route
        .as_ref()
        .filter(|route| route.subscription);
    let reference = if let Some(route) = subscription {
        format!(
            "chatgpt.tokens.{}",
            route
                .subscription_id
                .as_deref()
                .ok_or("recall_provider_subscription_reconnect")?
        )
    } else if profile.auth_mode == "none" {
        return fingerprint(&profile);
    } else if profile.auth_mode == "api-key" {
        profile.api_key_ref.clone().ok_or("recall_key_missing")?
    } else {
        return Err("recall_profile_invalid".into());
    };
    let present = storage::get_app_setting(connection, &reference)
        .map_err(|_| "recall_storage_failed")?
        .is_some();
    if !present {
        return Err(if subscription.is_some() {
            "recall_provider_subscription_reconnect"
        } else {
            "recall_key_missing"
        }
        .into());
    }
    fingerprint(&profile)
}

fn fingerprint(profile: &settings::RecallExternalProfile) -> Result<String, String> {
    Ok(hex::encode(Sha256::digest(
        serde_json::to_vec(profile).map_err(|_| "recall_profile_invalid")?,
    )))
}
