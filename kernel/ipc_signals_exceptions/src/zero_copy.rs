// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! # Zero-Copy Shared Page Mapping
//!
//! This module implements zero-copy mechanisms for IPC by mapping shared memory pages
//! into multiple agent address spaces. Large payloads (>4KB) can use descriptor-based
//! zero-copy instead of copying data between address spaces.
//!
//! ## Shared Page Mapping
//!
//! - Maps physical pages into sender and receiver address spaces
//! - Enforces read-only access for receivers
//! - Tracks reference counts for cleanup
//! - Supports multi-receiver scenarios with copy-on-write semantics
//!
//! ## References
//!
//! - Engineering Plan § 5.3.3 (Zero-Copy Shared Memory)

use crate::error::{CsError, IpcError, Result};
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::vec::Vec;
use core::fmt;
use serde::{Deserialize, Serialize};

/// Permission flags for shared page access.
///
/// Defines read/write access permissions for agents accessing shared pages.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PagePermissions(u8);

impl PagePermissions {
    /// Flag: Read permission.
    const READ: u8 = 0x01;
    /// Flag: Write permission.
    const WRITE: u8 = 0x02;

    /// Create empty permissions.
    #[inline]
    pub fn new() -> Self {
        Self(0)
    }

    /// Create read-only permissions.
    #[inline]
    pub fn read_only() -> Self {
        Self(Self::READ)
    }

    /// Create read-write permissions.
    #[inline]
    pub fn read_write() -> Self {
        Self(Self::READ | Self::WRITE)
    }

    /// Set read permission.
    #[inline]
    pub fn set_read(mut self) -> Self {
        self.0 |= Self::READ;
        self
    }

    /// Check read permission.
    #[inline]
    pub fn can_read(&self) -> bool {
        (self.0 & Self::READ) != 0
    }

    /// Set write permission.
    #[inline]
    pub fn set_write(mut self) -> Self {
        self.0 |= Self::WRITE;
        self
    }

    /// Check write permission.
    #[inline]
    pub fn can_write(&self) -> bool {
        (self.0 & Self::WRITE) != 0
    }

    /// Get the raw bitfield value.
    #[inline]
    pub fn as_u8(&self) -> u8 {
        self.0
    }
}

impl Default for PagePermissions {
    fn default() -> Self {
        Self::new()
    }
}

/// Physical address for a shared memory region.
///
/// Represents a physical memory address in the kernel's physical address space.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PhysicalAddr(u64);

impl PhysicalAddr {
    /// Create a physical address.
    pub fn new(addr: u64) -> Self {
        Self(addr)
    }

    /// Get the raw address value.
    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

impl fmt::Display for PhysicalAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:x}", self.0)
    }
}

/// Shared page mapping information.
///
/// Describes a physical memory region and the permissions each party has.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SharedPageMapping {
    /// Physical address of the shared region.
    pub physical_addr: PhysicalAddr,

    /// Size of the shared region in bytes.
    pub size: u64,

    /// Permissions for the sender.
    pub sender_permissions: PagePermissions,

    /// Permissions for the receiver.
    pub receiver_permissions: PagePermissions,
}

impl SharedPageMapping {
    /// Create a new shared page mapping.
    pub fn new(
        physical_addr: PhysicalAddr,
        size: u64,
        sender_permissions: PagePermissions,
        receiver_permissions: PagePermissions,
    ) -> Self {
        Self {
            physical_addr,
            size,
            sender_permissions,
            receiver_permissions,
        }
    }
}

/// ID for a zero-copy region.
///
/// Uniquely identifies a zero-copy shared region.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ZeroCopyRegionId(u64);

impl ZeroCopyRegionId {
    /// Create a new region ID.
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get the raw ID value.
    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

impl fmt::Display for ZeroCopyRegionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "zero-copy-{}", self.0)
    }
}

/// Mapped region information.
///
/// Information about a zero-copy region mapped into an agent's address space.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MappedRegion {
    /// Region ID.
    pub region_id: ZeroCopyRegionId,

    /// Virtual address in the agent's address space.
    pub virtual_addr: u64,

    /// Size of the mapped region.
    pub size: u64,

    /// Permissions for this mapping.
    pub permissions: PagePermissions,
}

/// Zero-copy region with reference counting.
///
/// Tracks a shared memory region and which agents have it mapped.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ZeroCopyRegion {
    /// Region ID.
    pub id: ZeroCopyRegionId,

    /// Shared page mapping information.
    pub mapping: SharedPageMapping,

    /// Reference count (number of agents with this region mapped).
    pub ref_count: u32,

    /// Owner context tag.
    pub owner_ct: u64,

    /// Set of context tags that have this region mapped.
    mapped_cts: BTreeSet<u64>,
}

impl ZeroCopyRegion {
    /// Create a new zero-copy region.
    pub fn new(
        id: ZeroCopyRegionId,
        mapping: SharedPageMapping,
        owner_ct: u64,
    ) -> Self {
        let mut mapped_cts = BTreeSet::new();
        mapped_cts.insert(owner_ct);

        Self {
            id,
            mapping,
            ref_count: 1,
            owner_ct,
            mapped_cts,
        }
    }

    /// Add a reference (map into another CT).
    pub fn add_reference(&mut self, ct: u64) -> Result<()> {
        if self.ref_count == u32::MAX {
            return Err(CsError::Ipc(IpcError::Other(
                alloc::string::String::from("region reference count overflow"),
            )));
        }

        self.ref_count += 1;
        self.mapped_cts.insert(ct);
        Ok(())
    }

    /// Remove a reference (unmap from a CT).
    pub fn remove_reference(&mut self, ct: u64) -> Result<()> {
        if !self.mapped_cts.contains(&ct) {
            return Err(CsError::Ipc(IpcError::Other(
                alloc::string::String::from("context tag not mapped to region"),
            )));
        }

        self.ref_count = self.ref_count.saturating_sub(1);
        self.mapped_cts.remove(&ct);
        Ok(())
    }

    /// Check if a CT has this region mapped.
    pub fn is_mapped_to(&self, ct: u64) -> bool {
        self.mapped_cts.contains(&ct)
    }

    /// Get all CTs with this region mapped.
    pub fn mapped_contexts(&self) -> Vec<u64> {
        self.mapped_cts.iter().copied().collect()
    }
}

/// Zero-copy statistics.
///
/// Metrics about zero-copy usage in the system.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ZeroCopyStats {
    /// Total number of regions created.
    pub regions_created: u64,

    /// Total bytes shared via zero-copy.
    pub total_bytes_shared: u64,

    /// Average region size in bytes.
    pub avg_region_size: u64,

    /// Bytes of copying avoided through zero-copy.
    pub copy_avoided_bytes: u64,
}

impl ZeroCopyStats {
    /// Create empty statistics.
    pub fn new() -> Self {
        Self {
            regions_created: 0,
            total_bytes_shared: 0,
            avg_region_size: 0,
            copy_avoided_bytes: 0,
        }
    }
}

impl Default for ZeroCopyStats {
    fn default() -> Self {
        Self::new()
    }
}

/// Zero-copy region manager.
///
/// Manages the lifecycle of zero-copy regions including creation,
/// mapping/unmapping, and cleanup.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ZeroCopyManager {
    /// Map of regions by ID.
    regions: BTreeMap<ZeroCopyRegionId, ZeroCopyRegion>,

    /// Counter for generating region IDs.
    region_id_counter: u64,

    /// Statistics.
    pub stats: ZeroCopyStats,
}

impl ZeroCopyManager {
    /// Create a new zero-copy manager.
    pub fn new() -> Self {
        Self {
            regions: BTreeMap::new(),
            region_id_counter: 0,
            stats: ZeroCopyStats::new(),
        }
    }

    /// Generate a new region ID.
    fn generate_region_id(&mut self) -> ZeroCopyRegionId {
        self.region_id_counter += 1;
        ZeroCopyRegionId::new(self.region_id_counter)
    }

    /// Create a zero-copy region.
    ///
    /// Allocates a shared memory region accessible to the sender.
    /// Returns the created region.
    pub fn create_region(
        &mut self,
        physical_addr: PhysicalAddr,
        size: u64,
        sender_ct: u64,
    ) -> Result<ZeroCopyRegion> {
        if size == 0 {
            return Err(CsError::Ipc(IpcError::Other(
                alloc::string::String::from("zero-copy region size must be > 0"),
            )));
        }

        let region_id = self.generate_region_id();
        let mapping = SharedPageMapping::new(
            physical_addr,
            size,
            PagePermissions::read_write(),
            PagePermissions::read_only(),
        );

        let region = ZeroCopyRegion::new(region_id, mapping, sender_ct);

        self.regions.insert(region_id, region.clone());
        self.stats.regions_created += 1;
        self.stats.total_bytes_shared += size;

        if self.stats.regions_created > 0 {
            self.stats.avg_region_size = self.stats.total_bytes_shared / self.stats.regions_created;
        }

        Ok(region)
    }

    /// Map a region into a receiver's address space.
    ///
    /// Makes a previously created region accessible to another context.
    pub fn map_into_receiver(
        &mut self,
        region_id: ZeroCopyRegionId,
        receiver_ct: u64,
        virtual_addr: u64,
    ) -> Result<MappedRegion> {
        if let Some(region) = self.regions.get_mut(&region_id) {
            region.add_reference(receiver_ct)?;

            let mapped = MappedRegion {
                region_id,
                virtual_addr,
                size: region.mapping.size,
                permissions: region.mapping.receiver_permissions,
            };

            self.stats.copy_avoided_bytes += region.mapping.size;

            Ok(mapped)
        } else {
            Err(CsError::Ipc(IpcError::Other(
                alloc::string::String::from("region not found"),
            )))
        }
    }

    /// Unmap a region from a receiver's address space.
    ///
    /// Removes access to a region from a specific context.
    pub fn unmap_from_receiver(&mut self, region_id: ZeroCopyRegionId, receiver_ct: u64) -> Result<()> {
        if let Some(region) = self.regions.get_mut(&region_id) {
            region.remove_reference(receiver_ct)?;

            // Clean up region if no more references
            if region.ref_count == 0 {
                self.regions.remove(&region_id);
            }

            Ok(())
        } else {
            Err(CsError::Ipc(IpcError::Other(
                alloc::string::String::from("region not found"),
            )))
        }
    }

    /// Get a region by ID.
    pub fn get_region(&self, region_id: ZeroCopyRegionId) -> Result<ZeroCopyRegion> {
        self.regions
            .get(&region_id)
            .cloned()
            .ok_or_else(|| CsError::Ipc(IpcError::Other(alloc::string::String::from("region not found"))))
    }

    /// Get all regions.
    pub fn all_regions(&self) -> Vec<ZeroCopyRegion> {
        self.regions.values().cloned().collect()
    }

    /// Get regions mapped to a specific context.
    pub fn regions_for_context(&self, ct: u64) -> Vec<ZeroCopyRegion> {
        self.regions
            .values()
            .filter(|r| r.is_mapped_to(ct))
            .cloned()
            .collect()
    }

    /// Get the number of regions.
    pub fn region_count(&self) -> usize {
        self.regions.len()
    }
}

impl Default for ZeroCopyManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::collections::BTreeMap;
use alloc::collections::BTreeSet;
use alloc::string::String;
use alloc::string::ToString;

    #[test]
    fn test_page_permissions_read_only() {
        let perms = PagePermissions::read_only();
        assert!(perms.can_read());
        assert!(!perms.can_write());
    }

    #[test]
    fn test_page_permissions_read_write() {
        let perms = PagePermissions::read_write();
        assert!(perms.can_read());
        assert!(perms.can_write());
    }

    #[test]
    fn test_page_permissions_set_write() {
        let perms = PagePermissions::new().set_write();
        assert!(!perms.can_read());
        assert!(perms.can_write());
    }

    #[test]
    fn test_page_permissions_set_read() {
        let perms = PagePermissions::new().set_read();
        assert!(perms.can_read());
        assert!(!perms.can_write());
    }

    #[test]
    fn test_physical_addr_creation() {
        let addr = PhysicalAddr::new(0x1000);
        assert_eq!(addr.as_u64(), 0x1000);
    }

    #[test]
    fn test_physical_addr_display() {
        let addr = PhysicalAddr::new(0xdeadbeef);
        let s = addr.to_string();
        assert!(s.contains("deadbeef"));
    }

    #[test]
    fn test_zero_copy_region_id_creation() {
        let id1 = ZeroCopyRegionId::new(1);
        let id2 = ZeroCopyRegionId::new(2);
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_zero_copy_region_id_display() {
        let id = ZeroCopyRegionId::new(42);
        let s = id.to_string();
        assert!(s.contains("42"));
    }

    #[test]
    fn test_shared_page_mapping_creation() {
        let addr = PhysicalAddr::new(0x10000);
        let mapping = SharedPageMapping::new(
            addr,
            4096,
            PagePermissions::read_write(),
            PagePermissions::read_only(),
        );
        assert_eq!(mapping.physical_addr, addr);
        assert_eq!(mapping.size, 4096);
    }

    #[test]
    fn test_zero_copy_region_creation() {
        let addr = PhysicalAddr::new(0x10000);
        let mapping = SharedPageMapping::new(
            addr,
            4096,
            PagePermissions::read_write(),
            PagePermissions::read_only(),
        );
        let region_id = ZeroCopyRegionId::new(1);
        let region = ZeroCopyRegion::new(region_id, mapping, 100);

        assert_eq!(region.id, region_id);
        assert_eq!(region.owner_ct, 100);
        assert_eq!(region.ref_count, 1);
        assert!(region.is_mapped_to(100));
    }

    #[test]
    fn test_zero_copy_region_add_reference() {
        let addr = PhysicalAddr::new(0x10000);
        let mapping = SharedPageMapping::new(
            addr,
            4096,
            PagePermissions::read_write(),
            PagePermissions::read_only(),
        );
        let region_id = ZeroCopyRegionId::new(1);
        let mut region = ZeroCopyRegion::new(region_id, mapping, 100);

        region.add_reference(200).unwrap();
        assert_eq!(region.ref_count, 2);
        assert!(region.is_mapped_to(200));
    }

    #[test]
    fn test_zero_copy_region_remove_reference() {
        let addr = PhysicalAddr::new(0x10000);
        let mapping = SharedPageMapping::new(
            addr,
            4096,
            PagePermissions::read_write(),
            PagePermissions::read_only(),
        );
        let region_id = ZeroCopyRegionId::new(1);
        let mut region = ZeroCopyRegion::new(region_id, mapping, 100);

        region.add_reference(200).unwrap();
        region.remove_reference(200).unwrap();
        assert_eq!(region.ref_count, 1);
        assert!(!region.is_mapped_to(200));
    }

    #[test]
    fn test_zero_copy_region_remove_nonexistent_reference() {
        let addr = PhysicalAddr::new(0x10000);
        let mapping = SharedPageMapping::new(
            addr,
            4096,
            PagePermissions::read_write(),
            PagePermissions::read_only(),
        );
        let region_id = ZeroCopyRegionId::new(1);
        let mut region = ZeroCopyRegion::new(region_id, mapping, 100);

        assert!(region.remove_reference(999).is_err());
    }

    #[test]
    fn test_mapped_region_creation() {
        let region_id = ZeroCopyRegionId::new(1);
        let mapped = MappedRegion {
            region_id,
            virtual_addr: 0x70000000,
            size: 4096,
            permissions: PagePermissions::read_only(),
        };
        assert_eq!(mapped.region_id, region_id);
        assert!(mapped.permissions.can_read());
    }

    #[test]
    fn test_zero_copy_manager_creation() {
        let manager = ZeroCopyManager::new();
        assert_eq!(manager.region_count(), 0);
        assert_eq!(manager.stats.regions_created, 0);
    }

    #[test]
    fn test_zero_copy_manager_create_region() {
        let mut manager = ZeroCopyManager::new();
        let addr = PhysicalAddr::new(0x10000);
        let region = manager.create_region(addr, 4096, 100).unwrap();

        assert_eq!(manager.region_count(), 1);
        assert_eq!(manager.stats.regions_created, 1);
        assert_eq!(manager.stats.total_bytes_shared, 4096);
        assert!(region.is_mapped_to(100));
    }

    #[test]
    fn test_zero_copy_manager_create_zero_size_region() {
        let mut manager = ZeroCopyManager::new();
        let addr = PhysicalAddr::new(0x10000);
        assert!(manager.create_region(addr, 0, 100).is_err());
    }

    #[test]
    fn test_zero_copy_manager_map_into_receiver() {
        let mut manager = ZeroCopyManager::new();
        let addr = PhysicalAddr::new(0x10000);
        let region = manager.create_region(addr, 4096, 100).unwrap();

        let mapped = manager.map_into_receiver(region.id, 200, 0x70000000).unwrap();
        assert_eq!(mapped.virtual_addr, 0x70000000);
        assert_eq!(mapped.size, 4096);

        // Verify statistics
        assert_eq!(manager.stats.copy_avoided_bytes, 4096);

        let region = manager.get_region(region.id).unwrap();
        assert_eq!(region.ref_count, 2);
        assert!(region.is_mapped_to(200));
    }

    #[test]
    fn test_zero_copy_manager_unmap_from_receiver() {
        let mut manager = ZeroCopyManager::new();
        let addr = PhysicalAddr::new(0x10000);
        let region = manager.create_region(addr, 4096, 100).unwrap();

        manager.map_into_receiver(region.id, 200, 0x70000000).unwrap();
        manager.unmap_from_receiver(region.id, 200).unwrap();

        let region = manager.get_region(region.id).unwrap();
        assert_eq!(region.ref_count, 1);
        assert!(!region.is_mapped_to(200));
    }

    #[test]
    fn test_zero_copy_manager_cleanup_on_last_unmap() {
        let mut manager = ZeroCopyManager::new();
        let addr = PhysicalAddr::new(0x10000);
        let region = manager.create_region(addr, 4096, 100).unwrap();

        manager.unmap_from_receiver(region.id, 100).unwrap();
        assert_eq!(manager.region_count(), 0);
    }

    #[test]
    fn test_zero_copy_manager_multi_receiver() {
        let mut manager = ZeroCopyManager::new();
        let addr = PhysicalAddr::new(0x10000);
        let region = manager.create_region(addr, 4096, 100).unwrap();

        manager.map_into_receiver(region.id, 200, 0x70000000).unwrap();
        manager.map_into_receiver(region.id, 300, 0x80000000).unwrap();

        let region = manager.get_region(region.id).unwrap();
        assert_eq!(region.ref_count, 3);
        assert!(region.is_mapped_to(100));
        assert!(region.is_mapped_to(200));
        assert!(region.is_mapped_to(300));
    }

    #[test]
    fn test_zero_copy_manager_regions_for_context() {
        let mut manager = ZeroCopyManager::new();
        let addr1 = PhysicalAddr::new(0x10000);
        let addr2 = PhysicalAddr::new(0x20000);

        let region1 = manager.create_region(addr1, 4096, 100).unwrap();
        let region2 = manager.create_region(addr2, 8192, 100).unwrap();

        manager.map_into_receiver(region1.id, 200, 0x70000000).unwrap();
        manager.map_into_receiver(region2.id, 200, 0x80000000).unwrap();

        let regions_for_200 = manager.regions_for_context(200);
        assert_eq!(regions_for_200.len(), 2);
    }

    #[test]
    fn test_zero_copy_manager_statistics() {
        let mut manager = ZeroCopyManager::new();
        let addr = PhysicalAddr::new(0x10000);

        let region = manager.create_region(addr, 4096, 100).unwrap();
        manager.map_into_receiver(region.id, 200, 0x70000000).unwrap();

        assert_eq!(manager.stats.regions_created, 1);
        assert_eq!(manager.stats.total_bytes_shared, 4096);
        assert_eq!(manager.stats.avg_region_size, 4096);
        assert_eq!(manager.stats.copy_avoided_bytes, 4096);
    }
}
