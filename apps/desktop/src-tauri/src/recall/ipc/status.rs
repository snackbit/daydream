//! Readiness and synthetic provider probes never expose captured content.
use super::{embedding::EmbeddingProvider, types::*};
use crate::{
    settings,
    storage::{self, AppStorage},
};
use rusqlite::OptionalExtension;
use tauri::Manager;

pub fn get_status(app: &tauri::AppHandle) -> Result<RecallStatus, String> {
    let storage = app.state::<AppStorage>();
    let provider = super::provider_config::build_embedding_provider(app);
    let generation = super::worker::profile(&storage);
    let progress = storage
        .with_connection(|connection| {
            storage::refresh_evidence(connection)?;
            let space = provider
                .as_ref()
                .map(|provider| provider.space().fingerprint)
                .unwrap_or_default();
            Ok((
                super::index::progress(connection, &space)?,
                super::jobs::list(connection)?,
                storage::recall_privacy_revision(connection)?,
            ))
        })
        .map_err(|_| "recall_status_unavailable")?;
    let embedding_error = embedding_unavailable_reason(app, &provider)?;
    Ok(RecallStatus {
        privacy_revision: progress.2,
        indexed_evidence_count: progress.0.indexed,
        pending_evidence_count: progress.0.pending,
        embedding_ready: embedding_error.is_none(),
        generation_ready: generation.is_ok(),
        embedding_unavailable_reason: embedding_error,
        generation_unavailable_reason: generation.err(),
        jobs: progress.1,
    })
}

pub(crate) fn embedding_unavailable_reason(
    app: &tauri::AppHandle,
    provider: &Result<super::embedding::ManagedEmbeddingProvider, String>,
) -> Result<Option<String>, String> {
    let storage = app.state::<AppStorage>();
    let mut embedding_error = match provider {
        Err(error) => Some(error.clone()),
        Ok(_) => {
            let profile = settings::load_app_settings(&storage)
                .map_err(|_| "recall_settings_failed")?
                .ai
                .recall
                .embedding;
            let models = app
                .path()
                .app_data_dir()
                .map_err(|_| "recall_storage_failed")?
                .join("models");
            super::embedding::availability(
                &profile,
                &models,
                &super::provider_config::worker_path(app)?,
            )
            .err()
        }
    };
    if embedding_error.is_none() {
        let profile = settings::load_app_settings(&storage)
            .map_err(|_| "recall_settings_failed")?
            .ai
            .recall
            .embedding;
        if profile.provider == "local" {
            embedding_error = storage
                .with_connection(|connection| model_test_error(connection, &profile.local_model_id))
                .map_err(|_| "recall_status_unavailable")?;
        }
    }
    Ok(embedding_error)
}

fn model_test_error(
    connection: &rusqlite::Connection,
    model_id: &str,
) -> rusqlite::Result<Option<String>> {
    connection
        .query_row(
            "SELECT self_test_status='failed' FROM ai_model_assets WHERE id=?1",
            [model_id],
            |row| row.get::<_, bool>(0),
        )
        .optional()
        .map(|failed| {
            failed
                .filter(|failed| *failed)
                .map(|_| "recall_embedding_model_test_failed".to_string())
        })
}

pub async fn test_provider(
    app: &tauri::AppHandle,
    input: &TestRecallProviderInput,
) -> Result<RecallProviderTestResult, String> {
    let started = std::time::Instant::now();
    let storage = app.state::<AppStorage>();
    let mut proposed =
        settings::load_app_settings(&storage).map_err(|_| "recall_settings_failed")?;
    proposed.ai.recall = input.settings.clone();
    settings::validate_settings(proposed).map_err(|_| "recall_profile_invalid")?;
    let result = match input.kind.as_str() {
        "embedding" => {
            let profile = &input.settings.embedding;
            let key = if profile.provider == "local" || profile.auth_mode == "none" {
                None
            } else if let Some(draft) = &input.key_draft {
                Some(draft.clone())
            } else {
                super::provider_config::credential(
                    &storage,
                    profile.api_key_ref.as_deref(),
                    &profile.auth_mode,
                )?
            };
            let models = app
                .path()
                .app_data_dir()
                .map_err(|_| "recall_storage_failed")?
                .join("models");
            super::embedding::test_embedding_provider(
                profile,
                models,
                super::provider_config::worker_path(app)?,
                key,
            )
            .await
            .map(|_| ())
        }
        "generation" => {
            let profile = super::worker::profile_from(
                &storage,
                &input.settings.generation,
                input.key_draft.clone(),
            )?;
            super::generation_store::test_provider(&profile).await
        }
        _ => return Err("recall_provider_kind_invalid".into()),
    };
    if input.kind == "embedding" && input.settings.embedding.provider == "local" {
        let saved = settings::load_app_settings(&storage).map_err(|_| "recall_settings_failed")?;
        if saved.ai.recall.embedding == input.settings.embedding
            && !matches!(
                result.as_ref().err().map(String::as_str),
                Some(
                    "recall_embedding_worker_unavailable"
                        | "recall_embedding_vulkan_unavailable"
                        | "recall_embedding_model_missing"
                )
            )
        {
            storage.with_connection(|connection| connection.execute("UPDATE ai_model_assets SET self_test_status=?2,updated_at_ms=?3 WHERE id=?1 AND download_status='installed'", rusqlite::params![input.settings.embedding.local_model_id, if result.is_ok() {"passed"} else {"failed"}, chrono::Utc::now().timestamp_millis()])).map_err(|_| "recall_storage_failed")?;
            super::mutations::emit_changed(app, None);
        }
    }
    Ok(RecallProviderTestResult {
        kind: input.kind.clone(),
        status: if result.is_ok() {
            "succeeded".into()
        } else {
            "failed".into()
        },
        error_code: result.err(),
        elapsed_ms: started.elapsed().as_millis().min(u64::MAX as u128) as u64,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn failed_installed_model_load_blocks_readiness_until_a_successful_test() {
        let connection = rusqlite::Connection::open_in_memory().unwrap();
        connection.execute_batch("CREATE TABLE ai_model_assets(id TEXT PRIMARY KEY,self_test_status TEXT NOT NULL); INSERT INTO ai_model_assets VALUES ('synthetic','failed');").unwrap();
        assert_eq!(
            model_test_error(&connection, "synthetic").unwrap(),
            Some("recall_embedding_model_test_failed".into())
        );
        connection
            .execute(
                "UPDATE ai_model_assets SET self_test_status='passed' WHERE id='synthetic'",
                [],
            )
            .unwrap();
        assert_eq!(model_test_error(&connection, "synthetic").unwrap(), None);
        connection
            .execute(
                "UPDATE ai_model_assets SET self_test_status='not-run' WHERE id='synthetic'",
                [],
            )
            .unwrap();
        assert_eq!(model_test_error(&connection, "synthetic").unwrap(), None);
        assert_eq!(model_test_error(&connection, "missing").unwrap(), None);
    }
}
