//! Durable local playback preparation. Foreground requests only look up files.
mod generation;
pub(super) mod lifecycle;
pub(super) mod privacy;
#[cfg(test)]
mod tests;
mod worker;
use crate::{
    library::LibraryPlaybackAssets,
    storage::{self, AppStorage, RecordingRow, RecordingSegmentRow},
};
use rusqlite::{params, OptionalExtension};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
pub(crate) use worker::cancel;
pub(crate) use worker::start;

const VERSION: &str = "studio-media-v2";

pub(crate) fn prioritize(storage: &AppStorage, id: &str) -> Result<(), String> {
    storage.with_connection(|c| c.execute(
        "INSERT INTO studio_media_queue(recording_id,priority) SELECT id,100 FROM recordings WHERE id=?1 ON CONFLICT(recording_id) DO UPDATE SET priority=100,status=CASE WHEN status='running' THEN status ELSE 'queued' END,error_code=NULL",
        [id],
    ).map(|_| ())).map_err(|_| "Playback preparation could not be queued.".into())
}

pub(super) fn fingerprint(parts: &[(String, i64)]) -> Result<String, String> {
    use sha2::{Digest, Sha256};
    let mut hash = Sha256::new();
    for (path, duration) in parts {
        let metadata = std::fs::metadata(path).map_err(|_| "Playback source is unavailable.")?;
        hash.update(path.as_bytes());
        hash.update(duration.to_le_bytes());
        hash.update(metadata.len().to_le_bytes());
        hash.update(
            metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_nanos())
                .unwrap_or(0)
                .to_le_bytes(),
        );
    }
    Ok(hex::encode(hash.finalize()))
}

fn key(segment: &str, role: &str, stream: i64) -> String {
    format!("{segment}/{role}-{stream}")
}

fn destination(
    recording: &RecordingRow,
    id: &str,
    role: &str,
    stream: i64,
    fingerprint: &str,
) -> PathBuf {
    // IDs come from SQLite, never a request path. Hash the complete key before joining.
    use sha2::{Digest, Sha256};
    let name = hex::encode(Sha256::digest(key(id, role, stream).as_bytes()));
    Path::new(&recording.storage_dir)
        .join("playback")
        .join(VERSION)
        .join(format!("{name}-{fingerprint}.mp4"))
}

pub(super) struct PreparedFiles {
    pub(super) paths: HashMap<String, PathBuf>,
    pub(super) leases: HashMap<String, std::sync::Arc<lifecycle::FileLease>>,
}

pub(super) fn attach(
    storage: &AppStorage,
    recording: &RecordingRow,
    segments: &[RecordingSegmentRow],
    assets: &mut LibraryPlaybackAssets,
    cleanup: Option<&crate::library::audio_cleanup::CleanupManifest>,
) -> Result<PreparedFiles, String> {
    // Serialize attachment with reclamation before reading saved paths.
    let mut leases = HashMap::new();
    let mut owners = lifecycle::lock()?;
    let masks = privacy::RecordingMasks::load(storage, &recording.id)?;
    let saved = storage.with_read_connection(|c| {
        let mut q=c.prepare("SELECT asset_key,source_fingerprint,path,size_bytes FROM studio_media_assets WHERE recording_id=?1")?;
        let rows=q.query_map([&recording.id],|r|Ok((r.get::<_,String>(0)?,(r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,i64>(3)?))))?.collect::<rusqlite::Result<HashMap<_,_>>>();
        rows
    }).map_err(|_| "Prepared playback metadata could not be read.")?;
    let sources = storage
        .with_read_connection(|c| storage::recording_video_streams(c, &recording.id))
        .map_err(|_| "Video sources unavailable.")?;
    let mut groups = super::runs::groups(segments, &sources)
        .into_iter()
        .map(|g| {
            (
                format!("run-{}", g[0].id),
                g.into_iter().cloned().collect::<Vec<_>>(),
            )
        })
        .collect::<HashMap<_, _>>();
    for segment in segments {
        groups.insert(segment.id.clone(), vec![segment.clone()]);
    }
    let privacy = storage
        .with_read_connection(|c| {
            crate::library::audio_cleanup::privacy_fingerprint(c, &recording.id)
        })
        .map_err(|_| "Playback privacy unavailable.")?;
    let mut files = HashMap::new();
    for asset in assets
        .segments
        .iter_mut()
        .chain(assets.runs.iter_mut().map(|r| &mut r.assets))
    {
        let Some(members) = groups.get(&asset.segment_id) else {
            continue;
        };
        let mut prepared = HashMap::new();
        let mut all_ready = true;
        for (role, stream) in asset
            .videos
            .iter()
            .map(|v| ("video", v.stream_index))
            .chain(asset.audio.iter().map(|a| {
                (
                    if a.kind == crate::library::LibraryTrackKind::Microphone {
                        "microphone"
                    } else {
                        "desktop-audio"
                    },
                    a.stream_index,
                )
            }))
        {
            if !members
                .iter()
                .all(|s| masks.allows_media(&s.id, role, stream))
            {
                all_ready = false;
                continue;
            }
            let parts = source_parts(members, role, cleanup);
            let fingerprint = format!("{VERSION}-{}-{privacy}", fingerprint(&parts)?);
            let key = key(&asset.segment_id, role, stream);
            let valid = saved.get(&key).filter(|(f, path, size)| {
                f == &fingerprint
                    && std::fs::metadata(path).is_ok_and(|m| {
                        m.is_file() && u64::try_from(*size).ok() == Some(m.len()) && *size > 0
                    })
            });
            if let Some((_, path, _)) = valid {
                prepared.insert(key, PathBuf::from(path));
            } else {
                all_ready = false;
            }
        }
        if all_ready && !asset.videos.is_empty() {
            for (role, stream, url) in asset
                .videos
                .iter_mut()
                .map(|v| ("video", v.stream_index, &mut v.url))
                .chain(asset.audio.iter_mut().map(|a| {
                    (
                        if a.kind == crate::library::LibraryTrackKind::Microphone {
                            "microphone"
                        } else {
                            "desktop-audio"
                        },
                        a.stream_index,
                        &mut a.url,
                    )
                }))
            {
                let prefix = url
                    .split_once(if role == "video" { "/video" } else { "/audio" })
                    .map(|(base, _)| base)
                    .ok_or("Playback URL unavailable.")?;
                *url = format!("{prefix}/file/{role}-{stream}");
            }
            leases.extend(
                prepared
                    .iter()
                    .map(|(key, path)| (key.clone(), owners.lease(storage, path))),
            );
            files.extend(prepared);
            asset.video_url = asset.videos[0].url.clone();
            asset.can_seek_in_stream = true;
        }
    }
    Ok(PreparedFiles {
        paths: files,
        leases,
    })
}

fn source_parts(
    members: &[RecordingSegmentRow],
    role: &str,
    cleanup: Option<&crate::library::audio_cleanup::CleanupManifest>,
) -> Vec<(String, i64)> {
    if role == "microphone" {
        if let Some(cleanup) = cleanup {
            return members
                .iter()
                .flat_map(|s| {
                    cleanup
                        .blocks
                        .iter()
                        .filter(move |b| b.segment_id == s.id)
                        .map(|b| (b.path.clone(), b.duration_ms))
                })
                .collect();
        }
    }
    members
        .iter()
        .map(|s| (s.file_path.clone(), s.duration_ms.unwrap_or(0)))
        .collect()
}

fn save(
    storage: &AppStorage,
    recording: &RecordingRow,
    id: &str,
    role: &str,
    stream: i64,
    fingerprint: &str,
    path: &Path,
) -> Result<(), String> {
    let size = i64::try_from(
        std::fs::metadata(path)
            .map_err(|_| "Prepared media unavailable.")?
            .len(),
    )
    .map_err(|_| "Prepared media is too large.")?;
    storage.with_connection(|c|c.execute("INSERT INTO studio_media_assets(recording_id,asset_key,source_fingerprint,path,size_bytes) VALUES(?1,?2,?3,?4,?5) ON CONFLICT(recording_id,asset_key) DO UPDATE SET source_fingerprint=excluded.source_fingerprint,path=excluded.path,size_bytes=excluded.size_bytes",params![recording.id,key(id,role,stream),fingerprint,path.to_string_lossy(),size]).map(|_|())).map_err(|_|"Prepared media could not be saved.".into())
}

fn cached(storage: &AppStorage, recording: &str, asset: &str, fingerprint: &str) -> bool {
    storage.with_read_connection(|c|c.query_row("SELECT path,size_bytes FROM studio_media_assets WHERE recording_id=?1 AND asset_key=?2 AND source_fingerprint=?3",params![recording,asset,fingerprint],|r|Ok((r.get::<_,String>(0)?,r.get::<_,i64>(1)?))).optional())
        .ok().flatten().is_some_and(|(path,size)|size>0&&std::fs::metadata(path).is_ok_and(|m|Some(m.len())==u64::try_from(size).ok()))
}
