//! Privacy-checked lexical/semantic recall. Vector scoring runs outside SQLite locks.
use super::{
    embedding::{EmbeddingProvider, EmbeddingPurpose},
    vector::{self, Ranked, SourceTopK},
};
use crate::storage::AppStorage;
use rusqlite::params;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, sync::atomic::AtomicBool};

fn cancellable_sql<T>(
    connection: &rusqlite::Connection,
    cancelled: &AtomicBool,
    read: impl FnOnce() -> rusqlite::Result<T>,
) -> rusqlite::Result<T> {
    use std::sync::atomic::Ordering;
    if cancelled.load(Ordering::Acquire) {
        return Err(rusqlite::Error::InvalidQuery);
    }
    let finished = AtomicBool::new(false);
    let interrupt = connection.get_interrupt_handle();
    std::thread::scope(|scope| {
        let watcher = scope.spawn(|| {
            while !finished.load(Ordering::Acquire) {
                if cancelled.load(Ordering::Acquire) {
                    interrupt.interrupt();
                }
                std::thread::park_timeout(std::time::Duration::from_millis(10));
            }
        });
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(read));
        finished.store(true, Ordering::Release);
        watcher.thread().unpark();
        watcher.join().map_err(|_| rusqlite::Error::InvalidQuery)?;
        match result {
            Ok(result) => result,
            Err(panic) => std::panic::resume_unwind(panic),
        }
    })
}

const VECTOR_BATCH: usize = 512;
const MAX_QUERY_BYTES: usize = 4096;
const RANK_WINDOW: usize = 10_101;
#[cfg(test)]
#[path = "search_benchmarks.rs"]
mod benchmarks;
#[cfg(test)]
#[path = "search_e2e_benchmark.rs"]
mod e2e_benchmark;
#[cfg(test)]
#[path = "search_tests.rs"]
mod tests;

pub(crate) fn vector_batch(
    connection: &rusqlite::Connection,
    space: &str,
    dimensions: usize,
    cursor: &str,
    scope: &SearchScope,
    sources: &str,
) -> rusqlite::Result<Vec<(String, Vec<u8>, String)>> {
    let mut statement=connection.prepare("SELECT i.evidence_id,i.vector,json_array(e.recording_id,e.source_kind,e.source_id,e.source_track_id,e.segment_id,e.start_offset_ms,e.end_offset_ms) FROM recall_index i JOIN recall_evidence e ON e.id=i.evidence_id AND e.revision=i.source_revision WHERE i.embedding_space=?1 AND i.dimensions=?2 AND i.evidence_id>?3 AND (?4 IS NULL OR e.ended_at_ms>?4) AND (?5 IS NULL OR e.started_at_ms<?5) AND (?6 IS NULL OR e.recording_id=?6) AND (?8='[]' OR e.source_kind IN (SELECT value FROM json_each(?8))) ORDER BY i.evidence_id LIMIT ?7")?;
    let rows = statement
        .query_map(
            params![
                space,
                dimensions as i64,
                cursor,
                scope.start_ms,
                scope.end_ms,
                scope.recording_id,
                VECTOR_BATCH as i64,
                sources
            ],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, Vec<u8>>(1)?,
                    r.get::<_, String>(2)?,
                ))
            },
        )?
        .collect();
    rows
}
#[derive(Clone, Debug, Default)]
pub struct SearchScope {
    pub start_ms: Option<i64>,
    pub end_ms: Option<i64>,
    pub recording_id: Option<String>,
    pub source_kinds: Vec<String>,
    pub cursor: Option<String>,
    pub lexical_only: bool,
}
#[derive(Clone, Debug)]
pub struct SearchPage {
    pub privacy_revision: i64,
    pub hits: Vec<(super::types::RecallEvidence, f64)>,
    pub complete: bool,
    pub unavailable_reason: Option<String>,
    pub next_cursor: Option<String>,
    pub semantic_used: bool,
    pub indexed_evidence_count: usize,
    pub eligible_evidence_count: usize,
}

fn fts_query(query: &str) -> String {
    query
        .split_whitespace()
        .map(|word| format!("\"{}\"", word.replace('"', "\"\"")))
        .collect::<Vec<_>>()
        .join(" OR ")
}

pub async fn search(
    storage: &AppStorage,
    provider: Option<&impl EmbeddingProvider>,
    query: &str,
    scope: &SearchScope,
    limit: usize,
    cancelled: &AtomicBool,
) -> Result<SearchPage, String> {
    if query.trim().is_empty() || query.len() > MAX_QUERY_BYTES || !(1..=100).contains(&limit) {
        return Err("Recall query or result limit is invalid".into());
    }
    if scope
        .start_ms
        .zip(scope.end_ms)
        .is_some_and(|(a, b)| a >= b)
    {
        return Err("Recall time range is invalid".into());
    }
    vector::check_cancelled(cancelled)?;
    if scope
        .source_kinds
        .iter()
        .any(|s| !["ocr", "desktop-audio", "microphone"].contains(&s.as_str()))
    {
        return Err("Recall source filter is invalid".into());
    }
    let sources = serde_json::to_string(&scope.source_kinds)
        .map_err(|_| "Recall source filter is invalid")?;
    let offset = scope
        .cursor
        .as_deref()
        .map(|c| {
            c.rsplit(':')
                .next()
                .and_then(|v| v.parse::<usize>().ok())
                .ok_or("Recall cursor is invalid")
        })
        .transpose()?
        .unwrap_or(0);
    if offset > 10_000 {
        return Err("Recall pagination limit reached; narrow the query".into());
    }
    let (revision,lexical) = storage.with_connection(|c| {
        super::evidence::refresh_evidence(c)?;
        let revision=super::evidence::privacy_revision(c)?;
        let ids=cancellable_sql(c,cancelled,||{
        let mut statement=c.prepare("WITH matches AS MATERIALIZED (SELECT e.id,json_array(e.recording_id,e.source_kind,e.source_id,e.source_track_id,e.segment_id,e.start_offset_ms,e.end_offset_ms) source_group,bm25(recall_fts) rank FROM recall_fts JOIN recall_evidence e ON e.rowid=recall_fts.rowid WHERE recall_fts MATCH ?1 AND (?2 IS NULL OR e.ended_at_ms>?2) AND (?3 IS NULL OR e.started_at_ms<?3) AND (?4 IS NULL OR e.recording_id=?4) AND (?6='[]' OR e.source_kind IN (SELECT value FROM json_each(?6)))), fragments AS (SELECT id,source_group,rank,row_number() OVER(PARTITION BY source_group ORDER BY rank,id) ordinal FROM matches) SELECT source_group,id FROM fragments WHERE ordinal=1 ORDER BY rank,id LIMIT ?5")?;
        let ids=statement.query_map(params![fts_query(query),scope.start_ms,scope.end_ms,scope.recording_id,RANK_WINDOW as i64,sources],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?)))?.collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(ids)
        })?;
        Ok((revision,ids))
    }).map_err(|_|if cancelled.load(std::sync::atomic::Ordering::Acquire){"Recall operation cancelled"}else{"Unable to search recall text"})?;
    let mut complete = false;
    let mut unavailable_reason =
        Some("Semantic embeddings are unavailable; showing text matches".into());
    let mut semantic = Vec::new();
    if let Some(provider) = provider.filter(|_| !scope.lexical_only) {
        storage
            .with_connection(|connection| {
                super::index::ensure_provider_current(connection, provider)
            })
            .map_err(|_| "Recall embedding configuration changed; retry the query")?;
        let space = provider.space();
        match provider
            .embed(EmbeddingPurpose::Query, &[query.to_owned()], cancelled)
            .await
        {
            Ok(mut vectors) => {
                if vectors.len() != 1 {
                    return Err("Embedding provider returned an invalid query".into());
                }
                let mut query_vector = vectors.pop().unwrap();
                vector::normalize(&mut query_vector, space.dimensions)?;
                let mut top = SourceTopK::new(RANK_WINDOW);
                let mut cursor = String::new();
                loop {
                    vector::check_cancelled(cancelled)?;
                    let batch = storage
                        .with_connection(|c| {
                            super::index::ensure_provider_current(c, provider)?;
                            if super::evidence::privacy_revision(c)? != revision {
                                return Err(rusqlite::Error::InvalidQuery);
                            }
                            vector_batch(
                                c,
                                &space.fingerprint,
                                space.dimensions,
                                &cursor,
                                scope,
                                &sources,
                            )
                        })
                        .map_err(|_| "Unable to read recall vectors")?;
                    if batch.is_empty() {
                        break;
                    }
                    for (id, blob, group) in batch {
                        vector::check_cancelled(cancelled)?;
                        cursor = id.clone();
                        top.push(
                            group,
                            Ranked {
                                id,
                                score: vector::cosine_blob(&query_vector, &blob)?,
                            },
                        );
                    }
                    // Give the async executor a cancellation/lifecycle boundary per bounded batch.
                    tokio::task::yield_now().await;
                }
                semantic = top.finish();
                complete = true;
                unavailable_reason = None;
            }
            Err(reason) => {
                vector::check_cancelled(cancelled)?;
                if reason == "recall_embedding_token_limit" {
                    unavailable_reason = Some(
                        "Query exceeds the embedding token limit; showing text matches".into(),
                    );
                }
            }
        }
    }
    let semantic_used = complete;
    if scope.lexical_only {
        complete = true;
        unavailable_reason = None;
    }
    let space = match provider {
        Some(provider) => provider.space().fingerprint,
        None => storage
            .with_connection(|connection| {
                let settings = super::index::current_embedding_settings(connection)?;
                Ok(super::embedding::configured_space(&settings)
                    .map(|space| space.fingerprint)
                    .unwrap_or_default())
            })
            .map_err(|_| "Unable to read recall embedding configuration")?,
    };
    let fingerprint=hex::encode(Sha256::digest(format!("{query:?}:{scope_start:?}:{scope_end:?}:{recording:?}:{sources}:{space}:{semantic_used}",scope_start=scope.start_ms,scope_end=scope.end_ms,recording=scope.recording_id).as_bytes()));
    let cursor_prefix = format!("{fingerprint}:{revision}:");
    if scope
        .cursor
        .as_deref()
        .is_some_and(|c| !c.starts_with(&cursor_prefix))
    {
        return Err("Recall cursor is stale; restart the query".into());
    }
    let mut scores = BTreeMap::<String, (String, f64, f64)>::new();
    for (rank, (group, id)) in lexical.into_iter().enumerate().chain(
        semantic
            .into_iter()
            .enumerate()
            .map(|(rank, (group, row))| (rank, (group, row.id))),
    ) {
        let contribution = 1.0 / (61.0 + rank as f64);
        let entry = scores
            .entry(group)
            .or_insert_with(|| (id.clone(), 0.0, contribution));
        if contribution > entry.2 || (contribution == entry.2 && id < entry.0) {
            entry.0 = id;
            entry.2 = contribution;
        }
        entry.1 += contribution;
    }
    let mut ranked = scores
        .into_values()
        .map(|(id, score, _)| (id, score))
        .collect::<Vec<_>>();
    ranked.sort_unstable_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    vector::check_cancelled(cancelled)?;
    let has_more = ranked.len() > offset + limit;
    let (hits,indexed_evidence_count,eligible_evidence_count,privacy_revision) = storage
        .with_connection(|c| {
            if semantic_used { if let Some(provider)=provider {super::index::ensure_provider_current(c,provider)?;} }
            super::evidence::refresh_evidence(c)?;
            if super::evidence::privacy_revision(c)? != revision {
                return Err(rusqlite::Error::InvalidQuery);
            }
            let counts=c.query_row("SELECT count(*),count(i.evidence_id) FROM recall_evidence e LEFT JOIN recall_index i ON i.evidence_id=e.id AND i.source_revision=e.revision AND i.embedding_space=?1 WHERE (?2 IS NULL OR e.ended_at_ms>?2) AND (?3 IS NULL OR e.started_at_ms<?3) AND (?4 IS NULL OR e.recording_id=?4) AND (?5='[]' OR e.source_kind IN (SELECT value FROM json_each(?5)))",params![space,scope.start_ms,scope.end_ms,scope.recording_id,sources],|r|Ok((r.get::<_,i64>(0)? as usize,r.get::<_,i64>(1)? as usize)))?;
            let hits=ranked
                .into_iter()
                .skip(offset)
                .take(limit)
                .map(|(id, score)| {
                    super::evidence::read_evidence(c, &id)?
                        .map(|e| (e, score))
                        .ok_or(rusqlite::Error::InvalidQuery)
                })
                .collect::<rusqlite::Result<Vec<_>>>()?;
            Ok((hits,counts.1,counts.0,crate::storage::recall_privacy_revision(c)?))
        })
        .map_err(|_| "Recall sources changed during search; retry the query")?;
    vector::check_cancelled(cancelled)?;
    if semantic_used && indexed_evidence_count < eligible_evidence_count {
        complete = false;
        unavailable_reason = Some(
            "Semantic indexing is incomplete; all currently indexed text and transcripts were searched".into(),
        );
    }
    let next_cursor = if has_more && offset + limit <= 10_000 {
        Some(format!("{cursor_prefix}{}", offset + limit))
    } else {
        None
    };
    if has_more && next_cursor.is_none() {
        complete = false;
        unavailable_reason = Some("Recall pagination limit reached; narrow the query".into());
    }
    Ok(SearchPage {
        privacy_revision,
        hits,
        complete,
        unavailable_reason,
        next_cursor,
        semantic_used,
        indexed_evidence_count,
        eligible_evidence_count,
    })
}
