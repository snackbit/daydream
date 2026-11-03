use super::*;

#[tauri::command(rename = "listLibraryRecordings")]
pub async fn list_library_recordings(
    storage: State<'_, AppStorage>,
) -> Result<Vec<LibraryRecordingSummary>, String> {
    let storage = storage.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let recordings = storage
            .with_connection(storage::list_recordings)
            .map_err(|error| error.to_string())?;

        recordings
            .into_iter()
            .map(|recording| {
                let segments = storage
                    .with_connection(|connection| {
                        storage::list_recording_segments(connection, &recording.id)
                    })
                    .map_err(|error| error.to_string())?;
                Ok(summary_from_rows(&recording, &segments))
            })
            .collect()
    })
    .await
    .map_err(|_| "Recording list could not be loaded. Try again.".to_owned())?
}

#[tauri::command(rename = "getLibraryRecording")]
pub async fn get_library_recording(
    recording_id: String,
    include_disk_usage: Option<bool>,
    cached_disk_usage: Option<bool>,
    storage: State<'_, AppStorage>,
) -> Result<LibraryRecordingDetail, String> {
    let storage = storage.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        detail_for_recording_with_size(
            &storage,
            &recording_id,
            include_disk_usage.unwrap_or(true),
            cached_disk_usage.unwrap_or(false),
        )
    })
    .await
    .map_err(|_| "Recording could not be loaded. Try again.".to_owned())?
}

#[tauri::command(rename = "getLibraryRecordingDebug")]
pub fn get_library_recording_debug(
    input: LibraryRecordingDebugInput,
    storage: State<'_, AppStorage>,
) -> Result<LibraryRecordingDebugData, String> {
    debug::debug_data_for_recording(&input.recording_id, storage.inner())
}

#[tauri::command(rename = "deleteLibraryRecording")]
pub async fn delete_library_recording(
    app: tauri::AppHandle,
    input: DeleteLibraryRecordingInput,
    storage: State<'_, AppStorage>,
) -> Result<(), String> {
    let _activity = crate::maintenance::runtime::admit(&app, "library-delete")?;
    let recordings_root = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?
        .join("recordings");
    let recording = storage
        .with_connection(|connection| storage::get_recording(connection, &input.recording_id))
        .map_err(|error| error.to_string())?;

    ensure_recording_is_deletable(&recording)?;
    crate::ai::cancel_recording_processing_for_privacy(&storage, &recording.id, now_ms())
        .map_err(|error| error.to_string())?;
    let _lease = crate::recording_privacy::mutation_lease(&recording.id).await;
    close_cleanup_playback(&app, &recording.id).await?;
    delete_recording_row(&storage, &recordings_root, &recording)?;
    crate::recall::mutations::emit_changed(&app, None);
    Ok(())
}

#[tauri::command(rename = "deleteLibraryRecordings")]
pub async fn delete_library_recordings(
    app: tauri::AppHandle,
    input: DeleteLibraryRecordingsInput,
    storage: State<'_, AppStorage>,
) -> Result<(), String> {
    let _activity = crate::maintenance::runtime::admit(&app, "library-delete")?;
    let recordings_root = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?
        .join("recordings");
    let recording_ids = unique_recording_ids(input.recording_ids);
    let recordings = recording_ids
        .iter()
        .map(|recording_id| {
            storage
                .with_connection(|connection| storage::get_recording(connection, recording_id))
                .map_err(|error| error.to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;

    for recording in &recordings {
        ensure_recording_is_deletable(recording)?;
    }

    for recording in recordings {
        crate::ai::cancel_recording_processing_for_privacy(&storage, &recording.id, now_ms())
            .map_err(|error| error.to_string())?;
        let _lease = crate::recording_privacy::mutation_lease(&recording.id).await;
        close_cleanup_playback(&app, &recording.id).await?;
        delete_recording_row(&storage, &recordings_root, &recording)?;
        crate::recall::mutations::emit_changed(&app, None);
    }

    Ok(())
}

#[tauri::command(rename = "renameLibraryRecording")]
pub fn rename_library_recording<R: tauri::Runtime>(
    input: RenameLibraryRecordingInput,
    app: tauri::AppHandle<R>,
    storage: State<'_, AppStorage>,
) -> Result<LibraryRecordingDetail, String> {
    let _activity = crate::maintenance::runtime::admit(&app, "library-rename")?;
    let title = normalize_recording_title(input.title)?;
    let recording = storage
        .with_connection(|connection| {
            storage::rename_recording(connection, &input.recording_id, title.as_deref())
        })
        .map_err(|error| error.to_string())?;
    detail_for_recording(storage.inner(), &recording.id)
}

#[tauri::command(rename = "createLibraryRedactions")]
pub async fn create_library_redactions<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    input: CreateLibraryRedactionsInput,
    storage: State<'_, AppStorage>,
) -> Result<LibraryRecordingDetail, String> {
    let _activity = crate::maintenance::runtime::admit(&app, "library-redaction")?;
    let recording = storage
        .with_connection(|connection| storage::get_recording(connection, &input.recording_id))
        .map_err(|error| error.to_string())?;
    ensure_recording_allows_redaction(&recording)?;
    let segments = storage
        .with_connection(|connection| {
            storage::list_recording_segments(connection, &input.recording_id)
        })
        .map_err(|error| error.to_string())?;
    let tracks = storage
        .with_connection(|connection| tracks_for_recording(connection, &recording.id, &segments))
        .map_err(|error| error.to_string())?;
    let duration_ms = timeline_duration_ms(&segments);
    let existing_redactions = storage
        .with_connection(|connection| storage::list_redactions(connection, &recording.id))
        .map_err(|error| error.to_string())?;
    build_normalized_redaction_inputs(
        &recording.id,
        &input.ranges,
        &existing_redactions,
        &tracks,
        duration_ms,
    )?;
    crate::ai::cancel_recording_processing_for_privacy(&storage, &recording.id, now_ms())
        .map_err(|error| error.to_string())?;
    let _lease = crate::recording_privacy::mutation_lease(&recording.id).await;
    close_cleanup_playback(&app, &recording.id).await?;
    let existing_redactions = storage
        .with_connection(|connection| storage::list_redactions(connection, &recording.id))
        .map_err(|error| error.to_string())?;
    let inputs = build_normalized_redaction_inputs(
        &recording.id,
        &input.ranges,
        &existing_redactions,
        &tracks,
        duration_ms,
    )?;

    storage
        .with_connection_mut(|connection| {
            let transaction = connection.transaction()?;
            storage::delete_redactions_for_recording(&transaction, &recording.id)?;
            for input in &inputs {
                storage::create_redaction(&transaction, input)?;
            }

            transaction.commit()
        })
        .map_err(|error| error.to_string())?;

    ai::emit_processing_state(&app, &recording.id);
    detail_for_recording(storage.inner(), &recording.id)
}

#[tauri::command(rename = "deleteLibraryRedaction")]
pub async fn delete_library_redaction(
    app: tauri::AppHandle,
    input: DeleteLibraryRedactionInput,
    storage: State<'_, AppStorage>,
) -> Result<LibraryRecordingDetail, String> {
    let _activity = crate::maintenance::runtime::admit(&app, "library-redaction")?;
    let recording = storage
        .with_connection(|connection| storage::get_recording(connection, &input.recording_id))
        .map_err(|error| error.to_string())?;
    ensure_recording_allows_redaction(&recording)?;
    crate::ai::cancel_recording_processing_for_privacy(&storage, &recording.id, now_ms())
        .map_err(|error| error.to_string())?;
    let _lease = crate::recording_privacy::mutation_lease(&recording.id).await;
    close_cleanup_playback(&app, &recording.id).await?;
    let deleted = storage
        .with_connection(|connection| {
            storage::delete_redaction(connection, &input.recording_id, &input.redaction_id)
        })
        .map_err(|error| error.to_string())?;
    if !deleted {
        return Err("Redaction was not found.".to_string());
    }

    ai::emit_processing_state(&app, &recording.id);
    detail_for_recording(storage.inner(), &recording.id)
}

#[tauri::command(rename = "unredactLibraryRedactions")]
pub async fn unredact_library_redactions<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    input: UnredactLibraryRedactionsInput,
    storage: State<'_, AppStorage>,
) -> Result<LibraryRecordingDetail, String> {
    let _activity = crate::maintenance::runtime::admit(&app, "library-redaction")?;
    let recording = storage
        .with_connection(|connection| storage::get_recording(connection, &input.recording_id))
        .map_err(|error| error.to_string())?;
    ensure_recording_allows_redaction(&recording)?;
    let segments = storage
        .with_connection(|connection| {
            storage::list_recording_segments(connection, &input.recording_id)
        })
        .map_err(|error| error.to_string())?;
    let tracks = storage
        .with_connection(|connection| tracks_for_recording(connection, &recording.id, &segments))
        .map_err(|error| error.to_string())?;
    let duration_ms = timeline_duration_ms(&segments);
    let existing_redactions = storage
        .with_connection(|connection| storage::list_redactions(connection, &recording.id))
        .map_err(|error| error.to_string())?;
    build_unredacted_redaction_inputs(
        &recording.id,
        &input.ranges,
        &existing_redactions,
        &tracks,
        duration_ms,
    )?;
    crate::ai::cancel_recording_processing_for_privacy(&storage, &recording.id, now_ms())
        .map_err(|error| error.to_string())?;
    let _lease = crate::recording_privacy::mutation_lease(&recording.id).await;
    close_cleanup_playback(&app, &recording.id).await?;
    let existing_redactions = storage
        .with_connection(|connection| storage::list_redactions(connection, &recording.id))
        .map_err(|error| error.to_string())?;
    let inputs = build_unredacted_redaction_inputs(
        &recording.id,
        &input.ranges,
        &existing_redactions,
        &tracks,
        duration_ms,
    )?;

    storage
        .with_connection_mut(|connection| {
            let transaction = connection.transaction()?;
            storage::delete_redactions_for_recording(&transaction, &recording.id)?;
            for input in &inputs {
                storage::create_redaction(&transaction, input)?;
            }

            transaction.commit()
        })
        .map_err(|error| error.to_string())?;

    ai::emit_processing_state(&app, &recording.id);
    detail_for_recording(storage.inner(), &recording.id)
}

mod playback;
pub use playback::*;
