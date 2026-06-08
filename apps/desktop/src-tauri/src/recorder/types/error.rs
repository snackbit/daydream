use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum RecorderErrorCategory {
    Audio,
    Dependency,
    Encoder,
    Filesystem,
    Lifecycle,
    Permission,
    Unavailable,
    Video,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecorderError {
    pub(crate) category: RecorderErrorCategory,
    pub(crate) code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) details: Option<BTreeMap<String, serde_json::Value>>,
    pub(crate) message: String,
}
