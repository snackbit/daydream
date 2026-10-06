use super::*;

pub(super) fn debug_data_for_recording(
    recording_id: &str,
    storage: &AppStorage,
) -> Result<LibraryRecordingDebugData, String> {
    ensure_debug_tools_enabled(storage)?;

    let recording = storage
        .with_connection(|connection| storage::get_recording(connection, recording_id))
        .map_err(|error| error.to_string())?;
    let segments = storage
        .with_connection(|connection| storage::list_recording_segments(connection, recording_id))
        .map_err(|error| error.to_string())?;
    let tracks = storage
        .with_connection(|connection| storage::list_recording_tracks(connection, recording_id))
        .map_err(|error| error.to_string())?;
    let jobs = storage
        .with_connection(|connection| storage::list_processing_jobs(connection, Some(recording_id)))
        .map_err(|error| error.to_string())?;
    let chunks = jobs
        .iter()
        .map(|job| {
            storage
                .with_connection(|connection| storage::list_processing_chunks(connection, &job.id))
                .map_err(|error| error.to_string())
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    let transcript_segments = ai::visible_transcript_segments_for_recording(storage, recording_id)
        .map_err(|error| format!("{}: {}", error.code, error.message))?;
    let transcript_words = transcript_segments
        .iter()
        .map(|segment| {
            storage
                .with_connection(|connection| {
                    storage::list_transcript_words(connection, &segment.id)
                })
                .map_err(|error| error.to_string())
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    let activity_intervals = storage
        .with_connection(|connection| storage::list_activity_intervals(connection, recording_id))
        .map_err(|error| error.to_string())?;
    let visual_rows = ai::visible_visual_observation_rows_for_recording(storage, recording_id)
        .map_err(|error| format!("{}: {}", error.code, error.message))?;
    let redactions = storage
        .with_connection(|connection| storage::list_redactions(connection, recording_id))
        .map_err(|error| error.to_string())?;
    let processing_state = ai::processing_state_for_recording(storage, recording_id)
        .map_err(|error| format!("{}: {}", error.code, error.message))?;
    let visual_observations = ai::visual_observations_for_recording(storage, recording_id)
        .map_err(|error| format!("{}: {}", error.code, error.message))?;
    let detail = detail_for_recording(storage, recording_id)?;

    Ok(LibraryRecordingDebugData {
        raw: json!({
            "recording": recording_row_json(&recording),
            "segments": segments.iter().map(recording_segment_row_json).collect::<Vec<_>>(),
            "tracks": tracks.iter().map(recording_track_row_json).collect::<Vec<_>>(),
            "processingJobs": jobs.iter().map(processing_job_row_json).collect::<Vec<_>>(),
            "processingChunks": chunks.iter().map(processing_chunk_row_json).collect::<Vec<_>>(),
            "ocrRegions": visual_rows.ocr_regions.iter().map(ocr_region_row_json).collect::<Vec<_>>(),
            "vlmObservations": visual_rows.vlm_observations.iter().map(vlm_observation_row_json).collect::<Vec<_>>(),
            "transcriptSegments": transcript_segments.iter().map(transcript_segment_row_json).collect::<Vec<_>>(),
            "transcriptWords": transcript_words.iter().map(transcript_word_row_json).collect::<Vec<_>>(),
            "redactions": redactions.iter().map(redaction_row_json).collect::<Vec<_>>(),
            "activityIntervals": activity_intervals.iter().map(|interval| json!({
                "chunkId": interval.chunk_id,
                "endOffsetMs": interval.end_offset_ms,
                "id": interval.id,
                "jobId": interval.job_id,
                "recordingId": interval.recording_id,
                "regionsJson": interval.regions_json,
                "score": interval.score,
                "sourceKind": interval.source_kind,
                "startOffsetMs": interval.start_offset_ms,
                "state": interval.state,
            })).collect::<Vec<_>>(),
        }),
        detail,
        processing_state,
        recording_id: recording_id.to_string(),
        visual_observations,
    })
}

pub(super) fn ensure_debug_tools_enabled(storage: &AppStorage) -> Result<(), String> {
    let app_settings = settings::load_app_settings(storage).map_err(|error| error.message)?;
    if !app_settings.debug.enabled {
        return Err("Debug tools are disabled.".to_string());
    }

    Ok(())
}

fn recording_row_json(row: &RecordingRow) -> Value {
    json!({
        "activeSceneId": row.active_scene_id,
        "createdAtMs": row.created_at_ms,
        "defaultFps": row.default_fps,
        "endedAtMs": row.ended_at_ms,
        "id": row.id,
        "outputRoot": row.output_root,
        "qualityLabel": row.quality_label,
        "startedAtMs": row.started_at_ms,
        "status": row.status,
        "storageDir": row.storage_dir,
        "title": row.title,
    })
}

fn recording_segment_row_json(row: &RecordingSegmentRow) -> Value {
    json!({
        "desktopAudioStreamIndex": row.desktop_audio_stream_index,
        "durationMs": row.duration_ms,
        "endedAtMs": row.ended_at_ms,
        "errorCode": row.error_code,
        "ffmpegExitCode": row.ffmpeg_exit_code,
        "filePath": row.file_path,
        "id": row.id,
        "microphoneAudioStreamIndex": row.microphone_audio_stream_index,
        "probeJson": row.probe_json,
        "recordingId": row.recording_id,
        "segmentIndex": row.segment_index,
        "startedAtMs": row.started_at_ms,
        "state": row.state,
        "videoStreamIndex": row.video_stream_index,
    })
}

fn recording_track_row_json(row: &RecordingTrackRow) -> Value {
    json!({
        "codec": row.codec,
        "createdAtMs": row.created_at_ms,
        "id": row.id,
        "kind": row.kind,
        "label": row.label,
        "path": row.path,
        "recordingId": row.recording_id,
        "sourceId": row.source_id,
    })
}

fn redaction_row_json(row: &RedactionRow) -> Value {
    json!({
        "createdAtMs": row.created_at_ms,
        "endOffsetMs": row.end_offset_ms,
        "id": row.id,
        "reason": row.reason,
        "recordingId": row.recording_id,
        "startOffsetMs": row.start_offset_ms,
        "trackIdsJson": row.track_ids_json,
    })
}

fn processing_job_row_json(row: &ProcessingJobRow) -> Value {
    json!({
        "createdAtMs": row.created_at_ms,
        "error": row.error,
        "id": row.id,
        "kind": row.kind,
        "recordingId": row.recording_id,
        "status": row.status,
        "updatedAtMs": row.updated_at_ms,
    })
}

fn processing_chunk_row_json(row: &ProcessingChunkRow) -> Value {
    json!({
        "attemptCount": row.attempt_count,
        "createdAtMs": row.created_at_ms,
        "endOffsetMs": row.end_offset_ms,
        "errorCode": row.error_code,
        "id": row.id,
        "jobId": row.job_id,
        "progress": row.progress,
        "recordingId": row.recording_id,
        "segmentId": row.segment_id,
        "sourceKind": row.source_kind,
        "startOffsetMs": row.start_offset_ms,
        "status": row.status,
        "trackId": row.track_id,
        "updatedAtMs": row.updated_at_ms,
    })
}

fn transcript_segment_row_json(row: &TranscriptSegmentRow) -> Value {
    json!({
        "chunkId": row.chunk_id,
        "confidence": row.confidence,
        "createdAtMs": row.created_at_ms,
        "endOffsetMs": row.end_offset_ms,
        "id": row.id,
        "jobId": row.job_id,
        "model": row.model,
        "provider": row.provider,
        "recordingId": row.recording_id,
        "segmentId": row.segment_id,
        "sourceRole": row.source_role,
        "startOffsetMs": row.start_offset_ms,
        "text": row.text,
        "trackId": row.track_id,
    })
}

fn transcript_word_row_json(row: &TranscriptWordRow) -> Value {
    json!({
        "endOffsetMs": row.end_offset_ms,
        "id": row.id,
        "probability": row.probability,
        "startOffsetMs": row.start_offset_ms,
        "text": row.text,
        "transcriptSegmentId": row.transcript_segment_id,
    })
}

fn ocr_region_row_json(row: &OcrRegionRow) -> Value {
    json!({
        "chunkId": row.chunk_id,
        "confidence": row.confidence,
        "frameTimestampMs": row.frame_timestamp_ms,
        "id": row.id,
        "jobId": row.job_id,
        "model": row.model,
        "provider": row.provider,
        "recordingId": row.recording_id,
        "regionHeight": row.region_height,
        "regionWidth": row.region_width,
        "regionX": row.region_x,
        "regionY": row.region_y,
        "sourceFrameRef": row.source_frame_ref,
        "text": row.text,
    })
}

fn vlm_observation_row_json(row: &VlmObservationRow) -> Value {
    json!({
        "chunkId": row.chunk_id,
        "confidence": row.confidence,
        "frameTimestampMs": row.frame_timestamp_ms,
        "id": row.id,
        "jobId": row.job_id,
        "model": row.model,
        "observationKind": row.observation_kind,
        "promptVersion": row.prompt_version,
        "provider": row.provider,
        "recordingId": row.recording_id,
        "regionHeight": row.region_height,
        "regionWidth": row.region_width,
        "regionX": row.region_x,
        "regionY": row.region_y,
        "sourceFrameRef": row.source_frame_ref,
        "text": row.text,
    })
}
