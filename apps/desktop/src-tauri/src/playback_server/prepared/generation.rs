use super::*;
use crate::library::audio_cleanup::CleanupManifest;
use std::{
    fs,
    process::Command,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

pub(super) fn recording(storage: &AppStorage, id: &str, cancel: &AtomicBool) -> Result<(), String> {
    let (recording, segments, sources, cleanup, privacy) = storage
        .with_read_connection(|c| {
            Ok((
                storage::get_recording(c, id)?,
                storage::list_recording_segments(c, id)?,
                storage::recording_video_streams(c, id)?,
                crate::library::audio_cleanup::active_manifest(c, id)?,
                crate::library::audio_cleanup::privacy_fingerprint(c, id)?,
            ))
        })
        .map_err(|_| "Playback preparation metadata unavailable.")?;
    let masks = super::privacy::RecordingMasks::load(storage, id)?;
    let mut groups = segments
        .iter()
        .filter(|s| masks.segment_eligible(&s.id) && Path::new(&s.file_path).is_file())
        .map(|s| (s.id.clone(), vec![s.clone()]))
        .collect::<Vec<_>>();
    if !storage::is_active_recording_status(&recording.status) {
        groups.extend(
            crate::playback_server::runs::groups(&segments, &sources)
                .into_iter()
                .filter(|group| group.iter().all(|s| masks.segment_eligible(&s.id)))
                .map(|g| (format!("run-{}", g[0].id), g.into_iter().cloned().collect())),
        );
    }
    for (group, members) in groups {
        if cancel.load(Ordering::Acquire) {
            return Err("Playback preparation cancelled.".into());
        }
        let first = &members[0];
        let videos = sources
            .get(&first.id)
            .map(|v| v.iter().map(|v| v.stream_index).collect::<Vec<_>>())
            .unwrap_or_else(|| first.video_stream_index.into_iter().collect());
        for (role, stream) in videos
            .into_iter()
            .map(|v| ("video", v))
            .chain(
                first
                    .desktop_audio_stream_index
                    .map(|v| ("desktop-audio", v)),
            )
            .chain(
                first
                    .microphone_audio_stream_index
                    .map(|v| ("microphone", v)),
            )
        {
            if !members
                .iter()
                .all(|s| masks.allows_media(&s.id, role, stream))
            {
                storage
                    .with_connection(|c| {
                        c.execute(
                            "DELETE FROM studio_media_assets
                             WHERE recording_id=?1 AND asset_key=?2",
                            params![id, key(&group, role, stream)],
                        )
                    })
                    .map_err(|_| "Hidden playback metadata could not be removed.")?;
                continue;
            }
            let parts = source_parts(&members, role, cleanup.as_ref());
            if parts.is_empty() {
                return Err("Playback sources unavailable.".into());
            }
            let fingerprint = format!("{VERSION}-{}-{privacy}", fingerprint(&parts)?);
            if cached(storage, id, &key(&group, role, stream), &fingerprint) {
                continue;
            }
            if members.len() == 1 && group.starts_with("run-") {
                let existing=storage.with_read_connection(|c|c.query_row("SELECT path FROM studio_media_assets WHERE recording_id=?1 AND asset_key=?2 AND source_fingerprint=?3",params![id,key(&first.id,role,stream),fingerprint],|r|r.get::<_,String>(0)).optional()).map_err(|_|"Prepared segment unavailable.")?;
                if let Some(path) = existing {
                    save(
                        storage,
                        &recording,
                        &group,
                        role,
                        stream,
                        &fingerprint,
                        Path::new(&path),
                    )?;
                    continue;
                }
            }
            let path = destination(&recording, &group, role, stream, &fingerprint);
            // Own the output through publication so a retiring session cannot
            // unlink a reused fingerprint between rename and metadata save.
            let _output_lease = super::lifecycle::lock()?.lease(storage, &path);
            prepare(
                &recording,
                &parts,
                role,
                stream,
                cleanup.as_ref(),
                &path,
                cancel,
            )?;
            save(
                storage,
                &recording,
                &group,
                role,
                stream,
                &fingerprint,
                &path,
            )?;
        }
        // Physical segments own thumbnails, including sparse-frame hold semantics.
        if members.len() == 1 && !group.starts_with("run-") {
            super::super::thumbnails::prepare_segment_thumbnails(
                first, &sources, &masks, &privacy, cancel,
            )?;
        }
    }
    storage
        .with_connection(|c| {
            c.execute(
                "DELETE FROM studio_media_assets
                 WHERE recording_id=?1 AND source_fingerprint NOT LIKE ?2",
                params![id, format!("%-{privacy}")],
            )
        })
        .map_err(|_| "Stale playback metadata could not be removed.")?;
    super::lifecycle::reclaim(storage, &recording)?;
    let size = i64::try_from(crate::library::recording_directory_size(Path::new(
        &recording.storage_dir,
    ))?)
    .map_err(|_| "Recording size is too large.")?;
    storage.with_connection(|c|c.execute("INSERT INTO recording_disk_usage(recording_id,size_bytes,measured_at_ms) VALUES(?1,?2,?3) ON CONFLICT(recording_id) DO UPDATE SET size_bytes=excluded.size_bytes,measured_at_ms=excluded.measured_at_ms",params![id,size,crate::background_tasks::now_ms()]).map(|_|())).map_err(|_|"Storage accounting could not be saved.".into())
}

fn prepare(
    recording: &RecordingRow,
    parts: &[(String, i64)],
    role: &str,
    stream: i64,
    cleanup: Option<&CleanupManifest>,
    path: &Path,
    cancel: &AtomicBool,
) -> Result<(), String> {
    let directory = path.parent().ok_or("Playback directory unavailable.")?;
    fs::create_dir_all(directory).map_err(|_| "Playback directory unavailable.")?;
    let temporary = directory.join(format!("{}.partial.mp4", uuid::Uuid::now_v7()));
    let manifest = directory.join(format!("{}.ffconcat", uuid::Uuid::now_v7()));
    let input = if parts.len() == 1 {
        vec!["-i".into(), parts[0].0.clone()]
    } else {
        fs::write(
            &manifest,
            crate::playback_server::runs::concat_manifest(parts)?,
        )
        .map_err(|_| "Playback manifest unavailable.")?;
        vec![
            "-f".into(),
            "concat".into(),
            "-safe".into(),
            "0".into(),
            "-i".into(),
            manifest.to_string_lossy().into_owned(),
        ]
    };
    let duration = parts.iter().map(|(_, duration)| duration).sum::<i64>();
    let cleaned = role == "microphone" && cleanup.is_some();
    let result = (|| {
        let command = |copy: bool| {
            let mut command = Command::new(crate::ffmpeg::binary_path());
            command
                .args(["-hide_banner", "-v", "error", "-y", "-threads", "2"])
                .args(&input)
                .args([
                    "-map",
                    &format!("0:{}", if cleaned { 0 } else { stream }),
                    "-t",
                    &format!("{:.3}", duration as f64 / 1000.),
                ]);
            if role == "video" {
                command.arg("-an");
                if copy {
                    // Preserve encoded frames and PTS/DTS, but bound the final
                    // packet's display duration to the recording timeline.
                    // Explicit PTS/DTS matter: setts' default TS loses B-frame order.
                    command.args(["-c:v", "copy", "-bsf:v", &format!(
                        "setts=pts=PTS:dts=DTS:duration='if(eq(NEXT_DTS,NOPTS),max(1,{duration}/1000/TB-PTS),max(1,min(DURATION,{duration}/1000/TB-PTS)))'"
                    )]);
                } else {
                    super::super::encoders::append_video_encoder_args_with_tail(
                        &mut command,
                        &super::super::encoders::VideoEncoder::Libx264,
                        recording.default_fps,
                        duration,
                    );
                    command.args(["-force_key_frames", "expr:gte(t,n_forced*1)"]);
                }
            } else {
                command.args([
                    "-vn",
                    "-af",
                    &crate::ffmpeg::audio_timeline_filter(48_000, duration),
                    "-c:a",
                    "aac",
                    "-b:a",
                    "128k",
                ]);
            }
            command
                .args(["-movflags", "+faststart", "-f", "mp4"])
                .arg(&temporary);
            command
        };
        let execute = |copy| {
            crate::subprocess::run(&mut command(copy), cancel, Duration::from_secs(1800), 65536)
                .map_err(|_| "Playback preparation interrupted.")
        };
        let copied = execute(true)?;
        if !copied.status.success() && (role != "video" || !execute(false)?.status.success()) {
            return Err("Playback preparation failed.");
        }
        if cancel.load(Ordering::Acquire) {
            return Err("Playback preparation cancelled.");
        }
        if fs::metadata(&temporary).map_or(0, |m| m.len()) == 0 {
            return Err("Playback preparation created no media.");
        }
        fs::rename(&temporary, path).map_err(|_| "Playback cache could not be published.")
    })();
    let _ = fs::remove_file(&manifest);
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result.map_err(str::to_owned)
}
