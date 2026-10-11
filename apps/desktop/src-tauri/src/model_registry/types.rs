use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Capability {
    Text,
    Vision,
    Embedding,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Protocol {
    Responses,
    #[default]
    ChatCompletions,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum TokenLimit {
    Modern,
    #[default]
    Legacy,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Connection {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub auth_mode: String,
    pub base_url: String,
    pub api_key_ref: Option<String>,
    pub subscription_id: Option<String>,
    pub protocol: Protocol,
    pub token_limit: TokenLimit,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Model {
    pub id: String,
    pub name: String,
    pub model: String,
    pub connection_id: String,
    pub capabilities: Vec<Capability>,
    pub embedding_dimensions: Option<u32>,
    pub embedding_convention: String,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Bindings {
    pub embedding: Option<String>,
    pub summary: Option<String>,
    pub vision: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Registry {
    pub version: u32,
    pub connections: Vec<Connection>,
    pub models: Vec<Model>,
    pub bindings: Bindings,
}
impl Default for Registry {
    fn default() -> Self {
        Self {
            version: 1,
            connections: vec![],
            models: vec![],
            bindings: Bindings::default(),
        }
    }
}
/// Resolved wire policy pinned alongside historical feature settings; no tokens.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Route {
    pub protocol: Protocol,
    pub token_limit: TokenLimit,
    pub subscription_id: Option<String>,
    #[serde(default)]
    pub subscription: bool,
}
impl From<&Connection> for Route {
    fn from(value: &Connection) -> Self {
        Self {
            protocol: value.protocol.clone(),
            token_limit: value.token_limit.clone(),
            subscription_id: value.subscription_id.clone(),
            subscription: value.auth_mode == "chatgpt",
        }
    }
}
