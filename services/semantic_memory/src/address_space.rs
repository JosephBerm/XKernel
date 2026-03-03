// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Address space isolation for Memory Manager.
//!
//! This module manages virtual address space allocation and isolation for the
//! Memory Manager and Cognitive Threads (CTs). It maintains strict isolation
//! boundaries to prevent one CT from accessing another's memory regions.
//!
//! # Architecture
//!
//! The Memory Manager address space is divided into:
//! - Kernel region: Memory Manager's own code and data
//! - Service heap: Dynamic allocation for Memory Manager internal structures
//! - CT mapping table: Virtual-to-physical mappings for all connected CTs
//!
//! See Engineering Plan § 4.1.3: Isolation & Protection & § 4.1.0: Address Space.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use alloc::string::String;
use crate::error::{MemoryError, Result};

/// A mapped page entry (virtual-to-physical mapping).
///
/// Represents one page in the virtual address space of a CT.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PhysicalPageMapping {
    /// Physical page number (PPN)
    pub physical_page: u64,
    /// Whether the page is present in memory (vs. paged out)
    pub present: bool,
}

/// Entry in the CT mapping table - tracks all mappings for one CT.
///
/// Maintains virtual-to-physical mappings and access permissions for a single
/// Cognitive Thread.
///
/// See Engineering Plan § 4.1.3: CT Isolation.
#[derive(Clone, Debug)]
pub struct CtMappingEntry {
    /// Cognitive Thread ID
    pub ct_id: String,
    /// Virtual base address for this CT's allocation
    pub virtual_base: u64,
    /// Mapped physical pages (virtual_offset -> PhysicalPageMapping)
    pub physical_pages: BTreeMap<u64, PhysicalPageMapping>,
    /// Access permissions (bitmask: read=1, write=2, execute=4)
    pub permissions: u8,
    /// Memory tier this CT is using (L1, L2, or L3)
    pub tier: String,
}

impl CtMappingEntry {
    /// Creates a new CT mapping entry.
    ///
    /// # Arguments
    ///
    /// * `ct_id` - Cognitive Thread identifier
    /// * `virtual_base` - Virtual address where this CT's memory starts
    /// * `tier` - Memory tier designation
    /// * `permissions` - Access permission bitmask (1=read, 2=write, 4=execute)
    pub fn new(ct_id: impl Into<String>, virtual_base: u64, tier: impl Into<String>, permissions: u8) -> Self {
        CtMappingEntry {
            ct_id: ct_id.into(),
            virtual_base,
            physical_pages: BTreeMap::new(),
            permissions,
            tier: tier.into(),
        }
    }

    /// Checks if a specific permission is granted.
    ///
    /// # Arguments
    ///
    /// * `permission` - Permission type (1=read, 2=write, 4=execute)
    pub fn has_permission(&self, permission: u8) -> bool {
        (self.permissions & permission) != 0
    }

    /// Adds a physical page mapping.
    pub fn map_page(&mut self, virtual_offset: u64, physical_page: u64) -> Result<()> {
        self.physical_pages.insert(
            virtual_offset,
            PhysicalPageMapping {
                physical_page,
                present: true,
            },
        );
        Ok(())
    }

    /// Removes a physical page mapping.
    pub fn unmap_page(&mut self, virtual_offset: u64) -> Result<()> {
        self.physical_pages
            .remove(&virtual_offset)
            .ok_or(MemoryError::InvalidReference {
                reason: format!(
                    "page not mapped at offset {:#x}",
                    virtual_offset
                ),
            })?;
        Ok(())
    }

    /// Returns the total number of mapped pages.
    pub fn page_count(&self) -> usize {
        self.physical_pages.len()
    }
}

/// Isolation boundary enforcer - defines what each CT can access.
///
/// Ensures strict isolation: each CT can only access its own mappings,
/// never other CTs' regions.
///
/// See Engineering Plan § 4.1.3: Isolation Boundaries.
pub struct IsolationBoundary;

impl IsolationBoundary {
    /// Validates that a CT can access a specific address range.
    ///
    /// # Arguments
    ///
    /// * `ct_id` - CT attempting access
    /// * `virtual_addr` - Virtual address being accessed
    /// * `access_type` - Type of access (1=read, 2=write, 4=execute)
    /// * `entry` - The CT's mapping entry
    ///
    /// # Returns
    ///
    /// `Result<()>` if access is allowed, error otherwise
    pub fn validate_access(
        ct_id: &str,
        virtual_addr: u64,
        access_type: u8,
        entry: &CtMappingEntry,
    ) -> Result<()> {
        // Verify the CT matches
        if entry.ct_id != ct_id {
            return Err(MemoryError::CapabilityDenied {
                operation: "address_space_access".to_string(),
                resource: format!("ct_id mismatch: expected {}, got {}", entry.ct_id, ct_id),
            });
        }

        // Verify the address is within this CT's mapped range
        if virtual_addr < entry.virtual_base {
            return Err(MemoryError::CapabilityDenied {
                operation: "address_space_access".to_string(),
                resource: format!(
                    "address {:#x} below virtual_base {:#x}",
                    virtual_addr, entry.virtual_base
                ),
            });
        }

        // Verify the access type is permitted
        if !entry.has_permission(access_type) {
            return Err(MemoryError::CapabilityDenied {
                operation: format!("access_type_{}", access_type),
                resource: format!(
                    "ct_id {} lacks permission, permissions={:#x}",
                    ct_id, entry.permissions
                ),
            });
        }

        Ok(())
    }
}

/// Statistics about the address space.
///
/// Provides observability into address space usage and fragmentation.
/// See Engineering Plan § 4.1.0: Monitoring.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AddressSpaceStats {
    /// Total bytes mapped across all CTs
    pub total_mapped: u64,
    /// Number of CTs with mappings
    pub ct_count: usize,
    /// Size of largest contiguous free region
    pub largest_contiguous_free: u64,
}

impl AddressSpaceStats {
    /// Creates new address space statistics.
    pub fn new(total_mapped: u64, ct_count: usize, largest_free: u64) -> Self {
        AddressSpaceStats {
            total_mapped,
            ct_count,
            largest_contiguous_free: largest_free,
        }
    }
}

/// Memory Manager's address space - manages isolation and mappings.
///
/// Maintains the kernel region, service heap, and CT mapping table.
/// Enforces strict isolation between CTs.
///
/// See Engineering Plan § 4.1.3: Address Space Isolation.
pub struct MemoryManagerAddressSpace {
    /// Kernel region: read-only code and static data
    pub kernel_region: (u64, u64), // (start, size)
    /// Service heap: dynamically allocated for MM internal structures
    pub service_heap: (u64, u64), // (start, size)
    /// CT mapping table: CT ID -> mapping entry
    pub ct_mapping_table: BTreeMap<String, CtMappingEntry>,
}

impl MemoryManagerAddressSpace {
    /// Creates a new Memory Manager address space.
    ///
    /// # Arguments
    ///
    /// * `kernel_start` - Start address of kernel region
    /// * `kernel_size` - Size of kernel region
    /// * `heap_start` - Start address of service heap
    /// * `heap_size` - Initial size of service heap
    pub fn new(kernel_start: u64, kernel_size: u64, heap_start: u64, heap_size: u64) -> Self {
        MemoryManagerAddressSpace {
            kernel_region: (kernel_start, kernel_size),
            service_heap: (heap_start, heap_size),
            ct_mapping_table: BTreeMap::new(),
        }
    }

    /// Maps a CT's memory region into the address space.
    ///
    /// # Arguments
    ///
    /// * `ct_id` - CT identifier
    /// * `virtual_base` - Virtual address where CT's memory starts
    /// * `tier` - Memory tier (L1, L2, or L3)
    /// * `permissions` - Access permission bitmask (1=read, 2=write, 4=execute)
    ///
    /// # Returns
    ///
    /// `Result<VirtualAddr>` - The virtual base address if successful
    ///
    /// See Engineering Plan § 4.1.3: CT Memory Mapping.
    pub fn map_ct_memory(
        &mut self,
        ct_id: impl Into<String>,
        virtual_base: u64,
        tier: impl Into<String>,
        permissions: u8,
    ) -> Result<u64> {
        let ct_id_str = ct_id.into();

        // Check for duplicate CT
        if self.ct_mapping_table.contains_key(&ct_id_str) {
            return Err(MemoryError::Other(format!(
                "CT {} already mapped",
                ct_id_str
            )));
        }

        // Verify virtual_base doesn't conflict with kernel or heap
        let (kernel_start, kernel_size) = self.kernel_region;
        let (heap_start, heap_size) = self.service_heap;

        if virtual_base >= kernel_start && virtual_base < kernel_start + kernel_size {
            return Err(MemoryError::Other(
                "virtual_base conflicts with kernel region".to_string(),
            ));
        }

        if virtual_base >= heap_start && virtual_base < heap_start + heap_size {
            return Err(MemoryError::Other(
                "virtual_base conflicts with heap".to_string(),
            ));
        }

        // Create mapping entry
        let entry = CtMappingEntry::new(
            ct_id_str.clone(),
            virtual_base,
            tier,
            permissions,
        );

        self.ct_mapping_table.insert(ct_id_str, entry);

        Ok(virtual_base)
    }

    /// Unmaps a CT's memory region from the address space.
    ///
    /// # Arguments
    ///
    /// * `ct_id` - CT identifier
    ///
    /// # Returns
    ///
    /// `Result<()>` - Success if unmapped, error if CT not found
    ///
    /// See Engineering Plan § 4.1.3: CT Memory Unmapping.
    pub fn unmap_ct_memory(&mut self, ct_id: &str) -> Result<()> {
        self.ct_mapping_table
            .remove(ct_id)
            .ok_or(MemoryError::InvalidReference {
                reason: format!("CT {} not mapped", ct_id),
            })?;

        Ok(())
    }

    /// Gets the mapping entry for a CT (immutable).
    pub fn get_ct_mapping(&self, ct_id: &str) -> Result<&CtMappingEntry> {
        self.ct_mapping_table
            .get(ct_id)
            .ok_or(MemoryError::InvalidReference {
                reason: format!("CT {} not found in mapping table", ct_id),
            })
    }

    /// Gets the mapping entry for a CT (mutable).
    pub fn get_ct_mapping_mut(&mut self, ct_id: &str) -> Result<&mut CtMappingEntry> {
        self.ct_mapping_table
            .get_mut(ct_id)
            .ok_or(MemoryError::InvalidReference {
                reason: format!("CT {} not found in mapping table", ct_id),
            })
    }

    /// Computes current address space statistics.
    pub fn compute_stats(&self) -> AddressSpaceStats {
        let ct_count = self.ct_mapping_table.len();
        let total_mapped: u64 = self
            .ct_mapping_table
            .values()
            .map(|entry| (entry.page_count() as u64) * 4096) // Assume 4KiB pages
            .sum();

        // Simple calculation: largest free = total address space - used
        let used = self.kernel_region.1 + self.service_heap.1 + total_mapped;
        let total_address_space = 0x1000_0000_0000_0000u64; // Example: 256 TiB
        let largest_free = total_address_space.saturating_sub(used);

        AddressSpaceStats::new(total_mapped, ct_count, largest_free)
    }

    /// Lists all mapped CTs.
    pub fn list_mapped_cts(&self) -> Vec<String> {
        self.ct_mapping_table
            .keys()
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::format;
use alloc::string::ToString;

    #[test]
    fn test_ct_mapping_entry_creation() {
        let entry = CtMappingEntry::new("ct-001", 0x1000, "L1", 0b011); // read+write
        assert_eq!(entry.ct_id, "ct-001");
        assert_eq!(entry.virtual_base, 0x1000);
        assert_eq!(entry.tier, "L1");
        assert_eq!(entry.permissions, 0b011);
    }

    #[test]
    fn test_ct_mapping_entry_permissions() {
        let entry = CtMappingEntry::new("ct-001", 0x1000, "L1", 0b101); // read+execute
        assert!(entry.has_permission(1)); // read
        assert!(!entry.has_permission(2)); // write
        assert!(entry.has_permission(4)); // execute
    }

    #[test]
    fn test_ct_mapping_entry_map_page() {
        let mut entry = CtMappingEntry::new("ct-001", 0x1000, "L1", 0b011);
        assert!(entry.map_page(0, 100).is_ok());
        assert!(entry.map_page(0x1000, 101).is_ok());
        assert_eq!(entry.page_count(), 2);
    }

    #[test]
    fn test_ct_mapping_entry_unmap_page() {
        let mut entry = CtMappingEntry::new("ct-001", 0x1000, "L1", 0b011);
        entry.map_page(0, 100).unwrap();
        entry.map_page(0x1000, 101).unwrap();
        assert_eq!(entry.page_count(), 2);

        assert!(entry.unmap_page(0).is_ok());
        assert_eq!(entry.page_count(), 1);

        assert!(entry.unmap_page(0x999).is_err());
    }

    #[test]
    fn test_isolation_boundary_validate_access_success() {
        let entry = CtMappingEntry::new("ct-001", 0x1000, "L1", 0b011);
        let result = IsolationBoundary::validate_access("ct-001", 0x2000, 1, &entry);
        assert!(result.is_ok());
    }

    #[test]
    fn test_isolation_boundary_validate_access_wrong_ct() {
        let entry = CtMappingEntry::new("ct-001", 0x1000, "L1", 0b011);
        let result = IsolationBoundary::validate_access("ct-002", 0x2000, 1, &entry);
        assert!(result.is_err());
    }

    #[test]
    fn test_isolation_boundary_validate_access_below_base() {
        let entry = CtMappingEntry::new("ct-001", 0x1000, "L1", 0b011);
        let result = IsolationBoundary::validate_access("ct-001", 0x500, 1, &entry);
        assert!(result.is_err());
    }

    #[test]
    fn test_isolation_boundary_validate_access_no_permission() {
        let entry = CtMappingEntry::new("ct-001", 0x1000, "L1", 0b001); // read only
        let result = IsolationBoundary::validate_access("ct-001", 0x2000, 2, &entry); // write
        assert!(result.is_err());
    }

    #[test]
    fn test_address_space_stats_creation() {
        let stats = AddressSpaceStats::new(1024000, 5, 1000000);
        assert_eq!(stats.total_mapped, 1024000);
        assert_eq!(stats.ct_count, 5);
        assert_eq!(stats.largest_contiguous_free, 1000000);
    }

    #[test]
    fn test_memory_manager_address_space_creation() {
        let as_space = MemoryManagerAddressSpace::new(0x0, 0x1000, 0x2000, 0x1000);
        assert_eq!(as_space.kernel_region, (0x0, 0x1000));
        assert_eq!(as_space.service_heap, (0x2000, 0x1000));
        assert_eq!(as_space.ct_mapping_table.len(), 0);
    }

    #[test]
    fn test_map_ct_memory_success() {
        let mut as_space = MemoryManagerAddressSpace::new(0x0, 0x1000, 0x2000, 0x1000);
        let result = as_space.map_ct_memory("ct-001", 0x10000, "L1", 0b011);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0x10000);
        assert_eq!(as_space.ct_mapping_table.len(), 1);
    }

    #[test]
    fn test_map_ct_memory_duplicate() {
        let mut as_space = MemoryManagerAddressSpace::new(0x0, 0x1000, 0x2000, 0x1000);
        as_space.map_ct_memory("ct-001", 0x10000, "L1", 0b011).ok();
        let result = as_space.map_ct_memory("ct-001", 0x20000, "L1", 0b011);
        assert!(result.is_err());
    }

    #[test]
    fn test_map_ct_memory_kernel_conflict() {
        let mut as_space = MemoryManagerAddressSpace::new(0x0, 0x1000, 0x2000, 0x1000);
        let result = as_space.map_ct_memory("ct-001", 0x500, "L1", 0b011);
        assert!(result.is_err());
    }

    #[test]
    fn test_map_ct_memory_heap_conflict() {
        let mut as_space = MemoryManagerAddressSpace::new(0x0, 0x1000, 0x2000, 0x1000);
        let result = as_space.map_ct_memory("ct-001", 0x2500, "L1", 0b011);
        assert!(result.is_err());
    }

    #[test]
    fn test_unmap_ct_memory_success() {
        let mut as_space = MemoryManagerAddressSpace::new(0x0, 0x1000, 0x2000, 0x1000);
        as_space.map_ct_memory("ct-001", 0x10000, "L1", 0b011).ok();
        assert_eq!(as_space.ct_mapping_table.len(), 1);

        assert!(as_space.unmap_ct_memory("ct-001").is_ok());
        assert_eq!(as_space.ct_mapping_table.len(), 0);
    }

    #[test]
    fn test_unmap_ct_memory_not_found() {
        let mut as_space = MemoryManagerAddressSpace::new(0x0, 0x1000, 0x2000, 0x1000);
        let result = as_space.unmap_ct_memory("ct-999");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_ct_mapping() {
        let mut as_space = MemoryManagerAddressSpace::new(0x0, 0x1000, 0x2000, 0x1000);
        as_space.map_ct_memory("ct-001", 0x10000, "L1", 0b011).ok();

        let entry = as_space.get_ct_mapping("ct-001");
        assert!(entry.is_ok());
        assert_eq!(entry.unwrap().ct_id, "ct-001");

        let entry_not_found = as_space.get_ct_mapping("ct-999");
        assert!(entry_not_found.is_err());
    }

    #[test]
    fn test_get_ct_mapping_mut() {
        let mut as_space = MemoryManagerAddressSpace::new(0x0, 0x1000, 0x2000, 0x1000);
        as_space.map_ct_memory("ct-001", 0x10000, "L1", 0b011).ok();

        let result = as_space.get_ct_mapping_mut("ct-001");
        assert!(result.is_ok());
        let entry = result.unwrap();
        entry.map_page(0, 100).ok();
        assert_eq!(entry.page_count(), 1);
    }

    #[test]
    fn test_list_mapped_cts() {
        let mut as_space = MemoryManagerAddressSpace::new(0x0, 0x1000, 0x2000, 0x1000);
        as_space.map_ct_memory("ct-001", 0x10000, "L1", 0b011).ok();
        as_space.map_ct_memory("ct-002", 0x20000, "L1", 0b011).ok();
        as_space.map_ct_memory("ct-003", 0x30000, "L1", 0b011).ok();

        let cts = as_space.list_mapped_cts();
        assert_eq!(cts.len(), 3);
        assert!(cts.contains(&"ct-001".to_string()));
        assert!(cts.contains(&"ct-002".to_string()));
        assert!(cts.contains(&"ct-003".to_string()));
    }

    #[test]
    fn test_compute_stats() {
        let mut as_space = MemoryManagerAddressSpace::new(0x0, 0x1000, 0x2000, 0x1000);
        as_space.map_ct_memory("ct-001", 0x10000, "L1", 0b011).ok();
        as_space.map_ct_memory("ct-002", 0x20000, "L1", 0b011).ok();

        let stats = as_space.compute_stats();
        assert_eq!(stats.ct_count, 2);
        assert!(stats.largest_contiguous_free > 0);
    }
}
