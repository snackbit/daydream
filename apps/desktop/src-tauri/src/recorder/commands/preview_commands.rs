use tauri::{AppHandle, State};
use tracing::{debug, info, warn};

use crate::storage::AppStorage;

use super::super::{
    manager::{now_ms, PreviewStartDecision, RecorderManager},
    platform,
    storage_mapping::recorder_scene_for_storage,
    types::{RecorderError, RecorderPreviewStatus, RecorderScene, StartRecorderPreviewInput},
};

pub fn get_recorder_preview_status(
    manager: State<'_, RecorderManager>,
) -> Result<RecorderPreviewStatus, RecorderError> {
    manager.preview_status()
}

pub fn update_recorder_preview_demand(
    input: super::super::types::RecorderPreviewDemand,
    manager: State<'_, RecorderManager>,
) -> Result<(), RecorderError> {
    if input.scene_source_id.is_empty()
        || input.scene_source_id.len() > 256
        || input.width == 0
        || input.height == 0
        || input.width > 3840
        || input.height > 3840
        || u64::from(input.width) * u64::from(input.height) > 4_000_000
        || input.generation == 0
    {
        return Err(super::super::errors::invalid_scene_error(
            "Preview size is invalid.",
        ));
    }
    let mut demand = manager.preview_demand.lock().map_err(|_| {
        super::super::errors::capture_adapter_failed_error("Preview demand is unavailable.")
    })?;
    if demand
        .as_ref()
        .is_none_or(|current| current.generation < input.generation)
    {
        *demand = Some(input);
    }
    Ok(())
}

pub async fn start_recorder_preview(
    input: StartRecorderPreviewInput,
    app: AppHandle,
    storage: State<'_, AppStorage>,
    manager: State<'_, RecorderManager>,
) -> Result<RecorderPreviewStatus, RecorderError> {
    let _activity =
        crate::maintenance::runtime::admit(&app, "capture-preview").map_err(|message| {
            super::super::errors::lifecycle_error("recorder_maintenance_active", &message)
        })?;
    let timestamp_ms = now_ms();
    info!(scene_id = %input.scene_id, "recorder_preview_start_requested");
    let (scene_id, generation) = match manager.begin_preview_start(&input.scene_id, timestamp_ms)? {
        PreviewStartDecision::AlreadyCurrent(status) => {
            debug!(
                scene_id = %input.scene_id,
                preview_state = ?status.state,
                "recorder_preview_start_reused"
            );
            return Ok(status);
        }
        PreviewStartDecision::Pending { generation, .. } => (input.scene_id, generation),
    };

    let loaded_scene = if let Some(scene) = input.scene {
        if scene.id == scene_id {
            Ok(scene)
        } else {
            Err(super::super::errors::invalid_scene_error(
                "Preview scene identity",
            ))
        }
    } else {
        recorder_scene_for_storage(&storage, &scene_id)
    };
    let scene = match loaded_scene {
        Ok(scene) => scene,
        Err(error) => {
            warn!(
                scene_id = %scene_id,
                error_code = %error.code,
                error_category = ?error.category,
                "recorder_preview_scene_load_failed"
            );
            let _ = manager.fail_preview_start(scene_id, generation, error.clone(), now_ms());
            return Err(error);
        }
    };
    debug!(
        scene_id = %scene.id,
        scene_source_count = scene.sources.len(),
        scene_source_ids = %scene
            .sources
            .iter()
            .map(|source| source.source_id.as_str())
            .collect::<Vec<_>>()
            .join(","),
        "recorder_preview_scene_loaded"
    );
    if !manager.preview_start_is_current(&scene_id, generation) {
        debug!(
            scene_id = %scene_id,
            generation,
            "recorder_preview_start_stale_before_platform_start"
        );
        return manager.preview_status();
    }
    if let Some(status) = stop_empty_preview_start(&manager, &scene, &scene_id, generation)? {
        return Ok(status);
    }
    let preview = match platform::start_preview_session(app, &storage, &scene).await {
        Ok(preview) => preview,
        Err(error) => {
            warn!(
                scene_id = %scene_id,
                scene_source_count = scene.sources.len(),
                scene_source_ids = %scene
                    .sources
                    .iter()
                    .map(|source| source.source_id.as_str())
                    .collect::<Vec<_>>()
                    .join(","),
                error_code = %error.code,
                error_category = ?error.category,
                "recorder_preview_start_failed"
            );
            let _ = manager.fail_preview_start(scene_id, generation, error.clone(), now_ms());
            return Err(error);
        }
    };
    info!(
        scene_id = %scene_id,
        scene_source_ids = %preview.source_ids.join(","),
        "recorder_preview_start_succeeded"
    );

    manager.activate_preview(
        scene_id,
        generation,
        preview.source_ids,
        preview.session,
        now_ms(),
    )
}

pub fn stop_recorder_preview(
    manager: State<'_, RecorderManager>,
) -> Result<RecorderPreviewStatus, RecorderError> {
    info!("recorder_preview_stop_requested");
    manager.stop_preview(now_ms())
}

pub(crate) fn stop_empty_preview_start(
    manager: &RecorderManager,
    scene: &RecorderScene,
    scene_id: &str,
    generation: u64,
) -> Result<Option<RecorderPreviewStatus>, RecorderError> {
    if !scene.sources.is_empty() {
        return Ok(None);
    }

    info!(
        scene_id = %scene_id,
        generation,
        "recorder_preview_start_skipped_empty_scene"
    );

    manager.stop_preview(now_ms()).map(Some)
}
