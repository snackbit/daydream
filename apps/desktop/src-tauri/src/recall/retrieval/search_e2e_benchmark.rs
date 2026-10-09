//! Full native warm search over actual, eligible synthetic transcript sources.
use super::super::embedding::{configuration_fingerprint, configured_space, EmbeddingSpace};
use super::*;
use crate::{settings::RecallSettings, storage};
use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

struct SyntheticQuery;
impl EmbeddingProvider for SyntheticQuery {
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
        cancelled: &AtomicBool,
    ) -> Result<Vec<Vec<f32>>, String> {
        vector::check_cancelled(cancelled)?;
        let mut value = vec![1.0; 384];
        vector::normalize(&mut value, 384)?;
        Ok(texts.iter().map(|_| value.clone()).collect())
    }
}
fn rss_kib() -> usize {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|value| {
            value
                .lines()
                .find(|line| line.starts_with("VmRSS:"))
                .and_then(|line| line.split_whitespace().nth(1))
                .and_then(|v| v.parse().ok())
        })
        .unwrap_or(0)
}
struct Cleanup(std::path::PathBuf);
impl Drop for Cleanup {
    fn drop(&mut self) {
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{suffix}", self.0.display()));
        }
    }
}

#[test]
#[ignore = "explicit end-to-end100k SQLite/native benchmark: needs root CPU slot"]
fn benchmark_full_warm_search_100k() {
    let path = std::env::temp_dir().join(format!(
        "daydream-search-e2e-{}.sqlite",
        uuid::Uuid::now_v7()
    ));
    let _cleanup = Cleanup(path.clone());
    let storage = storage::test_file_storage(path);
    let fixture = storage
        .with_connection(|c| Ok(storage::seed_recall_fixture(c)))
        .unwrap();
    let provider = SyntheticQuery;
    let mut value = vec![1.0; 384];
    vector::normalize(&mut value, 384).unwrap();
    let blob = vector::encode(&value);
    storage.with_connection_mut(|connection|{
        let transaction=connection.transaction()?;
        {
            let mut insert=transaction.prepare("INSERT INTO transcript_segments(id,recording_id,segment_id,track_id,source_role,start_offset_ms,end_offset_ms,provider,model,text,confidence,job_id,chunk_id,created_at_ms) SELECT ?1,recording_id,segment_id,track_id,source_role,start_offset_ms,end_offset_ms,provider,model,?2,confidence,job_id,chunk_id,created_at_ms FROM transcript_segments WHERE id=?3")?;
            for n in 0..99_999 {
                let text=if n%10==0{"Synthetic planning meeting note"}else{"Synthetic orchard maintenance note"};
                insert.execute(params![uuid::Uuid::now_v7().to_string(),text,fixture.transcript])?;
            }
        }
        transaction.commit()?;
        super::super::evidence::refresh_evidence(connection)?;
        connection.execute("INSERT INTO recall_index SELECT id,revision,?1,384,?2 FROM recall_evidence",params![provider.space().fingerprint,blob])?;
        connection.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")?;
        Ok(())
    }).unwrap();
    let cancelled = AtomicBool::new(false);
    let mut previous = Vec::new();
    for warm in [false, true] {
        let baseline = rss_kib();
        let peak = Arc::new(AtomicUsize::new(baseline));
        let done = Arc::new(AtomicBool::new(false));
        let observer_peak = peak.clone();
        let observer_done = done.clone();
        let observer = std::thread::spawn(move || {
            while !observer_done.load(Ordering::Acquire) {
                observer_peak.fetch_max(rss_kib(), Ordering::Relaxed);
                std::thread::park_timeout(Duration::from_millis(5));
            }
        });
        let start = Instant::now();
        let result = tauri::async_runtime::block_on(search(
            &storage,
            Some(&provider),
            "planning",
            &SearchScope::default(),
            100,
            &cancelled,
        ));
        let elapsed = start.elapsed();
        done.store(true, Ordering::Release);
        observer.thread().unpark();
        observer.join().unwrap();
        let result = result.unwrap();
        assert!(result.complete && result.semantic_used);
        assert_eq!(result.eligible_evidence_count, 100_000);
        assert_eq!(result.indexed_evidence_count, 100_000);
        assert_eq!(result.hits.len(), 100);
        assert!(result.next_cursor.is_some());
        let ids = result
            .hits
            .iter()
            .map(|(e, _)| e.citation.evidence_id.clone())
            .collect::<Vec<_>>();
        if warm {
            assert_eq!(ids, previous);
        }
        previous = ids;
        let incremental = peak.load(Ordering::Relaxed).saturating_sub(baseline);
        eprintln!("recall-full-search benchmark vectors=100000 dimensions=384 warm={warm} elapsed_ms={} incremental_rss_kib={incremental} model_boundary=synthetic lexical_matching_sources=10001",elapsed.as_millis());
        if warm {
            assert!(
                incremental <= 32 * 1024,
                "full warm search exceeds32MiB incremental RSS"
            );
        }
    }
}
