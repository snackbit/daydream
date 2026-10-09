//! Cursor-based section inspection without the legacy 500-section truncation.
use super::*;
use crate::{
    ai::AiError,
    settings,
    storage::{self, AppStorage},
};
use rusqlite::params;

pub(super) fn get(
    input: GetRecordingProcessingLaneInput,
    storage: &AppStorage,
) -> Result<RecordingProcessingLaneDetail, AiError> {
    groups::validate_lane(&input.source_kind)?;
    let settings = settings::load_app_settings(storage).map_err(|_| groups::failure())?;
    storage.with_connection(|connection| {
        let lane=groups::lane(connection,&input.recording_id,&input.source_kind,&settings)?;
        let selected_ids=if input.history {
            let mut query=connection.prepare("SELECT DISTINCT c.job_id FROM processing_chunks c WHERE c.recording_id=?1 AND c.source_kind=?2 AND c.job_id NOT IN(SELECT value FROM json_each(?3))")?;
            let current=serde_json::to_string(&lane.job_ids).map_err(|_|rusqlite::Error::InvalidQuery)?;
            let ids=query.query_map(params![input.recording_id,input.source_kind,current],|row|row.get::<_,String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
            ids
        } else { lane.job_ids.clone() };
        let jobs=serde_json::to_string(&selected_ids).map_err(|_|rusqlite::Error::InvalidQuery)?;
        let limit=input.limit.unwrap_or(100).clamp(1,200);
        let mut query=connection.prepare(r#"
WITH chunks AS (SELECT c.* FROM processing_chunks c WHERE c.recording_id=?1 AND c.source_kind=?2 AND c.job_id IN(SELECT value FROM json_each(?3))),
speech AS (SELECT r.* FROM processing_audio_ranges r JOIN chunks c ON c.id=r.chunk_id WHERE NOT EXISTS(SELECT 1 FROM processing_audio_ranges n WHERE n.chunk_id=r.chunk_id AND n.start_ms=r.start_ms AND n.end_ms=r.end_ms AND (n.updated_at_ms>r.updated_at_ms OR (n.updated_at_ms=r.updated_at_ms AND n.rowid>r.rowid)))),
units AS (
 SELECT v.id,c.id chunk_id,c.job_id,c.source_kind,v.frame_timestamp_ms start_ms,NULL end_ms,v.status,v.progress,v.error_code,c.attempt_count,c.track_id,c.segment_id FROM chunks c JOIN visual_frame_processing_states v ON v.chunk_id=c.id WHERE c.source_kind IN('ocr','vlm')
 UNION ALL SELECT 'speech:'||c.id||':'||r.start_ms||':'||r.end_ms,c.id,c.job_id,c.source_kind,c.start_offset_ms+r.start_ms,c.start_offset_ms+r.end_ms,r.status,1.0,r.error_code,c.attempt_count,c.track_id,c.segment_id FROM chunks c JOIN speech r ON r.chunk_id=c.id WHERE c.source_kind='audio'
 UNION ALL SELECT c.id,c.id,c.job_id,c.source_kind,c.start_offset_ms,c.end_offset_ms,c.status,c.progress,c.error_code,c.attempt_count,c.track_id,c.segment_id FROM chunks c WHERE
   (NOT EXISTS(SELECT 1 FROM visual_frame_processing_states v WHERE v.chunk_id=c.id) AND NOT EXISTS(SELECT 1 FROM speech r WHERE r.chunk_id=c.id)) OR
   (c.error_code IS NOT NULL AND NOT EXISTS(SELECT 1 FROM visual_frame_processing_states v WHERE v.chunk_id=c.id AND v.status IN('failed','completed-with-errors')) AND NOT EXISTS(SELECT 1 FROM speech r WHERE r.chunk_id=c.id AND r.status='failed'))
)
SELECT u.*,t.kind FROM units u LEFT JOIN recording_tracks t ON t.id=u.track_id WHERE (?4 IS NULL OR (u.chunk_id,u.id)>(?4,?5)) ORDER BY u.chunk_id,u.id LIMIT ?6
"#)?;
        let rows=query.query_map(params![input.recording_id,input.source_kind,jobs,input.cursor.as_ref().map(|cursor|&cursor.chunk_id),input.cursor.as_ref().map(|cursor|&cursor.unit_id),(limit+1) as i64],|row| {
            let kind:String=row.get(3)?;
            let track:Option<String>=row.get(10)?;
            let role:Option<String>=row.get(12)?;
            let role=role.filter(|role|matches!(role.as_str(),"desktop-audio"|"microphone"));
            Ok((RecordingProcessingUnit { job_id:row.get(2)?,source_track_id:track,source_role:role,
                unit:BackgroundTaskUnit{id:row.get(0)?,chunk_id:row.get(1)?,source_kind:kind,start_offset_ms:row.get(4)?,end_offset_ms:row.get(5)?,status:row.get(6)?,progress:row.get(7)?,error_code:row.get(8)?,attempt_count:row.get(9)?}},row.get::<_,Option<String>>(11)?))
        })?.collect::<rusqlite::Result<Vec<_>>>()?;
        let has_more=rows.len()>limit;
        let mut units=Vec::new();
        let segments=storage::list_recording_segments(connection,&input.recording_id)?;
        for (mut unit,segment_id) in rows.into_iter().take(limit) {
            // UI playback uses source aliases, not processing track identifiers.
            if let (Some(role),Some(segment_id))=(&unit.source_role,segment_id) {
                unit.source_track_id=segments.iter().find(|segment|segment.id==segment_id).and_then(|segment|crate::recording_privacy::source_track_id(segment,role));
            }
            units.push(unit);
        }
        let next_cursor=if has_more {units.last().map(|unit|RecordingProcessingUnitCursor{chunk_id:unit.unit.chunk_id.clone(),unit_id:unit.unit.id.clone()})}else{None};
        Ok(RecordingProcessingLaneDetail{lane,units,next_cursor})
    }).map_err(|_|groups::failure())
}
