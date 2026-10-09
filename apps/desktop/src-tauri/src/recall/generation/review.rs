//! Local feedback is deliberately never part of provider input.
use super::types::{
    SetRecallDayReviewedInput, SetRecallFeedbackInput, SetRecallSessionCorrectionInput,
};
use rusqlite::{params, Connection};

pub fn set_feedback(
    connection: &Connection,
    input: &SetRecallFeedbackInput,
    now_ms: i64,
) -> Result<(), String> {
    if input.day_id.is_empty()
        || input.target_id.is_empty()
        || input.comment.len() > 8_000
        || input
            .rating
            .as_deref()
            .is_some_and(|rating| !matches!(rating, "helpful" | "unhelpful"))
    {
        return Err("recall_feedback_invalid".into());
    }
    connection.execute("INSERT INTO recall_feedback(day_key,target_id,rating,correction) VALUES(?1,?2,?3,?4) ON CONFLICT(day_key,target_id) DO UPDATE SET rating=excluded.rating,correction=excluded.correction", params![input.day_id,input.target_id,input.rating,input.comment]).map_err(|_| "recall_storage_failed")?;
    let _ = now_ms;
    Ok(())
}
pub fn set_correction(
    connection: &Connection,
    input: &SetRecallSessionCorrectionInput,
    now_ms: i64,
) -> Result<(), String> {
    if input.day_id.is_empty()
        || input.session_id.is_empty()
        || input
            .title_correction
            .as_ref()
            .is_some_and(|value| value.len() > 2000)
        || input
            .category_correction
            .as_ref()
            .is_some_and(|value| value.len() > 128)
        || input
            .correction
            .as_ref()
            .is_some_and(|value| value.len() > 8_000)
    {
        return Err("recall_correction_invalid".into());
    }
    connection.execute("INSERT INTO recall_session_corrections(day_key,session_id,correction,updated_at,title_correction,category_correction) VALUES(?1,?2,?3,?4,?5,?6) ON CONFLICT(day_key,session_id) DO UPDATE SET correction=excluded.correction,title_correction=excluded.title_correction,category_correction=excluded.category_correction,updated_at=excluded.updated_at", params![input.day_id,input.session_id,input.correction,now_ms,input.title_correction,input.category_correction]).map_err(|_| "recall_storage_failed")?;
    Ok(())
}
pub fn set_reviewed(
    connection: &Connection,
    input: &SetRecallDayReviewedInput,
    current_fingerprint: &str,
) -> Result<(), String> {
    if input.input_fingerprint != current_fingerprint {
        return Err("recall_input_changed".into());
    }
    if input.reviewed {
        connection.execute("INSERT INTO recall_day_reviews(day_key,fingerprint) VALUES(?1,?2) ON CONFLICT(day_key) DO UPDATE SET fingerprint=excluded.fingerprint", params![input.day_id,current_fingerprint])
    } else { connection.execute("DELETE FROM recall_day_reviews WHERE day_key=?1", [&input.day_id]) }.map_err(|_| "recall_storage_failed")?;
    Ok(())
}
