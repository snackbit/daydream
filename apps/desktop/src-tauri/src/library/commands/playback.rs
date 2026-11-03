use super::*;

#[tauri::command(rename = "getPlaybackDecoderWarmupUrl")]
pub fn get_playback_decoder_warmup_url(server: State<'_, PlaybackServer>) -> String {
    server.decoder_warmup_url()
}

#[tauri::command(rename = "getRecordingWaveforms")]
pub async fn get_recording_waveforms(
    input: LibraryWaveformRequest,
    app: tauri::AppHandle,
) -> Result<LibraryWaveforms, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let _activity = crate::maintenance::runtime::admit(&app, "waveform-read")?;
        let operation = crate::background_tasks::operations::Operation::start(
            &app,
            "waveforms",
            &input.recording_id,
            "Audio waveforms",
        );
        let storage = app.state::<AppStorage>();
        let recording = storage
            .with_read_connection(|connection| {
                storage::get_recording(connection, &input.recording_id)
            })
            .map_err(|error| error.to_string())?;
        let segments = storage
            .with_read_connection(|connection| {
                storage::list_recording_segments(connection, &input.recording_id)
            })
            .map_err(|error| error.to_string())?;

        let cleanup = storage
            .with_connection(|c| audio_cleanup::active_manifest(c, &recording.id))
            .map_err(|_| "Cleanup is unavailable. Retry cleanup or turn it off.")?;
        let value = waveforms::waveforms_with_cleanup(&recording, &segments, cleanup.as_ref());
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
    let lease = crate::recording_privacy::processing_lease(&input.recording_id).await;
    tauri::async_runtime::spawn_blocking(move || {
        let _lease = lease;
        let _activity = crate::maintenance::runtime::admit(&app, "waveform-read")?;
        let storage = app.state::<AppStorage>();
        let value = waveforms::viewport::read(&storage, &input)?;
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
    let lease = crate::recording_privacy::processing_lease(&input.recording_id).await;
    tauri::async_runtime::spawn_blocking(move || {
        let _lease = lease;
        let _activity = crate::maintenance::runtime::admit(&app, "playback-prepare")?;
        let operation = crate::background_tasks::operations::Operation::start(
            &app,
            "media-preparation",
            &input.recording_id,
            "Playback preparation",
        );
        let storage = app.state::<AppStorage>();
        let playback_server = app.state::<PlaybackServer>();
        let recording = storage
            .with_read_connection(|connection| {
                storage::get_recording(connection, &input.recording_id)
            })
            .map_err(|error| error.to_string())?;
        let segments = storage
            .with_read_connection(|connection| {
                storage::list_recording_segments(connection, &input.recording_id)
            })
            .map_err(|error| error.to_string())?;
        if storage::is_active_recording_status(&recording.status)
            || !segments.iter().any(|segment| {
                segment.state == "finalized"
                    && segment.duration_ms.is_some_and(|duration| duration > 0)
                    && segment.video_stream_index.is_some()
                    && Path::new(&segment.file_path).is_file()
            })
        {
            return Ok(LibraryPlaybackAssets {
                can_seek_in_stream: false,
                runs: vec![],
                error: Some("Recording is not available for playback.".to_string()),
                recording_id: recording.id,
                segments: Vec::new(),
                session_id: String::new(),
            });
        }

        let sources = storage
            .with_read_connection(|connection| {
                storage::recording_video_streams(connection, &recording.id)
            })
            .map_err(|_| "Video sources could not be read.".to_string())?;
        let result = playback_server.create_recording_source_session(
            &recording,
            &segments,
            &sources,
            storage.inner().clone(),
            input.lazy_thumbnails.unwrap_or(false),
            input.include_runs.unwrap_or(true),
        );
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
    let _activity = crate::maintenance::runtime::admit(&app, "thumbnail-prepare")?;
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
            error: path
                .is_none()
                .then(|| "Recording has no video preview.".to_string()),
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
