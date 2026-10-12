use super::*;
use crate::storage::{self, AppStorage};
use std::{
    fs,
    path::Path,
    sync::{atomic::AtomicBool, Arc},
};
use tauri::Manager;

pub(super) async fn preview(
    app: tauri::AppHandle,
    input: PreviewAudioCleanupInput,
) -> Result<AudioCleanupPreview, String> {
    input.config.validate()?;
    if input.start_offset_ms < 0
        || input.end_offset_ms <= input.start_offset_ms
        || input.end_offset_ms - input.start_offset_ms > 15_000
    {
        return Err("Select up to 15 seconds to preview cleanup.".into());
    }
    let activity = crate::maintenance::runtime::admit(&app, "audio-cleanup-preview")?;
    let storage = app.state::<AppStorage>().inner().clone();
    let cancellation = activity.as_ref().map(|lease| lease.cancellation());
    let cancel = Arc::new(AtomicBool::new(false));
    let monitor_cancel = cancel.clone();
    let monitor = tauri::async_runtime::spawn(async move {
        loop {
            if cancellation
                .as_ref()
                .is_some_and(|cancel| cancel.is_cancelled())
            {
                monitor_cancel.store(true, std::sync::atomic::Ordering::Release);
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    });
    let permit = match runtime::worker_slot(&cancel).await {
        Ok(permit) => permit,
        Err(error) => {
            monitor.abort();
            return Err(error);
        }
    };
    let lease = crate::recording_privacy::processing_lease(&input.recording_id).await;
    let result = tauri::async_runtime::spawn_blocking(move || {
        let (_activity, _lease, _permit) = (activity, lease, permit);
        let (recording, segments, cuts) = storage
            .with_connection(|c| {
                let recording = storage::get_recording(c, &input.recording_id)?;
                if storage::is_active_recording_status(&recording.status)
                    || !crate::recording_privacy::protection_ready(c, &recording.id)?
                {
                    return Err(rusqlite::Error::InvalidQuery);
                }
                Ok((
                    recording,
                    storage::list_recording_segments(c, &input.recording_id)?,
                    persistence::eligible_redactions(c, &input.recording_id)?,
                ))
            })
            .map_err(|_| "Finish recording before previewing cleanup.")?;
        let (segment, base) = segments
            .iter()
            .zip(storage::segment_playback_bases(&segments))
            .find(|(segment, base)| {
                *base <= input.start_offset_ms
                    && *base + segment.duration_ms.unwrap_or(0) > input.start_offset_ms
                    && segment.microphone_audio_stream_index.is_some()
            })
            .ok_or("Microphone audio is unavailable at this time.")?;
        let start = input.start_offset_ms - base;
        let end = (input.end_offset_ms - base).min(segment.duration_ms.unwrap_or(0));
        let dir = Path::new(&recording.storage_dir)
            .join("audio-cleanup")
            .join(format!("preview-{}", uuid::Uuid::now_v7()));
        fs::create_dir_all(&dir).map_err(|_| "Preview storage could not be prepared.")?;
        let prepared = (|| {
            // Render the same fixed block as Apply, then crop the requested comparison.
            let block_start = start / BLOCK_MS * BLOCK_MS;
            let block_end = (block_start + BLOCK_MS).min(segment.duration_ms.unwrap_or(0));
            let end = end.min(block_end);
            let (original, cleaned, channels, matched) = media::render_block(
                media::AudioBlockInput {
                    segment,
                    base,
                    start: block_start,
                    duration: block_end - block_start,
                },
                &cuts,
                &input.config,
                &dir,
                &cancel,
            )?;
            let from = (start - block_start) as usize * 48 * channels;
            let to = (end - block_start) as usize * 48 * channels;
            let raw = dir.join("original.flac");
            let filtered = dir.join("cleaned.flac");
            media::encode(&original[from..to], channels, &raw, &cancel)?;
            media::encode(&cleaned[from..to], channels, &filtered, &cancel)?;
            let (session, original_url, cleaned_url) = app
                .state::<crate::playback_server::PlaybackServer>()
                .create_cleanup_preview(
                    &raw,
                    &filtered,
                    end - start,
                    dir.clone(),
                    storage.clone(),
                    &input.recording_id,
                )?;
            Ok(AudioCleanupPreview {
                recording_id: input.recording_id.clone(),
                session_id: session,
                original_url,
                cleaned_url,
                matched_ms: matched.min(end - start),
                duration_ms: end - start,
            })
        })();
        if prepared.is_err() {
            let _ = fs::remove_dir_all(dir);
        }
        prepared
    })
    .await
    .unwrap_or_else(|_| Err("Audio preview worker stopped.".to_string()));
    monitor.abort();
    result
}
