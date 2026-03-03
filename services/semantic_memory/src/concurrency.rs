// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Concurrency control and atomicity models for the 3-tier memory hierarchy.
//!
//! This module defines concurrency strategies for each memory tier:
//! - L1: Lock-free reads, atomic writes (single-writer, multiple-reader)
//! - L2: Optimistic concurrency with version vectors for conflict detection
//! - L3: Distributed consensus (Raft-like) for cross-node operations
//!
//! See Engineering Plan § 4.1.0: Concurrency & Atomicity Control.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use crate::error::{MemoryError, Result};

/// Atomicity levels for memory operations.
///
/// Defines the guarantee strength for memory accesses across tiers.
/// See Engineering Plan § 4.1.0: Atomicity Levels.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AtomicityLevel {
    /// No atomicity guarantees; reads may return partial updates.
    Atomic,

    /// Linearizable: All operations appear to occur in a single linear order.
    /// Strongest consistency model, highest latency.
    Linearizable,

    /// Sequential consistency: All operations on an object are ordered,
    /// but operations on different objects may reorder.
    Sequential,

    /// Eventual consistency: Updates propagate asynchronously but eventually
    /// become consistent. Weakest model, lowest latency.
    Eventual,
}

impl AtomicityLevel {
    /// Returns a human-readable name for this atomicity level.
    pub fn name(&self) -> &'static str {
        match self {
            AtomicityLevel::Atomic => "atomic",
            AtomicityLevel::Linearizable => "linearizable",
            AtomicityLevel::Sequential => "sequential",
            AtomicityLevel::Eventual => "eventual",
        }
    }
}

/// Trait for acquiring and releasing concurrency guards.
///
/// Implements the guard pattern for exclusive/shared access.
/// See Engineering Plan § 4.1.0: Operation Guards.
pub trait OperationGuard: Sized {
    /// Acquires the guard, blocking if necessary.
    ///
    /// # Returns
    ///
    /// A new guard instance if acquisition succeeds, error otherwise.
    fn acquire() -> Result<Self>;

    /// Releases the guard, allowing other operations to proceed.
    fn release(&mut self) -> Result<()>;

    /// Returns whether the guard is currently held.
    fn is_held(&self) -> bool;
}

/// Memory tier enumeration.
///
/// Identifies which tier a memory operation targets.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum MemoryTier {
    /// L1 Working Memory (GPU-local)
    L1,

    /// L2 Episodic Memory (Host DRAM)
    L2,

    /// L3 Long-Term Memory (NVMe persistent)
    L3,
}

impl MemoryTier {
    /// Returns a human-readable name for this tier.
    pub fn name(&self) -> &'static str {
        match self {
            MemoryTier::L1 => "L1",
            MemoryTier::L2 => "L2",
            MemoryTier::L3 => "L3",
        }
    }
}

/// Memory operation types for access control and conflict detection.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum MemoryOperation {
    /// Read operation
    Read,

    /// Write operation
    Write,

    /// Delete operation
    Delete,

    /// Migration operation
    Migrate,

    /// Admin/maintenance operation
    Admin,
}

impl MemoryOperation {
    /// Returns a human-readable name for this operation.
    pub fn name(&self) -> &'static str {
        match self {
            MemoryOperation::Read => "read",
            MemoryOperation::Write => "write",
            MemoryOperation::Delete => "delete",
            MemoryOperation::Migrate => "migrate",
            MemoryOperation::Admin => "admin",
        }
    }
}

/// Access policy for concurrent operations.
///
/// Defines how multiple agents can access the same region.
/// See Engineering Plan § 4.1.0: Concurrent Access Policy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConcurrentAccessPolicy {
    /// Multiple readers, no writers (read-shared access)
    ReadShared,

    /// Single writer, no readers (exclusive write access)
    WriteExclusive,

    /// Single writer that also reads, no other readers (exclusive read-write)
    ReadWriteExclusive,
}

impl ConcurrentAccessPolicy {
    /// Returns whether this policy allows reads.
    pub fn allows_reads(&self) -> bool {
        matches!(
            self,
            ConcurrentAccessPolicy::ReadShared | ConcurrentAccessPolicy::ReadWriteExclusive
        )
    }

    /// Returns whether this policy allows writes.
    pub fn allows_writes(&self) -> bool {
        matches!(
            self,
            ConcurrentAccessPolicy::WriteExclusive | ConcurrentAccessPolicy::ReadWriteExclusive
        )
    }

    /// Returns whether this policy allows concurrent readers.
    pub fn allows_concurrent_readers(&self) -> bool {
        matches!(self, ConcurrentAccessPolicy::ReadShared)
    }
}

/// Version vector for tracking causal ordering in L2 episodic memory.
///
/// Maps agent IDs to their local version numbers, enabling detection
/// of concurrent updates and causal ordering.
/// See Engineering Plan § 4.1.2: L2 Optimistic Concurrency.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VersionVector {
    /// Map of agent ID to their local version number
    versions: BTreeMap<alloc::string::String, u64>,
}

impl VersionVector {
    /// Creates a new empty version vector.
    pub fn new() -> Self {
        VersionVector {
            versions: BTreeMap::new(),
        }
    }

    /// Increments the version for a specific agent.
    ///
    /// # Arguments
    ///
    /// * `agent_id` - Identifier of the agent performing the operation
    pub fn increment(&mut self, agent_id: impl Into<alloc::string::String>) {
        let agent_id = agent_id.into();
        let current = self.versions.get(&agent_id).copied().unwrap_or(0);
        self.versions.insert(agent_id, current.saturating_add(1));
    }

    /// Returns the version number for a specific agent.
    pub fn get(&self, agent_id: &str) -> u64 {
        self.versions.get(agent_id).copied().unwrap_or(0)
    }

    /// Merges two version vectors (returns the maximum for each agent).
    pub fn merge(&mut self, other: &VersionVector) {
        for (agent_id, other_version) in &other.versions {
            let current = self.versions.get(agent_id).copied().unwrap_or(0);
            if *other_version > current {
                self.versions.insert(agent_id.clone(), *other_version);
            }
        }
    }

    /// Returns whether this vector is causally ordered before another.
    ///
    /// Returns true if all versions in this are <= corresponding versions in other,
    /// with at least one strict <.
    pub fn happens_before(&self, other: &VersionVector) -> bool {
        let mut has_strict_less = false;

        for (agent_id, version) in &self.versions {
            let other_version = other.get(agent_id);
            if version > &other_version {
                return false;
            }
            if version < &other_version {
                has_strict_less = true;
            }
        }

        has_strict_less
    }

    /// Returns whether this vector is concurrent with another.
    ///
    /// Returns true if neither happens-before the other.
    pub fn is_concurrent(&self, other: &VersionVector) -> bool {
        !self.happens_before(other) && !other.happens_before(self)
    }
}

impl Default for VersionVector {
    fn default() -> Self {
        Self::new()
    }
}

/// Conflict resolution strategy for concurrent writes in L2.
///
/// See Engineering Plan § 4.1.2: Conflict Resolution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConflictResolution {
    /// Last writer wins: accept the most recent write based on timestamp
    LastWriterWins,

    /// Merge via CRDT: apply CRDT merge semantics
    MergeViaCrdt,

    /// Reject the conflict and return an error
    RejectConflict,

    /// Escalate to agent for resolution
    EscalateToAgent,
}

impl ConflictResolution {
    /// Returns a human-readable name for this strategy.
    pub fn name(&self) -> &'static str {
        match self {
            ConflictResolution::LastWriterWins => "last_writer_wins",
            ConflictResolution::MergeViaCrdt => "merge_via_crdt",
            ConflictResolution::RejectConflict => "reject_conflict",
            ConflictResolution::EscalateToAgent => "escalate_to_agent",
        }
    }
}

/// Per-tier concurrency model configuration.
///
/// Defines the concurrency semantics for each memory tier.
/// See Engineering Plan § 4.1.0: Tier Concurrency Models.
#[derive(Clone, Debug)]
pub struct TierConcurrencyModel {
    /// The memory tier this model applies to
    tier: MemoryTier,

    /// Atomicity level for this tier
    atomicity: AtomicityLevel,

    /// Access policy for concurrent operations
    access_policy: ConcurrentAccessPolicy,

    /// Conflict resolution strategy (for L2, L3)
    conflict_resolution: ConflictResolution,

    /// Maximum concurrent readers (for read-shared policy)
    max_concurrent_readers: u32,
}

impl TierConcurrencyModel {
    /// Creates a new L1 concurrency model (lock-free reads, atomic writes).
    ///
    /// L1 uses a single-writer, multiple-reader model with atomic operations.
    /// See Engineering Plan § 4.1.1: L1 Concurrency.
    pub fn l1_swmr() -> Self {
        TierConcurrencyModel {
            tier: MemoryTier::L1,
            atomicity: AtomicityLevel::Linearizable,
            access_policy: ConcurrentAccessPolicy::WriteExclusive,
            conflict_resolution: ConflictResolution::LastWriterWins,
            max_concurrent_readers: u32::MAX,
        }
    }

    /// Creates a new L2 concurrency model (optimistic with version vectors).
    ///
    /// L2 uses optimistic concurrency control with version vectors for conflict detection.
    /// See Engineering Plan § 4.1.2: L2 Concurrency.
    pub fn l2_optimistic() -> Self {
        TierConcurrencyModel {
            tier: MemoryTier::L2,
            atomicity: AtomicityLevel::Sequential,
            access_policy: ConcurrentAccessPolicy::ReadShared,
            conflict_resolution: ConflictResolution::MergeViaCrdt,
            max_concurrent_readers: u32::MAX,
        }
    }

    /// Creates a new L3 concurrency model (distributed consensus).
    ///
    /// L3 uses Raft-like consensus for cross-node operations.
    /// See Engineering Plan § 4.1.3: L3 Concurrency.
    pub fn l3_distributed() -> Self {
        TierConcurrencyModel {
            tier: MemoryTier::L3,
            atomicity: AtomicityLevel::Eventual,
            access_policy: ConcurrentAccessPolicy::ReadShared,
            conflict_resolution: ConflictResolution::MergeViaCrdt,
            max_concurrent_readers: u32::MAX,
        }
    }

    /// Returns the memory tier this model applies to.
    pub fn tier(&self) -> MemoryTier {
        self.tier
    }

    /// Returns the atomicity level.
    pub fn atomicity(&self) -> AtomicityLevel {
        self.atomicity
    }

    /// Returns the access policy.
    pub fn access_policy(&self) -> &ConcurrentAccessPolicy {
        &self.access_policy
    }

    /// Returns the conflict resolution strategy.
    pub fn conflict_resolution(&self) -> ConflictResolution {
        self.conflict_resolution
    }

    /// Returns the maximum concurrent readers.
    pub fn max_concurrent_readers(&self) -> u32 {
        self.max_concurrent_readers
    }
}

impl Default for TierConcurrencyModel {
    fn default() -> Self {
        TierConcurrencyModel::l1_swmr()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::string::String;

    #[test]
    fn test_atomicity_level_ordering() {
        assert!(AtomicityLevel::Atomic < AtomicityLevel::Linearizable);
        assert!(AtomicityLevel::Linearizable < AtomicityLevel::Sequential);
        assert!(AtomicityLevel::Sequential < AtomicityLevel::Eventual);
    }

    #[test]
    fn test_atomicity_level_names() {
        assert_eq!(AtomicityLevel::Atomic.name(), "atomic");
        assert_eq!(AtomicityLevel::Linearizable.name(), "linearizable");
        assert_eq!(AtomicityLevel::Sequential.name(), "sequential");
        assert_eq!(AtomicityLevel::Eventual.name(), "eventual");
    }

    #[test]
    fn test_memory_tier_names() {
        assert_eq!(MemoryTier::L1.name(), "L1");
        assert_eq!(MemoryTier::L2.name(), "L2");
        assert_eq!(MemoryTier::L3.name(), "L3");
    }

    #[test]
    fn test_memory_operation_names() {
        assert_eq!(MemoryOperation::Read.name(), "read");
        assert_eq!(MemoryOperation::Write.name(), "write");
        assert_eq!(MemoryOperation::Delete.name(), "delete");
        assert_eq!(MemoryOperation::Migrate.name(), "migrate");
        assert_eq!(MemoryOperation::Admin.name(), "admin");
    }

    #[test]
    fn test_access_policy_read_shared() {
        let policy = ConcurrentAccessPolicy::ReadShared;
        assert!(policy.allows_reads());
        assert!(!policy.allows_writes());
        assert!(policy.allows_concurrent_readers());
    }

    #[test]
    fn test_access_policy_write_exclusive() {
        let policy = ConcurrentAccessPolicy::WriteExclusive;
        assert!(!policy.allows_reads());
        assert!(policy.allows_writes());
        assert!(!policy.allows_concurrent_readers());
    }

    #[test]
    fn test_access_policy_read_write_exclusive() {
        let policy = ConcurrentAccessPolicy::ReadWriteExclusive;
        assert!(policy.allows_reads());
        assert!(policy.allows_writes());
        assert!(!policy.allows_concurrent_readers());
    }

    #[test]
    fn test_version_vector_creation() {
        let vv = VersionVector::new();
        assert_eq!(vv.get("agent-001"), 0);
    }

    #[test]
    fn test_version_vector_increment() {
        let mut vv = VersionVector::new();
        vv.increment("agent-001");
        assert_eq!(vv.get("agent-001"), 1);

        vv.increment("agent-001");
        assert_eq!(vv.get("agent-001"), 2);
    }

    #[test]
    fn test_version_vector_merge() {
        let mut vv1 = VersionVector::new();
        vv1.increment("agent-001");
        vv1.increment("agent-001");

        let mut vv2 = VersionVector::new();
        vv2.increment("agent-002");
        vv2.increment("agent-002");

        vv1.merge(&vv2);
        assert_eq!(vv1.get("agent-001"), 2);
        assert_eq!(vv1.get("agent-002"), 2);
    }

    #[test]
    fn test_version_vector_happens_before() {
        let mut vv1 = VersionVector::new();
        vv1.increment("agent-001");

        let mut vv2 = VersionVector::new();
        vv2.increment("agent-001");
        vv2.increment("agent-001");

        assert!(vv1.happens_before(&vv2));
        assert!(!vv2.happens_before(&vv1));
    }

    #[test]
    fn test_version_vector_is_concurrent() {
        let mut vv1 = VersionVector::new();
        vv1.increment("agent-001");

        let mut vv2 = VersionVector::new();
        vv2.increment("agent-002");

        assert!(vv1.is_concurrent(&vv2));
    }

    #[test]
    fn test_conflict_resolution_names() {
        assert_eq!(ConflictResolution::LastWriterWins.name(), "last_writer_wins");
        assert_eq!(ConflictResolution::MergeViaCrdt.name(), "merge_via_crdt");
        assert_eq!(ConflictResolution::RejectConflict.name(), "reject_conflict");
        assert_eq!(ConflictResolution::EscalateToAgent.name(), "escalate_to_agent");
    }

    #[test]
    fn test_tier_concurrency_model_l1_swmr() {
        let model = TierConcurrencyModel::l1_swmr();
        assert_eq!(model.tier(), MemoryTier::L1);
        assert_eq!(model.atomicity(), AtomicityLevel::Linearizable);
        assert_eq!(model.access_policy(), &ConcurrentAccessPolicy::WriteExclusive);
    }

    #[test]
    fn test_tier_concurrency_model_l2_optimistic() {
        let model = TierConcurrencyModel::l2_optimistic();
        assert_eq!(model.tier(), MemoryTier::L2);
        assert_eq!(model.atomicity(), AtomicityLevel::Sequential);
        assert_eq!(model.access_policy(), &ConcurrentAccessPolicy::ReadShared);
    }

    #[test]
    fn test_tier_concurrency_model_l3_distributed() {
        let model = TierConcurrencyModel::l3_distributed();
        assert_eq!(model.tier(), MemoryTier::L3);
        assert_eq!(model.atomicity(), AtomicityLevel::Eventual);
        assert_eq!(model.access_policy(), &ConcurrentAccessPolicy::ReadShared);
    }

    #[test]
    fn test_tier_concurrency_model_default() {
        let model = TierConcurrencyModel::default();
        assert_eq!(model.tier(), MemoryTier::L1);
    }
}
