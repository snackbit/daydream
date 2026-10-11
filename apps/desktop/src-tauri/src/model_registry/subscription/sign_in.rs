use super::{
    callback::callback,
    session::{now, sign_in_owner},
    *,
};
use crate::storage::AppStorage;
use openidconnect::{
    core::{CoreClient, CoreIdToken, CoreProviderMetadata},
    ClientId, CsrfToken, IssuerUrl, Nonce, PkceCodeChallenge,
};
use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};
use tauri_plugin_opener::OpenerExt;

pub(crate) async fn sign_in(
    app: &tauri::AppHandle,
    storage: &AppStorage,
    id: Option<String>,
    cancelled: &AtomicBool,
) -> Result<AccountState, String> {
    diagnostics::stage("registration_load");
    let mut registration = if let Some(id) = id {
        load_registration(storage, &id)?
    } else {
        Registration {
            id: uuid::Uuid::now_v7().to_string(),
            host_id: format!("urn:uuid:{}", uuid::Uuid::new_v4()),
            client_id: None,
        }
    };
    if let Ok(id) = uuid::Uuid::parse_str(&registration.id) {
        tracing::Span::current().record("subscription_id", id.to_string());
    }
    save_registration(storage, &registration)?;
    diagnostics::stage("account_lock");
    let (_guard, account_cancel) =
        super::super::transport::cancellable(sign_in_owner(&registration.id), cancelled).await?;
    diagnostics::stage("callback_listener");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|_| "model_subscription_callback_failed")?;
    let redirect = format!(
        "http://127.0.0.1:{}/auth/callback",
        listener
            .local_addr()
            .map_err(|_| "model_subscription_callback_failed")?
            .port()
    );
    let state = CsrfToken::new_random();
    let nonce = Nonce::new_random();
    let (challenge, verifier) = PkceCodeChallenge::new_random_sha256();
    let client_id = registration
        .client_id
        .as_deref()
        .unwrap_or("dynamic_agent_client");
    let mut url = reqwest::Url::parse("https://auth.openai.com/api/accounts/authorize")
        .map_err(|_| "model_subscription_unavailable")?;
    url.query_pairs_mut().extend_pairs(&[
        ("client_id", client_id),
        ("ext_agent_host_id", &registration.host_id),
        ("response_type", "code"),
        ("redirect_uri", &redirect),
        (
            "scope",
            "openid profile email offline_access resource.invoke chatgpt.tokens.use.direct",
        ),
        ("resource", "https://api.openai.com/v1"),
        ("state", state.secret()),
        ("nonce", nonce.secret()),
        ("code_challenge", challenge.as_str()),
        ("code_challenge_method", "S256"),
    ]);
    if registration.client_id.is_none() {
        url.query_pairs_mut()
            .append_pair("agent_name_hint", "Daydream");
    }
    diagnostics::stage("browser_open");
    app.opener()
        .open_url(url.as_str(), None::<&str>)
        .map_err(|_| "model_subscription_browser_failed")?;
    let operation = async {
        diagnostics::stage("callback_wait");
        let (code, issued) = callback(&listener, state.secret(), &account_cancel).await?;
        diagnostics::stage("registration_validate");
        if registration
            .client_id
            .as_ref()
            .is_some_and(|id| issued.as_ref().is_some_and(|issued| issued != id))
        {
            return Err("model_subscription_registration_mismatch".into());
        }
        if registration.client_id.is_none() {
            registration.client_id = Some(issued.ok_or("model_subscription_registration_missing")?);
            save_registration(storage, &registration)?;
        }
        let client_id = registration
            .client_id
            .as_deref()
            .ok_or("model_subscription_registration_missing")?;
        diagnostics::stage("token_exchange");
        let response = http()?
            .post("https://auth.openai.com/api/accounts/oauth/token")
            .form(&[
                ("grant_type", "authorization_code"),
                ("client_id", client_id),
                ("code", code.as_str()),
                ("code_verifier", verifier.secret()),
                ("redirect_uri", redirect.as_str()),
                ("resource", "https://api.openai.com/v1"),
            ])
            .send()
            .await
            .map_err(|error| diagnostics::transport_error(&error))?;
        let value = bounded_json(response).await?;
        diagnostics::stage("scope_validate");
        let scopes = value["scope"]
            .as_str()
            .ok_or("model_subscription_permission_required")?;
        if !["resource.invoke", "chatgpt.tokens.use.direct"]
            .iter()
            .all(|scope| scopes.split_whitespace().any(|s| s == *scope))
        {
            return Err("model_subscription_permission_required".into());
        }
        // OIDC verifies signature, issuer, audience, expiration and the attempt nonce.
        diagnostics::stage("identity_discovery");
        let metadata = CoreProviderMetadata::discover_async(
            IssuerUrl::new("https://auth.openai.com".into())
                .map_err(|_| "model_subscription_identity_invalid")?,
            &http()?,
        )
        .await
        .map_err(|error| {
            let category = match error {
                openidconnect::DiscoveryError::Response(status, _, _) => {
                    tracing::warn!(
                        http_status = status.as_u16(),
                        "chatgpt_identity_discovery_response"
                    );
                    "response"
                }
                openidconnect::DiscoveryError::Request(_) => "transport",
                openidconnect::DiscoveryError::Parse(_) => "parse",
                openidconnect::DiscoveryError::Validation(_) => "metadata",
                _ => "discovery",
            };
            tracing::warn!(category, "chatgpt_identity_discovery_failed");
            "model_subscription_identity_invalid"
        })?;
        let client =
            CoreClient::from_provider_metadata(metadata, ClientId::new(client_id.into()), None);
        diagnostics::stage("identity_validate");
        let token: CoreIdToken = value["id_token"]
            .as_str()
            .ok_or("model_subscription_identity_invalid")?
            .parse()
            .map_err(|_| "model_subscription_identity_invalid")?;
        let claims = token
            .claims(&client.id_token_verifier(), &nonce)
            .map_err(|error| {
                let category = match error {
                    openidconnect::ClaimsVerificationError::Expired(_) => "expired",
                    openidconnect::ClaimsVerificationError::InvalidAudience(_) => "audience",
                    openidconnect::ClaimsVerificationError::InvalidIssuer(_) => "issuer",
                    openidconnect::ClaimsVerificationError::InvalidNonce(_) => "nonce",
                    openidconnect::ClaimsVerificationError::SignatureVerification(_) => "signature",
                    _ => "claims",
                };
                tracing::warn!(category, "chatgpt_identity_validation_failed");
                "model_subscription_identity_invalid"
            })?;
        let label = claims
            .email()
            .map(|email| email.as_str().to_owned())
            .unwrap_or_else(|| "ChatGPT".into());
        let tokens = Tokens {
            access_token: value["access_token"]
                .as_str()
                .filter(|v| !v.is_empty())
                .ok_or("model_subscription_identity_invalid")?
                .into(),
            refresh_token: value["refresh_token"]
                .as_str()
                .filter(|v| !v.is_empty())
                .ok_or("model_subscription_identity_invalid")?
                .into(),
            expires_at: now()
                + value["expires_in"]
                    .as_u64()
                    .ok_or("model_subscription_identity_invalid")?
                    .min(86400),
            label: label.clone(),
        };
        if cancelled.load(Ordering::Acquire) || account_cancel.load(Ordering::Acquire) {
            return Err("model_cancelled".into());
        }
        diagnostics::stage("credential_store");
        save_tokens(storage, &registration.id, &tokens)?;
        tracing::info!("chatgpt_credentials_saved");
        Ok(AccountState {
            id: registration.id.clone(),
            label,
            status: "connected".into(),
            models: vec![],
        })
    };
    tokio::pin!(operation);
    let deadline = tokio::time::sleep(Duration::from_secs(300));
    tokio::pin!(deadline);
    loop {
        tokio::select! {biased;
            _=&mut deadline=>return Err("model_subscription_sign_in_timeout".into()),
            _=tokio::time::sleep(Duration::from_millis(25))=>if cancelled.load(Ordering::Acquire)||account_cancel.load(Ordering::Acquire){return Err("model_cancelled".into());},
            result=&mut operation=>return result,
        }
    }
}
