use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::{
    settings,
    storage::AppStorage,
    transcription::{TranscriptionError, TranscriptionProviderManager},
};

pub const APP_STARTUP_STATUS_CHANGED_EVENT: &str = "app-startup-status-changed";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppStartupStatus {
    pub active_stage_id: Option<String>,
    pub error: Option<AppStartupIssue>,
    pub phase: String,
    pub stages: Vec<AppStartupStage>,
    pub warnings: Vec<AppStartupIssue>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppStartupStage {
    pub description: String,
    pub id: String,
    pub label: String,
    pub status: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppStartupIssue {
    pub code: String,
    pub message: String,
    pub stage_id: String,
}

#[derive(Clone, Default)]
pub struct AppStartupManager {
    inner: Arc<Mutex<AppStartupState>>,
}

#[derive(Clone)]
struct AppStartupState {
    running: bool,
    status: AppStartupStatus,
}

impl Default for AppStartupState {
    fn default() -> Self {
        Self {
            running: false,
            status: initial_status(),
        }
    }
}

impl AppStartupManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn status(&self) -> AppStartupStatus {
        self.inner.lock().expect("startup state").status.clone()
    }

    fn begin(&self) -> bool {
        let mut state = self.inner.lock().expect("startup state");
        if state.running {
            return false;
        }
        state.running = true;
        state.status = initial_status();
        true
    }

    fn finish(&self) {
        self.inner.lock().expect("startup state").running = false;
    }

    fn set_ready(&self, app: &AppHandle) {
        self.update(app, |status| {
            status.active_stage_id = None;
            status.error = None;
            status.phase = "ready".to_string();
        });
    }

    fn set_stage(&self, app: &AppHandle, stage_id: &str, stage_status: &str) {
        self.update(app, |status| {
            status.active_stage_id = (stage_status == "loading").then(|| stage_id.to_string());
            if let Some(stage) = status.stages.iter_mut().find(|stage| stage.id == stage_id) {
                stage.status = stage_status.to_string();
            }
        });
    }

    fn push_warning(&self, app: &AppHandle, issue: AppStartupIssue) {
        self.update(app, |status| {
            if let Some(stage) = status
                .stages
                .iter_mut()
                .find(|stage| stage.id == issue.stage_id)
            {
                stage.status = "warning".to_string();
            }
            status.warnings.push(issue);
        });
    }

    fn set_error(&self, app: &AppHandle, issue: AppStartupIssue) {
        self.update(app, |status| {
            status.active_stage_id = None;
            status.phase = "error".to_string();
            status.error = Some(issue.clone());
            if let Some(stage) = status
                .stages
                .iter_mut()
                .find(|stage| stage.id == issue.stage_id)
            {
                stage.status = "error".to_string();
            }
        });
    }

    fn update(&self, app: &AppHandle, updater: impl FnOnce(&mut AppStartupStatus)) {
        let status = {
            let mut state = self.inner.lock().expect("startup state");
            updater(&mut state.status);
            state.status.clone()
        };
        let _ = app.emit(APP_STARTUP_STATUS_CHANGED_EVENT, status);
    }
}

#[tauri::command(rename = "getAppStartupStatus")]
pub fn get_app_startup_status(manager: State<'_, AppStartupManager>) -> AppStartupStatus {
    manager.status()
}

#[tauri::command(rename = "retryAppStartup")]
pub fn retry_app_startup(
    app: AppHandle,
    manager: State<'_, AppStartupManager>,
) -> AppStartupStatus {
    start_app_startup(app);
    manager.status()
}

pub fn start_app_startup(app: AppHandle) {
    let manager = AppStartupManager {
        inner: app.state::<AppStartupManager>().inner.clone(),
    };
    if !manager.begin() {
        return;
    }

    tauri::async_runtime::spawn(async move {
        run_app_startup(&app, &manager).await;
        manager.finish();
    });
}

async fn run_app_startup(app: &AppHandle, manager: &AppStartupManager) {
    manager.set_stage(app, "core", "loading");
    match load_core_settings(app) {
        Ok(()) => manager.set_stage(app, "core", "ready"),
        Err(issue) => {
            manager.set_error(app, issue);
            return;
        }
    }

    manager.set_stage(app, "transcription", "loading");
    match warm_local_transcription(app.clone()).await {
        Ok(WarmupResult::Ready) | Ok(WarmupResult::Skipped) => {
            manager.set_stage(app, "transcription", "ready");
        }
        Err(issue) => {
            manager.push_warning(app, issue);
        }
    }

    manager.set_ready(app);
}

fn load_core_settings(app: &AppHandle) -> Result<(), AppStartupIssue> {
    let storage = app.state::<AppStorage>();
    settings::load_app_settings(&storage)
        .map(|_settings| ())
        .map_err(|error| AppStartupIssue {
            code: error.code,
            message: error.message,
            stage_id: "core".to_string(),
        })
}

enum WarmupResult {
    Ready,
    Skipped,
}

async fn warm_local_transcription(app: AppHandle) -> Result<WarmupResult, AppStartupIssue> {
    tauri::async_runtime::spawn_blocking(move || {
        let storage = app.state::<AppStorage>();
        let transcription = app.state::<TranscriptionProviderManager>();
        transcription
            .warm_selected_local_model(&app, &storage)
            .map(|warmup| {
                if warmup.is_some() {
                    WarmupResult::Ready
                } else {
                    WarmupResult::Skipped
                }
            })
            .map_err(transcription_startup_issue)
    })
    .await
    .map_err(|error| AppStartupIssue {
        code: "app_startup_transcription_task_failed".to_string(),
        message: format!("Local transcription warmup task failed: {error}"),
        stage_id: "transcription".to_string(),
    })?
}

fn transcription_startup_issue(error: TranscriptionError) -> AppStartupIssue {
    AppStartupIssue {
        code: error.code,
        message: error.message,
        stage_id: "transcription".to_string(),
    }
}

fn initial_status() -> AppStartupStatus {
    AppStartupStatus {
        active_stage_id: None,
        error: None,
        phase: "loading".to_string(),
        stages: vec![
            AppStartupStage {
                description: "Loading persisted settings and local metadata.".to_string(),
                id: "core".to_string(),
                label: "Core app state".to_string(),
                status: "pending".to_string(),
            },
            AppStartupStage {
                description: "Preparing the selected local transcription model when configured."
                    .to_string(),
                id: "transcription".to_string(),
                label: "Local transcription".to_string(),
                status: "pending".to_string(),
            },
        ],
        warnings: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_status_gates_the_app() {
        let manager = AppStartupManager::new();
        let status = manager.status();

        assert_eq!(status.phase, "loading");
        assert_eq!(status.stages.len(), 2);
        assert!(status.error.is_none());
    }
}
