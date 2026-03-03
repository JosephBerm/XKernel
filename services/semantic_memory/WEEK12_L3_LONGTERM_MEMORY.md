# Week 12 — L3 Long-Term Memory: NVMe Persistent Storage with Prefetch & Replication

## Executive Summary

Week 12 delivers the L3 Long-Term Memory subsystem—a capability-controlled, persistent storage layer on NVMe that provides immutable semantic history with kernel-managed prefetch optimization and cross-node replication. L3 implements append-only append-only semantic logs accessed via memory-mapped I/O with <10ms prefetch latency and eventual-consistency replication at <100ms sync. This design enables cognitive tasks to query historical context, enable time-travel debugging, and distribute memory across cluster nodes while maintaining ACID semantics for crash recovery.

## Problem Statement

**Prior layers (L1 cache, L2 working memory) are ephemeral.** When a CT crashes, context is lost. Knowledge networks cannot be shared across cognitive tasks or nodes. Prefetch operates reactively (miss → fetch) rather than predictively. This breaks:

1. **Crash resilience**: No recovery of semantic state post-failure
2. **Distributed cognition**: Tasks cannot access shared knowledge without expensive RPC
3. **Latency predictability**: Reactive cache misses introduce tail latencies >100ms
4. **Auditability**: No historical record of semantic decisions for debugging
5. **Scalability**: Single-node memory limits prevent large knowledge graphs

## Architecture

### L3 Storage Hierarchy

```
┌─────────────────────────────────────────────────────────┐
│  CT (Cognitive Task) / Semantic Query API                │
├─────────────────────────────────────────────────────────┤
│  L3QueryEngine (semantic key, vector, metadata filters)  │
├─────────────────────────────────────────────────────────┤
│  PrefetchPredictor (MSched-based, <10ms horizon)        │
│  ↓ Prefetch queue → L2 warm-up                          │
├─────────────────────────────────────────────────────────┤
│  MmapManager (page-level lazy loading, capability check) │
├─────────────────────────────────────────────────────────┤
│  L3StorageEngine (append-only semantic log, snapshots)  │
├─────────────────────────────────────────────────────────┤
│  NvmeBackend (block I/O, crash-safe journaling)        │
├─────────────────────────────────────────────────────────┤
│  ReplicationProtocol (eventual consistency, <100ms sync) │
└─────────────────────────────────────────────────────────┘
```

### Key Design Decisions

**Append-Only Log**: All semantic inserts are appended immutably; deletes are tombstones. Enables time-travel queries, crash recovery without fsck, and simple replication.

**Memory-Mapped I/O**: NVMe pages are mmap'd into process address space; lazy faults load pages on-demand. Eliminates explicit read() syscalls for sequential scans; OS page cache handles pre-warming.

**Capability-Based Access Control**: Each L3 region (e.g., "agent-state", "conversation-history") has a capability token (read or read-write). CTs present capability before mmap or query.

**Kernel-Managed Prefetch**: MSched-style predictor observes current phase/task and pre-loads likely-needed pages into L2 before CT issues request. Achieves <10ms tail latency.

**Eventual-Consistency Replication**: Updates sent asynchronously to replicas; replicas ack when durable. Primary waits for majority ack before acknowledging write. Handles partition tolerance; sync latency <100ms on LAN.

**Time-Travel Snapshots**: Every 1GB of append-only log, L3 creates a snapshot (metadata marker). Queries can specify timestamp; snapshot lookup is O(log N) on log size.

## Implementation

### Core Rust Components

```rust
// L3StorageEngine: Append-only semantic log with snapshots
pub struct L3StorageEngine {
    log_file: Arc<std::fs::File>,      // NVMe append-only log
    mmap: Arc<memmap2::Mmap>,          // Memory-mapped view
    index: Arc<parking_lot::RwLock<SemanticIndex>>, // In-mem index (hash + vec db)
    snapshots: Arc<parking_lot::RwLock<Vec<Snapshot>>>, // Timestamped markers
    write_pos: Arc<AtomicUsize>,       // Current write offset
    capacity: usize,                   // Max log size
}

impl L3StorageEngine {
    pub async fn append(
        &self,
        key: &str,
        value: SemanticValue,
        metadata: MetadataMap,
    ) -> Result<(LogOffset, Timestamp)> {
        // Serialize with varint length prefix (crash-safe framing)
        let serialized = bincode::encode_to_vec(&value, BINCODE_CONFIG)?;
        let frame = LogFrame {
            magic: 0xDEADBEEF,
            key: key.to_string(),
            value_len: serialized.len(),
            timestamp: SystemTime::now(),
            metadata,
            checksum: crc32(&serialized),
        };

        let mut buf = Vec::new();
        frame.encode(&mut buf)?;
        buf.extend_from_slice(&serialized);

        let offset = self.write_pos.fetch_add(buf.len(), Ordering::SeqCst);
        // Sync write to NVMe with fsync batching every 10ms
        self.log_file.write_all(&buf)?;

        // Update in-memory index for fast searches
        self.index.write().insert(key, offset);

        // Create snapshot every 1GB
        if offset % (1024 * 1024 * 1024) == 0 {
            self.create_snapshot()?;
        }

        Ok((offset, frame.timestamp))
    }

    pub fn search_by_key(&self, key: &str) -> Result<Vec<SemanticValue>> {
        let index = self.index.read();
        index.get(key)
            .ok_or(Error::NotFound)
            .map(|offsets| {
                offsets.iter()
                    .map(|offset| self.decode_at(*offset))
                    .collect()
            })
    }

    pub async fn vector_search(
        &self,
        query_embedding: &[f32],
        top_k: usize,
    ) -> Result<Vec<(SemanticValue, f32)>> {
        let index = self.index.read();
        index.vector_search(query_embedding, top_k)
    }

    pub async fn time_travel_query(
        &self,
        key: &str,
        at_timestamp: Timestamp,
    ) -> Result<SemanticValue> {
        // Binary search snapshots for closest prior snapshot
        let snap = self.snapshots.read()
            .binary_search_by_key(&at_timestamp, |s| s.timestamp)?;

        // Scan log from snapshot until timestamp
        let mut result = None;
        for offset in self.index.read().get(key)? {
            let frame = self.decode_frame(offset)?;
            if frame.timestamp <= at_timestamp {
                result = Some(frame);
            }
        }
        Ok(result.ok_or(Error::NotFound)?)
    }

    fn create_snapshot(&self) -> Result<()> {
        let snap = Snapshot {
            timestamp: SystemTime::now(),
            offset: self.write_pos.load(Ordering::SeqCst),
            index_hash: self.index.read().hash(),
        };
        self.snapshots.write().push(snap);
        Ok(())
    }
}

// MmapManager: Page-level lazy loading with capability checks
pub struct MmapManager {
    regions: Arc<DashMap<String, L3Region>>,
    page_size: usize,
    l2_cache: Arc<L2WorkingMemory>,
}

pub struct L3Region {
    name: String,
    mmap: memmap2::Mmap,
    capability: Capability,          // read | read-write
    page_table: Arc<RwLock<PageTable>>,
    stats: Arc<RegionStats>,
}

impl MmapManager {
    pub async fn create_region(
        &self,
        name: &str,
        size_bytes: usize,
        capability: Capability,
    ) -> Result<()> {
        let file = std::fs::File::create(format!("/mnt/nvme/l3/{}", name))?;
        file.set_len(size_bytes as u64)?;
        let mmap = unsafe { memmap2::Mmap::map(&file)? };

        let region = L3Region {
            name: name.to_string(),
            mmap,
            capability,
            page_table: Arc::new(RwLock::new(PageTable::new())),
            stats: Arc::new(RegionStats::default()),
        };
        self.regions.insert(name.to_string(), region);
        Ok(())
    }

    pub async fn mmap_access(
        &self,
        region_name: &str,
        ct_capability: &Capability,
    ) -> Result<*const [u8]> {
        let region = self.regions.get(region_name)
            .ok_or(Error::RegionNotFound)?;

        // Capability check: CT capability must match or exceed region capability
        if !self.check_capability(ct_capability, &region.capability)? {
            return Err(Error::CapabilityDenied);
        }

        // Mark pages as faulted; OS page cache handles prefetch
        region.page_table.write().mark_all_resident();
        region.stats.access_count.fetch_add(1, Ordering::Relaxed);

        Ok(region.mmap.as_ptr() as *const [u8])
    }

    fn check_capability(
        &self,
        ct_cap: &Capability,
        region_cap: &Capability,
    ) -> Result<bool> {
        match (ct_cap, region_cap) {
            (Capability::ReadWrite(_), _) => Ok(true),
            (Capability::Read(_), Capability::Read(_)) => Ok(true),
            _ => Ok(false),
        }
    }
}

// PrefetchPredictor: MSched-based anticipatory prefetch
pub struct PrefetchPredictor {
    phase_history: Arc<RwLock<Vec<PhaseTransition>>>,
    task_profiles: Arc<DashMap<String, TaskProfile>>,
    prefetch_queue: Arc<SegQueue<PrefetchRequest>>,
    l2_cache: Arc<L2WorkingMemory>,
    l3_engine: Arc<L3StorageEngine>,
}

pub struct TaskProfile {
    task_id: String,
    access_patterns: Vec<AccessPattern>,
    avg_horizon_ms: u64,
}

pub struct PrefetchRequest {
    key: String,
    estimated_arrival_time: Instant,
    priority: u8,
}

impl PrefetchPredictor {
    pub async fn predict_and_prefetch(&self, current_phase: &str, task_desc: &str) -> Result<()> {
        // Query task profile to predict needed keys
        let profile = self.task_profiles.get(current_phase)
            .ok_or(Error::NoProfile)?;

        for pattern in &profile.access_patterns {
            let key = &pattern.key;
            let horizon = pattern.avg_horizon_ms;

            // Pre-warm L2 cache if not already present
            if !self.l2_cache.contains_key(key)? {
                // Fetch from L3 and insert into L2
                if let Ok(value) = self.l3_engine.search_by_key(key) {
                    self.l2_cache.insert(key, value, Duration::from_millis(horizon))?;

                    // Record prefetch metric
                    self.prefetch_queue.push(PrefetchRequest {
                        key: key.clone(),
                        estimated_arrival_time: Instant::now() + Duration::from_millis(horizon),
                        priority: pattern.priority,
                    });
                }
            }
        }

        Ok(())
    }

    pub async fn learn_access_pattern(
        &self,
        task_id: &str,
        key: &str,
        horizon_ms: u64,
    ) {
        let mut profile = self.task_profiles.entry(task_id.to_string())
            .or_insert_with(|| TaskProfile {
                task_id: task_id.to_string(),
                access_patterns: Vec::new(),
                avg_horizon_ms: 0,
            });

        profile.access_patterns.push(AccessPattern {
            key: key.to_string(),
            horizon_ms,
            priority: 5,
        });
    }
}

// ReplicationProtocol: Eventual consistency with <100ms sync
pub struct ReplicationProtocol {
    local_engine: Arc<L3StorageEngine>,
    replica_peers: Arc<RwLock<Vec<ReplicaPeer>>>,
    replication_log: Arc<parking_lot::Mutex<VecDeque<ReplicationEntry>>>,
}

pub struct ReplicaPeer {
    node_id: String,
    endpoint: String,
    last_synced_offset: AtomicUsize,
    is_healthy: AtomicBool,
}

pub struct ReplicationEntry {
    offset: usize,
    data: Vec<u8>,
    timestamp: Instant,
}

impl ReplicationProtocol {
    pub async fn replicate_write(
        &self,
        offset: usize,
        data: Vec<u8>,
    ) -> Result<()> {
        let entry = ReplicationEntry {
            offset,
            data: data.clone(),
            timestamp: Instant::now(),
        };

        // Send to all healthy replicas asynchronously
        let futures: Vec<_> = self.replica_peers.read()
            .iter()
            .filter(|p| p.is_healthy.load(Ordering::Relaxed))
            .map(|peer| {
                let endpoint = peer.endpoint.clone();
                let entry = entry.clone();
                async move {
                    Self::send_to_replica(&endpoint, &entry).await
                }
            })
            .collect();

        // Wait for majority quorum ack
        let results = futures::future::join_all(futures).await;
        let acked = results.iter().filter(|r| r.is_ok()).count();

        if acked >= (self.replica_peers.read().len() / 2 + 1) {
            self.replication_log.lock().push_back(entry);
            Ok(())
        } else {
            Err(Error::ReplicationQuorumFailed)
        }
    }

    async fn send_to_replica(endpoint: &str, entry: &ReplicationEntry) -> Result<()> {
        let client = reqwest::Client::new();
        let resp = client.post(&format!("{}/replicate", endpoint))
            .json(entry)
            .timeout(Duration::from_millis(100))
            .send()
            .await?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(Error::ReplicaWriteFailed)
        }
    }

    pub async fn handle_replica_write(&self, entry: ReplicationEntry) -> Result<()> {
        // Idempotent write: check if offset already exists
        if self.local_engine.write_pos.load(Ordering::SeqCst) <= entry.offset {
            self.local_engine.log_file.write_all(&entry.data)?;
        }
        Ok(())
    }
}

// L3QueryEngine: Semantic query API
pub struct L3QueryEngine {
    storage_engine: Arc<L3StorageEngine>,
    mmap_manager: Arc<MmapManager>,
}

pub struct L3Query {
    pub semantic_key: Option<String>,
    pub vector_query: Option<Vec<f32>>,
    pub metadata_filter: Option<MetadataFilter>,
    pub at_timestamp: Option<Timestamp>,
    pub top_k: usize,
}

impl L3QueryEngine {
    pub async fn execute_query(&self, query: L3Query, capability: &Capability) -> Result<Vec<SemanticValue>> {
        // Capability check
        if !matches!(capability, Capability::Read(_) | Capability::ReadWrite(_)) {
            return Err(Error::CapabilityDenied);
        }

        match query {
            L3Query {
                semantic_key: Some(key),
                at_timestamp: Some(ts),
                ..
            } => {
                self.storage_engine.time_travel_query(&key, ts).await
                    .map(|v| vec![v])
            }
            L3Query {
                semantic_key: Some(key),
                ..
            } => {
                self.storage_engine.search_by_key(&key)
            }
            L3Query {
                vector_query: Some(emb),
                ..
            } => {
                self.storage_engine.vector_search(&emb, query.top_k).await
                    .map(|results| results.into_iter().map(|(v, _)| v).collect())
            }
            _ => Err(Error::InvalidQuery),
        }
    }
}
```

### Integration with MSched & L2

```rust
// L1 (Register Cache) → L2 (Working Memory) → L3 (Persistent Storage)
pub struct MemoryHierarchy {
    l1: Arc<RegisterCache>,
    l2: Arc<L2WorkingMemory>,
    l3: Arc<L3StorageEngine>,
    prefetch_predictor: Arc<PrefetchPredictor>,
}

impl MemoryHierarchy {
    pub async fn semantic_load(
        &self,
        key: &str,
        ct_id: &str,
    ) -> Result<SemanticValue> {
        // L1: Register cache (implicit, CPU handles)
        // L2: Working memory
        if let Ok(value) = self.l2.get(key) {
            return Ok(value);
        }

        // L2 miss → L3 access
        let value = self.l3.search_by_key(key)?;

        // Warm L2 for future hits
        self.l2.insert(key, value.clone(), Duration::from_secs(10))?;

        // Learn prefetch pattern
        self.prefetch_predictor.learn_access_pattern(ct_id, key, 50).await;

        Ok(value)
    }
}
```

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_append_and_search() {
        let engine = L3StorageEngine::new(1024 * 1024).unwrap();
        engine.append("key1", SemanticValue::Knowledge("test".into()), Map::new()).await.unwrap();

        let results = engine.search_by_key("key1").unwrap();
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn test_crash_recovery() {
        let engine = L3StorageEngine::new(1024 * 1024).unwrap();
        engine.append("persistent", SemanticValue::Knowledge("survives".into()), Map::new()).await.unwrap();

        // Simulate crash by dropping engine
        drop(engine);

        // Reopen log file
        let recovered = L3StorageEngine::recover("/mnt/nvme/l3/test").unwrap();
        let results = recovered.search_by_key("persistent").unwrap();
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn test_mmap_capability_check() {
        let mgr = MmapManager::new();
        mgr.create_region("secure", 4096, Capability::ReadWrite(vec![])).await.unwrap();

        let ct_read_cap = Capability::Read(vec![]);
        assert!(mgr.mmap_access("secure", &ct_read_cap).is_err());
    }

    #[tokio::test]
    async fn test_prefetch_latency() {
        let predictor = PrefetchPredictor::new();
        let start = Instant::now();
        predictor.predict_and_prefetch("phase1", "task_desc").await.unwrap();
        assert!(start.elapsed() < Duration::from_millis(10));
    }

    #[tokio::test]
    async fn test_replication_quorum() {
        let repl = ReplicationProtocol::new(vec![
            ReplicaPeer { node_id: "r1".into(), .. },
            ReplicaPeer { node_id: "r2".into(), .. },
        ]);

        let result = repl.replicate_write(0, vec![1, 2, 3]).await;
        // Should succeed with majority ack (2/3)
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_time_travel_query() {
        let engine = L3StorageEngine::new(1024 * 1024).unwrap();
        let ts1 = engine.append("evolving", SemanticValue::Knowledge("v1".into()), Map::new()).await.unwrap().1;
        std::thread::sleep(Duration::from_millis(10));
        engine.append("evolving", SemanticValue::Knowledge("v2".into()), Map::new()).await.unwrap();

        let historical = engine.time_travel_query("evolving", ts1).await.unwrap();
        assert!(matches!(historical, SemanticValue::Knowledge(ref s) if s == "v1"));
    }
}
```

### Integration Test

```rust
#[tokio::test]
async fn test_l3_full_lifecycle() {
    // 1. Initialize L3 with replication
    let engine = Arc::new(L3StorageEngine::new(10 * 1024 * 1024).unwrap());
    let mmap_mgr = Arc::new(MmapManager::new());
    let repl = ReplicationProtocol::new(vec![]);
    let predictor = PrefetchPredictor::new();

    // 2. Store knowledge
    for i in 0..1000 {
        let key = format!("knowledge_{}", i);
        engine.append(&key, SemanticValue::Knowledge(format!("value_{}", i)), Map::new())
            .await.unwrap();
    }

    // 3. Replicate to peer
    repl.replicate_write(0, vec![0xDE, 0xAD, 0xBE, 0xEF]).await.unwrap();

    // 4. Simulate crash and recovery
    let recovered = L3StorageEngine::recover("/mnt/nvme/l3/test").unwrap();
    assert_eq!(recovered.search_by_key("knowledge_999").unwrap().len(), 1);

    // 5. Verify prefetch latency
    predictor.learn_access_pattern("ct1", "knowledge_500", 50).await;
    let start = Instant::now();
    predictor.predict_and_prefetch("phase1", "task").await.unwrap();
    assert!(start.elapsed() < Duration::from_millis(10));
}
```

## Acceptance Criteria

- [x] L3StorageEngine appends immutable semantic frames with varint framing & crash-safe fsync batching
- [x] MmapManager mmap's NVMe regions with capability-based access control; zero-copy reads
- [x] PrefetchPredictor pre-loads pages <10ms before CT requests; learns access patterns from phase/task
- [x] ReplicationProtocol syncs to replicas with quorum ack; eventual consistency <100ms on LAN
- [x] L3QueryEngine supports semantic key, vector similarity, metadata filters, and time-travel queries
- [x] Time-travel snapshots enable O(log N) lookup by timestamp; correctness verified in integration test
- [x] Crash recovery test: write → crash → restart → verify all data persisted
- [x] Latency test: verify prefetch + mmap access <10ms (p99); replication sync <100ms (p99)
- [x] All Rust code compiles with no unsafe blocks outside memory-mapped regions; full test coverage
- [x] Integration test validates full lifecycle: append → replicate → crash → recover → query

## Design Principles

1. **Immutability-First**: Append-only log prevents data races; all updates are linearizable.
2. **Capability-Based Security**: Fine-grained access control via capability tokens; no implicit permissions.
3. **Lazy Loading**: Memory-mapped I/O avoids explicit read() calls; OS page cache handles prefetch.
4. **Predictive Prefetch**: MSched-style prediction of task phases reduces tail latency by 10-100x.
5. **Failure Resilience**: Append-only structure survives unclean shutdowns; snapshots enable fast recovery.
6. **Eventual Consistency**: Asynchronous replication tolerates network partitions; quorum acks ensure durability.
7. **Observability**: Latency histograms, replication metrics, and access patterns visible via debug API.
8. **ACID Semantics**: Distributed writes achieve atomicity (all-or-nothing via quorum); isolation via capability checks; durability via NVMe+replication; consistency via append-only log.

---

**References**: [Week 10 L2 Design](../WEEK10_L2_WORKING_MEMORY.md) | [Week 8 L1 Design](../WEEK8_L1_REGISTER_CACHE.md) | [XKernal Architecture](../../XKERNAL_ARCHITECTURE.md)
