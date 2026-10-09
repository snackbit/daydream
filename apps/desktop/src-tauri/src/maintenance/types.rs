use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum MaintenanceAction {
    ResetPreferences,
    ClearLibrary,
    FactoryReset,
}
impl MaintenanceAction {
    pub(crate) fn key(self) -> &'static str {
        match self {
            Self::ResetPreferences => "reset-preferences",
            Self::ClearLibrary => "clear-library",
            Self::FactoryReset => "factory-reset",
        }
    }
    pub(crate) fn confirmation(self) -> &'static str {
        match self {
            Self::ResetPreferences => "RESET PREFERENCES",
            Self::ClearLibrary => "CLEAR LIBRARY",
            Self::FactoryReset => "FACTORY RESET",
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MaintenancePreview {
    pub action: MaintenanceAction,
    pub token: String,
    pub recording_count: u64,
    pub library_row_count: u64,
    pub model_count: u64,
    pub file_count: u64,
    pub size_bytes: u64,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PreviewMaintenanceInput {
    pub action: MaintenanceAction,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExecuteMaintenanceInput {
    pub action: MaintenanceAction,
    pub preview_token: String,
    pub confirmation: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MaintenanceStatus {
    pub operation_id: String,
    pub action: MaintenanceAction,
    pub phase: String,
    pub error: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AcknowledgeMaintenanceThemeResetInput {
    pub operation_id: String,
}
