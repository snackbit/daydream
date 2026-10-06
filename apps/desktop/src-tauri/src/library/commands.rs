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
    detail_for_recording(storage.inner(), &recording_id)
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
    delete_recording_row(&storage, &recordings_root, &recording)
}

#[tauri::command(rename = "deleteLibraryRecordings")]
pub async fn delete_library_recordings(
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
        crate::ai::cancel_recording_processing_for_privacy(&storage, &recording.id, now_ms())
            .map_err(|error| error.to_string())?;
        let _lease = crate::recording_privacy::mutation_lease(&recording.id).await;
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
    detail_for_recording(storage.inner(), &recording.id)
}

#[tauri::command(rename = "createLibraryRedactions")]
pub async fn create_library_redactions<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
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
    let recording = storage
        .with_connection(|connection| storage::get_recording(connection, &input.recording_id))
        .map_err(|error| error.to_string())?;
    ensure_recording_allows_redaction(&recording)?;
    crate::ai::cancel_recording_processing_for_privacy(&storage, &recording.id, now_ms())
        .map_err(|error| error.to_string())?;
    let _lease = crate::recording_privacy::mutation_lease(&recording.id).await;
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

#[tauri::command(rename = "getRecordingWaveforms")]
pub async fn get_recording_waveforms(
    input: LibraryWaveformRequest,
    app: tauri::AppHandle,
) -> Result<LibraryWaveforms, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let operation = crate::background_tasks::operations::Operation::start(
            &app,
            "waveforms",
            &input.recording_id,
            "Audio waveforms",
        );
        let storage = app.state::<AppStorage>();
        let recording = storage
            .with_connection(|connection| storage::get_recording(connection, &input.recording_id))
            .map_err(|error| error.to_string())?;
        let segments = storage
            .with_connection(|connection| {
                storage::list_recording_segments(connection, &input.recording_id)
            })
            .map_err(|error| error.to_string())?;

        let value = waveforms::waveforms_for_recording(&recording, &segments);
        operation.finish(
            value
                .error
                .as_ref()
                .map(|_| "ai_waveform_generation_failed"),
        );
        Ok(value)
    })
    .await
    .map_err(|_| "Waveform worker failed.".to_string())?
}

#[tauri::command(rename = "getRecordingWaveformTiles")]
pub async fn get_recording_waveform_tiles(
    input: LibraryWaveformTileRequest,
    app: tauri::AppHandle,
) -> Result<LibraryWaveformTiles, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let operation = crate::background_tasks::operations::Operation::start(
            &app,
            "waveforms",
            &input.recording_id,
            "Audio waveforms",
        );
        let storage = app.state::<AppStorage>();
        let recording = storage
            .with_connection(|connection| storage::get_recording(connection, &input.recording_id))
            .map_err(|error| error.to_string())?;
        let segments = storage
            .with_connection(|connection| {
                storage::list_recording_segments(connection, &input.recording_id)
            })
            .map_err(|error| error.to_string())?;

        let value = waveforms::waveform_tiles_for_recording(&recording, &segments, &input);
        operation.finish(
            value
                .error
                .as_ref()
                .map(|_| "ai_waveform_generation_failed"),
        );
        Ok(value)
    })
    .await
    .map_err(|_| "Waveform worker failed.".to_string())?
}

#[tauri::command(rename = "createLibraryPlaybackSession")]
pub async fn create_library_playback_session(
    input: LibraryPlaybackAssetRequest,
    app: tauri::AppHandle,
) -> Result<LibraryPlaybackAssets, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let operation = crate::background_tasks::operations::Operation::start(
            &app,
            "media-preparation",
            &input.recording_id,
            "Playback preparation",
        );
        let storage = app.state::<AppStorage>();
        let playback_server = app.state::<PlaybackServer>();
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

        let sources = storage
            .with_connection(|connection| {
                storage::recording_video_streams(connection, &recording.id)
            })
            .map_err(|_| "Video sources could not be read.".to_string())?;
        let result = playback_server.create_source_session(&recording, &segments, &sources);
        operation.finish(result.as_ref().err().map(|_| "ai_media_preparation_failed"));
        result
    })
    .await
    .map_err(|_| "Playback preparation worker failed.".to_string())?
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
    app: tauri::AppHandle,
    storage: State<'_, AppStorage>,
) -> Result<LibraryRecordingThumbnail, String> {
    let lease = crate::recording_privacy::processing_lease(&input.recording_id).await;
    let recording = storage
        .with_connection(|connection| storage::get_recording(connection, &input.recording_id))
        .map_err(|error| error.to_string())?;
    let segments = storage
        .with_connection(|connection| {
            storage::list_recording_segments(connection, &input.recording_id)
        })
        .map_err(|error| error.to_string())?;
    let detail = detail_for_recording(storage.inner(), &recording.id)?;
    let first_source = detail
        .tracks
        .iter()
        .find(|track| track.kind == LibraryTrackKind::Video)
        .map(|track| &track.id);
    if detail.redactions.iter().any(|cut| {
        cut.start_offset_ms <= 0
            && cut.end_offset_ms > 0
            && (cut.track_ids.is_empty()
                || first_source.is_some_and(|id| cut.track_ids.contains(id)))
    }) {
        return Ok(LibraryRecordingThumbnail {
            error: None,
            path: None,
            recording_id: recording.id,
        });
    }
    if detail.availability != LibraryRecordingAvailability::Available {
        return Ok(LibraryRecordingThumbnail {
            error: Some("Recording is not available for thumbnails.".to_string()),
            path: None,
            recording_id: recording.id,
        });
    }

    let recording_id = recording.id.clone();
    match tauri::async_runtime::spawn_blocking(move || {
        let _lease = lease;
        let operation = crate::background_tasks::operations::Operation::start(
            &app,
            "thumbnails",
            &recording.id,
            "Recording thumbnails",
        );
        let result = recording_thumbnail_for_recording(&recording, &segments);
        operation.finish(
            result
                .as_ref()
                .err()
                .map(|_| "ai_thumbnail_generation_failed"),
        );
        result
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
