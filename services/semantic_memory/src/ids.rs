// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Strongly-typed memory identifiers and references.
//!
//! All memory references in the Semantic Memory system use strongly typed IDs
//! to prevent accidental confusion between different reference types.
//! This module provides the fundamental identity types for the 3-tier hierarchy.
//!
//! See Engineering Plan § 4.1.0: Core Memory Model & § 4.1.5: Typed References.

use alloc::fmt;
use alloc::string::{String, ToString};
use core::fmt::{Debug, Display};
use core::hash::{Hash, Hasher};

/// A strongly-typed memory region identifier.
///
/// Uniquely identifies a memory region within the semantic memory system.
/// Used across all three tiers (L1, L2, L3) to prevent region mix-ups.
///
/// See Engineering Plan § 4.1.0: Memory Region Identity.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct MemoryRegionID(String);

impl MemoryRegionID {
    /// Creates a new memory region ID from a string.
    ///
    /// # Arguments
    ///
    /// * `id` - A unique identifier for the memory region (e.g., "l1-gpu-0", "l2-host-dram", "l3-nvme-crew")
    pub fn new(id: impl Into<String>) -> Self {
        MemoryRegionID(id.into())
    }

    /// Returns a reference to the underlying string.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Predefined ID for L1 GPU local memory.
    pub fn l1_gpu_local() -> Self {
        MemoryRegionID("l1-gpu-local".to_string())
    }

    /// Predefined ID for L2 host DRAM.
    pub fn l2_host_dram() -> Self {
        MemoryRegionID("l2-host-dram".to_string())
    }

    /// Predefined ID for L3 long-term storage.
    pub fn l3_longterm() -> Self {
        MemoryRegionID("l3-longterm".to_string())
    }
}

impl Display for MemoryRegionID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Region({})", self.0)
    }
}

/// A strongly-typed reference to L1 (Working Memory) data.
///
/// L1 references are microsecond-scale access pointers into the GPU-local HBM pool.
/// These references become invalid when the referenced data is evicted to L2.
///
/// See Engineering Plan § 4.1.1: L1 Working Memory & Microsecond Access.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct L1Ref {
    /// The memory region this reference points into
    region_id: MemoryRegionID,
    /// Byte offset within the region
    offset: u64,
    /// Size of the referenced data in bytes
    size: u64,
}

impl L1Ref {
    /// Creates a new L1 reference.
    ///
    /// # Arguments
    ///
    /// * `region_id` - The L1 region being referenced
    /// * `offset` - Byte offset within the region
    /// * `size` - Size of the data in bytes
    pub fn new(region_id: MemoryRegionID, offset: u64, size: u64) -> Self {
        L1Ref {
            region_id,
            offset,
            size,
        }
    }

    /// Returns the region ID this reference points to.
    pub fn region_id(&self) -> &MemoryRegionID {
        &self.region_id
    }

    /// Returns the byte offset within the region.
    pub fn offset(&self) -> u64 {
        self.offset
    }

    /// Returns the size of the referenced data.
    pub fn size(&self) -> u64 {
        self.size
    }
}

impl Display for L1Ref {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "L1Ref({}, offset={}, size={})",
            self.region_id, self.offset, self.size
        )
    }
}

/// A strongly-typed reference to L2 (Episodic Memory) data.
///
/// L2 references point to indexed entries in host DRAM. These references persist
/// longer than L1 refs but may still be evicted to L3 under memory pressure.
///
/// See Engineering Plan § 4.1.2: L2 Episodic Memory & Millisecond Access.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct L2Ref {
    /// The memory region this reference points into
    region_id: MemoryRegionID,
    /// Index entry ID within the region
    entry_id: String,
    /// Optional semantic vector embedding hash for search purposes
    embedding_hash: Option<u64>,
}

impl L2Ref {
    /// Creates a new L2 reference.
    ///
    /// # Arguments
    ///
    /// * `region_id` - The L2 region being referenced
    /// * `entry_id` - Identifier of the indexed entry
    pub fn new(region_id: MemoryRegionID, entry_id: impl Into<String>) -> Self {
        L2Ref {
            region_id,
            entry_id: entry_id.into(),
            embedding_hash: None,
        }
    }

    /// Creates a new L2 reference with embedding metadata.
    ///
    /// # Arguments
    ///
    /// * `region_id` - The L2 region being referenced
    /// * `entry_id` - Identifier of the indexed entry
    /// * `embedding_hash` - Hash of the semantic embedding for search
    pub fn with_embedding(
        region_id: MemoryRegionID,
        entry_id: impl Into<String>,
        embedding_hash: u64,
    ) -> Self {
        L2Ref {
            region_id,
            entry_id: entry_id.into(),
            embedding_hash: Some(embedding_hash),
        }
    }

    /// Returns the region ID this reference points to.
    pub fn region_id(&self) -> &MemoryRegionID {
        &self.region_id
    }

    /// Returns the entry ID within the region.
    pub fn entry_id(&self) -> &str {
        &self.entry_id
    }

    /// Returns the embedding hash if present.
    pub fn embedding_hash(&self) -> Option<u64> {
        self.embedding_hash
    }
}

impl Display for L2Ref {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(hash) = self.embedding_hash {
            write!(
                f,
                "L2Ref({}, entry={}, embedding_hash={})",
                self.region_id, self.entry_id, hash
            )
        } else {
            write!(f, "L2Ref({}, entry={})", self.region_id, self.entry_id)
        }
    }
}

/// A strongly-typed reference to L3 (Long-Term Memory) data.
///
/// L3 references point to persistent knowledge in NVMe-backed storage.
/// These references remain valid across session boundaries and are shared
/// within a crew using CRDT-based consistency.
///
/// See Engineering Plan § 4.1.3: L3 Long-Term Memory & Persistence.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct L3Ref {
    /// The memory region this reference points into
    region_id: MemoryRegionID,
    /// Knowledge ID within the store
    knowledge_id: String,
    /// Version counter for CRDT tracking
    version: u64,
}

impl L3Ref {
    /// Creates a new L3 reference.
    ///
    /// # Arguments
    ///
    /// * `region_id` - The L3 region being referenced
    /// * `knowledge_id` - Identifier of the knowledge entry
    pub fn new(region_id: MemoryRegionID, knowledge_id: impl Into<String>) -> Self {
        L3Ref {
            region_id,
            knowledge_id: knowledge_id.into(),
            version: 0,
        }
    }

    /// Creates a new L3 reference with version information.
    ///
    /// # Arguments
    ///
    /// * `region_id` - The L3 region being referenced
    /// * `knowledge_id` - Identifier of the knowledge entry
    /// * `version` - Version number for CRDT consistency
    pub fn with_version(
        region_id: MemoryRegionID,
        knowledge_id: impl Into<String>,
        version: u64,
    ) -> Self {
        L3Ref {
            region_id,
            knowledge_id: knowledge_id.into(),
            version,
        }
    }

    /// Returns the region ID this reference points to.
    pub fn region_id(&self) -> &MemoryRegionID {
        &self.region_id
    }

    /// Returns the knowledge ID within the store.
    pub fn knowledge_id(&self) -> &str {
        &self.knowledge_id
    }

    /// Returns the version number.
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Increments the version for CRDT updates.
    pub fn increment_version(&mut self) {
        self.version = self.version.saturating_add(1);
    }
}

impl Display for L3Ref {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "L3Ref({}, knowledge={}, version={})",
            self.region_id, self.knowledge_id, self.version
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_region_id_creation() {
        let id = MemoryRegionID::new("test-region");
        assert_eq!(id.as_str(), "test-region");
    }

    #[test]
    fn test_memory_region_id_predefined() {
        assert_eq!(MemoryRegionID::l1_gpu_local().as_str(), "l1-gpu-local");
        assert_eq!(MemoryRegionID::l2_host_dram().as_str(), "l2-host-dram");
        assert_eq!(MemoryRegionID::l3_longterm().as_str(), "l3-longterm");
    }

    #[test]
    fn test_memory_region_id_display() {
        let id = MemoryRegionID::new("test");
        let display = id.to_string();
        assert!(display.contains("Region(test)"));
    }

    #[test]
    fn test_memory_region_id_equality() {
        let id1 = MemoryRegionID::new("same");
        let id2 = MemoryRegionID::new("same");
        let id3 = MemoryRegionID::new("different");

        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_memory_region_id_hash() {
        use std::collections::hash_map::DefaultHasher;

        let id1 = MemoryRegionID::new("test");
        let id2 = MemoryRegionID::new("test");

        let mut h1 = DefaultHasher::new();
        id1.hash(&mut h1);
        let hash1 = h1.finish();

        let mut h2 = DefaultHasher::new();
        id2.hash(&mut h2);
        let hash2 = h2.finish();

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_l1_ref_creation() {
        let region = MemoryRegionID::l1_gpu_local();
        let l1_ref = L1Ref::new(region.clone(), 1024, 512);

        assert_eq!(l1_ref.region_id(), &region);
        assert_eq!(l1_ref.offset(), 1024);
        assert_eq!(l1_ref.size(), 512);
    }

    #[test]
    fn test_l1_ref_display() {
        let region = MemoryRegionID::l1_gpu_local();
        let l1_ref = L1Ref::new(region, 100, 200);
        let display = l1_ref.to_string();
        assert!(display.contains("L1Ref"));
        assert!(display.contains("100"));
        assert!(display.contains("200"));
    }

    #[test]
    fn test_l1_ref_equality() {
        let region = MemoryRegionID::new("test");
        let ref1 = L1Ref::new(region.clone(), 100, 200);
        let ref2 = L1Ref::new(region.clone(), 100, 200);
        let ref3 = L1Ref::new(region, 100, 300);

        assert_eq!(ref1, ref2);
        assert_ne!(ref1, ref3);
    }

    #[test]
    fn test_l2_ref_creation() {
        let region = MemoryRegionID::l2_host_dram();
        let l2_ref = L2Ref::new(region.clone(), "entry-001");

        assert_eq!(l2_ref.region_id(), &region);
        assert_eq!(l2_ref.entry_id(), "entry-001");
        assert_eq!(l2_ref.embedding_hash(), None);
    }

    #[test]
    fn test_l2_ref_with_embedding() {
        let region = MemoryRegionID::l2_host_dram();
        let l2_ref = L2Ref::with_embedding(region.clone(), "entry-001", 0xdeadbeef);

        assert_eq!(l2_ref.region_id(), &region);
        assert_eq!(l2_ref.entry_id(), "entry-001");
        assert_eq!(l2_ref.embedding_hash(), Some(0xdeadbeef));
    }

    #[test]
    fn test_l2_ref_display() {
        let region = MemoryRegionID::l2_host_dram();
        let l2_ref = L2Ref::new(region, "entry-001");
        let display = l2_ref.to_string();
        assert!(display.contains("L2Ref"));
        assert!(display.contains("entry-001"));
    }

    #[test]
    fn test_l2_ref_display_with_embedding() {
        let region = MemoryRegionID::l2_host_dram();
        let l2_ref = L2Ref::with_embedding(region, "entry-001", 0xdead);
        let display = l2_ref.to_string();
        assert!(display.contains("L2Ref"));
        assert!(display.contains("embedding_hash"));
    }

    #[test]
    fn test_l3_ref_creation() {
        let region = MemoryRegionID::l3_longterm();
        let l3_ref = L3Ref::new(region.clone(), "knowledge-001");

        assert_eq!(l3_ref.region_id(), &region);
        assert_eq!(l3_ref.knowledge_id(), "knowledge-001");
        assert_eq!(l3_ref.version(), 0);
    }

    #[test]
    fn test_l3_ref_with_version() {
        let region = MemoryRegionID::l3_longterm();
        let l3_ref = L3Ref::with_version(region.clone(), "knowledge-001", 42);

        assert_eq!(l3_ref.region_id(), &region);
        assert_eq!(l3_ref.knowledge_id(), "knowledge-001");
        assert_eq!(l3_ref.version(), 42);
    }

    #[test]
    fn test_l3_ref_increment_version() {
        let region = MemoryRegionID::l3_longterm();
        let mut l3_ref = L3Ref::with_version(region, "knowledge-001", 10);

        assert_eq!(l3_ref.version(), 10);
        l3_ref.increment_version();
        assert_eq!(l3_ref.version(), 11);
    }

    #[test]
    fn test_l3_ref_version_saturating_add() {
        let region = MemoryRegionID::l3_longterm();
        let mut l3_ref = L3Ref::with_version(region, "knowledge-001", u64::MAX);

        l3_ref.increment_version();
        assert_eq!(l3_ref.version(), u64::MAX); // saturates at MAX
    }

    #[test]
    fn test_l3_ref_display() {
        let region = MemoryRegionID::l3_longterm();
        let l3_ref = L3Ref::with_version(region, "knowledge-001", 42);
        let display = l3_ref.to_string();
        assert!(display.contains("L3Ref"));
        assert!(display.contains("knowledge-001"));
        assert!(display.contains("42"));
    }

    #[test]
    fn test_refs_are_distinct_types() {
        let region = MemoryRegionID::new("test");
        let l1_ref = L1Ref::new(region.clone(), 0, 100);
        let l2_ref = L2Ref::new(region.clone(), "entry");
        let l3_ref = L3Ref::new(region, "knowledge");

        // These are compile-time distinct types, preventing accidental mixing
        let _ = (l1_ref, l2_ref, l3_ref);
    }
}
