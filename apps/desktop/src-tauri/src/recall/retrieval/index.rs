//! Restartable indexing: each batch is independently committed after eligibility recheck.
use super::{
    embedding::{EmbeddingProvider, EmbeddingPurpose},
    vector,
};
use crate::storage::AppStorage;
use rusqlite::{params, Connection};
use std::sync::atomic::AtomicBool;

const DOCUMENT_BATCH: usize = 16;
#[derive(Clone, Debug, Default)]
pub struct IndexProgress {
    pub indexed: usize,
    pub pending: usize,
}

pub fn current_embedding_settings(
    connection: &Connection,
) -> rusqlite::Result<crate::settings::RecallEmbeddingSettings> {
    let settings = match crate::storage::get_app_setting(connection, "ai.recall")? {
        Some(row) => serde_json::from_str::<crate::settings::RecallSettings>(&row.value_json)
            .map_err(|_| rusqlite::Error::InvalidQuery)?,
        None => crate::settings::RecallSettings::default(),
    };
    Ok(settings.embedding)
}
pub fn ensure_provider_current(
    connection: &Connection,
    provider: &impl EmbeddingProvider,
) -> rusqlite::Result<()> {
    let settings = current_embedding_settings(connection)?;
    let configured =
        super::embedding::configured_space(&settings).map_err(|_| rusqlite::Error::InvalidQuery)?;
    if configured.fingerprint != provider.space().fingerprint
        || super::embedding::configuration_fingerprint(&settings)
            != provider.configuration_fingerprint()
    {
        return Err(rusqlite::Error::InvalidQuery);
    }
    Ok(())
}

async fn embed_batch(
    storage: &AppStorage,
    provider: &impl EmbeddingProvider,
    rows: &[(String, String, String, String)],
    cancelled: &AtomicBool,
) -> Result<
    (
        Vec<Vec<f32>>,
        Vec<crate::recording_privacy::ProcessingLease>,
    ),
    String,
> {
    let space = provider.space();
    vector::check_cancelled(cancelled)?;
    // Validation and provider inputs share the recording's privacy owner.
    // Revalidate after waiting: a queued mutation may have removed this text.
    let recordings = rows
        .iter()
        .map(|row| row.3.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let mut privacy = Vec::with_capacity(recordings.len());
    for recording in recordings {
        let acquiring = crate::recording_privacy::processing_lease(recording);
        tokio::pin!(acquiring);
        loop {
            vector::check_cancelled(cancelled)?;
            if let Ok(lease) =
                tokio::time::timeout(std::time::Duration::from_millis(50), &mut acquiring).await
            {
                privacy.push(lease);
                break;
            }
        }
    }
    vector::check_cancelled(cancelled)?;
    let eligible = storage
        .with_connection(|connection| {
            ensure_provider_current(connection, provider)?;
            // The clean refresh checks source-file availability without
            // rebuilding unchanged text. Also cover files removed outside SQL.
            super::evidence::refresh_evidence(connection)?;
            let mut query = connection.prepare("SELECT EXISTS(SELECT 1 FROM recall_evidence WHERE id=?1 AND revision=?2 AND recording_id=?3)")?;
            for (id, revision, _, recording) in rows {
                if !query.query_row(params![id, revision, recording], |row| row.get::<_, bool>(0))? { return Ok(false); }
            }
            Ok(true)
        })
        .map_err(|_| "Unable to revalidate recall indexing source")?;
    if !eligible {
        return Err("recall_index_inputs_changed".into());
    }
    let vectors = provider
        .embed(
            EmbeddingPurpose::Document,
            &rows.iter().map(|row| row.2.clone()).collect::<Vec<_>>(),
            cancelled,
        )
        .await?;
    if vectors.len() != rows.len() {
        return Err("Embedding provider returned an invalid document batch".into());
    }
    let mut vectors = vectors;
    for embedding in &mut vectors {
        vector::normalize(embedding, space.dimensions)?;
    }
    Ok((vectors, privacy))
}

pub fn progress(connection: &Connection, space: &str) -> rusqlite::Result<IndexProgress> {
    let total: usize = connection.query_row("SELECT count(*) FROM recall_evidence", [], |r| {
        r.get::<_, i64>(0).map(|v| v as usize)
    })?;
    let indexed: usize = connection.query_row("SELECT count(*) FROM recall_index i JOIN recall_evidence e ON e.id=i.evidence_id AND e.revision=i.source_revision WHERE i.embedding_space=?1",[space],|r|r.get::<_,i64>(0).map(|v|v as usize))?;
    Ok(IndexProgress {
        indexed,
        pending: total.saturating_sub(indexed),
    })
}

pub async fn index_pending(
    storage: &AppStorage,
    provider: &impl EmbeddingProvider,
    cancelled: &AtomicBool,
    mut report: impl FnMut(IndexProgress),
) -> Result<IndexProgress, String> {
    let space = provider.space();
    loop {
        vector::check_cancelled(cancelled)?;
        let rows = storage.with_connection(|connection| {
            ensure_provider_current(connection,provider)?;
            super::evidence::refresh_evidence(connection)?;
            let mut statement = connection.prepare("SELECT e.id,e.revision,e.text,e.recording_id FROM recall_evidence e LEFT JOIN recall_index i ON i.evidence_id=e.id AND i.source_revision=e.revision AND i.embedding_space=?1 WHERE i.evidence_id IS NULL ORDER BY e.id LIMIT ?2")?;
            let rows=statement.query_map(params![space.fingerprint,DOCUMENT_BATCH as i64],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?)))?.collect::<rusqlite::Result<Vec<_>>>();
            rows
        }).map_err(|_| "Unable to read recall indexing inputs")?;
        if rows.is_empty() {
            return storage
                .with_connection(|c| progress(c, &space.fingerprint))
                .map_err(|_| "Unable to read recall indexing progress".into());
        }
        let (vectors, _privacy) = embed_batch(storage, provider, &rows, cancelled).await?;
        vector::check_cancelled(cancelled)?;
        let state = storage.with_connection_mut(|connection| {
            ensure_provider_current(connection,provider)?;
            super::evidence::refresh_evidence(connection)?;
            let transaction=connection.transaction()?;
            for ((id,revision,_,_),embedding) in rows.iter().zip(&vectors) {
                transaction.execute("INSERT INTO recall_index(evidence_id,source_revision,embedding_space,dimensions,vector) SELECT id,revision,?3,?4,?5 FROM recall_evidence WHERE id=?1 AND revision=?2 ON CONFLICT(evidence_id) DO UPDATE SET source_revision=excluded.source_revision,embedding_space=excluded.embedding_space,dimensions=excluded.dimensions,vector=excluded.vector",params![id,revision,space.fingerprint,space.dimensions as i64,vector::encode(embedding)])?;
            }
            transaction.commit()?;
            progress(connection,&space.fingerprint)
        }).map_err(|_|"Unable to commit recall index")?;
        report(state);
    }
}
