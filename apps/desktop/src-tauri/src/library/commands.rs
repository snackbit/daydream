use super::*;

#[tauri::command(rename = "listLibraryRecordings")]
pub fn list_library_recordings(
    storage: State<'_, AppStorage>,
) -> Result<Vec<LibraryRecordingSummary>, String> {
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
}

#[tauri::command(rename = "getLibraryRecording")]
pub fn get_library_recording(
    recording_id: String,
    storage: State<'_, AppStorage>,
) -> Result<LibraryRecordingDetail, String> {
    let recording = storage
        .with_connection(|connection| storage::get_recording(connection, &recording_id))
        .map_err(|error| error.to_string())?;
    let segments = storage
        .with_connection(|connection| storage::list_recording_segments(connection, &recording_id))
        .map_err(|error| error.to_string())?;
    let redactions = storage
        .with_connection(|connection| storage::list_redactions(connection, &recording_id))
        .map_err(|error| error.to_string())?;

    Ok(detail_from_rows(&recording, &segments, &redactions))
}

#[tauri::command(rename = "getLibraryRecordingDebug")]
pub fn get_library_recording_debug(
    input: LibraryRecordingDebugInput,
    storage: State<'_, AppStorage>,
) -> Result<LibraryRecordingDebugData, String> {
    debug::debug_data_for_recording(&input.recording_id, storage.inner())
}

#[tauri::command(rename = "deleteLibraryRecording")]
pub fn delete_library_recording(
    app: tauri::AppHandle,
    input: DeleteLibraryRecordingInput,
    storage: State<'_, AppStorage>,
) -> Result<(), String> {
    let recordings_root = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?
        .join("recordings");
    let recording = storage
        .with_connection(|connection| storage::get_recording(connection, &input.recording_id))
        .map_err(|error| error.to_string())?;

    ensure_recording_is_deletable(&recording)?;
    delete_recording_row(&storage, &recordings_root, &recording)
}

#[tauri::command(rename = "deleteLibraryRecordings")]
pub fn delete_library_recordings(
    app: tauri::AppHandle,
    input: DeleteLibraryRecordingsInput,
    storage: State<'_, AppStorage>,
) -> Result<(), String> {
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
        delete_recording_row(&storage, &recordings_root, &recording)?;
    }

    Ok(())
}

#[tauri::command(rename = "renameLibraryRecording")]
pub fn rename_library_recording(
    input: RenameLibraryRecordingInput,
    storage: State<'_, AppStorage>,
) -> Result<LibraryRecordingDetail, String> {
    let title = normalize_recording_title(input.title)?;
    let recording = storage
        .with_connection(|connection| {
            storage::rename_recording(connection, &input.recording_id, title.as_deref())
        })
        .map_err(|error| error.to_string())?;
    let segments = storage
        .with_connection(|connection| storage::list_recording_segments(connection, &recording.id))
        .map_err(|error| error.to_string())?;
    let redactions = storage
        .with_connection(|connection| storage::list_redactions(connection, &recording.id))
        .map_err(|error| error.to_string())?;

    Ok(detail_from_rows(&recording, &segments, &redactions))
}

#[tauri::command(rename = "createLibraryRedactions")]
pub fn create_library_redactions(
    input: CreateLibraryRedactionsInput,
    storage: State<'_, AppStorage>,
) -> Result<LibraryRecordingDetail, String> {
    let recording = storage
        .with_connection(|connection| storage::get_recording(connection, &input.recording_id))
        .map_err(|error| error.to_string())?;
    ensure_recording_allows_redaction(&recording)?;
    let segments = storage
        .with_connection(|connection| {
            storage::list_recording_segments(connection, &input.recording_id)
        })
        .map_err(|error| error.to_string())?;
    let tracks = tracks_from_segments(&segments);
    let duration_ms = timeline_duration_ms(&segments);
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

    detail_for_recording(storage.inner(), &recording.id)
}

#[tauri::command(rename = "deleteLibraryRedaction")]
pub fn delete_library_redaction(
    input: DeleteLibraryRedactionInput,
    storage: State<'_, AppStorage>,
) -> Result<LibraryRecordingDetail, String> {
    let recording = storage
        .with_connection(|connection| storage::get_recording(connection, &input.recording_id))
        .map_err(|error| error.to_string())?;
    ensure_recording_allows_redaction(&recording)?;
    let deleted = storage
        .with_connection(|connection| {
            storage::delete_redaction(connection, &input.recording_id, &input.redaction_id)
        })
        .map_err(|error| error.to_string())?;
    if !deleted {
        return Err("Redaction was not found.".to_string());
    }

    detail_for_recording(storage.inner(), &recording.id)
}

#[tauri::command(rename = "unredactLibraryRedactions")]
pub fn unredact_library_redactions(
    input: UnredactLibraryRedactionsInput,
    storage: State<'_, AppStorage>,
) -> Result<LibraryRecordingDetail, String> {
    let recording = storage
        .with_connection(|connection| storage::get_recording(connection, &input.recording_id))
        .map_err(|error| error.to_string())?;
    ensure_recording_allows_redaction(&recording)?;
    let segments = storage
        .with_connection(|connection| {
            storage::list_recording_segments(connection, &input.recording_id)
        })
        .map_err(|error| error.to_string())?;
    let tracks = tracks_from_segments(&segments);
    let duration_ms = timeline_duration_ms(&segments);
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

    detail_for_recording(storage.inner(), &recording.id)
}

#[tauri::command(rename = "getRecordingWaveforms")]
pub fn get_recording_waveforms(
    input: LibraryWaveformRequest,
    storage: State<'_, AppStorage>,
) -> Result<LibraryWaveforms, String> {
    let recording = storage
        .with_connection(|connection| storage::get_recording(connection, &input.recording_id))
        .map_err(|error| error.to_string())?;
    let segments = storage
        .with_connection(|connection| {
            storage::list_recording_segments(connection, &input.recording_id)
        })
        .map_err(|error| error.to_string())?;

    Ok(waveforms::waveforms_for_recording(&recording, &segments))
}

#[tauri::command(rename = "getRecordingWaveformTiles")]
pub fn get_recording_waveform_tiles(
    input: LibraryWaveformTileRequest,
    storage: State<'_, AppStorage>,
) -> Result<LibraryWaveformTiles, String> {
    let recording = storage
        .with_connection(|connection| storage::get_recording(connection, &input.recording_id))
        .map_err(|error| error.to_string())?;
    let segments = storage
        .with_connection(|connection| {
            storage::list_recording_segments(connection, &input.recording_id)
        })
        .map_err(|error| error.to_string())?;

    Ok(waveforms::waveform_tiles_for_recording(
        &recording, &segments, &input,
    ))
}

#[tauri::command(rename = "createLibraryPlaybackSession")]
pub fn create_library_playback_session(
    input: LibraryPlaybackAssetRequest,
    storage: State<'_, AppStorage>,
    playback_server: State<'_, PlaybackServer>,
) -> Result<LibraryPlaybackAssets, String> {
    let recording = storage
        .with_connection(|connection| storage::get_recording(connection, &input.recording_id))
        .map_err(|error| error.to_string())?;
    let segments = storage
        .with_connection(|connection| {
            storage::list_recording_segments(connection, &input.recording_id)
        })
        .map_err(|error| error.to_string())?;
    let detail = detail_from_rows(&recording, &segments, &[]);
    if detail.availability != LibraryRecordingAvailability::Available {
        return Ok(LibraryPlaybackAssets {
            error: Some("Recording is not available for playback.".to_string()),
            recording_id: recording.id,
            segments: Vec::new(),
            session_id: String::new(),
        });
    }

    playback_server.create_session(&recording, &segments)
}

#[tauri::command(rename = "closeLibraryPlaybackSession")]
pub fn close_library_playback_session(
    input: CloseLibraryPlaybackSessionRequest,
    playback_server: State<'_, PlaybackServer>,
) -> Result<(), String> {
    playback_server.close_session(&input.session_id)
}

#[tauri::command(rename = "prepareLibraryRecordingThumbnail")]
pub async fn prepare_library_recording_thumbnail(
    input: LibraryRecordingThumbnailRequest,
    storage: State<'_, AppStorage>,
) -> Result<LibraryRecordingThumbnail, String> {
    let recording = storage
        .with_connection(|connection| storage::get_recording(connection, &input.recording_id))
        .map_err(|error| error.to_string())?;
    let segments = storage
        .with_connection(|connection| {
            storage::list_recording_segments(connection, &input.recording_id)
        })
        .map_err(|error| error.to_string())?;
    let detail = detail_from_rows(&recording, &segments, &[]);
    if detail.availability != LibraryRecordingAvailability::Available {
        return Ok(LibraryRecordingThumbnail {
            error: Some("Recording is not available for thumbnails.".to_string()),
            path: None,
            recording_id: recording.id,
        });
    }

    let recording_id = recording.id.clone();
    match tauri::async_runtime::spawn_blocking(move || {
        recording_thumbnail_for_recording(&recording, &segments)
    })
    .await
    .map_err(|error| error.to_string())?
    {
        Ok(path) => Ok(LibraryRecordingThumbnail {
            error: None,
            path,
            recording_id,
        }),
        Err(error) => Ok(LibraryRecordingThumbnail {
            error: Some(error),
            path: None,
            recording_id,
        }),
    }
}
