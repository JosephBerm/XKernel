// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Top-level SemanticMemory structure — Phase 0 Foundation (L1 Only).
//!
//! This module provides the main SemanticMemory type. In Phase 0, only L1
//! (Working Memory) is implemented. L2 (Episodic) and L3 (Long-Term) tiers
//! will be added in Phase 1 (Weeks 7–9).
//!
//! See Engineering Plan § 4.1: Semantic Memory Architecture.

use alloc::string::String;
use alloc::format;
use crate::error::Result;
use crate::error::MemoryError;
use crate::ids::MemoryRegionID;
use crate::isolation::{IsolationLevel, MemoryCapabilitySet};
use crate::l1_working::L1WorkingMemory;

/// Agent identifier type (simplified).
///
/// In a real implementation, this would reference the capability engine's AgentID.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentID(String);

impl AgentID {
    /// Creates a new agent ID.
    pub fn new(id: impl Into<String>) -> Self {
        AgentID(id.into())
    }

    /// Returns the ID as a string reference.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Top-level Semantic Memory structure — Phase 0 (L1 Only).
///
/// In Phase 0, SemanticMemory provides access to L1 working memory only.
/// L2 (Episodic, DRAM-backed) and L3 (Long-Term, NVMe-backed) tiers will
/// be added in Phase 1 when the corresponding modules are implemented.
///
/// # Design
///
/// The hierarchy is organized by access latency and persistence:
/// - **L1 (Working Memory)**: HBM/GPU-local, ~100-500ns access, microsecond scale
/// - **L2 (Episodic Memory)**: [Phase 1] Host DRAM, ~1-10ms access, with semantic indexing
/// - **L3 (Long-Term Memory)**: [Phase 1] NVMe/persistent, crew-wide with CRDT consistency
///
/// # Capabilities
///
/// Access to memory operations is controlled via capability sets. Agents must
/// have appropriate capabilities to allocate, read, write, or migrate data.
///
/// See Engineering Plan § 4.1: SemanticMemory & § 3.1: Capability-Based Security.
#[derive(Clone, Debug)]
pub struct SemanticMemory {
    /// L1 working memory (fast, GPU-local)
    l1: L1WorkingMemory,

    /// Owner/agent that controls this memory
    owner: AgentID,

    /// Capabilities granted to this memory's owner
    capabilities: MemoryCapabilitySet,

    /// Isolation level for this memory
    isolation_level: IsolationLevel,
}

impl SemanticMemory {
    /// Creates a new SemanticMemory structure with default configuration.
    ///
    /// # Arguments
    ///
    /// * `owner` - Agent that owns this memory
    /// * `capabilities` - Capabilities granted to the owner
    ///
    /// # See
    ///
    /// Engineering Plan § 4.1.0: SemanticMemory Initialization.
    pub fn new(owner: AgentID, capabilities: MemoryCapabilitySet) -> Self {
        let l1 = L1WorkingMemory::new(
            MemoryRegionID::l1_gpu_local(),
            8 * 1024 * 1024,        // 8 MB default for L1
            200,                     // 200 ns target latency
        );

        SemanticMemory {
            l1,
            owner,
            capabilities,
            isolation_level: IsolationLevel::PerAgent,
        }
    }

    /// Creates a SemanticMemory with custom L1 tier size.
    ///
    /// # Arguments
    ///
    /// * `owner` - Agent that owns this memory
    /// * `capabilities` - Capabilities granted to the owner
    /// * `l1_capacity` - L1 capacity in bytes
    pub fn with_capacities(
        owner: AgentID,
        capabilities: MemoryCapabilitySet,
        l1_capacity: u64,
        _l2_capacity: u64,
        _l3_capacity: u64,
    ) -> Self {
        let l1 = L1WorkingMemory::new(
            MemoryRegionID::l1_gpu_local(),
            l1_capacity,
            200,
        );

        SemanticMemory {
            l1,
            owner,
            capabilities,
            isolation_level: IsolationLevel::PerAgent,
        }
    }

    /// Returns the owner of this memory.
    pub fn owner(&self) -> &AgentID {
        &self.owner
    }

    /// Returns the capabilities granted to this memory.
    pub fn capabilities(&self) -> &MemoryCapabilitySet {
        &self.capabilities
    }

    /// Returns the isolation level.
    pub fn isolation_level(&self) -> &IsolationLevel {
        &self.isolation_level
    }

    /// Sets the isolation level.
    pub fn set_isolation_level(&mut self, level: IsolationLevel) {
        self.isolation_level = level;
    }

    /// Returns the L1 working memory tier.
    pub fn l1(&self) -> &L1WorkingMemory {
        &self.l1
    }

    /// Returns a mutable reference to the L1 working memory tier.
    pub fn l1_mut(&mut self) -> &mut L1WorkingMemory {
        &mut self.l1
    }

    /// Returns total memory capacity (L1 only in Phase 0).
    pub fn total_capacity(&self) -> u64 {
        self.l1.capacity_bytes()
    }

    /// Returns total used bytes (L1 only in Phase 0).
    pub fn total_used(&self) -> u64 {
        self.l1.used_bytes()
    }

    /// Returns total available bytes (L1 only in Phase 0).
    pub fn total_available(&self) -> u64 {
        self.l1.available_bytes()
    }

    /// Returns overall memory utilization (0.0 to 1.0).
    pub fn utilization(&self) -> f64 {
        let total = self.total_capacity();
        if total == 0 {
            0.0
        } else {
            self.total_used() as f64 / total as f64
        }
    }

    /// Allocates memory from L1.
    ///
    /// # Arguments
    ///
    /// * `size_bytes` - Bytes to allocate
    ///
    /// # Returns
    ///
    /// Returns offset within L1, or error if insufficient capacity.
    ///
    /// # See
    ///
    /// Engineering Plan § 4.1.1: Allocation & Tier Fallback.
    pub fn allocate(&mut self, size_bytes: u64) -> Result<u64> {
        // Check capability
        if !self.capabilities.has(crate::isolation::MemoryCapabilityFlags::ALLOCATE) {
            return Err(MemoryError::CapabilityDenied {
                operation: "allocate".to_string(),
                resource: "semantic_memory".to_string(),
            });
        }

        // L1 allocation (Phase 0: L1 only)
        if self.l1.available_bytes() >= size_bytes {
            return self.l1.allocate(size_bytes);
        }

        Err(MemoryError::AllocationFailed {
            requested: size_bytes,
            available: self.total_available(),
        })
    }

    /// Migrates data between tiers (stub — Phase 1 will implement actual migration).
    ///
    /// # See
    ///
    /// Engineering Plan § 4.1.2: Tier Migration.
    pub fn migrate(&mut self, _bytes: u64, from_tier: u32, to_tier: u32) -> Result<()> {
        if !self.capabilities.has(crate::isolation::MemoryCapabilityFlags::MIGRATE) {
            return Err(MemoryError::CapabilityDenied {
                operation: "migrate".to_string(),
                resource: "semantic_memory".to_string(),
            });
        }

        if from_tier > 3 || to_tier > 3 {
            return Err(MemoryError::InvalidTier {
                operation: "migrate".to_string(),
                tier: format!("invalid tier: {} -> {}", from_tier, to_tier),
            });
        }

        // Phase 0 stub: actual tier migration implemented in Phase 1
        Ok(())
    }

    /// Creates a snapshot of memory state (L1 only in Phase 0).
    pub fn snapshot(&self) -> MemorySnapshot {
        MemorySnapshot {
            owner: self.owner.clone(),
            l1_capacity: self.l1.capacity_bytes(),
            l1_used: self.l1.used_bytes(),
            isolation_level: self.isolation_level.clone(),
        }
    }
}

/// A snapshot of SemanticMemory state at a point in time.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemorySnapshot {
    /// Owner of the memory
    pub owner: AgentID,

    /// L1 total capacity
    pub l1_capacity: u64,
    /// L1 used bytes
    pub l1_used: u64,

    /// Isolation level at snapshot time
    pub isolation_level: IsolationLevel,
}

impl MemorySnapshot {
    /// Returns total capacity in snapshot.
    pub fn total_capacity(&self) -> u64 {
        self.l1_capacity
    }

    /// Returns total used in snapshot.
    pub fn total_used(&self) -> u64 {
        self.l1_used
    }

    /// Returns overall utilization in snapshot.
    pub fn utilization(&self) -> f64 {
        let total = self.total_capacity();
        if total == 0 {
            0.0
        } else {
            self.total_used() as f64 / total as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;

    #[test]
    fn test_agent_id_creation() {
        let agent = AgentID::new("agent-001");
        assert_eq!(agent.as_str(), "agent-001");
    }

    #[test]
    fn test_semantic_memory_creation() {
        let owner = AgentID::new("agent-001");
        let caps = MemoryCapabilitySet::full();
        let memory = SemanticMemory::new(owner.clone(), caps);

        assert_eq!(memory.owner(), &owner);
        assert_eq!(memory.isolation_level(), &IsolationLevel::PerAgent);
    }

    #[test]
    fn test_semantic_memory_with_capacities() {
        let owner = AgentID::new("agent-001");
        let caps = MemoryCapabilitySet::full();
        let memory = SemanticMemory::with_capacities(owner, caps, 1024, 0, 0);

        assert_eq!(memory.l1().capacity_bytes(), 1024);
    }

    #[test]
    fn test_semantic_memory_total_capacity() {
        let owner = AgentID::new("agent-001");
        let caps = MemoryCapabilitySet::full();
        let memory = SemanticMemory::with_capacities(owner, caps, 1000, 0, 0);

        assert_eq!(memory.total_capacity(), 1000);
    }

    #[test]
    fn test_semantic_memory_utilization() {
        let owner = AgentID::new("agent-001");
        let caps = MemoryCapabilitySet::full();
        let memory = SemanticMemory::with_capacities(owner, caps, 1000, 0, 0);

        assert_eq!(memory.utilization(), 0.0);
    }

    #[test]
    fn test_semantic_memory_isolation_level() {
        let owner = AgentID::new("agent-001");
        let caps = MemoryCapabilitySet::full();
        let mut memory = SemanticMemory::new(owner, caps);

        memory.set_isolation_level(IsolationLevel::PerCrew);
        assert_eq!(memory.isolation_level(), &IsolationLevel::PerCrew);
    }

    #[test]
    fn test_semantic_memory_allocate() {
        let owner = AgentID::new("agent-001");
        let caps = MemoryCapabilitySet::full();
        let mut memory = SemanticMemory::with_capacities(owner, caps, 1000, 0, 0);

        let offset = memory.allocate(500).unwrap();
        assert_eq!(offset, 0);
        assert_eq!(memory.l1().used_bytes(), 500);
    }

    #[test]
    fn test_semantic_memory_allocate_insufficient_capability() {
        let owner = AgentID::new("agent-001");
        let caps = MemoryCapabilitySet::new();
        let mut memory = SemanticMemory::with_capacities(owner, caps, 1000, 0, 0);

        let result = memory.allocate(500);
        assert!(result.is_err());
    }

    #[test]
    fn test_semantic_memory_migrate() {
        let owner = AgentID::new("agent-001");
        let caps = MemoryCapabilitySet::full();
        let mut memory = SemanticMemory::new(owner, caps);

        let result = memory.migrate(100, 1, 2);
        assert!(result.is_ok());
    }

    #[test]
    fn test_semantic_memory_migrate_invalid_tier() {
        let owner = AgentID::new("agent-001");
        let caps = MemoryCapabilitySet::full();
        let mut memory = SemanticMemory::new(owner, caps);

        let result = memory.migrate(100, 5, 2);
        assert!(result.is_err());
    }

    #[test]
    fn test_semantic_memory_snapshot() {
        let owner = AgentID::new("agent-001");
        let caps = MemoryCapabilitySet::full();
        let memory = SemanticMemory::with_capacities(owner.clone(), caps, 1000, 0, 0);

        let snapshot = memory.snapshot();
        assert_eq!(snapshot.owner, owner);
        assert_eq!(snapshot.l1_capacity, 1000);
    }

    #[test]
    fn test_memory_snapshot_total_capacity() {
        let owner = AgentID::new("agent-001");
        let snapshot = MemorySnapshot {
            owner,
            l1_capacity: 1000,
            l1_used: 0,
            isolation_level: IsolationLevel::PerAgent,
        };

        assert_eq!(snapshot.total_capacity(), 1000);
    }

    #[test]
    fn test_memory_snapshot_utilization() {
        let owner = AgentID::new("agent-001");
        let snapshot = MemorySnapshot {
            owner,
            l1_capacity: 1000,
            l1_used: 500,
            isolation_level: IsolationLevel::PerAgent,
        };

        let utilization = snapshot.utilization();
        assert_eq!(utilization, 0.5);
    }
}
