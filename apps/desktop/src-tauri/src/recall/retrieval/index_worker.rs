//! Background indexing owns its queue and failure transitions independently.
use super::worker::now_ms;
use crate::{
    settings,
    storage::{self, AppStorage},
};
use std::sync::atomic::AtomicBool;
use tauri::Manager;
#[path = "index_admission.rs"]
mod admission;

pub(super) fn start(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        use super::embedding::EmbeddingProvider;
        let storage = app.state::<AppStorage>();
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
            let Ok(_activity) = crate::maintenance::runtime::admit(&app, "recall-index") else {
                continue;
            };
            let provider = super::provider_config::build_embedding_provider(&app);
            if !super::status::embedding_unavailable_reason(&app, &provider)
                .is_ok_and(|reason| reason.is_none())
            {
                continue;
            }
            let Ok(provider) = provider else {
                continue;
            };
            let space = provider.space();
            let Some(settings) = settings::load_app_settings(&storage)
                .ok()
                .and_then(|settings| serde_json::to_string(&settings.ai.recall).ok())
            else {
                continue;
            };
            let progress = storage.with_connection(|connection| {
                storage::refresh_evidence(connection)?;
                super::index::progress(connection, &space.fingerprint)
            });
            if progress.as_ref().is_err() || progress.is_ok_and(|progress| progress.pending == 0) {
                continue;
            }
            let now = now_ms();
            let admitted = storage.with_connection_mut(|connection| {
                let transaction = connection.transaction()?;
                let id = admission::claim(&transaction, &space.fingerprint, &settings, now)?;
                transaction.commit()?;
                Ok(id)
            });
            let Ok(Some(id)) = admitted else {
                continue;
            };
            let execution = provider.configuration_fingerprint();
            let cancelled = AtomicBool::new(false);
            let work = super::index::index_pending(&storage, &provider, &cancelled, |progress| {
                let total = progress.indexed + progress.pending;
                let fraction = if total == 0 {
                    1.0
                } else {
                    progress.indexed as f64 / total as f64
                };
                let _=storage.with_connection(|connection|connection.execute("UPDATE recall_jobs SET progress=?2,updated_at_ms=?3 WHERE id=?1 AND status='running'",rusqlite::params![id,fraction,now_ms()]));
                super::mutations::emit_changed(&app, None);
            });
            tokio::pin!(work);
            let result: Result<(), String> = loop {
                tokio::select! {
                    result=&mut work=>break result.map(|_|()),
                    _=tokio::time::sleep(std::time::Duration::from_secs(1))=> {
                        let running=storage.with_connection(|connection|connection.query_row("SELECT status FROM recall_jobs WHERE id=?1",[&id],|row|row.get::<_,String>(0))).is_ok_and(|status|status=="running");
                        let same=super::provider_config::build_embedding_provider(&app).is_ok_and(|provider|provider.configuration_fingerprint()==execution);
                        if app.state::<crate::maintenance::RuntimeActivityRegistry>().is_maintenance_active() || !running || !same {cancelled.store(true,std::sync::atomic::Ordering::Release);break Err("recall_cancelled".into())}
                    }
                }
            };
            match result {
                Ok(()) => {
                    let _=storage.with_connection(|connection|connection.execute("UPDATE recall_jobs SET status='succeeded',progress=1,error_code=NULL,updated_at_ms=?2 WHERE id=?1 AND status='running'",rusqlite::params![id,now_ms()]));
                }
                Err(error) => {
                    if error == "recall_index_inputs_changed" {
                        let _=storage.with_connection(|connection|connection.execute("UPDATE recall_jobs SET status='pending',error_code=NULL,next_attempt_at_ms=NULL,updated_at_ms=?2 WHERE id=?1 AND status='running'",rusqlite::params![id,now_ms()]));
                        super::mutations::emit_changed(&app, None);
                        continue;
                    }
                    if error == "recall_cancelled" {
                        tracing::info!(job_id = %id, "recall_index_cancelled");
                    } else {
                        tracing::warn!(job_id = %id, error_code = crate::telemetry::failures::safe_error_code(&error), "recall_index_failed");
                    }
                    let _ = storage.with_connection(|connection| {
                        super::jobs::fail(connection, &id, &error, now_ms())
                    });
                }
            }
            super::mutations::emit_changed(&app, None);
        }
    });
}
