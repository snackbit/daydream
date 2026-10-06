//! Short-lived media work, bounded independently of durable AI job history.
use super::*;
use std::{
    collections::BTreeMap,
    sync::{Mutex, OnceLock},
};
fn registry() -> &'static Mutex<BTreeMap<String, BackgroundTask>> {
    static TASKS: OnceLock<Mutex<BTreeMap<String, BackgroundTask>>> = OnceLock::new();
    TASKS.get_or_init(|| Mutex::new(BTreeMap::new()))
}
pub(crate) fn list(recording: Option<&str>) -> Vec<BackgroundTask> {
    registry()
        .lock()
        .map(|tasks| {
            tasks
                .values()
                .filter(|task| recording.is_none_or(|id| task.recording_id.as_deref() == Some(id)))
                .cloned()
                .map(|mut task| {
                    if task.status == "running" {
                        task.elapsed_ms = (now_ms() - task.updated_at_ms).max(0);
                    }
                    task
                })
                .collect()
        })
        .unwrap_or_default()
}
pub(crate) fn get(id: &str) -> Option<BackgroundTask> {
    registry().lock().ok()?.get(id).cloned()
}

pub(crate) struct Operation {
    id: String,
    app: AppHandle,
    started: i64,
    finished: bool,
}
impl Operation {
    pub(crate) fn start(app: &AppHandle, kind: &str, recording_id: &str, title: &str) -> Self {
        let started = now_ms();
        let id = format!("{kind}:{}", uuid::Uuid::now_v7());
        if let Ok(mut tasks) = registry().lock() {
            while tasks.len() >= 128 {
                let oldest = tasks
                    .values()
                    .filter(|task| task.status != "running")
                    .min_by_key(|task| task.updated_at_ms)
                    .map(|task| task.id.clone());
                if let Some(id) = oldest {
                    tasks.remove(&id);
                } else {
                    break;
                }
            }
            tasks.insert(
                id.clone(),
                BackgroundTask {
                    id: id.clone(),
                    kind: kind.into(),
                    title: title.into(),
                    recording_id: Some(recording_id.into()),
                    model_id: None,
                    status: "running".into(),
                    phase: kind.into(),
                    progress: None,
                    elapsed_ms: 0,
                    eta_ms: None,
                    next_retry_at_ms: None,
                    input_open: false,
                    error_code: None,
                    updated_at_ms: started,
                    stages: vec![],
                },
            );
        }
        emit_changed(app, Some(recording_id));
        Self {
            id,
            app: app.clone(),
            started,
            finished: false,
        }
    }
    pub(crate) fn finish(mut self, error: Option<&str>) {
        self.set_finished(error);
    }
    fn set_finished(&mut self, error: Option<&str>) {
        if let Ok(mut tasks) = registry().lock() {
            if let Some(task) = tasks.get_mut(&self.id) {
                task.status = if error.is_some() {
                    "failed"
                } else {
                    "succeeded"
                }
                .into();
                task.error_code = error.map(str::to_owned);
                task.updated_at_ms = now_ms();
                task.elapsed_ms = (task.updated_at_ms - self.started).max(0);
                task.progress = Some(1.0);
                emit_changed(&self.app, task.recording_id.as_deref());
            }
        }
        self.finished = true;
    }
}
impl Drop for Operation {
    fn drop(&mut self) {
        if !self.finished {
            self.set_finished(Some("ai_media_worker_failed"));
        }
    }
}
