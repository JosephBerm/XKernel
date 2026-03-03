// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Shared memory regions for crew-wide collaboration.
//!
//! This module implements shared memory regions that allow multiple agents
//! (Cognitive Threads) within a crew to collaborate using shared L3 memory.
//! For read-write shared regions, CRDT (Conflict-free Replicated Data Type)
//! mechanisms resolve concurrent writes without global coordination.
//!
//! # Access Modes
//!
//! - SharedReadOnly: All agents can read, no writes allowed
//! - SharedReadWrite: All agents can read and write (CRDT-resolved)
//!
//! See Engineering Plan § 4.1.4: Crew-Wide Shared Memory & CRDT Consistency.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use crate::error::{MemoryError, Result};
use crate::crdt::CrdtType;

/// Access mode for a shared region.
///
/// Determines what operations are allowed on the region.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum SharedAccessMode {
    /// Read-only: all agents can read, no writes
    SharedReadOnly,
    /// Read-write: all agents can read and write (CRDT-resolved)
    SharedReadWrite,
}

impl SharedAccessMode {
    /// Returns a human-readable name.
    pub fn name(&self) -> &'static str {
        match self {
            SharedAccessMode::SharedReadOnly => "SharedReadOnly",
            SharedAccessMode::SharedReadWrite => "SharedReadWrite",
        }
    }

    /// Checks if write operations are allowed.
    pub fn allows_write(&self) -> bool {
        matches!(self, SharedAccessMode::SharedReadWrite)
    }
}

/// A shared memory region for crew collaboration.
///
/// Represents L3 memory that is accessible to multiple agents within a crew.
/// Uses CRDT for read-write mode to handle concurrent updates.
///
/// See Engineering Plan § 4.1.4: Shared Regions.
#[derive(Clone, Debug)]
pub struct SharedRegion {
    /// Unique region ID
    pub region_id: String,
    /// Crew that owns this region
    pub crew_id: String,
    /// Physical pages backing this region
    pub physical_pages: Vec<u64>,
    /// Agents mapped to this region (agent_id -> virtual_addr)
    pub mapped_agents: BTreeMap<String, u64>,
    /// Access mode (read-only or read-write)
    pub access_mode: SharedAccessMode,
    /// CRDT type if in read-write mode
    pub crdt_type: Option<CrdtType>,
    /// Total size in bytes
    pub size_bytes: u64,
}

impl SharedRegion {
    /// Creates a new shared region.
    ///
    /// # Arguments
    ///
    /// * `region_id` - Unique region identifier
    /// * `crew_id` - Crew owning this region
    /// * `size_bytes` - Total size in bytes
    /// * `access_mode` - Read-only or read-write
    /// * `crdt_type` - CRDT type (required if read-write)
    pub fn new(
        region_id: impl Into<String>,
        crew_id: impl Into<String>,
        size_bytes: u64,
        access_mode: SharedAccessMode,
        crdt_type: Option<CrdtType>,
    ) -> Self {
        SharedRegion {
            region_id: region_id.into(),
            crew_id: crew_id.into(),
            physical_pages: Vec::new(),
            mapped_agents: BTreeMap::new(),
            access_mode,
            crdt_type,
            size_bytes,
        }
    }

    /// Adds a physical page to this region.
    pub fn add_physical_page(&mut self, page: u64) {
        self.physical_pages.push(page);
    }

    /// Returns the number of physical pages.
    pub fn page_count(&self) -> usize {
        self.physical_pages.len()
    }

    /// Checks if an agent is mapped to this region.
    pub fn is_agent_mapped(&self, agent_id: &str) -> bool {
        self.mapped_agents.contains_key(agent_id)
    }

    /// Returns the virtual address where an agent is mapped (if mapped).
    pub fn get_agent_mapping(&self, agent_id: &str) -> Option<u64> {
        self.mapped_agents.get(agent_id).copied()
    }

    /// Returns all mapped agents.
    pub fn mapped_agent_list(&self) -> Vec<String> {
        self.mapped_agents.keys().cloned().collect()
    }

    /// Returns the number of agents mapped to this region.
    pub fn agent_count(&self) -> usize {
        self.mapped_agents.len()
    }
}

/// Statistics about shared regions in the system.
///
/// Provides observability into shared memory usage.
/// See Engineering Plan § 4.1.0: Monitoring.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SharedRegionStats {
    /// Total shared memory allocated across all regions
    pub total_shared_bytes: u64,
    /// Total number of shared regions
    pub region_count: usize,
    /// Average number of agents per region
    pub avg_agents_per_region: u64,
}

impl SharedRegionStats {
    /// Creates new shared region statistics.
    pub fn new(total_bytes: u64, regions: usize, avg_agents: u64) -> Self {
        SharedRegionStats {
            total_shared_bytes: total_bytes,
            region_count: regions,
            avg_agents_per_region: avg_agents,
        }
    }
}

/// CRDT conflict resolution mechanism for shared regions.
///
/// When two agents write concurrently to a shared read-write region,
/// the CRDT resolves the conflict without requiring global coordination.
///
/// See Engineering Plan § 4.1.4: CRDT Resolution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CrdtResolution {
    /// The CRDT type determining resolution strategy
    pub crdt_type: CrdtType,
    /// Winning value (after conflict resolution)
    pub resolved_value: Vec<u8>,
    /// Vector clock timestamp
    pub vector_clock: u64,
}

impl CrdtResolution {
    /// Creates a new CRDT resolution.
    pub fn new(crdt_type: CrdtType, resolved_value: Vec<u8>, vector_clock: u64) -> Self {
        CrdtResolution {
            crdt_type,
            resolved_value,
            vector_clock,
        }
    }
}

/// Manager for shared regions in the system.
///
/// Handles creation, mapping, unmapping, and statistics collection
/// for shared memory regions.
///
/// See Engineering Plan § 4.1.4: Shared Region Management.
pub struct SharedRegionManager {
    /// Map of region_id -> SharedRegion
    regions: BTreeMap<String, SharedRegion>,
}

impl SharedRegionManager {
    /// Creates a new shared region manager.
    pub fn new() -> Self {
        SharedRegionManager {
            regions: BTreeMap::new(),
        }
    }

    /// Creates a new shared region.
    ///
    /// # Arguments
    ///
    /// * `region_id` - Unique region identifier
    /// * `crew_id` - Crew owning this region
    /// * `size_bytes` - Total size in bytes
    /// * `access_mode` - Read-only or read-write
    ///
    /// # Returns
    ///
    /// `Result<String>` with the region ID if successful
    ///
    /// See Engineering Plan § 4.1.4: Region Creation.
    pub fn create_shared_region(
        &mut self,
        region_id: impl Into<String>,
        crew_id: impl Into<String>,
        size_bytes: u64,
        access_mode: SharedAccessMode,
    ) -> Result<String> {
        let region_id_str = region_id.into();

        // Check for duplicate
        if self.regions.contains_key(&region_id_str) {
            return Err(MemoryError::Other(format!(
                "region {} already exists",
                region_id_str
            )));
        }

        // Determine CRDT type for read-write mode
        let crdt_type = if access_mode.allows_write() {
            Some(CrdtType::LWWRegister) // Default to Last-Writer-Wins
        } else {
            None
        };

        let region = SharedRegion::new(
            region_id_str.clone(),
            crew_id,
            size_bytes,
            access_mode,
            crdt_type,
        );

        self.regions.insert(region_id_str.clone(), region);

        Ok(region_id_str)
    }

    /// Maps an agent to a shared region.
    ///
    /// # Arguments
    ///
    /// * `agent_id` - Agent to map
    /// * `region_id` - Region to map to
    /// * `virtual_addr` - Virtual address where agent accesses the region
    ///
    /// # Returns
    ///
    /// `Result<u64>` with the virtual address if successful
    ///
    /// See Engineering Plan § 4.1.4: Agent Mapping.
    pub fn map_agent_to_region(
        &mut self,
        agent_id: impl Into<String>,
        region_id: &str,
        virtual_addr: u64,
    ) -> Result<u64> {
        let agent_id_str = agent_id.into();

        let region = self
            .regions
            .get_mut(region_id)
            .ok_or(MemoryError::InvalidReference {
                reason: format!("region {} not found", region_id),
            })?;

        // Check if already mapped
        if region.is_agent_mapped(&agent_id_str) {
            return Err(MemoryError::Other(format!(
                "agent {} already mapped to region {}",
                agent_id_str, region_id
            )));
        }

        region.mapped_agents.insert(agent_id_str, virtual_addr);

        Ok(virtual_addr)
    }

    /// Unmaps an agent from a shared region.
    ///
    /// # Arguments
    ///
    /// * `agent_id` - Agent to unmap
    /// * `region_id` - Region to unmap from
    ///
    /// # Returns
    ///
    /// `Result<()>` if successful
    ///
    /// See Engineering Plan § 4.1.4: Agent Unmapping.
    pub fn unmap_agent_from_region(&mut self, agent_id: &str, region_id: &str) -> Result<()> {
        let region = self
            .regions
            .get_mut(region_id)
            .ok_or(MemoryError::InvalidReference {
                reason: format!("region {} not found", region_id),
            })?;

        region.mapped_agents.remove(agent_id).ok_or(MemoryError::InvalidReference {
            reason: format!(
                "agent {} not mapped to region {}",
                agent_id, region_id
            ),
        })?;

        Ok(())
    }

    /// Gets a shared region (immutable).
    pub fn get_region(&self, region_id: &str) -> Result<&SharedRegion> {
        self.regions
            .get(region_id)
            .ok_or(MemoryError::InvalidReference {
                reason: format!("region {} not found", region_id),
            })
    }

    /// Gets a shared region (mutable).
    pub fn get_region_mut(&mut self, region_id: &str) -> Result<&mut SharedRegion> {
        self.regions
            .get_mut(region_id)
            .ok_or(MemoryError::InvalidReference {
                reason: format!("region {} not found", region_id),
            })
    }

    /// Computes statistics about all shared regions.
    pub fn compute_stats(&self) -> SharedRegionStats {
        let region_count = self.regions.len();
        let total_shared: u64 = self.regions.values().map(|r| r.size_bytes).sum();

        let avg_agents = if region_count > 0 {
            let total_agents: u64 = self
                .regions
                .values()
                .map(|r| r.agent_count() as u64)
                .sum();
            total_agents / region_count as u64
        } else {
            0
        };

        SharedRegionStats::new(total_shared, region_count, avg_agents)
    }

    /// Lists all shared regions.
    pub fn list_regions(&self) -> Vec<String> {
        self.regions.keys().cloned().collect()
    }

    /// Returns the total number of regions.
    pub fn region_count(&self) -> usize {
        self.regions.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::format;
use alloc::string::ToString;
use alloc::vec;

    #[test]
    fn test_shared_access_mode_names() {
        assert_eq!(SharedAccessMode::SharedReadOnly.name(), "SharedReadOnly");
        assert_eq!(
            SharedAccessMode::SharedReadWrite.name(),
            "SharedReadWrite"
        );
    }

    #[test]
    fn test_shared_access_mode_allows_write() {
        assert!(!SharedAccessMode::SharedReadOnly.allows_write());
        assert!(SharedAccessMode::SharedReadWrite.allows_write());
    }

    #[test]
    fn test_shared_region_creation() {
        let region = SharedRegion::new(
            "region-001",
            "crew-001",
            1024,
            SharedAccessMode::SharedReadOnly,
            None,
        );

        assert_eq!(region.region_id, "region-001");
        assert_eq!(region.crew_id, "crew-001");
        assert_eq!(region.size_bytes, 1024);
        assert_eq!(region.page_count(), 0);
        assert_eq!(region.agent_count(), 0);
    }

    #[test]
    fn test_shared_region_add_page() {
        let mut region = SharedRegion::new(
            "region-001",
            "crew-001",
            1024,
            SharedAccessMode::SharedReadOnly,
            None,
        );

        region.add_physical_page(100);
        region.add_physical_page(101);
        assert_eq!(region.page_count(), 2);
    }

    #[test]
    fn test_shared_region_agent_mapping() {
        let mut region = SharedRegion::new(
            "region-001",
            "crew-001",
            1024,
            SharedAccessMode::SharedReadWrite,
            Some(CrdtType::LWWRegister),
        );

        region.mapped_agents.insert("agent-001".to_string(), 0x1000);
        assert!(region.is_agent_mapped("agent-001"));
        assert!(!region.is_agent_mapped("agent-999"));
        assert_eq!(region.get_agent_mapping("agent-001"), Some(0x1000));
        assert_eq!(region.agent_count(), 1);
    }

    #[test]
    fn test_shared_region_mapped_agent_list() {
        let mut region = SharedRegion::new(
            "region-001",
            "crew-001",
            1024,
            SharedAccessMode::SharedReadWrite,
            Some(CrdtType::LWWRegister),
        );

        region.mapped_agents.insert("agent-001".to_string(), 0x1000);
        region.mapped_agents.insert("agent-002".to_string(), 0x2000);

        let agents = region.mapped_agent_list();
        assert_eq!(agents.len(), 2);
        assert!(agents.contains(&"agent-001".to_string()));
        assert!(agents.contains(&"agent-002".to_string()));
    }

    #[test]
    fn test_shared_region_stats_creation() {
        let stats = SharedRegionStats::new(1024000, 5, 10);
        assert_eq!(stats.total_shared_bytes, 1024000);
        assert_eq!(stats.region_count, 5);
        assert_eq!(stats.avg_agents_per_region, 10);
    }

    #[test]
    fn test_crdt_resolution_creation() {
        let data = vec![1, 2, 3];
        let resolution = CrdtResolution::new(CrdtType::LWWRegister, data.clone(), 42);
        assert_eq!(resolution.crdt_type, CrdtType::LWWRegister);
        assert_eq!(resolution.resolved_value, data);
        assert_eq!(resolution.vector_clock, 42);
    }

    #[test]
    fn test_shared_region_manager_creation() {
        let manager = SharedRegionManager::new();
        assert_eq!(manager.region_count(), 0);
    }

    #[test]
    fn test_shared_region_manager_create_region() {
        let mut manager = SharedRegionManager::new();
        let result = manager.create_shared_region(
            "region-001",
            "crew-001",
            2048,
            SharedAccessMode::SharedReadOnly,
        );

        assert!(result.is_ok());
        assert_eq!(manager.region_count(), 1);
    }

    #[test]
    fn test_shared_region_manager_create_region_duplicate() {
        let mut manager = SharedRegionManager::new();
        manager
            .create_shared_region(
                "region-001",
                "crew-001",
                2048,
                SharedAccessMode::SharedReadOnly,
            )
            .ok();

        let result =
            manager.create_shared_region("region-001", "crew-001", 2048, SharedAccessMode::SharedReadOnly);
        assert!(result.is_err());
    }

    #[test]
    fn test_shared_region_manager_map_agent() {
        let mut manager = SharedRegionManager::new();
        manager
            .create_shared_region(
                "region-001",
                "crew-001",
                2048,
                SharedAccessMode::SharedReadWrite,
            )
            .ok();

        let result = manager.map_agent_to_region("agent-001", "region-001", 0x1000);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0x1000);

        let region = manager.get_region("region-001").unwrap();
        assert!(region.is_agent_mapped("agent-001"));
    }

    #[test]
    fn test_shared_region_manager_map_agent_duplicate() {
        let mut manager = SharedRegionManager::new();
        manager
            .create_shared_region(
                "region-001",
                "crew-001",
                2048,
                SharedAccessMode::SharedReadWrite,
            )
            .ok();

        manager.map_agent_to_region("agent-001", "region-001", 0x1000).ok();
        let result = manager.map_agent_to_region("agent-001", "region-001", 0x2000);
        assert!(result.is_err());
    }

    #[test]
    fn test_shared_region_manager_map_agent_not_found() {
        let mut manager = SharedRegionManager::new();
        let result = manager.map_agent_to_region("agent-001", "region-999", 0x1000);
        assert!(result.is_err());
    }

    #[test]
    fn test_shared_region_manager_unmap_agent() {
        let mut manager = SharedRegionManager::new();
        manager
            .create_shared_region(
                "region-001",
                "crew-001",
                2048,
                SharedAccessMode::SharedReadWrite,
            )
            .ok();
        manager.map_agent_to_region("agent-001", "region-001", 0x1000).ok();

        let result = manager.unmap_agent_from_region("agent-001", "region-001");
        assert!(result.is_ok());

        let region = manager.get_region("region-001").unwrap();
        assert!(!region.is_agent_mapped("agent-001"));
    }

    #[test]
    fn test_shared_region_manager_unmap_agent_not_found() {
        let mut manager = SharedRegionManager::new();
        manager
            .create_shared_region(
                "region-001",
                "crew-001",
                2048,
                SharedAccessMode::SharedReadWrite,
            )
            .ok();

        let result = manager.unmap_agent_from_region("agent-999", "region-001");
        assert!(result.is_err());
    }

    #[test]
    fn test_shared_region_manager_get_region() {
        let mut manager = SharedRegionManager::new();
        manager
            .create_shared_region(
                "region-001",
                "crew-001",
                2048,
                SharedAccessMode::SharedReadOnly,
            )
            .ok();

        let result = manager.get_region("region-001");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().crew_id, "crew-001");

        let result_not_found = manager.get_region("region-999");
        assert!(result_not_found.is_err());
    }

    #[test]
    fn test_shared_region_manager_list_regions() {
        let mut manager = SharedRegionManager::new();
        manager
            .create_shared_region("region-001", "crew-001", 2048, SharedAccessMode::SharedReadOnly)
            .ok();
        manager
            .create_shared_region("region-002", "crew-001", 2048, SharedAccessMode::SharedReadWrite)
            .ok();

        let regions = manager.list_regions();
        assert_eq!(regions.len(), 2);
        assert!(regions.contains(&"region-001".to_string()));
        assert!(regions.contains(&"region-002".to_string()));
    }

    #[test]
    fn test_shared_region_manager_compute_stats() {
        let mut manager = SharedRegionManager::new();
        manager
            .create_shared_region("region-001", "crew-001", 1024, SharedAccessMode::SharedReadOnly)
            .ok();
        manager
            .create_shared_region("region-002", "crew-001", 2048, SharedAccessMode::SharedReadWrite)
            .ok();

        manager.map_agent_to_region("agent-001", "region-001", 0x1000).ok();
        manager.map_agent_to_region("agent-002", "region-001", 0x2000).ok();
        manager.map_agent_to_region("agent-001", "region-002", 0x3000).ok();

        let stats = manager.compute_stats();
        assert_eq!(stats.total_shared_bytes, 3072);
        assert_eq!(stats.region_count, 2);
        assert_eq!(stats.avg_agents_per_region, 1); // (2 + 1) / 2 = 1
    }
}
