use super::matching::MatchedRule;
use crate::{
    settings::ProtectedWindowAction,
    storage::{
        self, AppStorage, CreateRecordingProtectionSpanInput, CreateRedactionInput,
        RecordingProtectionSpanRow,
    },
};
use uuid::Uuid;

pub(super) fn create_span(
    storage: &AppStorage,
    recording_id: &str,
    started_at_ms: u64,
    since_ms: u64,
    timestamp_ms: u64,
    rule: &MatchedRule,
) -> rusqlite::Result<String> {
    let id = Uuid::now_v7().to_string();
    storage.with_connection(|connection| {
        storage::create_recording_protection_span(
            connection,
            &CreateRecordingProtectionSpanInput {
                action: match rule.action {
                    ProtectedWindowAction::Pause => "pause",
                    ProtectedWindowAction::Redact => "redact",
                }
                .into(),
                id: id.clone(),
                opened_at_ms: timestamp_ms.try_into().unwrap_or(i64::MAX),
                recording_id: recording_id.into(),
                rule_id: rule.rule_id.clone(),
                rule_label: rule.label.clone(),
                start_offset_ms: since_ms
                    .saturating_sub(started_at_ms)
                    .try_into()
                    .unwrap_or(i64::MAX),
            },
        )
    })?;
    Ok(id)
}

pub(super) fn close_span(
    storage: &AppStorage,
    id: &str,
    end_offset_ms: i64,
    timestamp_ms: i64,
) -> rusqlite::Result<()> {
    storage.with_connection_mut(|connection| {
        let transaction = connection.transaction()?;
        let span = storage::close_recording_protection_span(
            &transaction,
            id,
            end_offset_ms,
            timestamp_ms,
        )?;
        materialize_span(&transaction, &span)?;
        transaction.commit()
    })
}

pub(crate) fn close_open_protection_spans(
    storage: &AppStorage,
    recording_id: &str,
    end_offset_ms: i64,
    timestamp_ms: i64,
) -> rusqlite::Result<()> {
    storage.with_connection_mut(|connection| {
        let transaction = connection.transaction()?;
        for span in storage::list_open_recording_protection_spans(&transaction, recording_id)? {
            let closed = storage::close_recording_protection_span(
                &transaction,
                &span.id,
                end_offset_ms.max(span.start_offset_ms),
                timestamp_ms,
            )?;
            materialize_span(&transaction, &closed)?;
        }
        for span in storage::list_recording_protection_spans(&transaction, recording_id)?
            .iter()
            .filter(|span| span.status == "closed" && span.redaction_id.is_none())
        {
            materialize_span(&transaction, span)?;
        }
        transaction.commit()
    })
}

fn materialize_span(
    connection: &rusqlite::Connection,
    span: &RecordingProtectionSpanRow,
) -> rusqlite::Result<()> {
    let Some(end) = span.end_offset_ms else {
        return Err(rusqlite::Error::InvalidQuery);
    };
    if end > span.start_offset_ms && span.redaction_id.is_none() {
        let id = Uuid::now_v7().to_string();
        storage::create_redaction(
            connection,
            &CreateRedactionInput {
                created_at_ms: span.closed_at_ms.unwrap_or(span.opened_at_ms),
                end_offset_ms: end,
                id: id.clone(),
                reason: Some(format!("Protected window: {}", span.rule_label)),
                recording_id: span.recording_id.clone(),
                start_offset_ms: span.start_offset_ms,
                track_ids_json: "[]".into(),
            },
        )?;
        storage::mark_recording_protection_span_materialized(connection, &span.id, &id)?;
    } else if end <= span.start_offset_ms {
        connection.execute("UPDATE recording_protection_spans SET status = 'materialized' WHERE id = ?1 AND status = 'closed'", [&span.id])?;
    }
    Ok(())
}
