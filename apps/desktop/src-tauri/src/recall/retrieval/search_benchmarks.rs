//! Explicit fixture benchmark: never scans the user's database or real content.
use super::*;
use std::time::Instant;

fn rss_kib() -> usize {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|line| line.starts_with("VmRSS:"))
                .and_then(|line| line.split_whitespace().nth(1))
                .and_then(|v| v.parse().ok())
        })
        .unwrap_or(0)
}

#[test]
#[ignore = "explicit disk/CPU benchmark: needs native resource slot and ~2GB /tmp"]
fn benchmark_sqlite_streaming_10k_100k_1m() {
    let path = std::env::temp_dir().join(format!(
        "daydream-vector-benchmark-{}.sqlite",
        uuid::Uuid::now_v7()
    ));
    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }
    let _cleanup = Cleanup(path.clone());
    let mut connection = rusqlite::Connection::open(&path).unwrap();
    connection.execute_batch("PRAGMA journal_mode=OFF; PRAGMA synchronous=OFF; PRAGMA cache_size=-2048; CREATE TABLE recall_evidence(id TEXT PRIMARY KEY,revision TEXT,recording_id TEXT,source_kind TEXT,started_at_ms INTEGER,ended_at_ms INTEGER,source_id TEXT,source_track_id TEXT,segment_id TEXT,start_offset_ms INTEGER,end_offset_ms INTEGER); CREATE TABLE recall_index(evidence_id TEXT PRIMARY KEY,source_revision TEXT,embedding_space TEXT,dimensions INTEGER,vector BLOB); CREATE INDEX recall_index_space ON recall_index(embedding_space,evidence_id);").unwrap();
    let mut query = vec![1.0; 384];
    vector::normalize(&mut query, 384).unwrap();
    let blob = vector::encode(&query);
    let mut previous = 0;
    for count in [10_000, 100_000, 1_000_000] {
        let transaction = connection.transaction().unwrap();
        {
            let mut evidence = transaction
                .prepare("INSERT INTO recall_evidence VALUES(?1,'r','recording','ocr',0,1,?1,'track','segment',0,1)")
                .unwrap();
            let mut index = transaction
                .prepare("INSERT INTO recall_index VALUES(?1,'r','space',384,?2)")
                .unwrap();
            for id in previous..count {
                let id = format!("{id:08}");
                evidence.execute([&id]).unwrap();
                index.execute(params![id, blob]).unwrap();
            }
        }
        transaction.commit().unwrap();
        previous = count;
        for warm in [false, true] {
            let baseline = rss_kib();
            let mut peak = baseline;
            let start = Instant::now();
            let mut cursor = String::new();
            let mut scored = 0;
            let mut top = SourceTopK::new(RANK_WINDOW);
            loop {
                let batch = vector_batch(
                    &connection,
                    "space",
                    384,
                    &cursor,
                    &SearchScope::default(),
                    "[]",
                )
                .unwrap();
                if batch.is_empty() {
                    break;
                }
                peak = peak.max(rss_kib());
                for (id, blob, group) in batch {
                    cursor = id.clone();
                    top.push(
                        group,
                        Ranked {
                            id,
                            score: vector::cosine_blob(&query, &blob).unwrap(),
                        },
                    );
                    scored += 1;
                }
            }
            assert_eq!(scored, count);
            assert_eq!(top.finish().len(), count.min(RANK_WINDOW));
            eprintln!("recall-sqlite benchmark vectors={count} dimensions=384 warm={warm} elapsed_ms={} incremental_rss_kib={}",start.elapsed().as_millis(),peak.saturating_sub(baseline));
            if count == 100_000 && warm {
                assert!(
                    peak.saturating_sub(baseline) <= 32 * 1024,
                    "incremental memory exceeds32MiB"
                );
            }
        }
    }
    let cancelled = std::sync::Arc::new(AtomicBool::new(false));
    let trigger = cancelled.clone();
    let thread = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(10));
        let requested = Instant::now();
        trigger.store(true, std::sync::atomic::Ordering::Release);
        requested
    });
    let mut cursor = String::new();
    'scan: loop {
        if vector::check_cancelled(&cancelled).is_err() {
            break;
        }
        let batch = vector_batch(
            &connection,
            "space",
            384,
            &cursor,
            &SearchScope::default(),
            "[]",
        )
        .unwrap();
        for (id, blob, _) in batch {
            if vector::check_cancelled(&cancelled).is_err() {
                break 'scan;
            }
            cursor = id;
            std::hint::black_box(vector::cosine_blob(&query, &blob).unwrap());
        }
    }
    let finished = Instant::now();
    let requested = thread.join().unwrap();
    let latency = finished.saturating_duration_since(requested);
    eprintln!(
        "recall-sqlite cancellation_latency_ms={}",
        latency.as_millis()
    );
    assert!(latency < std::time::Duration::from_millis(100));
    drop(connection);
    std::fs::remove_file(path).unwrap();
}
