use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecallExternalProfile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_route: Option<crate::model_registry::types::Route>,
    pub enabled: bool,
    pub base_url: Option<String>,
    pub model: Option<String>,
    pub auth_mode: String,
    pub api_key_ref: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecallEmbeddingSettings {
    #[serde(default = "default_embedding_dimensions")]
    pub dimensions: u32,
    #[serde(default = "default_embedding_convention")]
    pub convention: String,
    pub provider: String,
    pub local_model_id: String,
    pub local_backend: String,
    pub enabled: bool,
    pub base_url: Option<String>,
    pub model: Option<String>,
    pub auth_mode: String,
    pub api_key_ref: Option<String>,
}
fn default_embedding_dimensions() -> u32 {
    384
}
fn default_embedding_convention() -> String {
    "plain".into()
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecallSettings {
    pub embedding: RecallEmbeddingSettings,
    pub generation: RecallExternalProfile,
    pub auto_generate: bool,
}
impl Default for RecallSettings {
    fn default() -> Self {
        Self {
            embedding: RecallEmbeddingSettings {
                dimensions: 384,
                convention: "plain".into(),
                provider: "local".into(),
                local_model_id: "multilingual-e5-small-q8_0".into(),
                local_backend: "cpu".into(),
                enabled: false,
                base_url: None,
                model: None,
                auth_mode: "api-key".into(),
                api_key_ref: None,
            },
            generation: RecallExternalProfile {
                model_route: None,
                enabled: false,
                base_url: None,
                model: None,
                auth_mode: "api-key".into(),
                api_key_ref: None,
            },
            auto_generate: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn historical_settings_without_recall_restore_local_defaults() {
        let original = crate::settings::default_settings();
        let mut value = serde_json::to_value(&original).unwrap();
        value["ai"].as_object_mut().unwrap().remove("recall");
        let decoded: crate::settings::AppSettings = serde_json::from_value(value).unwrap();
        assert_eq!(decoded.ai.recall, RecallSettings::default());
        assert_eq!(decoded.record, original.record);
    }
}
