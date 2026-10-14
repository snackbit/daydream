use super::super::{
    embedding::{configuration_fingerprint, configured_space, EmbeddingPurpose, EmbeddingSpace},
    index,
};
use super::*;
use crate::{settings::RecallSettings, storage};

struct SyntheticProvider<'a> {
    storage: &'a AppStorage,
    invalidate: bool,
    fail: bool,
}
struct ParagraphProvider;
struct PrivacyProvider {
    recording_id: String,
    fail: bool,
}
struct BatchProvider {
    calls: std::sync::Mutex<Vec<usize>>,
    invalid: bool,
}
impl EmbeddingProvider for BatchProvider {
    fn space(&self) -> EmbeddingSpace {
        configured_space(&RecallSettings::default().embedding).unwrap()
    }
    fn configuration_fingerprint(&self) -> String {
        configuration_fingerprint(&RecallSettings::default().embedding)
    }
    async fn embed(
        &self,
        _: EmbeddingPurpose,
        texts: &[String],
        _: &AtomicBool,
    ) -> Result<Vec<Vec<f32>>, String> {
        self.calls.lock().unwrap().push(texts.len());
        Ok(texts
            .iter()
            .enumerate()
            .map(|(index, _)| {
                let mut vector = vec![0.0; 384];
                vector[0] = 1.;
                if self.invalid && index == 1 {
                    vector[0] = f32::NAN;
                }
                vector
            })
            .collect())
    }
}
#[test]
fn indexing_submits_one_batch_and_never_publishes_partial_invalid_vectors() {
    for invalid in [false, true] {
        let storage = storage::test_storage(std::env::temp_dir());
        let fixtures = storage
            .with_connection(|connection| {
                Ok((0..3)
                    .map(|_| storage::seed_recall_fixture(connection))
                    .collect::<Vec<_>>())
            })
            .unwrap();
        let provider = BatchProvider {
            calls: std::sync::Mutex::new(Vec::new()),
            invalid,
        };
        let result = tauri::async_runtime::block_on(index::index_pending(
            &storage,
            &provider,
            &AtomicBool::new(false),
            |_| {},
        ));
        assert_eq!(result.is_err(), invalid);
        assert_eq!(*provider.calls.lock().unwrap(), vec![3]);
        let count = storage
            .with_connection(|connection| {
                connection.query_row("SELECT count(*) FROM recall_index", [], |row| {
                    row.get::<_, i64>(0)
                })
            })
            .unwrap();
        assert_eq!(count, if invalid { 0 } else { 3 });
        for fixture in &fixtures {
            assert!(crate::recording_privacy::try_mutation_lease(&fixture.recording).is_some());
        }
    }
}
impl EmbeddingProvider for PrivacyProvider {
    fn space(&self) -> EmbeddingSpace {
        configured_space(&RecallSettings::default().embedding).unwrap()
    }
    fn configuration_fingerprint(&self) -> String {
        configuration_fingerprint(&RecallSettings::default().embedding)
    }
    async fn embed(
        &self,
        _: EmbeddingPurpose,
        texts: &[String],
        _: &AtomicBool,
    ) -> Result<Vec<Vec<f32>>, String> {
        assert!(
            crate::recording_privacy::try_mutation_lease(&self.recording_id).is_none(),
            "indexing must own source privacy before a provider can queue or upload inputs"
        );
        tokio::task::yield_now().await;
        assert!(
            crate::recording_privacy::try_mutation_lease(&self.recording_id).is_none(),
            "privacy ownership must survive provider waits"
        );
        if self.fail {
            return Err("Synthetic embedding failure".into());
        }
        Ok(texts
            .iter()
            .map(|_| {
                let mut vector = vec![0.0; 384];
                vector[0] = 1.0;
                vector
            })
            .collect())
    }
}

#[test]
fn indexing_owns_recording_privacy_until_provider_completion_and_releases_on_failure() {
    for fail in [false, true] {
        let storage = storage::test_storage(std::env::temp_dir());
        let fixture = storage
            .with_connection(|connection| Ok(storage::seed_recall_fixture(connection)))
            .unwrap();
        let provider = PrivacyProvider {
            recording_id: fixture.recording.clone(),
            fail,
        };
        let result = tauri::async_runtime::block_on(index::index_pending(
            &storage,
            &provider,
            &AtomicBool::new(false),
            |_| {},
        ));
        assert_eq!(result.is_err(), fail);
        assert!(crate::recording_privacy::try_mutation_lease(&fixture.recording).is_some());
    }
}

#[test]
fn indexing_revalidates_after_waiting_for_a_recording_privacy_mutation() {
    let storage = storage::test_storage(std::env::temp_dir());
    let fixture = storage
        .with_connection(|connection| Ok(storage::seed_recall_fixture(connection)))
        .unwrap();
    let provider = SyntheticProvider {
        storage: &storage,
        invalidate: false,
        fail: false,
    };
    tauri::async_runtime::block_on(async {
        let mutation = crate::recording_privacy::mutation_lease(&fixture.recording).await;
        let cancelled = AtomicBool::new(false);
        let indexing = index::index_pending(&storage, &provider, &cancelled, |_| {});
        tokio::pin!(indexing);
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(20), &mut indexing)
                .await
                .is_err(),
            "indexing must wait for the mutation before validating provider inputs"
        );
        storage
            .with_connection(|connection| {
                connection.execute_batch("DELETE FROM transcript_segments")
            })
            .unwrap();
        drop(mutation);
        assert!(
            indexing.await.is_err(),
            "removed text must never reach the provider after the lease wait"
        );
    });
}
impl EmbeddingProvider for ParagraphProvider {
    fn space(&self) -> EmbeddingSpace {
        configured_space(&RecallSettings::default().embedding).unwrap()
    }
    fn configuration_fingerprint(&self) -> String {
        configuration_fingerprint(&RecallSettings::default().embedding)
    }
    async fn embed(
        &self,
        purpose: EmbeddingPurpose,
        texts: &[String],
        _: &AtomicBool,
    ) -> Result<Vec<Vec<f32>>, String> {
        Ok(texts
            .iter()
            .map(|text| {
                let mut vector = vec![0.0; 384];
                let relevant = matches!(purpose, EmbeddingPurpose::Query)
                    || text.contains("spacecraft orbital navigation");
                vector[if relevant { 0 } else { 1 }] = 1.0;
                vector
            })
            .collect())
    }
}
impl EmbeddingProvider for SyntheticProvider<'_> {
    fn space(&self) -> EmbeddingSpace {
        configured_space(&RecallSettings::default().embedding).unwrap()
    }
    fn configuration_fingerprint(&self) -> String {
        configuration_fingerprint(&RecallSettings::default().embedding)
    }
    async fn embed(
        &self,
        _: EmbeddingPurpose,
        texts: &[String],
        _: &AtomicBool,
    ) -> Result<Vec<Vec<f32>>, String> {
        if self.fail {
            return Err("synthetic provider offline".into());
        }
        if self.invalidate {
            self.storage
                .with_connection(|c| c.execute_batch("DELETE FROM transcript_segments"))
                .unwrap();
        }
        Ok(texts
            .iter()
            .map(|_| {
                let mut v = vec![0.0; 384];
                v[0] = 1.0;
                v
            })
            .collect())
    }
}

#[test]
fn real_transcript_lexical_fallback_and_source_filters() {
    let storage = storage::test_storage(std::env::temp_dir());
    let _fixture = storage
        .with_connection(|c| Ok(storage::seed_recall_fixture(c)))
        .unwrap();
    let provider = SyntheticProvider {
        storage: &storage,
        invalidate: false,
        fail: true,
    };
    let cancelled = AtomicBool::new(false);
    let result = tauri::async_runtime::block_on(search(
        &storage,
        Some(&provider),
        "planning",
        &SearchScope::default(),
        20,
        &cancelled,
    ))
    .unwrap();
    assert_eq!(result.hits.len(), 1);
    assert!(!result.semantic_used);
    assert!(!result.complete);
    assert_eq!(result.eligible_evidence_count, 1);
    assert!(result.unavailable_reason.is_some());
    let healthy_provider = SyntheticProvider {
        storage: &storage,
        invalidate: false,
        fail: false,
    };
    let partial = tauri::async_runtime::block_on(search(
        &storage,
        Some(&healthy_provider),
        "planning",
        &SearchScope::default(),
        20,
        &cancelled,
    ))
    .unwrap();
    assert!(partial.semantic_used);
    assert!(!partial.complete);
    assert!(partial
        .unavailable_reason
        .as_deref()
        .is_some_and(|reason| reason.contains("indexing is incomplete")));
    let unconfigured = tauri::async_runtime::block_on(search(
        &storage,
        None::<&SyntheticProvider<'_>>,
        "planning",
        &SearchScope::default(),
        20,
        &cancelled,
    ))
    .unwrap();
    assert!(!unconfigured.semantic_used);
    assert!(!unconfigured.complete);
    assert!(unconfigured
        .unavailable_reason
        .as_deref()
        .is_some_and(|reason| reason.contains("embeddings are unavailable")));
    let scope = SearchScope {
        source_kinds: vec!["microphone".into()],
        lexical_only: true,
        ..Default::default()
    };
    let result = tauri::async_runtime::block_on(search(
        &storage,
        Some(&provider),
        "planning",
        &scope,
        20,
        &cancelled,
    ))
    .unwrap();
    assert!(result.hits.is_empty());
    assert_eq!(result.eligible_evidence_count, 0);
    assert!(result.complete);
}

#[test]
fn source_removed_during_inference_never_returns_snippets_or_counts() {
    let storage = storage::test_storage(std::env::temp_dir());
    let _fixture = storage
        .with_connection(|c| Ok(storage::seed_recall_fixture(c)))
        .unwrap();
    let provider = SyntheticProvider {
        storage: &storage,
        invalidate: true,
        fail: false,
    };
    let result = tauri::async_runtime::block_on(search(
        &storage,
        Some(&provider),
        "planning",
        &SearchScope::default(),
        20,
        &AtomicBool::new(false),
    ));
    assert!(result.is_err());
}

#[test]
fn indexing_resumes_committed_rows_and_rejects_stale_configuration() {
    let storage = storage::test_storage(std::env::temp_dir());
    let fixture = storage
        .with_connection(|c| Ok(storage::seed_recall_fixture(c)))
        .unwrap();
    let provider = SyntheticProvider {
        storage: &storage,
        invalidate: false,
        fail: false,
    };
    let cancelled = AtomicBool::new(false);
    let first = tauri::async_runtime::block_on(index::index_pending(
        &storage,
        &provider,
        &cancelled,
        |_| {},
    ))
    .unwrap();
    assert_eq!(first.indexed, 1);
    assert_eq!(first.pending, 0);
    let second = tauri::async_runtime::block_on(index::index_pending(
        &storage,
        &provider,
        &cancelled,
        |_| panic!("committed work must not run again"),
    ))
    .unwrap();
    assert_eq!(second.indexed, 1);
    let additional_transcript_id = uuid::Uuid::now_v7().to_string();
    storage.with_connection(|c| {
        c.execute("INSERT INTO transcript_segments(id,recording_id,segment_id,track_id,source_role,start_offset_ms,end_offset_ms,provider,model,text,confidence,job_id,chunk_id,created_at_ms) SELECT ?1,recording_id,segment_id,track_id,source_role,start_offset_ms,end_offset_ms,provider,model,'Additional synthetic note',confidence,job_id,chunk_id,created_at_ms FROM transcript_segments WHERE id=?2",params![additional_transcript_id,fixture.transcript])?;
        super::super::evidence::refresh_evidence(c)?;
        let state=index::progress(c,&provider.space().fingerprint)?;
        assert_eq!(state.indexed,1,"unrelated new source must retain existing checkpoints");assert_eq!(state.pending,1);
        Ok(())
    }).unwrap();
    let mut settings = RecallSettings::default();
    settings.embedding.local_backend = "auto".into();
    storage.with_connection(|c| c.execute("INSERT INTO app_settings(key,value_json,created_at_ms,updated_at_ms) VALUES('ai.recall',?1,0,0) ON CONFLICT(key) DO UPDATE SET value_json=excluded.value_json",[serde_json::to_string(&settings).unwrap()]).map(|_|())).unwrap();
    assert!(tauri::async_runtime::block_on(index::index_pending(
        &storage,
        &provider,
        &cancelled,
        |_| {}
    ))
    .is_err());
}

#[test]
fn fts_input_cannot_inject_query_operators() {
    assert_eq!(
        fts_query("needle OR \""),
        "\"needle\" OR \"OR\" OR \"\"\"\""
    );
}

#[test]
fn cancellation_interrupts_long_sql_without_interrupting_the_next_owner() {
    let connection = rusqlite::Connection::open_in_memory().unwrap();
    let cancelled = std::sync::Arc::new(AtomicBool::new(false));
    let trigger = cancelled.clone();
    let thread = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(10));
        let requested = std::time::Instant::now();
        trigger.store(true, std::sync::atomic::Ordering::Release);
        requested
    });
    let result = cancellable_sql(&connection, &cancelled, || {
        connection.query_row("WITH RECURSIVE count(x) AS (VALUES(0) UNION ALL SELECT x+1 FROM count WHERE x<1000000000) SELECT sum(x) FROM count",[],|row|row.get::<_,i64>(0))
    });
    let finished = std::time::Instant::now();
    let requested = thread.join().unwrap();
    assert!(result.is_err());
    assert!(finished.saturating_duration_since(requested) < std::time::Duration::from_millis(100));
    assert_eq!(
        connection
            .query_row("SELECT 1", [], |row| row.get::<_, i64>(0))
            .unwrap(),
        1
    );
}

#[test]
fn semantic_search_scores_evidence_beyond_the_first_batch_and_pages_stably() {
    let storage = storage::test_storage(std::env::temp_dir());
    let fixture = storage
        .with_connection(|c| Ok(storage::seed_recall_fixture(c)))
        .unwrap();
    let provider = SyntheticProvider {
        storage: &storage,
        invalidate: false,
        fail: false,
    };
    let mut ordinary = vec![0.0; 384];
    ordinary[1] = 1.0;
    let mut best = vec![0.0; 384];
    best[0] = 1.0;
    let mut transcript_ids = (0..600)
        .map(|_| uuid::Uuid::now_v7().to_string())
        .collect::<Vec<_>>();
    transcript_ids.sort_unstable();
    let best_evidence_id = format!("transcript:{}", transcript_ids.last().unwrap());
    storage.with_connection(|c| {
        for id in &transcript_ids {
            c.execute("INSERT INTO transcript_segments(id,recording_id,segment_id,track_id,source_role,start_offset_ms,end_offset_ms,provider,model,text,confidence,job_id,chunk_id,created_at_ms) SELECT ?1,recording_id,segment_id,track_id,source_role,start_offset_ms,end_offset_ms,provider,model,text,confidence,job_id,chunk_id,created_at_ms FROM transcript_segments WHERE id=?2",params![id,fixture.transcript])?;
        }
        super::super::evidence::refresh_evidence(c)?;
        c.execute("INSERT INTO recall_index SELECT id,revision,?1,384,?2 FROM recall_evidence",params![provider.space().fingerprint,vector::encode(&ordinary)])?;
        c.execute("UPDATE recall_index SET vector=?1 WHERE evidence_id=?2",params![vector::encode(&best),best_evidence_id])?;
        Ok(())
    }).unwrap();
    let cancelled = AtomicBool::new(false);
    let first = tauri::async_runtime::block_on(search(
        &storage,
        Some(&provider),
        "unmatched",
        &SearchScope::default(),
        1,
        &cancelled,
    ))
    .unwrap();
    assert!(first.semantic_used && first.complete);
    assert_eq!(first.indexed_evidence_count, 601);
    assert!(first.unavailable_reason.is_none());
    assert_eq!(first.hits[0].0.citation.evidence_id, best_evidence_id);
    let scope = SearchScope {
        cursor: first.next_cursor,
        ..Default::default()
    };
    let second = tauri::async_runtime::block_on(search(
        &storage,
        Some(&provider),
        "unmatched",
        &scope,
        1,
        &cancelled,
    ))
    .unwrap();
    assert_ne!(
        first.hits[0].0.citation.evidence_id,
        second.hits[0].0.citation.evidence_id
    );
    cancelled.store(true, std::sync::atomic::Ordering::Relaxed);
    assert!(tauri::async_runtime::block_on(search(
        &storage,
        Some(&provider),
        "unmatched",
        &SearchScope::default(),
        1,
        &cancelled
    ))
    .is_err());
}

#[test]
fn long_source_returns_its_relevant_fragment_once_with_exact_source_provenance() {
    let storage = storage::test_storage(std::env::temp_dir());
    let fixture = storage
        .with_connection(|c| Ok(storage::seed_recall_fixture(c)))
        .unwrap();
    let irrelevant="The gardening committee reviewed irrigation schedules, composting plans, plant growth and watering records. ".repeat(100);
    let document=format!("{irrelevant}\nWe reviewed spacecraft orbital navigation and the trajectory correction schedule.\n{irrelevant}");
    storage
        .with_connection(|c| {
            c.execute(
                "UPDATE transcript_segments SET text=?1 WHERE id=?2",
                params![document, fixture.transcript],
            )
            .map(|_| ())
        })
        .unwrap();
    let cancelled = AtomicBool::new(false);
    let provider = ParagraphProvider;
    let state = tauri::async_runtime::block_on(index::index_pending(
        &storage,
        &provider,
        &cancelled,
        |_| {},
    ))
    .unwrap();
    assert!(state.indexed > 10);
    let result = tauri::async_runtime::block_on(search(
        &storage,
        Some(&provider),
        "rocket steering",
        &SearchScope::default(),
        20,
        &cancelled,
    ))
    .unwrap();
    assert_eq!(
        result.hits.len(),
        1,
        "one original source is one result despite many fragments"
    );
    let evidence = &result.hits[0].0;
    assert!(evidence.text.contains("spacecraft orbital navigation"));
    assert!(evidence.text.len() < 3000);
    assert_eq!(evidence.citation.source_id, fixture.transcript);
    assert_eq!(evidence.citation.recording_id, fixture.recording);
    assert_eq!(evidence.citation.absolute_start_ms, 1_001_000);
}
