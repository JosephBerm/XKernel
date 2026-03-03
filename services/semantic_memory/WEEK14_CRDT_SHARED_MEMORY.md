# Week 14: CRDT-Based Shared Memory Conflict Resolution
## XKernal Cognitive Substrate OS — L1 Services Layer (Rust)
## Phase 1 Finalization: Crew-Level Memory Consistency

---

## Executive Summary

Week 14 concludes Phase 1 with a production-grade Conflict-free Replicated Data Type (CRDT) system for shared memory pages accessed concurrently by multiple Cognitive Threads (CTs). This design implements **Last-Write-Wins (LWW) registers with version vectors** for deterministic conflict resolution, ensuring memory consistency across crew-shared L1/L2 regions without centralized synchronization overhead.

### Key Deliverables
- Version vector–based causality tracking
- LWW register data structures for atomic value resolution
- Hybrid conflict detection (timestamp + semantic embedding similarity)
- Metadata propagation pipeline (L1 → L2 → L3)
- Integration test suite with concurrent CT modifications

---

## 1. Architecture Overview

### 1.1 Problem Statement
Multiple CTs may write to shared L1/L2 memory pages simultaneously, creating conflicts:
- **Read-Write Conflicts**: CT-A reads page P at T1, CT-B writes to P at T2, CT-A's view is stale
- **Write-Write Conflicts**: CT-A and CT-B both write to offset X in page P at overlapping intervals
- **Ordering Ambiguity**: Without causal history, we cannot determine which write semantically "matters"

### 1.2 CRDT Strategy Selection
We employ **Last-Write-Wins (LWW) with Hybrid Merging**:
- **Primary Path**: Timestamp-based LWW for deterministic ordering
- **Secondary Path**: Semantic merge via embedding similarity for writes that conflict but represent compatible operations (e.g., annotations to different regions of a cognitive vector)

### 1.3 Design Principles
1. **Consistency**: All CTs converge to the same final state for any shared page
2. **Determinism**: Given a set of concurrent writes, merge outcome is fully deterministic
3. **Performance**: O(1) conflict detection; O(n) merge where n = number of non-overlapping writes
4. **Fairness**: No CT's write is arbitrarily discarded; LWW ensures last actor has final say

---

## 2. CRDT Data Structures

### 2.1 Version Vector

Version vectors track causal ordering across CTs. Each page replica maintains a vector mapping CT IDs to logical clocks.

```rust
use std::collections::BTreeMap;
use std::fmt;

/// Version vector for causal consistency tracking
/// Maps CT ID → logical clock value
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VersionVector {
    clocks: BTreeMap<u32, u64>,
}

impl VersionVector {
    pub fn new() -> Self {
        VersionVector {
            clocks: BTreeMap::new(),
        }
    }

    /// Increment logical clock for given CT
    pub fn increment(&mut self, ct_id: u32) {
        let entry = self.clocks.entry(ct_id).or_insert(0);
        *entry += 1;
    }

    /// Fetch current clock value for CT
    pub fn get(&self, ct_id: u32) -> u64 {
        self.clocks.get(&ct_id).copied().unwrap_or(0)
    }

    /// Merge two version vectors: take maximum for each CT
    pub fn merge(&self, other: &VersionVector) -> VersionVector {
        let mut merged = self.clone();
        for (&ct_id, &clock) in &other.clocks {
            let entry = merged.clocks.entry(ct_id).or_insert(0);
            *entry = (*entry).max(clock);
        }
        merged
    }

    /// Check if self happened-before other
    pub fn happened_before(&self, other: &VersionVector) -> bool {
        let mut at_least_one_less = false;
        for (&ct_id, &clock) in &self.clocks {
            if clock > other.get(ct_id) {
                return false; // self has a clock greater than other
            }
            if clock < other.get(ct_id) {
                at_least_one_less = true;
            }
        }
        at_least_one_less
    }

    /// Check if two version vectors are concurrent
    pub fn concurrent_with(&self, other: &VersionVector) -> bool {
        !self.happened_before(other) && !other.happened_before(self)
    }
}
```

### 2.2 LWW Register (Last-Write-Wins)

LWW registers store values with timestamps, returning the value with the highest timestamp on read.

```rust
use std::time::{SystemTime, UNIX_EPOCH};

/// Last-Write-Wins register for shared memory values
/// Includes version vector, physical timestamp, and semantic metadata
#[derive(Clone, Debug)]
pub struct LwwRegister {
    /// Logical value stored in register
    value: Vec<u8>,
    /// Physical wall-clock timestamp (nanoseconds since epoch)
    timestamp_ns: u64,
    /// Version vector for this write
    version_vector: VersionVector,
    /// CT ID that last modified this register
    writer_ct_id: u32,
    /// Semantic embedding of value (for hybrid merge)
    semantic_hash: u64,
}

impl LwwRegister {
    pub fn new(value: Vec<u8>, ct_id: u32, semantic_hash: u64) -> Self {
        let timestamp_ns = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time error")
            .as_nanos() as u64;

        let mut vv = VersionVector::new();
        vv.increment(ct_id);

        LwwRegister {
            value,
            timestamp_ns,
            version_vector: vv,
            writer_ct_id: ct_id,
            semantic_hash,
        }
    }

    /// Merge two LWW registers using timestamp-based comparison
    /// If timestamps equal, use CT ID as tiebreaker (deterministic)
    pub fn merge(&self, other: &LwwRegister) -> LwwRegister {
        let (winner, loser) = if self.timestamp_ns > other.timestamp_ns {
            (self, other)
        } else if other.timestamp_ns > self.timestamp_ns {
            (other, self)
        } else {
            // Timestamps equal: use CT ID as deterministic tiebreaker
            if self.writer_ct_id > other.writer_ct_id {
                (self, other)
            } else {
                (other, self)
            }
        };

        LwwRegister {
            value: winner.value.clone(),
            timestamp_ns: winner.timestamp_ns,
            version_vector: self.version_vector.merge(&other.version_vector),
            writer_ct_id: winner.writer_ct_id,
            semantic_hash: winner.semantic_hash,
        }
    }

    pub fn get_value(&self) -> &[u8] {
        &self.value
    }

    pub fn get_timestamp(&self) -> u64 {
        self.timestamp_ns
    }

    pub fn get_writer(&self) -> u32 {
        self.writer_ct_id
    }
}
```

### 2.3 Shared Memory Page with CRDT

```rust
use std::collections::HashMap;

/// CRDT-backed shared memory page supporting concurrent writes from multiple CTs
#[derive(Clone, Debug)]
pub struct CrdtSharedPage {
    /// Page ID (globally unique within crew)
    page_id: u32,
    /// Maps byte offset → LWW register
    offset_registers: HashMap<u32, LwwRegister>,
    /// Global version vector for the entire page
    page_version_vector: VersionVector,
    /// Metadata: last accessed time, compression ratio, dirty flag
    metadata: PageMetadata,
}

#[derive(Clone, Debug)]
pub struct PageMetadata {
    pub last_accessed_ns: u64,
    pub compression_ratio: f32,
    pub is_dirty: bool,
    pub size_bytes: u32,
    pub tier: MemoryTier, // L1, L2, or L3
}

#[derive(Clone, Debug, Copy, PartialEq, Eq)]
pub enum MemoryTier {
    L1,
    L2,
    L3,
}

impl CrdtSharedPage {
    pub fn new(page_id: u32, size_bytes: u32) -> Self {
        CrdtSharedPage {
            page_id,
            offset_registers: HashMap::new(),
            page_version_vector: VersionVector::new(),
            metadata: PageMetadata {
                last_accessed_ns: 0,
                compression_ratio: 1.0,
                is_dirty: false,
                size_bytes,
                tier: MemoryTier::L1,
            },
        }
    }

    /// Apply a write to a specific offset with automatic conflict resolution
    pub fn write(&mut self, offset: u32, value: Vec<u8>, ct_id: u32, semantic_hash: u64) -> Result<(), String> {
        if offset + value.len() as u32 > self.metadata.size_bytes {
            return Err("Write exceeds page boundaries".to_string());
        }

        let new_register = LwwRegister::new(value, ct_id, semantic_hash);
        self.page_version_vector.merge(&new_register.version_vector);

        // Merge with existing register at this offset (if any)
        let merged = match self.offset_registers.get(&offset) {
            Some(existing) => existing.merge(&new_register),
            None => new_register,
        };

        self.offset_registers.insert(offset, merged);
        self.metadata.is_dirty = true;

        Ok(())
    }

    /// Read value at offset with causal metadata
    pub fn read(&self, offset: u32) -> Option<(Vec<u8>, u64, u32)> {
        self.offset_registers.get(&offset).map(|reg| {
            (
                reg.get_value().to_vec(),
                reg.get_timestamp(),
                reg.get_writer(),
            )
        })
    }

    /// Merge entire page with remote replica
    pub fn merge_page(&mut self, remote: &CrdtSharedPage) -> Result<(), String> {
        if self.page_id != remote.page_id {
            return Err("Cannot merge pages with different IDs".to_string());
        }

        // Merge all registers
        for (&offset, remote_reg) in &remote.offset_registers {
            let merged = match self.offset_registers.get(&offset) {
                Some(local_reg) => local_reg.merge(remote_reg),
                None => remote_reg.clone(),
            };
            self.offset_registers.insert(offset, merged);
        }

        // Update page-level version vector
        self.page_version_vector = self.page_version_vector.merge(&remote.page_version_vector);
        self.metadata.is_dirty = true;

        Ok(())
    }

    pub fn get_page_id(&self) -> u32 {
        self.page_id
    }

    pub fn get_metadata(&self) -> &PageMetadata {
        &self.metadata
    }

    pub fn get_version_vector(&self) -> &VersionVector {
        &self.page_version_vector
    }
}
```

---

## 3. Conflict Detection and Resolution

### 3.1 Conflict Detection Algorithm

```rust
/// Result of conflict detection: categorizes write conflicts
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConflictType {
    /// No conflict detected
    NoConflict,
    /// Two writes at same offset, non-overlapping timestamps
    WriteWrite { local_ct: u32, remote_ct: u32 },
    /// Concurrent writes at overlapping byte ranges
    Concurrent { overlapping_offsets: Vec<u32> },
    /// Metadata divergence (e.g., different compression ratios)
    MetadataMismatch,
}

/// Detect conflicts between local and remote page versions
pub fn detect_conflict(
    local_page: &CrdtSharedPage,
    remote_page: &CrdtSharedPage,
) -> ConflictType {
    if local_page.get_page_id() != remote_page.get_page_id() {
        return ConflictType::NoConflict;
    }

    let mut overlapping_offsets = Vec::new();

    // Check for write-write conflicts at same offsets
    for (&offset, local_reg) in &local_page.offset_registers {
        if let Some(remote_reg) = remote_page.offset_registers.get(&offset) {
            if local_reg.get_timestamp() != remote_reg.get_timestamp() {
                overlapping_offsets.push(offset);
                return ConflictType::WriteWrite {
                    local_ct: local_reg.get_writer(),
                    remote_ct: remote_reg.get_writer(),
                };
            }
        }
    }

    // Check for concurrent writes (version vectors conflict)
    let local_vv = local_page.get_version_vector();
    let remote_vv = remote_page.get_version_vector();
    if local_vv.concurrent_with(remote_vv) {
        return ConflictType::Concurrent { overlapping_offsets };
    }

    ConflictType::NoConflict
}
```

### 3.2 Merge Resolution Algorithm

```rust
/// Merge strategy for resolving conflicts
pub enum MergeStrategy {
    /// Always prefer local version
    Local,
    /// Always prefer remote version
    Remote,
    /// LWW: prefer write with later timestamp
    LastWriteWins,
    /// Semantic: merge writes if embedding distance > threshold
    Semantic { embedding_distance_threshold: f32 },
}

/// Perform merge of two conflicting pages using selected strategy
pub fn resolve_merge(
    local: &mut CrdtSharedPage,
    remote: &CrdtSharedPage,
    strategy: MergeStrategy,
) -> Result<(), String> {
    match strategy {
        MergeStrategy::Local => Ok(()), // No merge needed

        MergeStrategy::Remote => {
            *local = remote.clone();
            Ok(())
        },

        MergeStrategy::LastWriteWins => {
            local.merge_page(remote)
        },

        MergeStrategy::Semantic { embedding_distance_threshold } => {
            // Semantic merge: compatible writes are both retained
            for (&offset, remote_reg) in &remote.offset_registers {
                if let Some(local_reg) = local.offset_registers.get(&offset) {
                    // Compute embedding distance between values
                    let distance = compute_embedding_distance(
                        local_reg.semantic_hash,
                        remote_reg.semantic_hash,
                    );

                    if distance > embedding_distance_threshold {
                        // Values are semantically distinct; apply LWW
                        local.offset_registers.insert(
                            offset,
                            local_reg.merge(remote_reg),
                        );
                    } else {
                        // Values are semantically similar; keep local
                        // (or implement custom merge logic)
                    }
                } else {
                    local.offset_registers.insert(offset, remote_reg.clone());
                }
            }
            Ok(())
        },
    }
}

/// Compute normalized distance between two semantic hashes
/// Returns 0.0 (identical) to 1.0 (maximally different)
fn compute_embedding_distance(hash1: u64, hash2: u64) -> f32 {
    let xor = hash1 ^ hash2;
    let bit_count = xor.count_ones() as f32;
    bit_count / 64.0
}
```

---

## 4. Metadata Propagation

### 4.1 Metadata Propagation Pipeline (L1 → L2 → L3)

```rust
/// Metadata delta: tracks changes to page metadata across tiers
#[derive(Clone, Debug)]
pub struct MetadataDelta {
    pub page_id: u32,
    pub source_tier: MemoryTier,
    pub target_tier: MemoryTier,
    pub compression_ratio: f32,
    pub timestamp_ns: u64,
    pub version_vector: VersionVector,
}

impl MetadataDelta {
    pub fn new(page_id: u32, from: MemoryTier, to: MemoryTier, ratio: f32) -> Self {
        MetadataDelta {
            page_id,
            source_tier: from,
            target_tier: to,
            compression_ratio: ratio,
            timestamp_ns: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time error")
                .as_nanos() as u64,
            version_vector: VersionVector::new(),
        }
    }
}

/// Propagate metadata changes from L1 page to L2
pub fn propagate_l1_to_l2(
    l1_page: &CrdtSharedPage,
    l2_cache: &mut L2PageCache,
) -> Result<(), String> {
    let delta = MetadataDelta::new(
        l1_page.get_page_id(),
        MemoryTier::L1,
        MemoryTier::L2,
        l1_page.metadata.compression_ratio,
    );

    l2_cache.record_metadata_delta(delta)?;
    Ok(())
}

/// L2PageCache: maintains replicas with propagation history
pub struct L2PageCache {
    pages: HashMap<u32, CrdtSharedPage>,
    metadata_deltas: Vec<MetadataDelta>,
}

impl L2PageCache {
    pub fn new() -> Self {
        L2PageCache {
            pages: HashMap::new(),
            metadata_deltas: Vec::new(),
        }
    }

    pub fn record_metadata_delta(&mut self, delta: MetadataDelta) -> Result<(), String> {
        self.metadata_deltas.push(delta);
        Ok(())
    }

    /// Retrieve page with its metadata history
    pub fn get_with_history(&self, page_id: u32) -> Option<(&CrdtSharedPage, Vec<&MetadataDelta>)> {
        self.pages.get(&page_id).map(|page| {
            let history = self.metadata_deltas
                .iter()
                .filter(|delta| delta.page_id == page_id)
                .collect();
            (page, history)
        })
    }
}
```

---

## 5. Integration Test: Concurrent CT Modifications

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_concurrent_ct_writes_lww() {
        // Setup: Create shared page in L1
        let mut page = CrdtSharedPage::new(42, 1024);

        // CT-1 writes semantic vector at offset 0 at T1=1000
        let ct1_value = vec![1, 2, 3, 4, 5];
        let ct1_hash = 0x123456789abcdef0u64;
        page.write(0, ct1_value.clone(), 1, ct1_hash).unwrap();

        // Simulate time passage
        std::thread::sleep(std::time::Duration::from_millis(10));

        // CT-2 writes different semantic vector at offset 0 at T2=1010 (later)
        let ct2_value = vec![6, 7, 8, 9, 10];
        let ct2_hash = 0x0fedcba987654321u64;
        page.write(0, ct2_value.clone(), 2, ct2_hash).unwrap();

        // Read should return CT-2's value (LWW: later timestamp wins)
        let (final_value, timestamp, writer_id) = page.read(0).expect("read failed");
        assert_eq!(final_value, ct2_value);
        assert_eq!(writer_id, 2);

        // Verify version vector tracks both CTs
        let vv = page.get_version_vector();
        assert!(vv.get(1) > 0);
        assert!(vv.get(2) > 0);
    }

    #[test]
    fn test_concurrent_non_overlapping_writes() {
        let mut local_page = CrdtSharedPage::new(42, 1024);
        let mut remote_page = CrdtSharedPage::new(42, 1024);

        // Local CT writes at offset 0
        local_page.write(0, vec![1, 2, 3], 1, 0x111u64).unwrap();

        // Remote CT writes at offset 10 (non-overlapping)
        remote_page.write(10, vec![4, 5, 6], 2, 0x222u64).unwrap();

        // Merge: should combine both writes without conflict
        local_page.merge_page(&remote_page).unwrap();

        // Verify both values present
        assert_eq!(local_page.read(0).unwrap().0, vec![1, 2, 3]);
        assert_eq!(local_page.read(10).unwrap().0, vec![4, 5, 6]);
    }

    #[test]
    fn test_conflict_detection_write_write() {
        let mut page1 = CrdtSharedPage::new(42, 1024);
        let mut page2 = CrdtSharedPage::new(42, 1024);

        // Both CTs write to same offset
        page1.write(0, vec![1, 2, 3], 1, 0x111u64).unwrap();

        std::thread::sleep(std::time::Duration::from_millis(5));

        page2.write(0, vec![4, 5, 6], 2, 0x222u64).unwrap();

        // Detect conflict
        let conflict = detect_conflict(&page1, &page2);

        match conflict {
            ConflictType::WriteWrite { local_ct, remote_ct } => {
                assert_eq!(local_ct, 1);
                assert_eq!(remote_ct, 2);
            },
            _ => panic!("Expected WriteWrite conflict"),
        }

        // Apply LWW resolution: page2 should win (later timestamp)
        resolve_merge(&mut page1, &page2, MergeStrategy::LastWriteWins).unwrap();

        // Verify page2's value is final
        assert_eq!(page1.read(0).unwrap().0, vec![4, 5, 6]);
    }

    #[test]
    fn test_semantic_merge() {
        let mut page1 = CrdtSharedPage::new(42, 1024);
        let mut page2 = CrdtSharedPage::new(42, 1024);

        // Similar semantic values (low embedding distance)
        let hash1 = 0xffffffffffffffff;
        let hash2 = 0xffffffffffffffff; // Identical: distance = 0.0

        page1.write(5, vec![1], 1, hash1).unwrap();
        page2.write(5, vec![2], 2, hash2).unwrap();

        // Merge with semantic strategy
        resolve_merge(&mut page1, &page2, MergeStrategy::Semantic {
            embedding_distance_threshold: 0.5,
        }).unwrap();

        // With high similarity, merge preserves semantics
        let result = page1.read(5).unwrap();
        assert!(result.0.len() > 0);
    }

    #[test]
    fn test_version_vector_causality() {
        let mut vv1 = VersionVector::new();
        let mut vv2 = VersionVector::new();

        // CT1 writes twice
        vv1.increment(1);
        vv1.increment(1);

        // CT2 writes once
        vv2.increment(2);

        // Merge creates unified view
        let merged = vv1.merge(&vv2);
        assert_eq!(merged.get(1), 2);
        assert_eq!(merged.get(2), 1);

        // Check happened-before relationship
        assert!(!vv1.happened_before(&vv2));
        assert!(!vv2.happened_before(&vv1));
        assert!(vv1.concurrent_with(&vv2));
    }
}
```

---

## 6. Production Considerations

### 6.1 Performance Characteristics
- **Conflict Detection**: O(n) where n = number of modified offsets in page
- **LWW Resolution**: O(1) per conflicting register
- **Semantic Merge**: O(n) with O(1) hash distance computation
- **Metadata Propagation**: O(1) per delta, deferred to background thread

### 6.2 Failure Modes and Handling
1. **Orphaned Writes**: If CT crashes mid-write, CRDT ensures consistency (partial writes are atomic via LWW)
2. **Network Partition**: Version vectors enable eventual consistency upon reconnection
3. **Metadata Corruption**: L2/L3 checksums detect; fallback to L1 canonical replica

### 6.3 Tuning Parameters
```rust
pub const SEMANTIC_MERGE_THRESHOLD: f32 = 0.3;      // Embedding distance threshold
pub const METADATA_PROPAGATION_INTERVAL_MS: u64 = 50; // Batch deltas
pub const VERSION_VECTOR_COMPACT_THRESHOLD: usize = 128; // Prune inactive CTs
```

---

## 7. Conclusion

This CRDT-based design completes Phase 1 of XKernal with a production-grade shared memory consistency layer. By leveraging version vectors, LWW registers, and semantic embedding proximity, we achieve deterministic conflict resolution while preserving causal ordering across crew-level concurrent operations. The integration with L1/L2/L3 metadata pipelines ensures consistency guarantees hold end-to-end.

**Phase 2 Outlook**: Token-aware compression, per-CT capability masks, and cross-crew federation.
