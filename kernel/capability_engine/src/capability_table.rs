// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! In-kernel capability storage and lookup.
//!
//! This module provides the central capability table for the kernel, enabling fast,
//! type-safe lookup and management of all active capabilities.
//! See Engineering Plan § 3.2.0: Capability Runtime & Tables.

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::vec::Vec;
use core::fmt::{self, Debug, Display};

use crate::capability::Capability;
use crate::error::CapError;
use crate::ids::{AgentID, CapID};
use crate::constraints::Timestamp;

/// Page table mapping information for a capability's granted resource.
///
/// Tracks the virtual-to-physical mapping and access permissions for a resource
/// granted by a capability.
/// See Engineering Plan § 3.2.0: Capability Runtime & Tables.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PageMapping {
    /// Virtual address of the mapped region.
    pub virtual_addr: u64,

    /// Physical address of the mapped region.
    pub physical_addr: u64,

    /// Size of the mapped region in bytes.
    pub size: u64,

    /// Permission bits for this mapping.
    pub permissions: PagePermission,
}

impl PageMapping {
    /// Creates a new page mapping.
    ///
    /// This is called when a capability is granted for a resource that requires
    /// page table setup.
    pub fn new(
        virtual_addr: u64,
        physical_addr: u64,
        size: u64,
        permissions: PagePermission,
    ) -> Self {
        PageMapping {
            virtual_addr,
            physical_addr,
            size,
            permissions,
        }
    }

    /// Returns the ending virtual address (exclusive).
    pub fn virtual_end(&self) -> u64 {
        self.virtual_addr.saturating_add(self.size)
    }

    /// Returns the ending physical address (exclusive).
    pub fn physical_end(&self) -> u64 {
        self.physical_addr.saturating_add(self.size)
    }

    /// Checks if this mapping overlaps with another.
    pub fn overlaps_with(&self, other: &PageMapping) -> bool {
        !(self.virtual_end() <= other.virtual_addr || other.virtual_end() <= self.virtual_addr)
    }
}

impl Display for PageMapping {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "PageMapping(virt=0x{:x}, phys=0x{:x}, size=0x{:x}, perms={})",
            self.virtual_addr, self.physical_addr, self.size, self.permissions
        )
    }
}

/// Page access permissions.
///
/// Determines how a capability holder can access a mapped page region.
/// See Engineering Plan § 3.2.0: Capability Runtime & Tables.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PagePermission {
    /// Read-only access to the page.
    ReadOnly,

    /// Read and write access to the page.
    ReadWrite,

    /// Execute access to the page.
    Execute,
}

impl Display for PagePermission {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PagePermission::ReadOnly => write!(f, "RO"),
            PagePermission::ReadWrite => write!(f, "RW"),
            PagePermission::Execute => write!(f, "X"),
        }
    }
}

/// A single entry in the capability table.
///
/// Stores a capability along with metadata about its holders, page mappings,
/// and access history.
/// See Engineering Plan § 3.2.0: Capability Runtime & Tables.
#[derive(Clone, Debug)]
pub struct CapabilityEntry {
    /// The capability itself.
    pub capability: Capability,

    /// Set of agents currently holding this capability.
    pub holder_set: BTreeSet<AgentID>,

    /// Page table mappings for resources granted by this capability.
    pub page_table_mappings: Vec<PageMapping>,

    /// When this capability was created.
    pub created_at: Timestamp,

    /// When this capability was last accessed.
    pub last_accessed: Timestamp,
}

impl CapabilityEntry {
    /// Creates a new capability entry.
    ///
    /// Per Engineering Plan § 3.2.0, every entry tracks holders and access.
    pub fn new(
        capability: Capability,
        initial_holder: AgentID,
        now: Timestamp,
    ) -> Self {
        let mut holder_set = BTreeSet::new();
        holder_set.insert(initial_holder);

        CapabilityEntry {
            capability,
            holder_set,
            page_table_mappings: Vec::new(),
            created_at: now,
            last_accessed: now,
        }
    }

    /// Adds a new holder to this capability entry.
    pub fn add_holder(&mut self, agent: AgentID) {
        self.holder_set.insert(agent);
    }

    /// Removes a holder from this capability entry.
    pub fn remove_holder(&mut self, agent: &AgentID) {
        self.holder_set.remove(agent);
    }

    /// Returns true if the given agent holds this capability.
    pub fn is_held_by(&self, agent: &AgentID) -> bool {
        self.holder_set.contains(agent)
    }

    /// Adds a page table mapping to this capability entry.
    pub fn add_page_mapping(&mut self, mapping: PageMapping) {
        self.page_table_mappings.push(mapping);
    }

    /// Updates the last accessed timestamp.
    pub fn update_access_time(&mut self, now: Timestamp) {
        self.last_accessed = now;
    }
}

impl Display for CapabilityEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CapEntry({}, holders={}, mappings={})",
            self.capability.id,
            self.holder_set.len(),
            self.page_table_mappings.len()
        )
    }
}

/// Statistics about the capability table.
///
/// Provides metrics on the number of stored capabilities for monitoring
/// and diagnostic purposes.
/// See Engineering Plan § 3.2.0: Capability Runtime & Tables.
#[derive(Clone, Copy, Debug, Default)]
pub struct CapabilityTableStats {
    /// Total number of entries in the table.
    pub total_entries: usize,

    /// Number of active (non-revoked) entries.
    pub active_entries: usize,

    /// Number of revoked entries still in the table.
    pub revoked_count: usize,
}

impl Display for CapabilityTableStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CapTableStats(total={}, active={}, revoked={})",
            self.total_entries, self.active_entries, self.revoked_count
        )
    }
}

/// The in-kernel capability table.
///
/// Provides O(log n) lookup, insertion, and deletion of capabilities indexed by CapID.
/// This is the central data structure for capability management in the kernel.
/// See Engineering Plan § 3.2.0: Capability Runtime & Tables.
#[derive(Clone, Debug, Default)]
pub struct CapabilityTable {
    /// Map from CapID to CapabilityEntry.
    entries: BTreeMap<CapID, CapabilityEntry>,
}

impl CapabilityTable {
    /// Creates a new, empty capability table.
    pub fn new() -> Self {
        CapabilityTable {
            entries: BTreeMap::new(),
        }
    }

    /// Looks up a capability by ID.
    ///
    /// Returns a reference to the capability entry if it exists.
    /// Target latency: <100ns (warm cache)
    /// See Engineering Plan § 3.2.0: Capability Runtime & Tables.
    pub fn lookup(&self, cap_id: &CapID) -> Result<&CapabilityEntry, CapError> {
        self.entries.get(cap_id).ok_or_else(|| {
            CapError::Other(format!("capability not found: {}", cap_id))
        })
    }

    /// Mutably looks up a capability by ID.
    ///
    /// Returns a mutable reference to the capability entry if it exists.
    pub fn lookup_mut(&mut self, cap_id: &CapID) -> Result<&mut CapabilityEntry, CapError> {
        self.entries.get_mut(cap_id).ok_or_else(|| {
            CapError::Other(format!("capability not found: {}", cap_id))
        })
    }

    /// Inserts a new capability into the table.
    ///
    /// If a capability with the same ID already exists, returns an error
    /// to prevent accidental overwrites.
    /// See Engineering Plan § 3.2.0: Capability Runtime & Tables.
    pub fn insert(&mut self, cap_id: CapID, entry: CapabilityEntry) -> Result<(), CapError> {
        if self.entries.contains_key(&cap_id) {
            return Err(CapError::Other(format!(
                "capability already exists: {}",
                cap_id
            )));
        }
        self.entries.insert(cap_id, entry);
        Ok(())
    }

    /// Removes a capability from the table.
    ///
    /// Returns the capability entry if it exists, or an error if not.
    /// See Engineering Plan § 3.2.0: Capability Runtime & Tables.
    pub fn remove(&mut self, cap_id: &CapID) -> Result<CapabilityEntry, CapError> {
        self.entries.remove(cap_id).ok_or_else(|| {
            CapError::Other(format!("capability not found for removal: {}", cap_id))
        })
    }

    /// Returns all capability entries held by a specific agent.
    ///
    /// Scans the entire table; complexity is O(n) where n is the number of entries.
    pub fn entries_for_agent(&self, agent_id: &AgentID) -> Vec<&CapabilityEntry> {
        self.entries
            .values()
            .filter(|entry| entry.is_held_by(agent_id))
            .collect()
    }

    /// Returns the number of capabilities in the table.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns true if the table is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns an iterator over all capability entries.
    pub fn iter(&self) -> impl Iterator<Item = (&CapID, &CapabilityEntry)> {
        self.entries.iter()
    }

    /// Returns a mutable iterator over all capability entries.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&CapID, &mut CapabilityEntry)> {
        self.entries.iter_mut()
    }

    /// Clears all entries from the table.
    ///
    /// This should only be called during kernel shutdown or testing.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Computes statistics about the table.
    ///
    /// Currently, all entries are considered active (revocation tracking
    /// is handled separately in the revoke module).
    pub fn compute_stats(&self) -> CapabilityTableStats {
        let total_entries = self.entries.len();
        // Note: revoked_count would be computed from a separate revocation tracking
        // structure. For now, all entries are active.
        CapabilityTableStats {
            total_entries,
            active_entries: total_entries,
            revoked_count: 0,
        }
    }
}

impl Display for CapabilityTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CapTable(entries={})", self.entries.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::ResourceID;
    use crate::operations::OperationSet;
    use crate::constraints::Timestamp;
use alloc::collections::BTreeMap;
use alloc::collections::BTreeSet;
use alloc::format;
use alloc::string::ToString;

    fn make_test_capability(cap_id_byte: u8) -> Capability {
        let mut bytes = [0u8; 32];
        bytes[0] = cap_id_byte;
        Capability::new(
            CapID::from_bytes(bytes),
            AgentID::new("test-agent"),
            crate::ids::ResourceType::file(),
            ResourceID::new("test-resource"),
            OperationSet::all(),
            Timestamp::new(1000),
        )
    }

    #[test]
    fn test_page_mapping_creation() {
        let mapping = PageMapping::new(0x1000, 0x2000, 0x1000, PagePermission::ReadWrite);
        assert_eq!(mapping.virtual_addr, 0x1000);
        assert_eq!(mapping.physical_addr, 0x2000);
        assert_eq!(mapping.size, 0x1000);
        assert_eq!(mapping.permissions, PagePermission::ReadWrite);
    }

    #[test]
    fn test_page_mapping_boundaries() {
        let mapping = PageMapping::new(0x1000, 0x2000, 0x1000, PagePermission::ReadWrite);
        assert_eq!(mapping.virtual_end(), 0x2000);
        assert_eq!(mapping.physical_end(), 0x3000);
    }

    #[test]
    fn test_page_mapping_overlap_detection() {
        let m1 = PageMapping::new(0x1000, 0x2000, 0x1000, PagePermission::ReadWrite);
        let m2 = PageMapping::new(0x1500, 0x2500, 0x1000, PagePermission::ReadWrite);
        let m3 = PageMapping::new(0x3000, 0x4000, 0x1000, PagePermission::ReadWrite);

        assert!(m1.overlaps_with(&m2));
        assert!(m2.overlaps_with(&m1));
        assert!(!m1.overlaps_with(&m3));
        assert!(!m3.overlaps_with(&m1));
    }

    #[test]
    fn test_capability_entry_creation() {
        let cap = make_test_capability(1);
        let agent = AgentID::new("holder-a");
        let now = Timestamp::new(1000);

        let entry = CapabilityEntry::new(cap.clone(), agent.clone(), now);
        assert_eq!(entry.holder_set.len(), 1);
        assert!(entry.is_held_by(&agent));
        assert_eq!(entry.created_at, now);
        assert_eq!(entry.last_accessed, now);
    }

    #[test]
    fn test_capability_entry_add_holder() {
        let cap = make_test_capability(1);
        let agent_a = AgentID::new("holder-a");
        let agent_b = AgentID::new("holder-b");
        let now = Timestamp::new(1000);

        let mut entry = CapabilityEntry::new(cap, agent_a.clone(), now);
        entry.add_holder(agent_b.clone());

        assert_eq!(entry.holder_set.len(), 2);
        assert!(entry.is_held_by(&agent_a));
        assert!(entry.is_held_by(&agent_b));
    }

    #[test]
    fn test_capability_entry_remove_holder() {
        let cap = make_test_capability(1);
        let agent_a = AgentID::new("holder-a");
        let agent_b = AgentID::new("holder-b");
        let now = Timestamp::new(1000);

        let mut entry = CapabilityEntry::new(cap, agent_a.clone(), now);
        entry.add_holder(agent_b.clone());
        entry.remove_holder(&agent_a);

        assert_eq!(entry.holder_set.len(), 1);
        assert!(!entry.is_held_by(&agent_a));
        assert!(entry.is_held_by(&agent_b));
    }

    #[test]
    fn test_capability_entry_add_page_mapping() {
        let cap = make_test_capability(1);
        let agent = AgentID::new("holder-a");
        let now = Timestamp::new(1000);
        let mut entry = CapabilityEntry::new(cap, agent, now);

        let mapping = PageMapping::new(0x1000, 0x2000, 0x1000, PagePermission::ReadWrite);
        entry.add_page_mapping(mapping);

        assert_eq!(entry.page_table_mappings.len(), 1);
        assert_eq!(entry.page_table_mappings[0].virtual_addr, 0x1000);
    }

    #[test]
    fn test_capability_entry_update_access_time() {
        let cap = make_test_capability(1);
        let agent = AgentID::new("holder-a");
        let initial_time = Timestamp::new(1000);
        let later_time = Timestamp::new(2000);

        let mut entry = CapabilityEntry::new(cap, agent, initial_time);
        assert_eq!(entry.last_accessed, initial_time);

        entry.update_access_time(later_time);
        assert_eq!(entry.last_accessed, later_time);
    }

    #[test]
    fn test_capability_table_insert_and_lookup() {
        let cap = make_test_capability(1);
        let agent = AgentID::new("holder-a");
        let now = Timestamp::new(1000);
        let entry = CapabilityEntry::new(cap.clone(), agent, now);

        let mut table = CapabilityTable::new();
        table.insert(cap.id.clone(), entry).unwrap();

        let looked_up = table.lookup(&cap.id).unwrap();
        assert!(looked_up.is_held_by(&AgentID::new("holder-a")));
    }

    #[test]
    fn test_capability_table_duplicate_insert_fails() {
        let cap = make_test_capability(1);
        let agent = AgentID::new("holder-a");
        let now = Timestamp::new(1000);
        let entry = CapabilityEntry::new(cap.clone(), agent, now);

        let mut table = CapabilityTable::new();
        table.insert(cap.id.clone(), entry).unwrap();

        let entry2 = CapabilityEntry::new(cap.clone(), AgentID::new("holder-b"), now);
        let result = table.insert(cap.id.clone(), entry2);
        assert!(result.is_err());
    }

    #[test]
    fn test_capability_table_lookup_nonexistent() {
        let table = CapabilityTable::new();
        let cap = make_test_capability(1);
        let result = table.lookup(&cap.id);
        assert!(result.is_err());
    }

    #[test]
    fn test_capability_table_remove() {
        let cap = make_test_capability(1);
        let agent = AgentID::new("holder-a");
        let now = Timestamp::new(1000);
        let entry = CapabilityEntry::new(cap.clone(), agent, now);

        let mut table = CapabilityTable::new();
        table.insert(cap.id.clone(), entry).unwrap();
        assert_eq!(table.len(), 1);

        table.remove(&cap.id).unwrap();
        assert_eq!(table.len(), 0);
    }

    #[test]
    fn test_capability_table_entries_for_agent() {
        let cap1 = make_test_capability(1);
        let cap2 = make_test_capability(2);
        let cap3 = make_test_capability(3);

        let agent_a = AgentID::new("holder-a");
        let agent_b = AgentID::new("holder-b");
        let now = Timestamp::new(1000);

        let entry1 = CapabilityEntry::new(cap1, agent_a.clone(), now);
        let entry2 = CapabilityEntry::new(cap2, agent_a.clone(), now);
        let entry3 = CapabilityEntry::new(cap3, agent_b.clone(), now);

        let mut table = CapabilityTable::new();
        table.insert(CapID::from_bytes([1u8; 32]), entry1).unwrap();
        table.insert(CapID::from_bytes([2u8; 32]), entry2).unwrap();
        table.insert(CapID::from_bytes([3u8; 32]), entry3).unwrap();

        let agent_a_caps = table.entries_for_agent(&agent_a);
        assert_eq!(agent_a_caps.len(), 2);

        let agent_b_caps = table.entries_for_agent(&agent_b);
        assert_eq!(agent_b_caps.len(), 1);

        let agent_c_caps = table.entries_for_agent(&AgentID::new("holder-c"));
        assert_eq!(agent_c_caps.len(), 0);
    }

    #[test]
    fn test_capability_table_clear() {
        let cap = make_test_capability(1);
        let agent = AgentID::new("holder-a");
        let now = Timestamp::new(1000);
        let entry = CapabilityEntry::new(cap.clone(), agent, now);

        let mut table = CapabilityTable::new();
        table.insert(cap.id.clone(), entry).unwrap();
        assert_eq!(table.len(), 1);

        table.clear();
        assert_eq!(table.len(), 0);
        assert!(table.is_empty());
    }

    #[test]
    fn test_capability_table_compute_stats() {
        let cap1 = make_test_capability(1);
        let cap2 = make_test_capability(2);

        let agent = AgentID::new("holder-a");
        let now = Timestamp::new(1000);

        let entry1 = CapabilityEntry::new(cap1, agent.clone(), now);
        let entry2 = CapabilityEntry::new(cap2, agent, now);

        let mut table = CapabilityTable::new();
        table.insert(CapID::from_bytes([1u8; 32]), entry1).unwrap();
        table.insert(CapID::from_bytes([2u8; 32]), entry2).unwrap();

        let stats = table.compute_stats();
        assert_eq!(stats.total_entries, 2);
        assert_eq!(stats.active_entries, 2);
        assert_eq!(stats.revoked_count, 0);
    }

    #[test]
    fn test_capability_table_len_and_is_empty() {
        let cap = make_test_capability(1);
        let agent = AgentID::new("holder-a");
        let now = Timestamp::new(1000);
        let entry = CapabilityEntry::new(cap.clone(), agent, now);

        let mut table = CapabilityTable::new();
        assert_eq!(table.len(), 0);
        assert!(table.is_empty());

        table.insert(cap.id.clone(), entry).unwrap();
        assert_eq!(table.len(), 1);
        assert!(!table.is_empty());
    }

    #[test]
    fn test_capability_table_iterator() {
        let cap1 = make_test_capability(1);
        let cap2 = make_test_capability(2);

        let agent = AgentID::new("holder-a");
        let now = Timestamp::new(1000);

        let entry1 = CapabilityEntry::new(cap1, agent.clone(), now);
        let entry2 = CapabilityEntry::new(cap2, agent, now);

        let mut table = CapabilityTable::new();
        table.insert(CapID::from_bytes([1u8; 32]), entry1).unwrap();
        table.insert(CapID::from_bytes([2u8; 32]), entry2).unwrap();

        let count = table.iter().count();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_page_permission_display() {
        assert_eq!(PagePermission::ReadOnly.to_string(), "RO");
        assert_eq!(PagePermission::ReadWrite.to_string(), "RW");
        assert_eq!(PagePermission::Execute.to_string(), "X");
    }

    #[test]
    fn test_capability_table_stats_display() {
        let stats = CapabilityTableStats {
            total_entries: 10,
            active_entries: 8,
            revoked_count: 2,
        };
        let display = stats.to_string();
        assert!(display.contains("10"));
        assert!(display.contains("8"));
        assert!(display.contains("2"));
    }
}
