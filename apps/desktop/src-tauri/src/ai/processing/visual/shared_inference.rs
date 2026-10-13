//! One inference owner publishes independently validated OCR/observation sections.
use super::super::super::*;
use super::super::ocr_documents::{extraction::FullFrame, recognition_lease};
use super::{
    change_protocol::{self, Change},
    VisionRequestConfig,
};
use crate::storage::OcrContentInput;
use rusqlite::OptionalExtension;
mod context;
use sha2::{Digest, Sha256};

pub(in crate::ai) fn enabled(storage: &AppStorage, job: &str) -> Result<bool, AiError> {
    storage.with_connection(|connection| connection.query_row(
        "SELECT visual_protocol='changes-v1' FROM processing_job_runtime WHERE job_id=?1", [job], |row| row.get(0),
    ).optional().map(|value| value.unwrap_or(false))).map_err(storage_error)
}
pub(in crate::ai) fn matching_observation(
    storage: &AppStorage,
    chunk: &ProcessingChunkRow,
    config: &VisionRequestConfig,
    timestamp: i64,
) -> Result<bool, AiError> {
    if !enabled(storage, &chunk.job_id)? {
        return Ok(false);
    }
    let peers = storage
        .with_connection(|connection| storage::list_processing_chunks(connection, &chunk.job_id))
        .map_err(storage_error)?;
    for peer in peers.into_iter().filter(|peer| {
        peer.source_kind == "vlm"
            && peer.track_id == chunk.track_id
            && peer.segment_id == chunk.segment_id
            && peer.status != "cancelled"
            && peer.start_offset_ms.is_some_and(|start| start <= timestamp)
            && peer.end_offset_ms.is_some_and(|end| end > timestamp)
    }) {
        let settings = super::super::worker::load_chunk_settings(storage, &peer)?;
        if !settings.ai.vision.observations_enabled {
            continue;
        }
        let recorded_fps = storage
            .with_connection(|connection| {
                storage::get_recording(connection, &chunk.recording_id)
                    .map(|recording| recording.default_fps)
            })
            .map_err(storage_error)?;
        let fps = if settings.ai.processing.frame_recognition_sync_with_recording {
            recorded_fps
        } else {
            settings.ai.processing.frame_recognition_fps
        };
        let delta = timestamp - peer.start_offset_ms.unwrap_or(0);
        let sample = (delta as f64 * fps / 1000.0).round();
        if (sample * 1000.0 / fps).round() as i64 != delta {
            continue;
        }
        let other = super::vlm::vision_request_config(storage, &settings)?;
        if fingerprint(config) == fingerprint(&other) {
            return Ok(true);
        }
    }
    Ok(false)
}
pub(super) fn fingerprint(config: &VisionRequestConfig) -> String {
    let material = serde_json::json!([
        change_protocol::VERSION,
        config.provider,
        config.model,
        config.base_url,
        config.api_key,
        config.route
    ]);
    format!("{:x}", Sha256::digest(material.to_string().as_bytes()))
}

pub(in crate::ai) struct Inference {
    pub ocr: Option<OcrContentInput>,
    pub(super) change: Option<Change>,
    pub context_start: i64,
}
pub(in crate::ai) async fn infer(
    app: &AppHandle,
    chunk: &ProcessingChunkRow,
    frame: &FullFrame,
    config: &VisionRequestConfig,
    want_ocr: bool,
    want_change: bool,
    cancel: Arc<AtomicBool>,
) -> Result<Inference, AiError> {
    let storage = app.state::<AppStorage>();
    let source = chunk
        .track_id
        .as_deref()
        .ok_or_else(|| ai_error("ai_ocr_source_invalid", "Screen source is unavailable."))?;
    let fingerprint = fingerprint(config);
    let _sequence = recognition_lease::acquire(
        &chunk.recording_id,
        source,
        "screen-sequence",
        &fingerprint,
        cancel.clone(),
    )
    .await?;
    ensure_chunk_active(&storage, chunk)?;
    let cached = storage.with_connection(|connection| connection.query_row(
        "SELECT ocr_json,observation_json,context_start_ms FROM studio_visual_samples WHERE job_id=?1 AND source_id=?2 AND frame_ms=?3 AND fingerprint=?4 AND pixel_hash=?5",
        rusqlite::params![chunk.job_id,source,frame.timestamp_ms,fingerprint,frame.pixel_hash],
        |row| Ok((row.get::<_,Option<String>>(0)?,row.get::<_,Option<String>>(1)?,row.get::<_,i64>(2)?)),
    ).optional()).map_err(storage_error)?;
    let mut output = Inference {
        ocr: None,
        change: None,
        context_start: frame.timestamp_ms,
    };
    if let Some((ocr, observation, start)) = cached {
        output.ocr = ocr.and_then(|value| serde_json::from_str(&value).ok());
        let context_visible = storage
            .with_connection(|connection| {
                let cuts = storage::list_redactions(connection, &chunk.recording_id)?;
                Ok(!crate::recording_privacy::range_redacted(
                    &cuts,
                    &[source.to_owned()],
                    start,
                    frame.end_ms,
                ) && crate::recording_privacy::range_protection_ready(
                    connection,
                    &chunk.recording_id,
                    start,
                    frame.end_ms,
                )?)
            })
            .map_err(storage_error)?;
        output.change = if context_visible {
            observation.and_then(|value| serde_json::from_str(&value).ok())
        } else {
            None
        };
        output.context_start = start;
        if (!want_ocr || output.ocr.is_some()) && (!want_change || output.change.is_some()) {
            return Ok(output);
        }
    }
    let need_change = want_change && output.change.is_none();
    let need_ocr = want_ocr && output.ocr.is_none();
    let prior = if need_change {
        context::read(app, chunk, frame, source, &fingerprint, &cancel).await?
    } else {
        context::PriorContext {
            frames: Vec::new(),
            start_ms: frame.timestamp_ms,
        }
    };
    if need_change {
        output.context_start = prior.start_ms;
    }
    super::vlm::verify_current_vision_configuration(&storage, config, chunk.source_kind == "ocr")?;
    if want_ocr && want_change {
        super::vlm::verify_current_vision_configuration(&storage, config, false)?;
    }
    // Restoring a pruned image awaits I/O; revalidate the complete inherited range.
    ensure_chunk_active(&storage, chunk)?;
    if need_change
        && !storage
            .with_connection(|connection| {
                context::eligible(
                    connection,
                    &chunk.recording_id,
                    source,
                    output.context_start,
                    frame.end_ms,
                )
            })
            .map_err(storage_error)?
    {
        return Err(ai_error(
            "ai_processing_cancelled",
            "Screen context eligibility changed.",
        ));
    }
    let (ocr, change) = change_protocol::request(
        config,
        frame,
        &prior.frames,
        need_ocr,
        need_change,
        cancel.clone(),
    )
    .await?;
    ensure_chunk_active(&storage, chunk)?;
    check_processing_cancel(&cancel)?;
    if need_change
        && !storage
            .with_connection(|connection| {
                context::eligible(
                    connection,
                    &chunk.recording_id,
                    source,
                    output.context_start,
                    frame.end_ms,
                )
            })
            .map_err(storage_error)?
    {
        return Err(ai_error(
            "ai_processing_cancelled",
            "Screen context eligibility changed.",
        ));
    }
    if need_ocr {
        output.ocr = ocr;
    }
    if need_change {
        output.change = change;
    }
    if need_change
        && prior.frames.is_empty()
        && output
            .change
            .as_ref()
            .is_some_and(|change| change.status != change_protocol::ChangeStatus::Initial)
    {
        output.change = None;
    }
    let recording = storage
        .with_connection(|connection| storage::get_recording(connection, &chunk.recording_id))
        .map_err(storage_error)?;
    let directory = Path::new(&recording.storage_dir)
        .join("visual-context")
        .join(&chunk.job_id)
        .join(&fingerprint);
    fs::create_dir_all(&directory).map_err(file_error)?;
    let path = directory.join(format!(
        "{:x}-{}-{}.png",
        Sha256::digest(source.as_bytes()),
        frame.timestamp_ms,
        frame.pixel_hash
    ));
    publish_image(&frame.path, &path)?;
    let ocr = output
        .ocr
        .as_ref()
        .map(serde_json::to_string)
        .transpose()
        .map_err(|_| {
            ai_error(
                "ai_processing_config_invalid",
                "Screen result could not be saved.",
            )
        })?;
    let change = output
        .change
        .as_ref()
        .map(serde_json::to_string)
        .transpose()
        .map_err(|_| {
            ai_error(
                "ai_processing_config_invalid",
                "Screen result could not be saved.",
            )
        })?;
    storage.with_connection(|connection| {
        connection.execute("INSERT INTO studio_visual_samples(job_id,recording_id,source_id,frame_ms,end_ms,fingerprint,pixel_hash,frame_path,chunk_id,ocr_json,observation_json,context_start_ms,width,height,source_frame_ref) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)
            ON CONFLICT(job_id,source_id,frame_ms,fingerprint) DO UPDATE SET ocr_json=excluded.ocr_json,observation_json=excluded.observation_json,frame_path=excluded.frame_path,context_start_ms=excluded.context_start_ms,pixel_hash=excluded.pixel_hash",
            rusqlite::params![chunk.job_id,chunk.recording_id,source,frame.timestamp_ms,frame.end_ms,fingerprint,frame.pixel_hash,path.to_string_lossy(),chunk.id,ocr,change,output.context_start,frame.width,frame.height,frame.source_ref])?;
        Ok(())
    }).map_err(storage_error)?;
    // Keep four images per source/model. Metadata survives for retry/provenance.
    prune_images(&storage, &chunk.job_id, source, &fingerprint)?;
    Ok(output)
}

fn prune_images(
    storage: &AppStorage,
    job: &str,
    source: &str,
    fingerprint: &str,
) -> Result<(), AiError> {
    let rows = storage.with_connection(|connection| {
        let mut query = connection.prepare("SELECT frame_ms,frame_path FROM studio_visual_samples WHERE job_id=?1 AND source_id=?2 AND fingerprint=?3 AND frame_path IS NOT NULL ORDER BY frame_ms DESC LIMIT -1 OFFSET 4")?;
        let rows = query.query_map(rusqlite::params![job,source,fingerprint], |row| Ok((row.get::<_,i64>(0)?,row.get::<_,String>(1)?)))?.collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }).map_err(storage_error)?;
    for (time, path) in rows {
        if Path::new(&path).is_file() {
            fs::remove_file(&path).map_err(file_error)?;
        }
        storage.with_connection(|connection| connection.execute("UPDATE studio_visual_samples SET frame_path=NULL WHERE job_id=?1 AND source_id=?2 AND fingerprint=?3 AND frame_ms=?4", rusqlite::params![job,source,fingerprint,time]).map(|_| ())).map_err(storage_error)?;
    }
    Ok(())
}

fn publish_image(source: &Path, destination: &Path) -> Result<(), AiError> {
    let temporary = destination.with_extension(format!("{}.tmp", Uuid::now_v7()));
    let result = (|| {
        fs::copy(source, &temporary)?;
        fs::File::open(&temporary)?.sync_all()?;
        fs::rename(&temporary, destination)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result.map_err(file_error)
}
