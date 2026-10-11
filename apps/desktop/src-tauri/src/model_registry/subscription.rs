//! Official ChatGPT OAuth. Credentials never cross IPC or select another account.
mod callback;
pub(crate) mod diagnostics;
mod http;
mod session;
mod sign_in;
pub(crate) use http::{bounded_json, http};
use serde::Serialize;
pub(crate) use session::{
    acquire, load_registration, prepare, save_registration, save_tokens, sign_out, state,
    Registration, Tokens,
};
pub(crate) use session::{cancel_all, configured};
pub(crate) use sign_in::sign_in;
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AccountState {
    pub id: String,
    pub label: String,
    pub status: String,
    pub models: Vec<AccountModel>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AccountModel {
    pub slug: String,
    pub display_name: String,
}
