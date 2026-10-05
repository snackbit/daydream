use super::{read::load_cached_ocr_document, types::*, write::normalize_ocr_text};
use crate::storage::ProcessingChunkRow;
use rusqlite::{params, Connection};

pub(super) fn store_content(
    connection: &Connection,
    chunk: &ProcessingChunkRow,
    input: &OcrFrameInput,
) -> rusqlite::Result<(String, serde_json::Map<String, serde_json::Value>)> {
    let (id, mut canonical) = if let Some(id) = &input.cache_content_id {
        let cache = load_cached_ocr_document(
            connection,
            &chunk.recording_id,
            &input.source_track_id,
            &input.pixel_hash,
            input.width,
            input.height,
            &input.config_fingerprint,
        )?
        .filter(|cache| cache.content_id == *id)
        .ok_or(rusqlite::Error::InvalidQuery)?;
        (cache.content_id, cache.content)
    } else {
        let content = input
            .content
            .as_ref()
            .ok_or(rusqlite::Error::InvalidQuery)?
            .clone();
        (uuid::Uuid::now_v7().to_string(), content)
    };
    validate_content(&canonical, input.width, input.height)?;
    for block in &mut canonical.blocks {
        block.raw_region_ids.clear();
        for word in &mut block.words {
            word.raw_region_id = None;
        }
    }
    if input.cache_content_id.is_none() {
        connection.execute("INSERT INTO ocr_document_contents(id,recording_id,source_track_id,pixel_hash,width,height,config_fingerprint,document_json,text,normalized_text) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",params![id,chunk.recording_id,input.source_track_id,input.pixel_hash,input.width,input.height,input.config_fingerprint,serde_json::to_string(&canonical).map_err(|_|rusqlite::Error::InvalidQuery)?,canonical.text,normalize_ocr_text(&canonical.text)])?;
    }
    let words = canonical
        .blocks
        .iter()
        .flat_map(|block| block.words.iter())
        .filter(|word| word.region.is_some())
        .collect::<Vec<_>>();
    if !input.raw_regions.is_empty() && words.len() != input.raw_regions.len() {
        return Err(rusqlite::Error::InvalidQuery);
    }
    let mut links = serde_json::Map::new();
    for (word, region) in words.iter().zip(&input.raw_regions) {
        links.insert(
            word.id.clone(),
            serde_json::Value::String(region.id.clone()),
        );
    }
    Ok((id, links))
}

fn validate_content(content: &OcrContentInput, width: i64, height: i64) -> rusqlite::Result<()> {
    let mut last = None;
    let mut ids = std::collections::HashSet::new();
    for block in &content.blocks {
        if block.order < 0
            || last.is_some_and(|order| block.order <= order)
            || !ids.insert(block.id.as_str())
        {
            return Err(rusqlite::Error::InvalidQuery);
        }
        last = Some(block.order);
        validate_metadata(block.confidence, block.region.as_ref(), width, height)?;
        for word in &block.words {
            validate_metadata(word.confidence, word.region.as_ref(), width, height)?;
        }
    }
    Ok(())
}
fn validate_metadata(
    confidence: Option<f64>,
    region: Option<&crate::ai::VisualRegion>,
    width: i64,
    height: i64,
) -> rusqlite::Result<()> {
    if confidence.is_some_and(|value| !value.is_finite() || !(0.0..=1.0).contains(&value))
        || region.is_some_and(|region| {
            region.x < 0
                || region.y < 0
                || region.width < 0
                || region.height < 0
                || region.x.saturating_add(region.width) > width
                || region.y.saturating_add(region.height) > height
        })
    {
        return Err(rusqlite::Error::InvalidQuery);
    }
    Ok(())
}
