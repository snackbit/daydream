use super::*;
use crate::storage::{self, AppStorage};
use rusqlite::{params, OptionalExtension};
use std::{
    fs,
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};
use tauri::Manager;

pub(crate) async fn cancel_recording(storage: &AppStorage, id: &str) -> Result<(), String> {
    runtime::cancel(id).await?;
    storage.with_connection(|c| c.execute("UPDATE audio_cleanup_generations SET status='cancelled',error=NULL WHERE recording_id=?1 AND status='running'",[id]))
        .map_err(|_| "Cleanup cancellation could not be saved.")?;
    Ok(())
}

pub(crate) fn read_state(storage: &AppStorage, id: &str) -> Result<AudioCleanupState, String> {
    storage.with_connection(|c| {
        let segments = storage::list_recording_segments(c,id)?;
        storage::get_recording(c,id)?;
        let total: i64 = segments.iter().filter(|part| part.microphone_audio_stream_index.is_some()).map(|part|part.duration_ms.unwrap_or(0)).sum();
        let revisions: (Option<String>,Option<String>) = c.query_row("SELECT active_revision,pending_revision FROM recording_audio_cleanup WHERE recording_id=?1",[id],|r|Ok((r.get(0)?,r.get(1)?))).optional()?.unwrap_or_default();
        let mut state = AudioCleanupState { recording_id:id.into(),config:AudioCleanupConfig::default(),pending_config:None,
            revision:revisions.0.clone(),phase:"idle".into(),progress:0.0,matched_ms:0,
            reference_available:segments.iter().any(|part|part.microphone_audio_stream_index.is_some()&&part.desktop_audio_stream_index.is_some()),transcripts_outdated:false,error:None };
        let saved_config:Option<String>=c.query_row("SELECT config_json FROM recording_audio_cleanup WHERE recording_id=?1",[id],|r|r.get(0)).optional()?;
        if let Some(config)=saved_config {state.config=serde_json::from_str(&config).map_err(|_|rusqlite::Error::InvalidQuery)?;}
        for (revision,pending) in [(revisions.0,false),(revisions.1,true)] {
            let Some(revision) = revision else { continue; };
            let (config,status,completed,matched,error,saved): (String,String,i64,i64,Option<String>,String) = c.query_row(
                "SELECT config_json,status,completed_ms,matched_ms,error,source_fingerprint FROM audio_cleanup_generations WHERE id=?1",[&revision],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?)))?;
            let config:AudioCleanupConfig = serde_json::from_str(&config).map_err(|_|rusqlite::Error::InvalidQuery)?;
            if pending { state.pending_config=Some(config); } else { state.config=config; }
            state.phase=if saved!=persistence::fingerprint(c,id)? { "invalidated".into() } else { status };
            state.progress=if total>0 { (completed as f64/total as f64).clamp(0.0,1.0) } else { 0.0 };
            state.matched_ms=matched; state.error=error;
        }
        state.transcripts_outdated=c.query_row("SELECT EXISTS(SELECT 1 FROM published_transcript_segments t WHERE t.recording_id=?1 AND t.source_role='microphone' AND COALESCE((SELECT revision FROM processing_audio_cleanup_inputs i WHERE i.job_id=t.job_id),'')!=COALESCE(?2,''))",params![id,state.revision],|r|r.get(0))?;
        Ok(state)
    }).map_err(|_| "Audio cleanup settings could not be read.".into())
}

pub(super) async fn apply(
    app: tauri::AppHandle,
    input: ApplyAudioCleanupInput,
) -> Result<AudioCleanupState, String> {
    input.config.validate()?;
    uuid::Uuid::parse_str(&input.recording_id).map_err(|_| "Recording identity is invalid.")?;
    let activity = crate::maintenance::runtime::admit(&app, "audio-cleanup")?;
    let storage = app.state::<AppStorage>().inner().clone();
    if input.config.mode == CleanupMode::Off {
        cancel_recording(&storage, &input.recording_id).await?;
        let config =
            serde_json::to_string(&input.config).map_err(|_| "Cleanup settings are invalid.")?;
        storage.with_connection(|c| c.execute("INSERT INTO recording_audio_cleanup(recording_id,config_json) VALUES(?1,?2) ON CONFLICT(recording_id) DO UPDATE SET active_revision=NULL,pending_revision=NULL,config_json=excluded.config_json",params![input.recording_id,config])).map_err(|_| "Cleanup settings could not be saved.")?;
        return read_state(&storage, &input.recording_id);
    }
    let control = runtime::Control::register(&input.recording_id)?;
    let prepared=storage.with_connection_mut(|c| {
        let recording=storage::get_recording(c,&input.recording_id)?;
        if storage::is_active_recording_status(&recording.status)||!crate::recording_privacy::protection_ready(c,&recording.id)? { return Err(rusqlite::Error::InvalidQuery); }
        let segments=storage::list_recording_segments(c,&recording.id)?;
        if !segments.iter().any(|s|s.microphone_audio_stream_index.is_some()&&s.desktop_audio_stream_index.is_some()) { return Err(rusqlite::Error::InvalidQuery); }
        let fingerprint=persistence::fingerprint(c,&recording.id)?;
        let config=serde_json::to_string(&input.config).map_err(|_|rusqlite::Error::InvalidQuery)?;
        let previous:Option<(String,String)>=c.query_row("SELECT id,manifest_json FROM audio_cleanup_generations WHERE recording_id=?1 AND source_fingerprint=?2 AND config_json=?3 AND status IN ('failed','cancelled') ORDER BY created_at_ms DESC LIMIT 1",params![recording.id,fingerprint,config],|r|Ok((r.get(0)?,r.get(1)?))).optional()?;
        let (revision,json)=previous.unwrap_or_else(||(uuid::Uuid::now_v7().to_string(),"[]".into()));
        let blocks:Vec<CleanupBlock>=serde_json::from_str(&json).map_err(|_|rusqlite::Error::InvalidQuery)?;
        let tx=c.transaction()?;
        tx.execute("INSERT INTO audio_cleanup_generations(id,recording_id,config_json,source_fingerprint,status,created_at_ms) VALUES(?1,?2,?3,?4,'running',?5) ON CONFLICT(id) DO UPDATE SET status='running',error=NULL",params![revision,recording.id,config,fingerprint,super::super::now_ms()])?;
        tx.execute("INSERT INTO recording_audio_cleanup(recording_id,pending_revision) VALUES(?1,?2) ON CONFLICT(recording_id) DO UPDATE SET pending_revision=excluded.pending_revision",params![recording.id,revision])?;
        tx.commit()?; Ok((revision,fingerprint,blocks))
    });
    let (revision, fingerprint, blocks) = match prepared {
        Ok(value) => value,
        Err(_) => {
            control.finish(&input.recording_id);
            return Err("Finish recording and ensure both audio feeds are available before applying cleanup.".into());
        }
    };
    let state = match read_state(&storage, &input.recording_id) {
        Ok(state) => state,
        Err(error) => {
            let _=storage.with_connection(|c|c.execute("UPDATE audio_cleanup_generations SET status='failed' WHERE id=?1 AND status='running'",[&revision]));
            control.finish(&input.recording_id);
            return Err(error);
        }
    };
    let app_copy = app.clone();
    tauri::async_runtime::spawn(async move {
        let maintenance = activity.as_ref().map(|lease| lease.cancellation());
        let monitor_control = control.clone();
        let monitor = tauri::async_runtime::spawn(async move {
            while !monitor_control.is_done() {
                if maintenance
                    .as_ref()
                    .is_some_and(|value| value.is_cancelled())
                {
                    monitor_control.cancel.store(true, Ordering::Release);
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }
        });
        let result = if let Ok(permit) = runtime::worker_slot(&control.cancel).await {
            let lease = crate::recording_privacy::processing_lease(&input.recording_id).await;
            let cancel = control.cancel.clone();
            let worker_storage = storage.clone();
            let worker_id = input.recording_id.clone();
            let worker_revision = revision.clone();
            tauri::async_runtime::spawn_blocking(move || {
                let (_activity, _permit, _lease) = (activity, permit, lease);
                generate(
                    &worker_storage,
                    &worker_id,
                    &worker_revision,
                    &fingerprint,
                    &input.config,
                    blocks,
                    &cancel,
                )
            })
            .await
            .unwrap_or_else(|_| Err("Cleanup worker stopped. Retry to resume.".into()))
        } else {
            Err("Cleanup was cancelled.".into())
        };
        if let Err(error) = result {
            let phase = if control.cancel.load(Ordering::Acquire) {
                "cancelled"
            } else {
                "failed"
            };
            let _=storage.with_connection(|c|c.execute("UPDATE audio_cleanup_generations SET status=?2,error=?3 WHERE id=?1 AND status='running'",params![revision,phase,error]));
        }
        control.finish(&input.recording_id);
        monitor.abort();
        crate::ai::emit_processing_state(&app_copy, &input.recording_id);
    });
    Ok(state)
}

pub(super) fn generate(
    storage: &AppStorage,
    id: &str,
    revision: &str,
    fingerprint: &str,
    config: &AudioCleanupConfig,
    mut blocks: Vec<CleanupBlock>,
    cancel: &AtomicBool,
) -> Result<(), String> {
    if storage
        .with_connection(|c| persistence::fingerprint(c, id))
        .map_err(|_| "Cleanup sources could not be read.")?
        != fingerprint
    {
        return Err("Cleanup sources changed. Apply cleanup again.".into());
    }
    let (recording, segments, redactions) = storage
        .with_connection(|c| {
            Ok((
                storage::get_recording(c, id)?,
                storage::list_recording_segments(c, id)?,
                persistence::eligible_redactions(c, id)?,
            ))
        })
        .map_err(|_| "Cleanup sources could not be read.")?;
    let dir = Path::new(&recording.storage_dir)
        .join("audio-cleanup")
        .join(revision);
    fs::create_dir_all(&dir).map_err(|_| "Cleanup storage could not be prepared.")?;
    for (segment, base) in segments
        .iter()
        .zip(storage::segment_playback_bases(&segments))
    {
        if segment.microphone_audio_stream_index.is_none() {
            continue;
        }
        let duration = segment
            .duration_ms
            .filter(|value| *value > 0)
            .ok_or("Audio duration is unavailable.")?;
        if segment.state != "finalized" || !Path::new(&segment.file_path).is_file() {
            return Err("Some microphone media is unavailable.".into());
        }
        if blocks.iter().any(|block| {
            block.segment_id == segment.id
                && block.start_ms == 0
                && block.duration_ms == duration
                && Path::new(&block.path).is_file()
        }) {
            continue;
        }
        for start in (0..duration).step_by(BLOCK_MS as usize) {
            check(cancel)?;
            if blocks.iter().any(|block| {
                block.segment_id == segment.id
                    && block.start_ms == start
                    && Path::new(&block.path).is_file()
            }) {
                continue;
            }
            blocks.retain(|block| !(block.segment_id == segment.id && block.start_ms == start));
            let length = BLOCK_MS.min(duration - start);
            let (_, cleaned, channels, matched_ms) = media::render_block(
                media::AudioBlockInput {
                    segment,
                    base,
                    start,
                    duration: length,
                },
                &redactions,
                config,
                &dir,
                cancel,
            )?;
            let path = dir.join(format!("{}-{start}.flac", segment.id));
            media::encode(&cleaned, channels, &path, cancel)?;
            blocks.push(CleanupBlock {
                segment_id: segment.id.clone(),
                start_ms: start,
                duration_ms: length,
                path: super::super::path_to_string(&path),
                matched_ms,
            });
            persistence::save_checkpoint(storage, revision, &blocks)?;
        }
        consolidate(&dir, segment, &mut blocks, cancel)?;
        persistence::save_checkpoint(storage, revision, &blocks)?;
    }
    check(cancel)?;
    storage.with_connection_mut(|c|{
        if persistence::fingerprint(c,id)?!=fingerprint{return Err(rusqlite::Error::InvalidQuery);}
        let tx=c.transaction()?;
        let changed=tx.execute("UPDATE audio_cleanup_generations SET status='ready' WHERE id=?1 AND status='running'",[revision])?;
        if changed!=1{return Err(rusqlite::Error::InvalidQuery);}
        let changed=tx.execute("UPDATE recording_audio_cleanup SET active_revision=?2,pending_revision=NULL,config_json=(SELECT config_json FROM audio_cleanup_generations WHERE id=?2) WHERE recording_id=?1 AND pending_revision=?2",params![id,revision])?;
        if changed!=1{return Err(rusqlite::Error::InvalidQuery);}
        tx.commit()
    }).map_err(|_|"Cleanup sources changed. Retry with the current recording.")?;
    Ok(())
}
fn consolidate(
    dir: &Path,
    segment: &storage::RecordingSegmentRow,
    blocks: &mut Vec<CleanupBlock>,
    cancel: &AtomicBool,
) -> Result<(), String> {
    let mut parts: Vec<_> = blocks
        .iter()
        .filter(|block| block.segment_id == segment.id)
        .cloned()
        .collect();
    parts.sort_by_key(|block| block.start_ms);
    let manifest = media::TemporaryFile(dir.join("concat.ffconcat"));
    media::private_file(&manifest.0)?;
    let text = crate::playback_server::cleanup_concat_manifest(
        &parts
            .iter()
            .map(|part| (part.path.clone(), part.duration_ms))
            .collect::<Vec<_>>(),
    )?;
    fs::write(&manifest.0, text).map_err(|_| "Cleanup audio could not be assembled.")?;
    let path = dir.join(format!("{}.flac", segment.id));
    let staged = media::TemporaryFile(path.with_extension("pending.flac"));
    media::private_file(&staged.0)?;
    let mut command = std::process::Command::new(crate::ffmpeg::binary_path());
    command
        .args([
            "-hide_banner",
            "-v",
            "error",
            "-y",
            "-f",
            "concat",
            "-safe",
            "0",
            "-i",
        ])
        .arg(&manifest.0)
        .args(["-c:a", "flac", "-threads", "2", "-f", "flac"])
        .arg(&staged.0);
    let result = crate::subprocess::run(
        &mut command,
        cancel,
        std::time::Duration::from_secs(120),
        64_000,
    )
    .map_err(|_| "Cleanup assembly did not finish.")?;
    if !result.status.success() {
        return Err("Cleanup audio could not be assembled.".into());
    }
    fs::rename(&staged.0, &path).map_err(|_| "Cleanup audio could not be published.")?;
    blocks.retain(|block| block.segment_id != segment.id);
    blocks.push(CleanupBlock {
        segment_id: segment.id.clone(),
        start_ms: 0,
        duration_ms: segment.duration_ms.unwrap_or(0),
        path: super::super::path_to_string(&path),
        matched_ms: parts.iter().map(|part| part.matched_ms).sum(),
    });
    // Keep checkpoint inputs until the complete revision is durably published.
    Ok(())
}
fn check(cancel: &AtomicBool) -> Result<(), String> {
    if cancel.load(Ordering::Acquire) {
        Err("Cleanup was cancelled.".into())
    } else {
        Ok(())
    }
}
