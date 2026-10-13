use super::*;

pub(super) struct PriorContext {
    pub frames: Vec<(i64, PathBuf, String)>,
    pub start_ms: i64,
}
struct ContextSample {
    time: i64,
    end: i64,
    path: Option<String>,
    owner: String,
    description: String,
    dependency_start: i64,
}

fn samples(
    connection: &rusqlite::Connection,
    chunk: &ProcessingChunkRow,
    frame: &FullFrame,
    source: &str,
    fingerprint: &str,
) -> rusqlite::Result<Vec<ContextSample>> {
    let mut query = connection.prepare("SELECT frame_ms,end_ms,frame_path,chunk_id,observation_json,context_start_ms FROM studio_visual_samples WHERE job_id=?1 AND source_id=?2 AND fingerprint=?3 AND frame_ms<?4 AND observation_json IS NOT NULL ORDER BY frame_ms DESC LIMIT 3")?;
    let rows = query
        .query_map(
            rusqlite::params![chunk.job_id, source, fingerprint, frame.timestamp_ms],
            |row| {
                let time = row.get::<_, i64>(0)?;
                Ok(ContextSample {
                    time,
                    end: row.get(1)?,
                    path: row.get(2)?,
                    owner: row.get(3)?,
                    description: row.get(4)?,
                    dependency_start: row.get::<_, i64>(5)?.min(time),
                })
            },
        )?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut next = frame.timestamp_ms;
    let mut visible = Vec::new();
    for sample in rows {
        if sample.end < next
            || !eligible(
                connection,
                &chunk.recording_id,
                source,
                sample.dependency_start,
                frame.end_ms,
            )?
        {
            break;
        }
        next = sample.time;
        visible.push(sample);
    }
    visible.reverse();
    Ok(visible)
}

pub(super) fn eligible(
    connection: &rusqlite::Connection,
    recording: &str,
    source: &str,
    start: i64,
    end: i64,
) -> rusqlite::Result<bool> {
    let cuts = storage::list_redactions(connection, recording)?;
    Ok(
        !crate::recording_privacy::range_redacted(&cuts, &[source.to_owned()], start, end)
            && crate::recording_privacy::range_protection_ready(connection, recording, start, end)?,
    )
}

pub(super) async fn read(
    app: &AppHandle,
    chunk: &ProcessingChunkRow,
    frame: &FullFrame,
    source: &str,
    fingerprint: &str,
    cancel: &Arc<AtomicBool>,
) -> Result<PriorContext, AiError> {
    let storage = app.state::<AppStorage>();
    let rows = storage
        .with_connection(|connection| samples(connection, chunk, frame, source, fingerprint))
        .map_err(storage_error)?;
    let mut output = PriorContext {
        frames: Vec::new(),
        start_ms: dependency_start(&rows, frame.timestamp_ms),
    };
    for sample in rows {
        let path = if let Some(path) = sample.path.filter(|path| Path::new(path).is_file()) {
            PathBuf::from(path)
        } else {
            restore_frame(app, &sample.owner, sample.time, cancel.clone()).await?
        };
        let description = serde_json::from_str::<Change>(&sample.description)
            .ok()
            .and_then(|change| change.text)
            .unwrap_or_default();
        output.frames.push((sample.time, path, description));
    }
    Ok(output)
}

fn dependency_start(samples: &[ContextSample], current: i64) -> i64 {
    samples
        .iter()
        .fold(current, |start, sample| start.min(sample.dependency_start))
}

async fn restore_frame(
    app: &AppHandle,
    owner: &str,
    time: i64,
    cancel: Arc<AtomicBool>,
) -> Result<PathBuf, AiError> {
    let app = app.clone();
    let owner = owner.to_owned();
    tauri::async_runtime::spawn_blocking(move || {
        let storage = app.state::<AppStorage>();
        let (mut chunk, segment, base, directory) = storage.with_connection(|connection| {
            let chunk = storage::get_processing_chunk(connection, &owner)?.ok_or(rusqlite::Error::QueryReturnedNoRows)?;
            let rows = storage::list_recording_segments(connection, &chunk.recording_id)?;
            let id = chunk.segment_id.as_deref().ok_or(rusqlite::Error::InvalidQuery)?;
            let base = playback_offset_for_segment(&rows, id).ok_or(rusqlite::Error::InvalidQuery)?;
            let segment = rows.into_iter().find(|row| row.id == id).ok_or(rusqlite::Error::InvalidQuery)?;
            let segment = source_segment(connection, &chunk, segment)?;
            let recording = storage::get_recording(connection, &chunk.recording_id)?;
            Ok((chunk,segment,base,Path::new(&recording.storage_dir).join("visual-context")))
        }).map_err(storage_error)?;
        chunk.start_offset_ms = Some(time);
        chunk.end_offset_ms = Some(time.saturating_add(1));
        let frames = super::super::super::ocr_documents::extraction::extract_context_frame(&app, &chunk, &segment, base, &cancel)?;
        if frames.len() == 0 { return Err(ai_error("ai_ocr_frame_extract_failed", "Screen context frame is unavailable.")); }
        let frame = frames.frame(0)?;
        fs::create_dir_all(&directory).map_err(file_error)?;
        let path = directory.join(format!("restore-{}-{}-{}.png", owner, time, frame.pixel_hash));
        publish_image(&frame.path, &path)?;
        storage.with_connection(|connection| connection.execute("UPDATE studio_visual_samples SET frame_path=?1 WHERE chunk_id=?2 AND frame_ms=?3", rusqlite::params![path.to_string_lossy(),owner,time]).map(|_| ())).map_err(storage_error)?;
        Ok(path)
    }).await.map_err(|_| ai_error("ai_ocr_frame_extract_failed", "Screen context frame worker stopped."))?
}

#[cfg(test)]
mod tests;
