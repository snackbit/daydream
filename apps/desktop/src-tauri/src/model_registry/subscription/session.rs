use super::diagnostics;
use super::{bounded_json, http};
use super::{AccountModel, AccountState};
use crate::{
    secret_crypto,
    storage::{self, AppStorage, SaveAppSettingInput},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, OnceLock,
    },
    time::{SystemTime, UNIX_EPOCH},
};
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Registration {
    pub id: String,
    pub host_id: String,
    pub client_id: Option<String>,
}
#[derive(Serialize, Deserialize)]
pub(crate) struct Tokens {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: u64,
    pub label: String,
}
struct Owner {
    gate: Arc<tokio::sync::Mutex<()>>,
    cancel: Arc<AtomicBool>,
}
static OWNERS: OnceLock<Mutex<HashMap<String, Arc<Owner>>>> = OnceLock::new();
fn owner(id: &str) -> Result<Arc<Owner>, String> {
    uuid::Uuid::parse_str(id).map_err(|_| "model_subscription_invalid")?;
    let mut owners = OWNERS
        .get_or_init(Default::default)
        .lock()
        .map_err(|_| "model_subscription_unavailable")?;
    Ok(owners
        .entry(id.into())
        .or_insert_with(|| {
            Arc::new(Owner {
                gate: Arc::new(tokio::sync::Mutex::new(())),
                cancel: Arc::new(AtomicBool::new(false)),
            })
        })
        .clone())
}
pub(crate) fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
fn read(storage: &AppStorage, key: &str) -> Result<Option<String>, String> {
    storage
        .with_connection(|c| storage::get_app_setting(c, key))
        .map(|row| row.map(|r| r.value_json))
        .map_err(|_| "model_subscription_storage_failed".into())
}
fn write(storage: &AppStorage, key: String, value: String, sensitive: bool) -> Result<(), String> {
    storage
        .with_connection(|c| {
            storage::save_app_setting(
                c,
                &SaveAppSettingInput {
                    key,
                    value_json: value,
                    is_sensitive: sensitive,
                    timestamp_ms: (now() * 1000) as i64,
                },
            )
        })
        .map(|_| ())
        .map_err(|_| "model_subscription_storage_failed".into())
}
pub(crate) fn load_registration(storage: &AppStorage, id: &str) -> Result<Registration, String> {
    owner(id)?;
    let json = read(storage, &format!("chatgpt.registration.{id}"))?
        .ok_or("model_subscription_missing")?;
    serde_json::from_str(&json).map_err(|_| "model_subscription_storage_failed".into())
}
pub(crate) fn save_registration(
    storage: &AppStorage,
    registration: &Registration,
) -> Result<(), String> {
    write(
        storage,
        format!("chatgpt.registration.{}", registration.id),
        serde_json::to_string(registration).map_err(|_| "model_subscription_storage_failed")?,
        false,
    )
}
pub(crate) fn prepare(storage: &AppStorage) -> Result<String, String> {
    let registration = Registration {
        id: uuid::Uuid::now_v7().to_string(),
        host_id: format!("urn:uuid:{}", uuid::Uuid::new_v4()),
        client_id: None,
    };
    save_registration(storage, &registration)?;
    tracing::info!(subscription_id = %registration.id, "chatgpt_registration_prepared");
    Ok(registration.id)
}
fn tokens(storage: &AppStorage, id: &str) -> Result<Tokens, String> {
    let json =
        read(storage, &format!("chatgpt.tokens.{id}"))?.ok_or("model_subscription_reconnect")?;
    let json =
        secret_crypto::decrypt_sensitive_setting_with_platform_key(&json).map_err(|error| {
            tracing::warn!(error_code = %error.code, "chatgpt_credential_decryption_failed");
            "model_subscription_credential_unavailable"
        })?;
    serde_json::from_str(&json).map_err(|_| "model_subscription_storage_failed".into())
}
pub(crate) fn save_tokens(storage: &AppStorage, id: &str, tokens: &Tokens) -> Result<(), String> {
    let json = serde_json::to_string(tokens).map_err(|_| "model_subscription_storage_failed")?;
    let encrypted =
        secret_crypto::encrypt_sensitive_setting_with_platform_key(&json).map_err(|error| {
            tracing::warn!(error_code = %error.code, "chatgpt_credential_encryption_failed");
            "model_subscription_credential_unavailable"
        })?;
    write(storage, format!("chatgpt.tokens.{id}"), encrypted, true)
}
pub(crate) struct Lease {
    pub key: String,
    pub cancelled: Arc<AtomicBool>,
    _guard: tokio::sync::OwnedMutexGuard<()>,
}
pub(crate) async fn acquire(storage: &AppStorage, id: &str) -> Result<Lease, String> {
    let owner = owner(id)?;
    // Closed account admission is distinct from cancelling an existing request.
    // Keep its cancellation flag set so signed-out credentials cannot be reused.
    if owner.cancel.load(Ordering::Acquire) {
        return Err("model_subscription_reconnect".into());
    }
    super::super::transport::cancellable(acquire_owned(storage, id, &owner), &owner.cancel).await
}
async fn acquire_owned(
    storage: &AppStorage,
    id: &str,
    owner: &Arc<Owner>,
) -> Result<Lease, String> {
    diagnostics::stage("credential_read");
    let guard = owner.gate.clone().lock_owned().await;
    if owner.cancel.load(Ordering::Acquire) {
        return Err("model_subscription_reconnect".into());
    }
    let mut tokens = tokens(storage, id)?;
    if tokens.expires_at <= now() + 60 {
        diagnostics::stage("token_refresh");
        let registration = load_registration(storage, id)?;
        let client_id = registration
            .client_id
            .ok_or("model_subscription_reconnect")?;
        let response = http()?
            .post("https://auth.openai.com/api/accounts/oauth/token")
            .form(&[
                ("grant_type", "refresh_token"),
                ("client_id", client_id.as_str()),
                ("refresh_token", tokens.refresh_token.as_str()),
                ("resource", "https://api.openai.com/v1"),
            ])
            .send()
            .await
            .map_err(|error| diagnostics::transport_error(&error))?;
        let value = bounded_json(response).await?;
        tokens.access_token = value["access_token"]
            .as_str()
            .filter(|v| !v.is_empty())
            .ok_or("model_subscription_reconnect")?
            .into();
        if let Some(refresh) = value["refresh_token"].as_str() {
            tokens.refresh_token = refresh.into();
        }
        tokens.expires_at = now()
            + value["expires_in"]
                .as_u64()
                .ok_or("model_subscription_reconnect")?
                .min(86400);
        diagnostics::stage("refreshed_credential_store");
        save_tokens(storage, id, &tokens)?;
        tracing::info!("chatgpt_token_refresh_succeeded");
    }
    Ok(Lease {
        key: tokens.access_token,
        cancelled: owner.cancel.clone(),
        _guard: guard,
    })
}
pub(crate) async fn state(storage: &AppStorage, id: &str) -> Result<AccountState, String> {
    let label = tokens(storage, id)
        .map(|t| t.label)
        .unwrap_or_else(|_| "ChatGPT".into());
    let lease = match acquire(storage, id).await {
        Ok(lease) => lease,
        Err(code) => {
            tracing::warn!(error_code = %code, "chatgpt_credentials_unavailable");
            let status = match code.as_str() {
                "model_subscription_reconnect" | "model_subscription_credential_unavailable" => {
                    "reconnect-required"
                }
                "model_subscription_missing" => "signed-out",
                "model_subscription_permission_required" => "permission-required",
                _ => return Err(code),
            };
            return Ok(AccountState {
                id: id.into(),
                label,
                status: status.into(),
                models: vec![],
            });
        }
    };
    diagnostics::stage("model_list");
    let value = bounded_json(
        http()?
            .get("https://api.openai.com/v1/models")
            .bearer_auth(&lease.key)
            .send()
            .await
            .map_err(|error| diagnostics::transport_error(&error))?,
    )
    .await?;
    let models = value["models"]
        .as_array()
        .ok_or("model_response_invalid")?
        .iter()
        .filter(|m| m["visibility"] == "list")
        .map(|m| {
            Ok(AccountModel {
                slug: m["slug"].as_str().ok_or("model_response_invalid")?.into(),
                display_name: m["display_name"]
                    .as_str()
                    .ok_or("model_response_invalid")?
                    .into(),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    tracing::info!(model_count = models.len(), "chatgpt_models_loaded");
    Ok(AccountState {
        id: id.into(),
        label,
        status: "connected".into(),
        models,
    })
}
pub(crate) fn configured(storage: &AppStorage, id: &str) -> bool {
    tokens(storage, id).is_ok()
}
pub(crate) fn cancel_all() {
    if let Ok(owners) = OWNERS.get_or_init(Default::default).lock() {
        for owner in owners.values() {
            owner.cancel.store(true, Ordering::Release);
        }
    }
}
pub(crate) async fn sign_out(storage: &AppStorage, id: &str) -> Result<bool, String> {
    diagnostics::stage("sign_out_drain");
    let owner = owner(id)?;
    owner.cancel.store(true, Ordering::Release);
    let _guard = owner.gate.clone().lock_owned().await;
    let had_tokens = read(storage, &format!("chatgpt.tokens.{id}"))?.is_some();
    let old = tokens(storage, id).ok();
    let registration = load_registration(storage, id)?;
    diagnostics::stage("credential_delete");
    storage
        .with_connection(|c| storage::delete_app_setting(c, &format!("chatgpt.tokens.{id}")))
        .map_err(|_| "model_subscription_storage_failed")?;
    if let (Some(tokens), Some(client_id)) = (old, registration.client_id) {
        diagnostics::stage("token_revocation");
        let revoke = async {
            let discovery = bounded_json(
                http()?
                    .get("https://auth.openai.com/.well-known/openid-configuration")
                    .send()
                    .await
                    .map_err(|error| diagnostics::transport_error(&error))?,
            )
            .await?;
            let endpoint = discovery["revocation_endpoint"]
                .as_str()
                .ok_or("model_subscription_revoke_failed")?;
            let url =
                reqwest::Url::parse(endpoint).map_err(|_| "model_subscription_revoke_failed")?;
            if url.scheme() != "https" || url.host_str() != Some("auth.openai.com") {
                return Err("model_subscription_revoke_failed".into());
            }
            Ok::<_, String>(
                http()?
                    .post(url)
                    .form(&[
                        ("token", tokens.refresh_token),
                        ("token_type_hint", "refresh_token".into()),
                        ("client_id", client_id),
                    ])
                    .send()
                    .await
                    .map(|r| {
                        tracing::info!(
                            http_status = r.status().as_u16(),
                            "chatgpt_revocation_response"
                        );
                        r.status().is_success()
                    })
                    .map_err(|error| diagnostics::transport_error(&error))?,
            )
        };
        let revoked = match revoke.await {
            Ok(revoked) => revoked,
            Err(code) => {
                tracing::warn!(error_code = %code, "chatgpt_revocation_failed");
                false
            }
        };
        tracing::info!(revoked, "chatgpt_signed_out_locally");
        return Ok(revoked);
    }
    tracing::info!("chatgpt_signed_out_locally");
    Ok(!had_tokens)
}
pub(super) async fn sign_in_owner(
    id: &str,
) -> Result<(tokio::sync::OwnedMutexGuard<()>, Arc<AtomicBool>), String> {
    let owner = owner(id)?;
    let guard = owner.gate.clone().lock_owned().await;
    owner.cancel.store(false, Ordering::Release);
    Ok((guard, owner.cancel.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn cancelled_waiter_never_reads_credentials_and_sign_out_drains_inflight_owner() {
        let storage = storage::test_storage("/tmp/synthetic-subscription-ownership".into());
        let id = prepare(&storage).unwrap();
        let owner = owner(&id).unwrap();
        let guard = owner.gate.clone().lock_owned().await;
        let cancelled = AtomicBool::new(true);
        assert_eq!(
            super::super::super::transport::cancellable(acquire(&storage, &id), &cancelled)
                .await
                .err()
                .as_deref(),
            Some("model_cancelled")
        );
        let lease = Lease {
            key: "synthetic-key".into(),
            cancelled: owner.cancel.clone(),
            _guard: guard,
        };
        let running = tokio::spawn(async move {
            let result = super::super::super::transport::cancellable(
                std::future::pending::<Result<(), String>>(),
                &lease.cancelled,
            )
            .await;
            drop(lease);
            result
        });
        assert!(sign_out(&storage, &id).await.unwrap());
        assert_eq!(running.await.unwrap().unwrap_err(), "model_cancelled");
        assert!(owner.cancel.load(Ordering::Acquire));
        for _ in 0..2 {
            assert_eq!(
                acquire(&storage, &id).await.err().as_deref(),
                Some("model_subscription_reconnect")
            );
            let account = state(&storage, &id).await.unwrap();
            assert_eq!(account.status, "reconnect-required");
            assert!(account.models.is_empty());
        }
        let (guard, cancelled) = sign_in_owner(&id).await.unwrap();
        assert!(!cancelled.load(Ordering::Acquire));
        drop(guard);
        assert_eq!(
            acquire(&storage, &id).await.err().as_deref(),
            Some("model_subscription_reconnect")
        );
    }
}
