use super::{types::*, write::normalize_ocr_text};
use crate::{
    ai::VisualRegion,
    storage::{self, OcrRegionRow},
};
use rusqlite::{params, Transaction};
use std::collections::BTreeMap;

/// Upgrade reconstructs readable geometry only. It never performs recognition
/// or invents frame coverage, hierarchy, or original image dimensions.
pub(in crate::storage) fn reconstruct_legacy_documents(
    transaction: &Transaction<'_>,
) -> rusqlite::Result<()> {
    let recordings = storage::list_recordings(transaction)?;
    for recording in recordings {
        let segments = storage::list_recording_segments(transaction, &recording.id)?;
        // Stream frame identities and reconstruct one frame at a time. Existing
        // full-day databases must not load every retained word into memory.
        let mut keys=transaction.prepare("SELECT DISTINCT chunk_id,frame_timestamp_ms FROM ocr_regions WHERE recording_id=?1 ORDER BY chunk_id,frame_timestamp_ms")?;
        let keys = keys.query_map([&recording.id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;
        for key in keys {
            let (chunk_id, timestamp) = key?;
            let mut statement=transaction.prepare("SELECT id,recording_id,frame_timestamp_ms,source_frame_ref,region_x,region_y,region_width,region_height,text,confidence,provider,model,job_id,chunk_id FROM ocr_regions WHERE chunk_id=?1 AND frame_timestamp_ms=?2 ORDER BY region_y,region_x,id")?;
            let regions = statement
                .query_map(
                    params![chunk_id, timestamp],
                    super::super::rows::ocr_region_row,
                )?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            let Some(chunk) = storage::get_processing_chunk(transaction, &chunk_id)? else {
                continue;
            };
            let Some(segment) = segments
                .iter()
                .find(|segment| Some(segment.id.as_str()) == chunk.segment_id.as_deref())
            else {
                continue;
            };
            let Some(source) = crate::recording_privacy::source_track_id(segment, "video") else {
                continue;
            };
            let Some(first) = regions.first() else {
                continue;
            };
            let id = format!("legacy:{}:{timestamp}", chunk.id);
            let content_id = format!("content:{id}");
            let content = reconstruct_content(&regions);
            let links = regions
                .iter()
                .map(|region| (region.id.clone(), region.id.clone()))
                .collect::<BTreeMap<_, _>>();
            let fingerprint = format!(
                "legacy:{}:{}",
                first.provider,
                first.model.as_deref().unwrap_or("")
            );
            transaction.execute("INSERT INTO ocr_document_contents(id,recording_id,source_track_id,pixel_hash,width,height,config_fingerprint,document_json,text,normalized_text) VALUES(?1,?2,?3,?4,0,0,?5,?6,?7,?8)",params![content_id,recording.id,source,id,fingerprint,serde_json::to_string(&content).map_err(|_|rusqlite::Error::InvalidQuery)?,content.text,normalize_ocr_text(&content.text)])?;
            transaction.execute("INSERT INTO ocr_frame_occurrences(id,recording_id,segment_id,source_track_id,frame_timestamp_ms,sample_end_ms,source_frame_ref,width,height,engine,provider,model,config_fingerprint,status,error_code,content_id,cache_hit,provenance,job_id,chunk_id,attempt_count,raw_links_json) VALUES(?1,?2,?3,?4,?5,?6,?7,0,0,'legacy',?8,?9,?10,'succeeded',NULL,?11,0,'reconstructed',?12,?13,?14,?15)",params![id,recording.id,segment.id,source,timestamp,timestamp.saturating_add(1),first.source_frame_ref,first.provider,first.model,fingerprint,content_id,chunk.job_id,chunk.id,chunk.attempt_count,serde_json::to_string(&links).map_err(|_|rusqlite::Error::InvalidQuery)?])?;
        }
    }
    Ok(())
}

fn reconstruct_content(regions: &[OcrRegionRow]) -> OcrContentInput {
    let mut ordered = regions.iter().collect::<Vec<_>>();
    ordered.sort_by_key(|region| (region.region_y, region.region_x, region.id.as_str()));
    let mut lines: Vec<Vec<&OcrRegionRow>> = vec![];
    for region in ordered {
        let line = lines.iter_mut().find(|line| {
            let first = line[0];
            let top = first.region_y.max(region.region_y);
            let bottom =
                (first.region_y + first.region_height).min(region.region_y + region.region_height);
            bottom > top && (bottom - top) * 2 >= first.region_height.min(region.region_height)
        });
        match line {
            Some(line) => line.push(region),
            None => lines.push(vec![region]),
        }
    }
    let blocks = lines
        .into_iter()
        .enumerate()
        .map(|(order, mut line)| {
            line.sort_by_key(|region| (region.region_x, region.region_y, region.id.as_str()));
            let text = line
                .iter()
                .map(|region| region.text.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            let words = line
                .iter()
                .map(|region| OcrWordInput {
                    id: region.id.clone(),
                    hierarchy: None,
                    text: region.text.clone(),
                    confidence: region.confidence,
                    region: Some(VisualRegion {
                        x: region.region_x,
                        y: region.region_y,
                        width: region.region_width,
                        height: region.region_height,
                    }),
                    raw_region_id: None,
                })
                .collect();
            OcrBlockInput {
                id: format!("legacy-line-{order}"),
                order: order as i64,
                text,
                confidence: None,
                region: None,
                raw_region_ids: vec![],
                words,
            }
        })
        .collect::<Vec<_>>();
    OcrContentInput {
        text: blocks
            .iter()
            .map(|block| block.text.as_str())
            .collect::<Vec<_>>()
            .join("\n"),
        blocks,
    }
}
