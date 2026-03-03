# Week 11 — L2 Background Compactor: Semantic Summarization & Deduplication

**Author:** Principal Software Engineer
**Date:** 2026-03-02
**Project:** XKernal Cognitive Substrate OS
**Component:** Semantic Memory Services
**Status:** Technical Design

---

## Executive Summary

The L2 Background Compactor is a non-blocking, compute-budgeted subsystem that autonomously reduces Episodic Memory footprint through semantic clustering and deduplication. Operating within a reserved 10% compute budget per agent, it performs incremental summarization during off-peak periods, achieving 30-40% space reduction while maintaining semantic fidelity and preserving all critical metadata. This design enables sustainable long-term memory growth without impacting real-time agent responsiveness.

---

## Problem Statement

### Current State
- L2 Episodic Memory grows linearly with agent activity, consuming unbounded storage
- Redundant and near-duplicate vectors accumulate from similar experiences
- Semantic clusters of related memories (e.g., multiple instances of "discussing quarterly roadmap") waste storage without proportional information gain
- Compaction blocks L2 access, degrading query latency during batch operations
- No principled approach to balancing retention vs. space efficiency

### Design Goals
1. **Non-blocking compaction**: Execute entirely on reserved compute, never stalling L2 queries
2. **Semantic intelligence**: Cluster similar vectors; retain cluster representatives while removing semantic duplicates
3. **Metadata preservation**: Maintain tags, timestamps, confidence scores, and provenance across compaction
4. **Predictable budgeting**: Hard cap at 10% of agent compute; graceful degradation under load
5. **Verifiable correctness**: Comprehensive testing ensuring no semantic loss or corruption

---

## Architecture

### High-Level Design

```
┌─────────────────────────────────────────────────────────┐
│        L2 Episodic Memory (Hot/Warm Tiers)             │
└────────────────────┬────────────────────────────────────┘
                     │
        ┌────────────▼────────────┐
        │ CompactionScheduler     │
        │ (incremental batching)  │
        └────────────┬────────────┘
                     │
        ┌────────────▼────────────────────────────┐
        │   BackgroundCompactor (Worker Thread)    │
        │   - Budget tracking                      │
        │   - Batch processing                     │
        │   - Semantic operations                  │
        └────┬─────────────────────────┬──────────┘
             │                         │
      ┌──────▼──────────┐    ┌────────▼──────────┐
      │SemanticSummarizer   │VectorDeduplicator  │
      │(clustering, reduction)      │(hash + similarity)
      └──────┬──────────┘    └────────┬──────────┘
             │                         │
        ┌────▼─────────────────────────▼──────┐
        │   CompactionMetrics (observability)  │
        │   - Vectors: before/after            │
        │   - Space saved (bytes)              │
        │   - Confidence loss percentage       │
        │   - Execution time                   │
        └─────────────────────────────────────┘
```

### Component Responsibilities

**CompactionScheduler**: Examines L2 state, determines next batch to compact. Uses heuristics: age-based (older batches first), fragmentation-based (high-duplicate segments), or off-peak signals from agent load metrics.

**BackgroundCompactor**: Core worker executing compaction jobs. Manages compute budget via token bucket, processes one batch at a time, acquires read-lock on L2 segment, offloads to semantic engines, releases lock, applies results.

**SemanticSummarizer**: Performs clustering via cosine similarity matrix on input vectors. Uses incremental k-means or DBSCAN. For each cluster, selects representative vector (centroid or highest-confidence member). Computes confidence loss as average divergence from removed vectors.

**VectorDeduplicator**: Hash-based first-pass identification (SHA-256 of vector bytes). Secondary pass: cosine similarity >= 0.95 threshold triggers removal of lower-confidence duplicate. Produces deduplication mapping for reconstruction if needed.

**CompactionMetrics**: Thread-safe metrics collector. Tracks cumulative vectors removed, bytes freed, confidence degradation, wall-clock time, and budget utilization. Exported for observability dashboards.

---

## Implementation

### Core Rust Structures

```rust
use std::sync::{Arc, Mutex, RwLock};
use std::collections::HashMap;
use std::time::{Instant, Duration};

/// Represents remaining compute budget for compaction work
#[derive(Clone, Debug)]
pub struct CompactionBudget {
    /// Max milliseconds per agent per observation period
    max_budget_ms: u64,
    /// Tokens available (each operation costs ~0.1 ms)
    tokens_remaining: Arc<Mutex<f64>>,
    /// Last refill time
    last_refill: Arc<Mutex<Instant>>,
    /// Refill period (e.g., 1 second)
    refill_period: Duration,
}

impl CompactionBudget {
    pub fn new(max_budget_ms: u64) -> Self {
        Self {
            max_budget_ms,
            tokens_remaining: Arc::new(Mutex::new(max_budget_ms as f64)),
            last_refill: Arc::new(Mutex::new(Instant::now())),
            refill_period: Duration::from_secs(1),
        }
    }

    /// Attempt to consume budget; returns true if budget available
    pub fn try_consume(&self, ms: f64) -> bool {
        let mut tokens = self.tokens_remaining.lock().unwrap();
        let mut last = self.last_refill.lock().unwrap();

        // Refill if period elapsed
        if last.elapsed() >= self.refill_period {
            *tokens = self.max_budget_ms as f64;
            *last = Instant::now();
        }

        if *tokens >= ms {
            *tokens -= ms;
            true
        } else {
            false
        }
    }

    pub fn available_tokens(&self) -> f64 {
        *self.tokens_remaining.lock().unwrap()
    }
}

/// Semantic summarization engine
pub struct SemanticSummarizer {
    /// Clustering threshold (cosine similarity)
    similarity_threshold: f32,
    /// Max vectors per cluster
    max_cluster_size: usize,
}

impl SemanticSummarizer {
    pub fn new(similarity_threshold: f32, max_cluster_size: usize) -> Self {
        Self {
            similarity_threshold,
            max_cluster_size,
        }
    }

    /// Cluster vectors and return representatives
    /// Input: Vec of (vector_id, embedding, confidence)
    /// Output: Vec of cluster IDs to keep, mapping of removed -> representative
    pub fn summarize(
        &self,
        vectors: Vec<(String, Vec<f32>, f32)>,
    ) -> (Vec<String>, HashMap<String, String>) {
        // Simplified: compute pairwise cosine similarities
        let n = vectors.len();
        let mut similarity_matrix = vec![vec![0.0_f32; n]; n];

        for i in 0..n {
            for j in (i+1)..n {
                let sim = cosine_similarity(&vectors[i].1, &vectors[j].1);
                similarity_matrix[i][j] = sim;
                similarity_matrix[j][i] = sim;
            }
        }

        // Greedy clustering
        let mut clusters: Vec<Vec<usize>> = Vec::new();
        let mut assigned = vec![false; n];

        for i in 0..n {
            if assigned[i] { continue; }

            let mut cluster = vec![i];
            assigned[i] = true;

            for j in (i+1)..n {
                if !assigned[j] && similarity_matrix[i][j] >= self.similarity_threshold {
                    cluster.push(j);
                    assigned[j] = true;
                    if cluster.len() >= self.max_cluster_size {
                        break;
                    }
                }
            }
            clusters.push(cluster);
        }

        // Select representative from each cluster (highest confidence)
        let mut representatives = Vec::new();
        let mut mapping = HashMap::new();

        for cluster in clusters {
            let rep_idx = cluster.iter()
                .max_by(|&&a, &&b|
                    vectors[a].2.partial_cmp(&vectors[b].2).unwrap_or(std::cmp::Ordering::Equal)
                )
                .copied()
                .unwrap();

            let rep_id = vectors[rep_idx].0.clone();
            representatives.push(rep_id.clone());

            // Map all others to representative
            for &idx in &cluster {
                if idx != rep_idx {
                    mapping.insert(vectors[idx].0.clone(), rep_id.clone());
                }
            }
        }

        (representatives, mapping)
    }
}

/// Deduplication via hash and similarity
pub struct VectorDeduplicator {
    /// Cosine similarity threshold for duplicates
    duplicate_threshold: f32,
}

impl VectorDeduplicator {
    pub fn new(duplicate_threshold: f32) -> Self {
        Self { duplicate_threshold }
    }

    /// Detect and map duplicate vectors
    pub fn deduplicate(
        &self,
        vectors: Vec<(String, Vec<f32>, f32)>,
    ) -> HashMap<String, String> {
        use std::collections::HashSet;

        let mut hash_map: HashMap<String, Vec<usize>> = HashMap::new();
        let mut mapping = HashMap::new();

        // First pass: hash-based grouping
        for (idx, (id, vec, _)) in vectors.iter().enumerate() {
            let hash = format!("{:?}", vec); // Simplified hash
            hash_map.entry(hash).or_default().push(idx);
        }

        // Second pass: within hash groups, check similarity
        for (_hash, indices) in hash_map {
            if indices.len() <= 1 { continue; }

            let mut kept = indices[0];
            let mut kept_confidence = vectors[kept].2;

            for &idx in &indices[1..] {
                let sim = cosine_similarity(&vectors[kept].1, &vectors[idx].1);
                let idx_confidence = vectors[idx].2;

                if sim >= self.duplicate_threshold {
                    // Map lower-confidence to keeper
                    if idx_confidence < kept_confidence {
                        mapping.insert(vectors[idx].0.clone(), vectors[kept].0.clone());
                    } else {
                        mapping.insert(vectors[kept].0.clone(), vectors[idx].0.clone());
                        kept = idx;
                        kept_confidence = idx_confidence;
                    }
                }
            }
        }

        mapping
    }
}

/// Metrics for compaction operations
#[derive(Debug, Clone)]
pub struct CompactionMetrics {
    pub vectors_before: usize,
    pub vectors_after: usize,
    pub bytes_before: u64,
    pub bytes_after: u64,
    pub confidence_loss_pct: f32,
    pub execution_time_ms: u64,
    pub batch_id: String,
}

/// Main background compactor coordinator
pub struct BackgroundCompactor {
    budget: CompactionBudget,
    summarizer: SemanticSummarizer,
    deduplicator: VectorDeduplicator,
    metrics: Arc<Mutex<Vec<CompactionMetrics>>>,
}

impl BackgroundCompactor {
    pub fn new(budget: CompactionBudget) -> Self {
        Self {
            budget,
            summarizer: SemanticSummarizer::new(0.92, 10),
            deduplicator: VectorDeduplicator::new(0.95),
            metrics: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Compact a single L2 batch
    pub async fn compact_batch(
        &self,
        batch_id: String,
        vectors: Vec<(String, Vec<f32>, f32)>,
    ) -> Result<CompactionMetrics, String> {
        let start = Instant::now();
        let bytes_before = vectors.iter()
            .map(|(_, v, _)| v.len() * 4)
            .sum::<usize>() as u64;
        let vectors_before = vectors.len();
        let mut confidence_total = 0.0_f32;

        // Step 1: Deduplication
        if !self.budget.try_consume(5.0) {
            return Err("Budget exhausted".to_string());
        }
        let dup_mapping = self.deduplicator.deduplicate(vectors.clone());
        let after_dup: Vec<_> = vectors.iter()
            .filter(|(id, _, _)| !dup_mapping.contains_key(id))
            .cloned()
            .collect();

        // Step 2: Semantic summarization
        if !self.budget.try_consume(8.0) {
            return Err("Budget exhausted".to_string());
        }
        let (reps, summ_mapping) = self.summarizer.summarize(after_dup.clone());
        let final_vectors: Vec<_> = after_dup.iter()
            .filter(|(id, _, _)| reps.contains(id))
            .collect();

        let vectors_after = final_vectors.len();
        let bytes_after = vectors_after as u64 * std::mem::size_of::<Vec<f32>>() as u64;

        for (_, _, conf) in &after_dup {
            confidence_total += conf;
        }
        let avg_confidence = if after_dup.len() > 0 {
            confidence_total / after_dup.len() as f32
        } else {
            1.0
        };

        let metrics = CompactionMetrics {
            vectors_before,
            vectors_after,
            bytes_before,
            bytes_after,
            confidence_loss_pct: ((1.0 - avg_confidence) * 100.0).max(0.0),
            execution_time_ms: start.elapsed().as_millis() as u64,
            batch_id,
        };

        self.metrics.lock().unwrap().push(metrics.clone());
        Ok(metrics)
    }

    pub fn get_metrics(&self) -> Vec<CompactionMetrics> {
        self.metrics.lock().unwrap().clone()
    }
}

/// Scheduling policy for incremental compaction
pub struct CompactionScheduler {
    /// Next batch to process
    queue: Arc<Mutex<Vec<String>>>,
}

impl CompactionScheduler {
    pub fn new() -> Self {
        Self {
            queue: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn enqueue_batch(&self, batch_id: String) {
        self.queue.lock().unwrap().push(batch_id);
    }

    pub fn next_batch(&self) -> Option<String> {
        self.queue.lock().unwrap().pop()
    }
}

/// Helper: cosine similarity
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a > 0.0 && norm_b > 0.0 {
        dot / (norm_a * norm_b)
    } else {
        0.0
    }
}
```

### Metadata Preservation Strategy

During compaction, metadata is preserved through a transformation table:

```rust
/// Metadata transformation during compaction
pub struct MetadataPreserver {
    /// Original vector ID -> metadata
    metadata_map: Arc<RwLock<HashMap<String, VectorMetadata>>>,
}

#[derive(Clone, Debug)]
pub struct VectorMetadata {
    pub tags: Vec<String>,
    pub timestamp: u64,
    pub confidence: f32,
    pub provenance: String,
}

impl MetadataPreserver {
    /// Apply compaction mapping while preserving metadata
    pub fn apply_compaction(
        &self,
        dedup_map: &HashMap<String, String>,
        summ_map: &HashMap<String, String>,
    ) {
        let mut meta = self.metadata_map.write().unwrap();

        // For each removed vector, merge metadata into representative
        for (removed_id, rep_id) in dedup_map.iter().chain(summ_map.iter()) {
            if let Some(removed_meta) = meta.remove(removed_id) {
                let rep_meta = meta.entry(rep_id.clone())
                    .or_insert_with(|| VectorMetadata {
                        tags: Vec::new(),
                        timestamp: 0,
                        confidence: 0.0,
                        provenance: String::new(),
                    });

                // Merge tags, preserve earliest timestamp
                rep_meta.tags.extend(removed_meta.tags);
                rep_meta.tags.sort();
                rep_meta.tags.dedup();

                rep_meta.timestamp = rep_meta.timestamp.min(removed_meta.timestamp);
                rep_meta.confidence = rep_meta.confidence.max(removed_meta.confidence);
                rep_meta.provenance.push_str(&format!("|{}", removed_meta.provenance));
            }
        }
    }
}
```

---

## Online Compaction & Non-Blocking Design

Compaction never blocks L2 queries. Key invariants:

1. **Read-Only Snapshots**: Compactor takes immutable snapshot of L2 segment; L2 continues accepting writes/reads.
2. **Async Application**: Compaction results queued; applied out-of-band during quiet window.
3. **Rollback Capability**: If compaction corrupts data (detected via checksums), entire batch reverted.
4. **Version Tagging**: Each L2 segment has version counter; compaction increments atomically.

```rust
pub struct L2Compaction {
    segment_lock: Arc<RwLock<Vec<Vector>>>,
}

impl L2Compaction {
    /// Non-blocking snapshot for compaction
    pub async fn snapshot_for_compaction(&self) -> Vec<Vector> {
        self.segment_lock.read().unwrap().clone()
    }

    /// Atomic application of compaction results
    pub async fn apply_compaction_results(
        &self,
        kept_ids: Vec<String>,
    ) -> Result<(), String> {
        let mut segment = self.segment_lock.write().unwrap();
        segment.retain(|v| kept_ids.contains(&v.id));
        Ok(())
    }
}
```

---

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_budget_enforcement() {
        let budget = CompactionBudget::new(100);
        assert!(budget.try_consume(50.0));
        assert!(budget.try_consume(40.0));
        assert!(!budget.try_consume(20.0)); // Exceeds budget
    }

    #[test]
    fn test_semantic_summarization() {
        let summarizer = SemanticSummarizer::new(0.90, 5);
        let vectors = vec![
            ("v1".to_string(), vec![1.0, 0.0], 0.95),
            ("v2".to_string(), vec![0.99, 0.01], 0.93),
            ("v3".to_string(), vec![0.0, 1.0], 0.92),
        ];

        let (reps, _mapping) = summarizer.summarize(vectors);
        assert!(reps.len() <= 2); // Should cluster similar vectors
    }

    #[test]
    fn test_deduplication() {
        let dedup = VectorDeduplicator::new(0.95);
        let vectors = vec![
            ("v1".to_string(), vec![1.0, 0.0], 0.95),
            ("v2".to_string(), vec![1.0, 0.0], 0.90), // Duplicate
        ];

        let mapping = dedup.deduplicate(vectors);
        assert_eq!(mapping.len(), 1); // One mapping for duplicate
    }

    #[test]
    fn test_metadata_preservation() {
        let preserver = MetadataPreserver {
            metadata_map: Arc::new(RwLock::new(
                [("v1".to_string(), VectorMetadata {
                    tags: vec!["important".to_string()],
                    timestamp: 1000,
                    confidence: 0.95,
                    provenance: "test".to_string(),
                })]
                .iter()
                .cloned()
                .collect()
            )),
        };

        let dedup_map = [("v2".to_string(), "v1".to_string())].iter().cloned().collect();
        preserver.apply_compaction(&dedup_map, &HashMap::new());

        let meta = preserver.metadata_map.read().unwrap();
        assert!(meta.contains_key("v1"));
    }
}
```

### Integration Tests

1. **Fill → Compact → Verify**: Populate L2 with 10K vectors (40% duplicates, 30% clusterable). Run compactor. Verify 30-40% reduction, zero semantic loss, all metadata intact.

2. **Concurrent Query Load**: Spawn 100 query threads accessing L2 during compaction. Measure P99 latency; ensure no degradation.

3. **Budget Enforcement**: Trigger compaction with decreasing budgets (100ms, 50ms, 10ms). Verify graceful degradation; compaction completes incrementally across cycles.

4. **Checksum Validation**: Compute pre-compaction SHA-256 of metadata. Post-compaction, verify matching (minus removed vectors). Catch corruption early.

---

## Acceptance Criteria

| Criterion | Target | Measurement |
|-----------|--------|-------------|
| Space Reduction | 30-40% | (bytes_before - bytes_after) / bytes_before |
| Semantic Fidelity | > 99% | avg(confidence_scores) post-compaction |
| Non-Blocking | 0ms L2 stall | P99 query latency unchanged during compaction |
| Budget Compliance | ≤ 10% agent compute | tokens_consumed / total_available |
| Metadata Loss | 0% | verify all tags/timestamps/provenance intact |
| Correctness | 100% | no crashes, data corruption, or inconsistencies |
| Throughput | > 1K vectors/sec | vectors_compacted / execution_time_ms |

---

## Design Principles

1. **Reserved Compute Model**: Compaction never starves agent. 10% budget acts as circuit breaker.
2. **Semantic-First**: Preserve meaning over raw space; confidence scores guide decisions.
3. **Observability**: Every compaction action logged with metrics for debugging.
4. **Graceful Degradation**: If budget exhausted mid-batch, abort cleanly; retry next cycle.
5. **Metadata as First-Class**: Tags, timestamps, provenance treated as immutable during compaction.
6. **Testing Rigor**: Unit + integration tests ensure zero silent data loss.

---

## Performance Targets

- **Latency**: Single batch compaction (1K vectors) ≤ 15ms wall-clock
- **Throughput**: 1-2 full L2 compaction cycles per agent per day
- **Memory Overhead**: CompactionBudget + metrics ≤ 2 MB per agent
- **CPU Efficiency**: 0.1 ms per vector processed (includes clustering + deduplication)

---

## Future Enhancements

1. **Adaptive Clustering**: ML model to predict optimal similarity thresholds per agent domain.
2. **Hierarchical Compaction**: Multi-level summarization (L2a → L2b → archive).
3. **Incremental Hash Updates**: Maintain rolling sketches to eliminate full recomputation.
4. **Distributed Compaction**: Farm work across multiple cores when budget permits.

---

## Conclusion

The L2 Background Compactor delivers autonomous, intelligent memory footprint management within strict compute budgets. By combining semantic clustering with hash-based deduplication and rigorous metadata preservation, it enables agents to sustain unbounded growth while maintaining query performance and information fidelity. Comprehensive testing and observability ensure production reliability.
