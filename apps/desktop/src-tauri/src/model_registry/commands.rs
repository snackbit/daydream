//! Thin IPC handlers for synthetic tests and account actions.
use super::{subscription, transport, types::*};
use crate::storage::{self, AppStorage};
use serde::{Deserialize, Serialize};
mod probe;
use probe::probe;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, OnceLock,
    },
    time::Instant,
};
use tracing::Instrument;

struct RequestState {
    cancelled: Arc<AtomicBool>,
    finished: AtomicBool,
}
static REQUESTS: OnceLock<Mutex<HashMap<String, Arc<RequestState>>>> = OnceLock::new();
struct Request {
    id: String,
    cancelled: Arc<AtomicBool>,
    state: Arc<RequestState>,
}
impl Request {
    fn new(id: String) -> Result<Self, String> {
        uuid::Uuid::parse_str(&id).map_err(|_| "model_request_invalid")?;
        let mut requests = REQUESTS
            .get_or_init(Default::default)
            .lock()
            .map_err(|_| "model_request_failed")?;
        if requests.len() >= 16 || requests.contains_key(&id) {
            return Err("model_request_busy".into());
        }
        let cancelled = Arc::new(AtomicBool::new(false));
        let state = Arc::new(RequestState {
            cancelled: cancelled.clone(),
            finished: AtomicBool::new(false),
        });
        requests.insert(id.clone(), state.clone());
        Ok(Self {
            id,
            cancelled,
            state,
        })
    }
}
impl Drop for Request {
    fn drop(&mut self) {
        self.state.finished.store(true, Ordering::Release);
        if let Ok(mut requests) = REQUESTS.get_or_init(Default::default).lock() {
            requests.remove(&self.id);
        }
    }
}
async fn activity<T, F: std::future::Future<Output = Result<T, String>>>(
    app: &tauri::AppHandle,
    future: F,
) -> Result<T, String> {
    let lease = crate::maintenance::runtime::admit(app, "model-connection")
        .map_err(|_| "model_maintenance_active")?;
    tokio::pin!(future);
    loop {
        tokio::select! {biased;
            _=tokio::time::sleep(std::time::Duration::from_millis(25))=>if lease.as_ref().is_some_and(|lease|lease.cancellation().is_cancelled()){return Err("model_cancelled".into());},
            result=&mut future=>return result,
        }
    }
}
#[tauri::command]
pub(crate) async fn cancel_model_request(request_id: String) -> Result<(), String> {
    let request = REQUESTS
        .get_or_init(Default::default)
        .lock()
        .map_err(|_| "model_request_failed")?
        .get(&request_id)
        .cloned();
    if let Some(request) = request {
        tracing::info!(request_id = %request_id, "model_request_cancel_requested");
        request.cancelled.store(true, Ordering::Release);
        while !request.finished.load(Ordering::Acquire) {
            tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        }
        tracing::info!(request_id = %request_id, "model_request_cancel_drained");
    }
    Ok(())
}
#[tauri::command]
pub(crate) async fn prepare_chatgpt_connection(
    app: tauri::AppHandle,
    storage: tauri::State<'_, AppStorage>,
) -> Result<String, String> {
    subscription::diagnostics::operation(
        "prepare",
        None,
        None,
        activity(&app, async { subscription::prepare(&storage) }),
    )
    .await
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct TestInput {
    connection: Connection,
    model: Model,
    key_draft: Option<String>,
    request_id: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TestResult {
    fingerprint: String,
    checks: Vec<Check>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Check {
    capability: Capability,
    status: String,
    message: String,
    error_code: Option<String>,
    elapsed_ms: u64,
    dimensions: Option<u32>,
}
pub(crate) fn key(
    storage: &AppStorage,
    connection: &Connection,
    draft: Option<&str>,
) -> Result<Option<String>, String> {
    if connection.auth_mode == "none" {
        return Ok(None);
    }
    if connection.auth_mode == "chatgpt" {
        return Ok(None);
    }
    if let Some(draft) = draft.filter(|key| !key.trim().is_empty()) {
        return Ok(Some(draft.trim().into()));
    }
    let reference = connection.api_key_ref.as_ref().ok_or("model_key_missing")?;
    let row = storage
        .with_connection(|c| storage::get_app_setting(c, reference))
        .map_err(|_| "model_credential_unavailable")?
        .ok_or("model_key_missing")?;
    crate::secret_crypto::decrypt_sensitive_setting_with_platform_key(&row.value_json)
        .map(Some)
        .map_err(|_| "model_credential_unavailable".into())
}
pub(crate) fn friendly(code: &str) -> &'static str {
    match code{
    "model_key_missing"=>"Add an API key before testing.",
    "model_auth_failed"=>"The connection rejected authentication or account access. Review the connection and model access.",
    "model_not_found"=>"The provider could not find this model. Check the model identifier and account access.",
    "model_server_failed"=>"The provider encountered a server error. Check Diagnostics and try again later.",
    "model_credential_unavailable"|"model_subscription_credential_unavailable"=>"Could not access local credentials. Unlock your system keyring and try again.",
    "model_subscription_reconnect"|"model_subscription_missing"=>"Sign in with ChatGPT again for this connection.",
    "model_subscription_permission_required"=>"Sign in again and allow direct model access.",
    "model_subscription_usage_limit"|"model_rate_limited"=>"The account's usage limit was reached. Check usage and try again later.",
    "model_subscription_usage_unavailable"=>"Subscription model access is unavailable. Check your ChatGPT plan and connected-app access.",
    "model_timeout"=>"The provider did not respond in time. Try again.",
    "model_connection_failed"=>"Could not reach the provider. Check the API URL and connection.",
    "model_request_invalid"=>"The provider rejected the request. Check the model identifier and API protocol.",
    "model_cancelled"=>"Test cancelled.",
    "model_response_invalid"=>"The provider's response could not be validated. Check Diagnostics for the failing stage.",
    "model_response_incomplete"=>"The provider stopped before completing the response. Check Diagnostics and try again.",
    "model_response_limit"=>"The provider's response exceeded the size limit. Check Diagnostics.",
    "model_subscription_request_failed"=>"ChatGPT could not complete the request. Check Diagnostics and try again.",
    "model_subscription_ineligible"=>"ChatGPT plan usage is unavailable for this account or workspace. Check Diagnostics.",
    "model_capability_unsupported"=>"ChatGPT rejected a model capability or request parameter. Check Diagnostics.",
    "model_subscription_route_unsupported"=>"ChatGPT rejected this API route. Check Diagnostics.",
    _=>"The provider did not return a complete valid result. Check that the model supports this capability.",
}
}
#[tauri::command]
pub(crate) async fn test_configured_model(
    app: tauri::AppHandle,
    storage: tauri::State<'_, AppStorage>,
    input: TestInput,
) -> Result<TestResult, String> {
    let request_id = input.request_id.clone();
    subscription::diagnostics::operation(
        "model_test",
        None,
        Some(&request_id),
        activity(&app, test_model(&storage, input)),
    )
    .await
}
async fn test_model(storage: &AppStorage, input: TestInput) -> Result<TestResult, String> {
    let request = Request::new(input.request_id)?;
    let registry = Registry {
        version: 1,
        connections: vec![input.connection.clone()],
        models: vec![input.model.clone()],
        bindings: Bindings::default(),
    };
    super::validate(&registry).map_err(|_| "model_configuration_invalid")?;
    if input.model.model.trim().is_empty() {
        return Err("model_identifier_missing".into());
    }
    let fingerprint = hex::encode(Sha256::digest(
        serde_json::to_vec(&registry).map_err(|_| "model_configuration_invalid")?,
    ));
    let mut checks = Vec::new();
    for capability in &input.model.capabilities {
        let started = Instant::now();
        let result = probe(
            storage,
            &input.connection,
            &input.model,
            capability,
            input.key_draft.as_deref(),
            &request.cancelled,
        )
        .instrument(tracing::info_span!("model_test", request_id = %request.id,
            capability = ?capability, stage = tracing::field::Empty))
        .await;
        if let Err(code) = &result {
            tracing::warn!(request_id = %request.id, capability = ?capability, error_code = %code,
                elapsed_ms = started.elapsed().as_millis() as u64, "model_test_failed");
        } else {
            tracing::info!(request_id = %request.id, capability = ?capability,
                elapsed_ms = started.elapsed().as_millis() as u64, "model_test_succeeded");
        }
        checks.push(Check {
            capability: capability.clone(),
            status: if result.is_ok() {
                "succeeded"
            } else {
                "failed"
            }
            .into(),
            message: result
                .as_ref()
                .err()
                .map(|code| friendly(code))
                .unwrap_or("Synthetic test passed.")
                .into(),
            error_code: result.as_ref().err().cloned(),
            elapsed_ms: started.elapsed().as_millis() as u64,
            dimensions: result.ok().flatten(),
        });
        if request.cancelled.load(Ordering::Acquire) {
            break;
        }
    }
    Ok(TestResult {
        fingerprint,
        checks,
    })
}
#[tauri::command]
pub(crate) async fn sign_in_chatgpt(
    app: tauri::AppHandle,
    storage: tauri::State<'_, AppStorage>,
    subscription_id: Option<String>,
    request_id: String,
) -> Result<subscription::AccountState, String> {
    let request = Request::new(request_id)?;
    subscription::diagnostics::operation(
        "sign_in",
        subscription_id.as_deref(),
        Some(&request.id),
        activity(
            &app,
            transport::cancellable(
                async {
                    let state = subscription::sign_in(
                        &app,
                        &storage,
                        subscription_id.clone(),
                        &request.cancelled,
                    )
                    .await?;
                    subscription::state(&storage, &state.id).await
                },
                &request.cancelled,
            ),
        ),
    )
    .await
}
#[tauri::command]
pub(crate) async fn get_chatgpt_connection(
    app: tauri::AppHandle,
    storage: tauri::State<'_, AppStorage>,
    subscription_id: String,
) -> Result<subscription::AccountState, String> {
    subscription::diagnostics::operation(
        "account_state",
        Some(&subscription_id),
        None,
        activity(&app, subscription::state(&storage, &subscription_id)),
    )
    .await
}
#[tauri::command]
pub(crate) async fn sign_out_chatgpt(
    app: tauri::AppHandle,
    storage: tauri::State<'_, AppStorage>,
    subscription_id: String,
) -> Result<bool, String> {
    subscription::diagnostics::operation(
        "sign_out",
        Some(&subscription_id),
        None,
        activity(&app, subscription::sign_out(&storage, &subscription_id)),
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn cancellation_acknowledges_only_after_the_request_owner_exits() {
        let request = Request::new(uuid::Uuid::now_v7().to_string()).unwrap();
        let id = request.id.clone();
        let state = request.state.clone();
        let cancellation = tokio::spawn(cancel_model_request(id));
        tokio::time::sleep(std::time::Duration::from_millis(60)).await;
        assert!(state.cancelled.load(Ordering::Acquire));
        assert!(!cancellation.is_finished());
        drop(request);
        cancellation.await.unwrap().unwrap();
    }
}
