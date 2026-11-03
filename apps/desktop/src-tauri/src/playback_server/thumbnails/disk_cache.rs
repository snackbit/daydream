use crate::storage::{RecordingSegmentRow, SegmentVideoStream};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::AtomicBool,
    time::Duration,
};

pub(super) fn path(
    source: &str,
    duration: i64,
    stream: i64,
    offset: i64,
    namespace: &str,
) -> Result<PathBuf, String> {
    let fingerprint = crate::playback_server::prepared::fingerprint(&[(source.into(), duration)])?;
    let parent = Path::new(source)
        .parent()
        .ok_or("Thumbnail source unavailable.")?;
    Ok(parent
        .join(".daydream-thumbnails-v1")
        .join(fingerprint)
        .join(namespace)
        .join(format!("stream-{stream}"))
        .join(format!("{offset}.jpg")))
}
pub(super) fn read(path: &Path) -> Option<Vec<u8>> {
    if fs::metadata(path).ok()?.len() > 8 * 1024 * 1024 {
        return None;
    }
    let bytes = fs::read(path).ok()?;
    (bytes.starts_with(&[0xff, 0xd8]) && bytes.ends_with(&[0xff, 0xd9])).then_some(bytes)
}
pub(super) fn write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    fs::create_dir_all(path.parent().ok_or("Thumbnail cache unavailable.")?)
        .map_err(|_| "Thumbnail cache unavailable.")?;
    let temporary = path.with_extension(format!("{}.partial", uuid::Uuid::now_v7()));
    let result = fs::write(&temporary, bytes).and_then(|_| fs::rename(&temporary, path));
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result.map_err(|_| "Thumbnail cache could not be saved.".into())
}
pub(in crate::playback_server) fn prepare_segment_thumbnails(
    segment: &RecordingSegmentRow,
    sources: &BTreeMap<String, Vec<SegmentVideoStream>>,
    masks: &crate::playback_server::prepared::privacy::RecordingMasks,
    namespace: &str,
    cancel: &AtomicBool,
) -> Result<(), String> {
    let duration = segment.duration_ms.unwrap_or(0);
    let streams = sources
        .get(&segment.id)
        .map(|s| s.iter().map(|s| s.stream_index).collect::<Vec<_>>())
        .unwrap_or_else(|| segment.video_stream_index.into_iter().collect());
    let offsets = super::thumbnail_offsets(duration);
    for stream in streams {
        let eligible = offsets
            .iter()
            .copied()
            .filter(|offset| masks.allows_thumbnail(&segment.id, stream, *offset))
            .collect::<Vec<_>>();
        if eligible.is_empty() {
            continue;
        }
        let paths = eligible
            .iter()
            .map(|offset| path(&segment.file_path, duration, stream, *offset, namespace))
            .collect::<Result<Vec<_>, _>>()?;
        if paths.iter().all(|p| read(p).is_some()) {
            continue;
        }
        if eligible.len() != offsets.len() {
            let source = super::PlaybackSegment {
                file_path: segment.file_path.clone(),
                duration_ms: duration,
                fragments: vec![],
                microphone_fragments: vec![],
                video_streams: vec![],
                desktop_audio_stream_index: None,
                microphone_audio_stream_index: None,
                playback_fps: 1.,
                video_stream_index: Some(stream),
            };
            for (offset, path) in eligible.iter().zip(&paths) {
                if read(path).is_some() {
                    continue;
                }
                let mut command = Command::new(crate::ffmpeg::binary_path());
                command.args(crate::playback_server::ffmpeg_args::thumbnail_ffmpeg_args(
                    &source, stream, *offset,
                ));
                let output = crate::subprocess::run(
                    &mut command,
                    cancel,
                    Duration::from_secs(60),
                    8 * 1024 * 1024,
                )
                .map_err(|_| "Thumbnail preparation interrupted.")?;
                if !output.status.success() || !output.stdout.starts_with(&[0xff, 0xd8]) {
                    return Err("Thumbnail preparation failed.".into());
                }
                write(path, &output.stdout)?;
            }
            continue;
        }
        let directory = paths
            .first()
            .and_then(|p| p.parent())
            .ok_or("Thumbnail directory unavailable.")?;
        fs::create_dir_all(directory).map_err(|_| "Thumbnail directory unavailable.")?;
        let temporary = directory.join(format!("batch-{}", uuid::Uuid::now_v7()));
        fs::create_dir(&temporary).map_err(|_| "Thumbnail batch unavailable.")?;
        let result = (|| {
            let mut command = Command::new(crate::ffmpeg::binary_path());
            command.args(["-hide_banner","-v","error","-threads","2","-i",&segment.file_path,"-map",&format!("0:{stream}"),"-an","-vf",
                "scale=320:-1,format=yuvj420p,tpad=stop_mode=clone:stop_duration=3,fps=1/3:start_time=0:round=up",
                "-frames:v",&offsets.len().to_string(),"-filter_threads","1","-threads","1","-q:v","4"])
                .arg(temporary.join("%08d.jpg"));
            let output =
                crate::subprocess::run(&mut command, cancel, Duration::from_secs(120), 65536)
                    .map_err(|_| "Thumbnail preparation interrupted.")?;
            if !output.status.success() {
                return Err("Thumbnail preparation failed.");
            }
            for (index, path) in paths.iter().enumerate() {
                let bytes = read(&temporary.join(format!("{:08}.jpg", index + 1)))
                    .ok_or("Thumbnail preparation incomplete.")?;
                write(path, &bytes).map_err(|_| "Thumbnail cache could not be saved.")?;
            }
            Ok(())
        })();
        let _ = fs::remove_dir_all(temporary);
        result.map_err(str::to_owned)?;
    }
    Ok(())
}
