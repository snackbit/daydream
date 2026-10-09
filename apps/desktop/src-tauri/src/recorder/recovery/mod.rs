//! Explicit recovery copies partial media; it never edits the failed original.
mod publication;
use crate::storage::recovery::RecordingRecoveryCoverage;
use crate::{
    maintenance::{quiesce, runtime::RuntimeActivityRegistry},
    storage::{self, AppStorage},
};
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};
use tauri::{Manager, State};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RecoverLibraryRecordingInput {
    recording_id: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RecordingRecoveryResult {
    operation_id: String,
    original_recording_id: String,
    recovered_recording_id: String,
    status: &'static str,
    recovered_segment_count: usize,
    skipped_segment_count: usize,
    coverage: Vec<RecordingRecoveryCoverage>,
}
pub(super) struct RecoveredSegment {
    original: storage::RecordingSegmentRow,
    id: String,
    path: PathBuf,
    probe: super::media_writer::MediaSegmentProbe,
    start_pts_ms: i64,
    end_pts_ms: i64,
    stream_mappings: Vec<(i64, i64, i64, i64)>,
}

#[tauri::command(rename = "recoverLibraryRecording")]
#[cfg(target_os = "linux")]
pub(crate) async fn recover_library_recording(
    app: tauri::AppHandle,
    input: RecoverLibraryRecordingInput,
    storage: State<'_, AppStorage>,
    registry: State<'_, RuntimeActivityRegistry>,
) -> Result<RecordingRecoveryResult, String> {
    uuid::Uuid::parse_str(&input.recording_id).map_err(|_| "Invalid recording identity.")?;
    let exclusive = registry.begin_maintenance()?;
    quiesce::ensure_capture_stopped(&app)?;
    quiesce::quiesce(&app).await?;
    exclusive.cancel_and_drain().await?;
    quiesce::release_models(&app).await?;
    crate::ai::cancel_recording_processing_for_privacy(
        &storage,
        &input.recording_id,
        chrono::Utc::now().timestamp_millis(),
    )
    .map_err(|_| "Unable to stop processing for recovery.")?;
    let _privacy = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        crate::recording_privacy::mutation_lease(&input.recording_id),
    )
    .await
    .map_err(|_| "Recording readers did not stop. Restart Daydream before recovering it.")?;
    let original = storage
        .with_connection(|connection| storage::get_recording(connection, &input.recording_id))
        .map_err(|_| "Unable to read the failed recording.")?;
    if original.status != "failed" {
        return Err(
            "Only failed recordings can be recovered. The original is kept unchanged.".into(),
        );
    }
    let mut segments = storage
        .with_connection(|connection| storage::list_recording_segments(connection, &original.id))
        .map_err(|_| "Unable to read recording segments.")?;
    // Capture sources cannot change within a recording. A finalized predecessor
    // supplies a durable audio role mapping for older interrupted fragments.
    let desktop = segments
        .iter()
        .find_map(|segment| segment.desktop_audio_stream_index);
    let microphone = segments
        .iter()
        .find_map(|segment| segment.microphone_audio_stream_index);
    for segment in &mut segments {
        segment.desktop_audio_stream_index = segment.desktop_audio_stream_index.or(desktop);
        segment.microphone_audio_stream_index =
            segment.microphone_audio_stream_index.or(microphone);
    }
    let operation_id = uuid::Uuid::now_v7().to_string();
    let recovered_id = uuid::Uuid::now_v7().to_string();
    let app_root = app
        .path()
        .app_data_dir()
        .map_err(|_| "App storage is unavailable.")?;
    let recording_root = app_root.join("recordings");
    let canonical_root = recording_root
        .canonicalize()
        .map_err(|_| "Recording storage is unavailable.")?;
    let original_root = PathBuf::from(&original.storage_dir)
        .canonicalize()
        .map_err(|_| "The original recording directory is missing.")?;
    if original_root == canonical_root || !original_root.starts_with(&canonical_root) {
        return Err("The original recording is outside app-owned storage.".into());
    }
    let output_root = recording_root.join(&recovered_id);
    crate::recording_deletion::remove_recovery_outputs(
        &storage,
        &recording_root,
        Some(&original.id),
    )?;
    storage
        .with_connection(|connection| {
            storage::recovery::start_operation(
                connection,
                &operation_id,
                &original.id,
                &recovered_id,
                chrono::Utc::now().timestamp_millis(),
            )
        })
        .map_err(|_| "Unable to persist recording recovery state.")?;
    if fs::create_dir(&output_root)
        .and_then(|_| fs::create_dir(output_root.join("segments")))
        .is_err()
    {
        fail_recovery(&storage, &operation_id, &original.id, &recording_root)?;
        return Err(
            "Unable to create recovery segment storage. The original recording is unchanged."
                .into(),
        );
    }
    let mut recovered = Vec::new();
    let mut skipped = 0;
    for segment in segments {
        let input_path = PathBuf::from(&segment.file_path);
        let valid = input_path
            .canonicalize()
            .is_ok_and(|path| path.starts_with(&original_root));
        if !valid {
            skipped += 1;
            continue;
        }
        let output_path = output_root
            .join("segments")
            .join(format!("{:04}.mkv", segment.segment_index));
        let result = match tauri::async_runtime::spawn_blocking(move || {
            super::media_worker::recover_media(&input_path, &output_path)
        })
        .await
        {
            Ok(result) => result,
            Err(_) => {
                fail_recovery(&storage, &operation_id, &original.id, &recording_root)?;
                return Err("Media recovery worker stopped unexpectedly. The original recording is unchanged.".into());
            }
        };
        match result {
            Ok(media) => {
                let path = output_root
                    .join("segments")
                    .join(format!("{:04}.mkv", segment.segment_index));
                recovered.push(RecoveredSegment {
                    original: segment,
                    id: uuid::Uuid::now_v7().to_string(),
                    path,
                    probe: media.probe,
                    start_pts_ms: media.start_pts_ms,
                    end_pts_ms: media.end_pts_ms,
                    stream_mappings: media
                        .streams
                        .into_iter()
                        .map(|stream| {
                            (
                                stream.original_index,
                                stream.output_index,
                                stream.start_pts_ms,
                                stream.end_pts_ms,
                            )
                        })
                        .collect(),
                });
            }
            Err(_) => {
                skipped += 1;
            }
        }
    }
    if recovered.is_empty() {
        fail_recovery(&storage, &operation_id, &original.id, &recording_root)?;
        return Err(
            "No usable media could be recovered. The original recording is unchanged.".into(),
        );
    }
    if !super::media_worker::is_quiescent() {
        fail_recovery(&storage, &operation_id, &original.id, &recording_root)?;
        return Err(
            "Recovery media writers have not stopped; retry after restarting Daydream.".into(),
        );
    }
    let coverage = publish_recovered(
        &storage,
        &operation_id,
        &original,
        &recovered_id,
        &output_root,
        &recovered,
    )?;
    crate::recall::mutations::emit_changed(&app, None);
    Ok(RecordingRecoveryResult {
        operation_id,
        original_recording_id: original.id,
        recovered_recording_id: recovered_id,
        status: "recovered-partial",
        recovered_segment_count: recovered.len(),
        skipped_segment_count: skipped,
        coverage,
    })
}

fn fail_recovery(
    storage: &AppStorage,
    operation_id: &str,
    original: &str,
    recordings_root: &std::path::Path,
) -> Result<(), String> {
    let status = storage
        .with_connection(|connection| storage::recovery::fail_operation(connection, operation_id));
    // A timed-out child may still own the files. Keep the durable pending ID
    // until startup can prove there is no surviving writer in this process.
    if !super::media_worker::is_quiescent() {
        return Err("Recovery files are still owned by a media worker. Restart Daydream before retrying recovery or deletion.".into());
    }
    crate::recording_deletion::remove_recovery_outputs(storage, recordings_root, Some(original))?;
    status.map_err(|_| "Recovery failed and its status could not be persisted.".into())
}

fn publish_recovered(
    storage: &AppStorage,
    operation_id: &str,
    original: &storage::RecordingRow,
    recovered_id: &str,
    output_root: &std::path::Path,
    recovered: &[RecoveredSegment],
) -> Result<Vec<RecordingRecoveryCoverage>, String> {
    match storage.with_connection_mut(|connection| {
        publication::publish(
            connection,
            operation_id,
            original,
            recovered_id,
            output_root,
            recovered,
        )
    }) {
        Ok(coverage) => Ok(coverage),
        Err(_) => {
            let root = output_root
                .parent()
                .ok_or("Recovery storage has no parent directory.")?;
            fail_recovery(storage, operation_id, &original.id, root)?;
            Err("Recovery publication failed. Unpublished files were removed and the original recording remains unchanged; retry recovery.".into())
        }
    }
}

#[cfg(not(target_os = "linux"))]
#[tauri::command(rename = "recoverLibraryRecording")]
pub(crate) async fn recover_library_recording(
    _app: tauri::AppHandle,
    _input: RecoverLibraryRecordingInput,
    _storage: State<'_, AppStorage>,
    _registry: State<'_, RuntimeActivityRegistry>,
) -> Result<RecordingRecoveryResult, String> {
    Err("Recording recovery is only available on Linux.".into())
}
