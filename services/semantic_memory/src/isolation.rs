// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Memory isolation boundaries and capability-based access control.
//!
//! This module defines the isolation model for semantic memory, ensuring that
//! agents cannot access memory they are not authorized for. Isolation operates
//! at multiple levels: per-agent, per-crew, and shared read-only.
//!
//! See Engineering Plan § 4.1.3: Isolation & Capability Checks.

use alloc::collections::BTreeSet;
use alloc::string::String;
use bitflags::bitflags;

/// Memory isolation levels defining scope of access.
///
/// Isolation levels form a hierarchy:
/// - PerAgent: Strict per-agent isolation
/// - PerCrew: Shared within a crew
/// - SharedReadOnly: Read-only shared access
///
/// See Engineering Plan § 4.1.3: Isolation Levels.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum IsolationLevel {
    /// Strict per-agent isolation - only the owning agent can access.
    /// This is the default for private working memory.
    PerAgent,

    /// Crew-wide sharing - all crew members can read and write.
    /// Used for collaborative memory and shared context.
    PerCrew,

    /// Read-only shared access - all crew members can read but not modify.
    /// Used for knowledge bases and reference data.
    SharedReadOnly,
}

impl IsolationLevel {
    /// Returns a human-readable name.
    pub fn name(&self) -> &'static str {
        match self {
            IsolationLevel::PerAgent => "per_agent",
            IsolationLevel::PerCrew => "per_crew",
            IsolationLevel::SharedReadOnly => "shared_read_only",
        }
    }

    /// Returns whether this level allows cross-agent access.
    pub fn allows_cross_agent_access(&self) -> bool {
        matches!(self, IsolationLevel::PerCrew | IsolationLevel::SharedReadOnly)
    }

    /// Returns whether this level allows writes.
    pub fn allows_write(&self) -> bool {
        !matches!(self, IsolationLevel::SharedReadOnly)
    }
}

bitflags! {
    /// Capability flags for memory operations.
    ///
    /// These flags represent permissions for different memory operations.
    /// Capabilities are checked before allowing operations to proceed.
    ///
    /// See Engineering Plan § 3.1 (Capability-Based Security).
    pub struct MemoryCapabilityFlags: u32 {
        /// Capability to allocate memory
        const ALLOCATE = 0x0001;

        /// Capability to read memory
        const READ = 0x0002;

        /// Capability to write memory
        const WRITE = 0x0004;

        /// Capability to evict memory
        const EVICT = 0x0008;

        /// Capability to migrate between tiers
        const MIGRATE = 0x0010;

        /// Capability to query memory contents
        const QUERY = 0x0020;

        /// Capability to snapshot memory
        const SNAPSHOT = 0x0040;

        /// Capability to compact/defragment
        const COMPACT = 0x0080;

        /// Capability to mount external sources
        const MOUNT = 0x0100;

        /// Capability to replicate (for L3)
        const REPLICATE = 0x0200;

        /// Capability to subscribe to updates
        const SUBSCRIBE = 0x0400;

        /// All capabilities (used for privileged access)
        const ALL = 0xFFFF;
    }
}

impl MemoryCapabilityFlags {
    /// Returns a human-readable description of the flags.
    pub fn description(&self) -> String {
        let mut parts = Vec::new();

        if self.contains(MemoryCapabilityFlags::ALLOCATE) {
            parts.push("allocate");
        }
        if self.contains(MemoryCapabilityFlags::READ) {
            parts.push("read");
        }
        if self.contains(MemoryCapabilityFlags::WRITE) {
            parts.push("write");
        }
        if self.contains(MemoryCapabilityFlags::EVICT) {
            parts.push("evict");
        }
        if self.contains(MemoryCapabilityFlags::MIGRATE) {
            parts.push("migrate");
        }
        if self.contains(MemoryCapabilityFlags::QUERY) {
            parts.push("query");
        }
        if self.contains(MemoryCapabilityFlags::SNAPSHOT) {
            parts.push("snapshot");
        }
        if self.contains(MemoryCapabilityFlags::COMPACT) {
            parts.push("compact");
        }
        if self.contains(MemoryCapabilityFlags::MOUNT) {
            parts.push("mount");
        }
        if self.contains(MemoryCapabilityFlags::REPLICATE) {
            parts.push("replicate");
        }
        if self.contains(MemoryCapabilityFlags::SUBSCRIBE) {
            parts.push("subscribe");
        }

        if parts.is_empty() {
            "none".to_string()
        } else {
            parts.join(", ")
        }
    }
}

/// Set of memory capabilities for an agent.
///
/// Links memory operations to capability checks, enforcing that only
/// authorized agents can perform sensitive operations.
///
/// See Engineering Plan § 4.1.3: Capability Sets.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryCapabilitySet {
    /// Flags representing granted capabilities
    flags: MemoryCapabilityFlags,

    /// Set of memory regions this agent can access
    authorized_regions: BTreeSet<String>,

    /// Maximum tier this agent can access (0=L1, 1=L2, 2=L3)
    max_tier_level: u32,
}

impl MemoryCapabilitySet {
    /// Creates a new capability set with minimal permissions.
    pub fn new() -> Self {
        MemoryCapabilitySet {
            flags: MemoryCapabilityFlags::empty(),
            authorized_regions: BTreeSet::new(),
            max_tier_level: 0,
        }
    }

    /// Creates a capability set with full permissions (for privileged agents).
    pub fn full() -> Self {
        MemoryCapabilitySet {
            flags: MemoryCapabilityFlags::ALL,
            authorized_regions: BTreeSet::new(), // Empty means all regions
            max_tier_level: 2, // Access to all tiers
        }
    }

    /// Creates a capability set for L1 working memory only.
    pub fn l1_only() -> Self {
        let mut caps = MemoryCapabilitySet::new();
        caps.flags = MemoryCapabilityFlags::ALLOCATE
            | MemoryCapabilityFlags::READ
            | MemoryCapabilityFlags::WRITE
            | MemoryCapabilityFlags::EVICT
            | MemoryCapabilityFlags::MIGRATE;
        caps.max_tier_level = 0;
        caps
    }

    /// Creates a capability set for L1 and L2 access.
    pub fn l1_l2_access() -> Self {
        let mut caps = MemoryCapabilitySet::new();
        caps.flags = MemoryCapabilityFlags::ALLOCATE
            | MemoryCapabilityFlags::READ
            | MemoryCapabilityFlags::WRITE
            | MemoryCapabilityFlags::EVICT
            | MemoryCapabilityFlags::MIGRATE
            | MemoryCapabilityFlags::QUERY;
        caps.max_tier_level = 1;
        caps
    }

    /// Creates a capability set for read-only L3 access.
    pub fn l3_readonly() -> Self {
        let mut caps = MemoryCapabilitySet::new();
        caps.flags = MemoryCapabilityFlags::READ | MemoryCapabilityFlags::QUERY;
        caps.max_tier_level = 2;
        caps
    }

    /// Grants a specific capability.
    pub fn grant(&mut self, capability: MemoryCapabilityFlags) {
        self.flags |= capability;
    }

    /// Revokes a specific capability.
    pub fn revoke(&mut self, capability: MemoryCapabilityFlags) {
        self.flags.remove(capability);
    }

    /// Checks if a capability is granted.
    pub fn has(&self, capability: MemoryCapabilityFlags) -> bool {
        self.flags.contains(capability)
    }

    /// Checks if multiple capabilities are granted (all must be present).
    pub fn has_all(&self, capabilities: MemoryCapabilityFlags) -> bool {
        self.flags.contains(capabilities)
    }

    /// Checks if any of the given capabilities are granted.
    pub fn has_any(&self, capabilities: MemoryCapabilityFlags) -> bool {
        self.flags.intersects(capabilities)
    }

    /// Authorizes access to a specific region.
    pub fn authorize_region(&mut self, region_id: impl Into<String>) {
        self.authorized_regions.insert(region_id.into());
    }

    /// Revokes access to a specific region.
    pub fn revoke_region(&mut self, region_id: &str) {
        self.authorized_regions.remove(region_id);
    }

    /// Checks if access to a region is authorized.
    pub fn is_region_authorized(&self, region_id: &str) -> bool {
        self.authorized_regions.is_empty() || self.authorized_regions.contains(region_id)
    }

    /// Sets the maximum tier level this agent can access.
    pub fn set_max_tier_level(&mut self, level: u32) {
        self.max_tier_level = level;
    }

    /// Gets the maximum tier level.
    pub fn max_tier_level(&self) -> u32 {
        self.max_tier_level
    }

    /// Checks if access to a tier is allowed.
    pub fn can_access_tier(&self, tier_level: u32) -> bool {
        tier_level <= self.max_tier_level
    }

    /// Returns a human-readable description of the capability set.
    pub fn description(&self) -> String {
        format!(
            "MemoryCapabilitySet {{ capabilities: [{}], max_tier: L{}, regions: {} }}",
            self.flags.description(),
            self.max_tier_level,
            if self.authorized_regions.is_empty() {
                "all".to_string()
            } else {
                format!("{:?}", self.authorized_regions.len())
            }
        )
    }
}

impl Default for MemoryCapabilitySet {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::format;
use alloc::string::ToString;
use alloc::vec::Vec;

    #[test]
    fn test_isolation_level_name() {
        assert_eq!(IsolationLevel::PerAgent.name(), "per_agent");
        assert_eq!(IsolationLevel::PerCrew.name(), "per_crew");
        assert_eq!(IsolationLevel::SharedReadOnly.name(), "shared_read_only");
    }

    #[test]
    fn test_isolation_level_cross_agent_access() {
        assert!(!IsolationLevel::PerAgent.allows_cross_agent_access());
        assert!(IsolationLevel::PerCrew.allows_cross_agent_access());
        assert!(IsolationLevel::SharedReadOnly.allows_cross_agent_access());
    }

    #[test]
    fn test_isolation_level_allows_write() {
        assert!(IsolationLevel::PerAgent.allows_write());
        assert!(IsolationLevel::PerCrew.allows_write());
        assert!(!IsolationLevel::SharedReadOnly.allows_write());
    }

    #[test]
    fn test_isolation_level_ordering() {
        assert!(IsolationLevel::PerAgent < IsolationLevel::PerCrew);
        assert!(IsolationLevel::PerCrew < IsolationLevel::SharedReadOnly);
    }

    #[test]
    fn test_memory_capability_flags_description() {
        let flags = MemoryCapabilityFlags::READ | MemoryCapabilityFlags::WRITE;
        let desc = flags.description();
        assert!(desc.contains("read"));
        assert!(desc.contains("write"));
    }

    #[test]
    fn test_memory_capability_flags_empty() {
        let flags = MemoryCapabilityFlags::empty();
        assert_eq!(flags.description(), "none");
    }

    #[test]
    fn test_memory_capability_flags_all() {
        let flags = MemoryCapabilityFlags::ALL;
        assert!(flags.contains(MemoryCapabilityFlags::ALLOCATE));
        assert!(flags.contains(MemoryCapabilityFlags::READ));
        assert!(flags.contains(MemoryCapabilityFlags::WRITE));
    }

    #[test]
    fn test_capability_set_new() {
        let caps = MemoryCapabilitySet::new();
        assert!(!caps.has(MemoryCapabilityFlags::READ));
        assert_eq!(caps.max_tier_level(), 0);
    }

    #[test]
    fn test_capability_set_full() {
        let caps = MemoryCapabilitySet::full();
        assert!(caps.has(MemoryCapabilityFlags::READ));
        assert!(caps.has(MemoryCapabilityFlags::WRITE));
        assert_eq!(caps.max_tier_level(), 2);
    }

    #[test]
    fn test_capability_set_l1_only() {
        let caps = MemoryCapabilitySet::l1_only();
        assert!(caps.has(MemoryCapabilityFlags::ALLOCATE));
        assert!(caps.has(MemoryCapabilityFlags::READ));
        assert_eq!(caps.max_tier_level(), 0);
    }

    #[test]
    fn test_capability_set_l1_l2_access() {
        let caps = MemoryCapabilitySet::l1_l2_access();
        assert!(caps.has(MemoryCapabilityFlags::QUERY));
        assert_eq!(caps.max_tier_level(), 1);
    }

    #[test]
    fn test_capability_set_l3_readonly() {
        let caps = MemoryCapabilitySet::l3_readonly();
        assert!(caps.has(MemoryCapabilityFlags::READ));
        assert!(!caps.has(MemoryCapabilityFlags::WRITE));
        assert_eq!(caps.max_tier_level(), 2);
    }

    #[test]
    fn test_capability_set_grant() {
        let mut caps = MemoryCapabilitySet::new();
        assert!(!caps.has(MemoryCapabilityFlags::READ));

        caps.grant(MemoryCapabilityFlags::READ);
        assert!(caps.has(MemoryCapabilityFlags::READ));
    }

    #[test]
    fn test_capability_set_revoke() {
        let mut caps = MemoryCapabilitySet::full();
        assert!(caps.has(MemoryCapabilityFlags::READ));

        caps.revoke(MemoryCapabilityFlags::READ);
        assert!(!caps.has(MemoryCapabilityFlags::READ));
    }

    #[test]
    fn test_capability_set_has_all() {
        let mut caps = MemoryCapabilitySet::new();
        caps.grant(MemoryCapabilityFlags::READ);
        caps.grant(MemoryCapabilityFlags::WRITE);

        assert!(caps.has_all(MemoryCapabilityFlags::READ | MemoryCapabilityFlags::WRITE));
        assert!(!caps.has_all(MemoryCapabilityFlags::READ | MemoryCapabilityFlags::ALLOCATE));
    }

    #[test]
    fn test_capability_set_has_any() {
        let mut caps = MemoryCapabilitySet::new();
        caps.grant(MemoryCapabilityFlags::READ);

        assert!(caps.has_any(MemoryCapabilityFlags::READ | MemoryCapabilityFlags::WRITE));
        assert!(!caps.has_any(MemoryCapabilityFlags::WRITE | MemoryCapabilityFlags::ALLOCATE));
    }

    #[test]
    fn test_capability_set_authorize_region() {
        let mut caps = MemoryCapabilitySet::new();
        caps.authorize_region("region-1");

        assert!(caps.is_region_authorized("region-1"));
        assert!(!caps.is_region_authorized("region-2"));
    }

    #[test]
    fn test_capability_set_revoke_region() {
        let mut caps = MemoryCapabilitySet::new();
        caps.authorize_region("region-1");
        assert!(caps.is_region_authorized("region-1"));

        caps.revoke_region("region-1");
        assert!(!caps.is_region_authorized("region-1"));
    }

    #[test]
    fn test_capability_set_region_all_authorized() {
        let caps = MemoryCapabilitySet::new();
        // Empty authorized regions means all regions are authorized
        assert!(caps.is_region_authorized("any-region"));
    }

    #[test]
    fn test_capability_set_max_tier_level() {
        let mut caps = MemoryCapabilitySet::new();
        caps.set_max_tier_level(1);

        assert!(caps.can_access_tier(0));
        assert!(caps.can_access_tier(1));
        assert!(!caps.can_access_tier(2));
    }

    #[test]
    fn test_capability_set_description() {
        let caps = MemoryCapabilitySet::full();
        let desc = caps.description();
        assert!(desc.contains("MemoryCapabilitySet"));
        assert!(desc.contains("max_tier"));
    }

    #[test]
    fn test_capability_set_default() {
        let caps = MemoryCapabilitySet::default();
        assert_eq!(caps, MemoryCapabilitySet::new());
    }
}
