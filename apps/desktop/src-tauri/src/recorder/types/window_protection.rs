use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ProtectableWindowProviderStatus {
    Available,
    PermissionDenied,
    ProviderUnavailable,
    SessionUnsupported,
    UnsupportedPlatform,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProtectableWindow {
    pub app_identity: Option<String>,
    pub id: String,
    pub platform: String,
    pub process_id: Option<u32>,
    pub process_name: Option<String>,
    pub title: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProtectableWindowList {
    pub provider: Option<String>,
    pub status: ProtectableWindowProviderStatus,
    pub windows: Vec<ProtectableWindow>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WindowProtectionStatus {
    pub message: String,
    pub platform: String,
    pub provider: Option<String>,
    pub status: ProtectableWindowProviderStatus,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct WindowMetadataSnapshot {
    pub focused_window: Option<ProtectableWindow>,
    pub provider: Option<String>,
    pub status: ProtectableWindowProviderStatus,
    pub windows: Vec<ProtectableWindow>,
}
