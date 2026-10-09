//! Bounded, deterministic normalized-vector ranking. No source text is retained here.
use std::{
    cmp::Ordering,
    collections::{BTreeMap, BinaryHeap},
    sync::atomic::{AtomicBool, Ordering as AtomicOrdering},
};

pub fn normalize(vector: &mut [f32], dimensions: usize) -> Result<(), String> {
    if vector.len() != dimensions || dimensions == 0 || vector.iter().any(|v| !v.is_finite()) {
        return Err("Embedding has invalid dimensions or values".into());
    }
    let norm = vector
        .iter()
        .map(|v| f64::from(*v).powi(2))
        .sum::<f64>()
        .sqrt();
    if norm < f64::EPSILON {
        return Err("Embedding has zero length".into());
    }
    for value in vector {
        *value = (f64::from(*value) / norm) as f32;
    }
    Ok(())
}

pub fn encode(vector: &[f32]) -> Vec<u8> {
    vector.iter().flat_map(|v| v.to_le_bytes()).collect()
}

pub fn cosine_blob(query: &[f32], blob: &[u8]) -> Result<f32, String> {
    if blob.len() != query.len() * 4 {
        return Err("Stored embedding dimensions are invalid".into());
    }
    let mut score = 0.0;
    for (q, bytes) in query.iter().zip(blob.chunks_exact(4)) {
        let v = f32::from_le_bytes(bytes.try_into().unwrap());
        if !v.is_finite() {
            return Err("Stored embedding values are invalid".into());
        }
        score += q * v;
    }
    Ok(score)
}

#[derive(Clone, Debug)]
pub struct Ranked {
    pub id: String,
    pub score: f32,
}
impl PartialEq for Ranked {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.score == other.score
    }
}
impl Eq for Ranked {}
impl Ord for Ranked {
    fn cmp(&self, other: &Self) -> Ordering {
        // The heap root is the worst retained candidate.
        other
            .score
            .total_cmp(&self.score)
            .then_with(|| self.id.cmp(&other.id))
    }
}
impl PartialOrd for Ranked {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone, Eq, PartialEq)]
struct GroupEntry {
    group: String,
    rank: Ranked,
}
impl Ord for GroupEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        self.rank
            .cmp(&other.rank)
            .then_with(|| self.group.cmp(&other.group))
    }
}
impl PartialOrd for GroupEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Keep the best fragment of each original source while bounding all heap state.
pub struct SourceTopK {
    limit: usize,
    heap: BinaryHeap<GroupEntry>,
    sources: BTreeMap<String, Ranked>,
}
impl SourceTopK {
    pub fn new(limit: usize) -> Self {
        Self {
            limit,
            heap: BinaryHeap::new(),
            sources: BTreeMap::new(),
        }
    }
    fn clean(&mut self) {
        while self
            .heap
            .peek()
            .is_some_and(|e| self.sources.get(&e.group) != Some(&e.rank))
        {
            self.heap.pop();
        }
    }
    pub fn push(&mut self, group: String, rank: Ranked) {
        if self.limit == 0 {
            return;
        }
        if self
            .sources
            .get(&group)
            .is_some_and(|previous| rank >= *previous)
        {
            return;
        }
        self.clean();
        if !self.sources.contains_key(&group)
            && self.sources.len() == self.limit
            && self.heap.peek().is_some_and(|worst| rank >= worst.rank)
        {
            return;
        }
        self.sources.insert(group.clone(), rank.clone());
        self.heap.push(GroupEntry { group, rank });
        self.clean();
        while self.sources.len() > self.limit {
            if let Some(worst) = self.heap.pop() {
                self.sources.remove(&worst.group);
            }
            self.clean();
        }
        if self.heap.len() > self.limit.saturating_mul(2) {
            self.heap = self
                .sources
                .iter()
                .map(|(group, rank)| GroupEntry {
                    group: group.clone(),
                    rank: rank.clone(),
                })
                .collect();
        }
    }
    pub fn finish(self) -> Vec<(String, Ranked)> {
        let mut rows = self.sources.into_iter().collect::<Vec<_>>();
        rows.sort_unstable_by(|a, b| {
            b.1.score
                .total_cmp(&a.1.score)
                .then_with(|| a.1.id.cmp(&b.1.id))
        });
        rows
    }
}

pub fn check_cancelled(cancelled: &AtomicBool) -> Result<(), String> {
    if cancelled.load(AtomicOrdering::Relaxed) {
        Err("Recall operation cancelled".into())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_ranking_keeps_best_and_breaks_ties_by_source() {
        let mut top = SourceTopK::new(2);
        for (id, score) in [("z", 0.9), ("b", 0.9), ("c", 0.2), ("a", 0.9)] {
            top.push(
                id.into(),
                Ranked {
                    id: id.into(),
                    score,
                },
            );
        }
        assert_eq!(
            top.finish()
                .iter()
                .map(|r| r.1.id.as_str())
                .collect::<Vec<_>>(),
            vec!["a", "b"]
        );
    }
    #[test]
    fn fragments_do_not_displace_other_sources_or_grow_heap_without_bound() {
        let mut top = SourceTopK::new(2);
        top.push(
            "other".into(),
            Ranked {
                id: "other-fragment".into(),
                score: 0.8,
            },
        );
        for n in 0..1000 {
            top.push(
                "long-document".into(),
                Ranked {
                    id: format!("fragment-{n}"),
                    score: n as f32 / 1000.0,
                },
            );
            assert!(top.heap.len() <= 4);
        }
        let rows = top.finish();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].1.id, "fragment-999");
        assert_eq!(rows[1].0, "other");
    }
    #[test]
    fn rejects_zero_nonfinite_and_mismatched_embeddings() {
        assert!(normalize(&mut [0.0, 0.0], 2).is_err());
        assert!(normalize(&mut [f32::NAN], 1).is_err());
        assert!(normalize(&mut [1.0], 2).is_err());
        let mut v = [3.0, 4.0];
        normalize(&mut v, 2).unwrap();
        assert!((cosine_blob(&v, &encode(&v)).unwrap() - 1.0).abs() < 1e-6);
    }

    /// Run explicitly in the root-owned native resource slot. This measures the
    /// streaming scorer; SQLite batch I/O must be measured separately on the host.
    #[test]
    #[ignore = "explicit CPU benchmark: 10k/100k/1m x384"]
    fn benchmark_streaming_vectors() {
        use std::time::Instant;
        let mut query = vec![1.0; 384];
        normalize(&mut query, 384).unwrap();
        let blob = encode(&query);
        for count in [10_000, 100_000, 1_000_000] {
            let start = Instant::now();
            let mut top = SourceTopK::new(100);
            for id in 0..count {
                top.push(
                    format!("{id:08}"),
                    Ranked {
                        id: format!("{id:08}"),
                        score: cosine_blob(
                            std::hint::black_box(&query),
                            std::hint::black_box(&blob),
                        )
                        .unwrap(),
                    },
                );
            }
            assert_eq!(top.finish().len(), 100);
            eprintln!(
                "recall-vector benchmark count={count} dimensions=384 elapsed_ms={}",
                start.elapsed().as_millis()
            );
        }
        let cancel = AtomicBool::new(true);
        let start = Instant::now();
        assert!(check_cancelled(&cancel).is_err());
        assert!(start.elapsed().as_millis() < 100);
    }
}
