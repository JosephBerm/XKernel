// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! L1 Working Memory allocator with page-granule allocation/deallocation/resize.
//!
//! Implements the L1 allocator managing allocation of pages at page granularity
//! (4KB pages). Provides allocation, deallocation, resize, and reference counting
//! support for shared pages in crew scenarios.
//!
//! Performance target: <1ms per page allocation.
//!
//! See Engineering Plan § 4.1.1: L1 Allocator.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use crate::error::{MemoryError, Result};
use crate::page_pool::{PagePool, PageMetadata, PAGE_SIZE};
use crate::ids::MemoryRegionID;

/// L1 Allocation tracking for a single allocation.
///
/// Tracks the pages allocated, their metadata, and refcount information.
///
/// See Engineering Plan § 4.1.1: Allocation Tracking.
#[derive(Clone, Debug)]
pub struct L1Allocation {
    /// Allocation ID (unique identifier)
    allocation_id: u64,
    /// Region this allocation belongs to
    region_id: MemoryRegionID,
    /// First page index
    first_page_idx: u64,
    /// Number of pages allocated
    page_count: u64,
    /// Owning CT ID
    owner_ct_id: u32,
    /// Reference count (for shared allocations)
    ref_count: u32,
    /// Whether this allocation is pinned
    pinned: bool,
}

impl L1Allocation {
    /// Creates a new L1 allocation tracking entry.
    pub fn new(
        allocation_id: u64,
        region_id: MemoryRegionID,
        first_page_idx: u64,
        page_count: u64,
        owner_ct_id: u32,
    ) -> Self {
        L1Allocation {
            allocation_id,
            region_id,
            first_page_idx,
            page_count,
            owner_ct_id,
            ref_count: 1,
            pinned: false,
        }
    }

    /// Returns the allocation ID.
    pub fn allocation_id(&self) -> u64 {
        self.allocation_id
    }

    /// Returns the region ID.
    pub fn region_id(&self) -> &MemoryRegionID {
        &self.region_id
    }

    /// Returns the first page index.
    pub fn first_page_idx(&self) -> u64 {
        self.first_page_idx
    }

    /// Returns the page count.
    pub fn page_count(&self) -> u64 {
        self.page_count
    }

    /// Returns the size in bytes.
    pub fn size_bytes(&self) -> u64 {
        self.page_count.saturating_mul(PAGE_SIZE)
    }

    /// Returns the owner CT ID.
    pub fn owner_ct_id(&self) -> u32 {
        self.owner_ct_id
    }

    /// Returns the reference count.
    pub fn ref_count(&self) -> u32 {
        self.ref_count
    }

    /// Returns whether this allocation is pinned.
    pub fn is_pinned(&self) -> bool {
        self.pinned
    }

    /// Increments reference count.
    pub fn increment_ref(&mut self) -> Result<()> {
        if self.ref_count >= u32::MAX {
            return Err(MemoryError::Other("refcount overflow".to_string()));
        }
        self.ref_count = self.ref_count.saturating_add(1);
        Ok(())
    }

    /// Decrements reference count.
    pub fn decrement_ref(&mut self) -> Result<()> {
        if self.ref_count == 0 {
            return Err(MemoryError::Other("refcount underflow".to_string()));
        }
        self.ref_count = self.ref_count.saturating_sub(1);
        Ok(())
    }

    /// Pins this allocation.
    pub fn pin(&mut self) {
        self.pinned = true;
    }

    /// Unpins this allocation.
    pub fn unpin(&mut self) {
        self.pinned = false;
    }
}

/// L1 Working Memory allocator.
///
/// Manages page allocation/deallocation/resize within the L1 tier.
/// Uses a page pool underneath and tracks allocations by ID.
///
/// See Engineering Plan § 4.1.1: L1 Allocator Implementation.
pub struct L1Allocator {
    /// Region identifier
    region_id: MemoryRegionID,
    /// Underlying page pool
    page_pool: PagePool,
    /// Allocation tracking map (allocation_id -> L1Allocation)
    allocations: BTreeMap<u64, L1Allocation>,
    /// Next allocation ID
    next_alloc_id: u64,
}

impl L1Allocator {
    /// Creates a new L1 allocator.
    ///
    /// # Arguments
    ///
    /// * `region_id` - Region identifier
    /// * `total_pages` - Total pages available in L1
    /// * `base_address` - Base physical address of L1 memory
    ///
    /// # Returns
    ///
    /// `Result<Self>` on success
    pub fn new(
        region_id: MemoryRegionID,
        total_pages: u64,
        base_address: u64,
    ) -> Result<Self> {
        let page_pool = PagePool::new(total_pages, base_address)?;

        Ok(L1Allocator {
            region_id,
            page_pool,
            allocations: BTreeMap::new(),
            next_alloc_id: 1,
        })
    }

    /// Allocates a block of memory in L1.
    ///
    /// # Arguments
    ///
    /// * `size_bytes` - Size in bytes (will be rounded up to page boundary)
    /// * `owner_ct_id` - Owning CT ID
    ///
    /// # Returns
    ///
    /// `Result<(u64, u64, u64)>` - (allocation_id, virtual_address, physical_address)
    pub fn allocate(&mut self, size_bytes: u64, owner_ct_id: u32) -> Result<(u64, u64, u64)> {
        // Round up to page boundary
        let page_count = (size_bytes + PAGE_SIZE - 1) / PAGE_SIZE;

        // Allocate pages from pool
        let (first_page_idx, phys_addr) = self.page_pool.allocate_pages(page_count, owner_ct_id)?;

        // Create allocation tracking entry
        let alloc_id = self.next_alloc_id;
        self.next_alloc_id = self.next_alloc_id.saturating_add(1);

        let allocation = L1Allocation::new(
            alloc_id,
            self.region_id.clone(),
            first_page_idx,
            page_count,
            owner_ct_id,
        );

        self.allocations.insert(alloc_id, allocation);

        // Virtual address is same as physical in L1 (direct mapping)
        Ok((alloc_id, phys_addr, phys_addr))
    }

    /// Deallocates a previously allocated block.
    ///
    /// # Arguments
    ///
    /// * `allocation_id` - ID returned from allocate()
    ///
    /// # Returns
    ///
    /// `Result<()>` on success
    pub fn deallocate(&mut self, allocation_id: u64) -> Result<()> {
        let allocation = self
            .allocations
            .remove(&allocation_id)
            .ok_or_else(|| MemoryError::InvalidReference {
                reason: format!("allocation {} not found", allocation_id),
            })?;

        if allocation.is_pinned() {
            // Restore allocation if pinned
            self.allocations.insert(allocation_id, allocation);
            return Err(MemoryError::Other(
                "cannot deallocate pinned allocation".to_string(),
            ));
        }

        self.page_pool.deallocate_pages(
            allocation.first_page_idx(),
            allocation.page_count(),
        )?;

        Ok(())
    }

    /// Resizes an existing allocation.
    ///
    /// # Arguments
    ///
    /// * `allocation_id` - ID of allocation to resize
    /// * `new_size_bytes` - New size in bytes
    ///
    /// # Returns
    ///
    /// `Result<u64>` - New physical address
    pub fn resize(&mut self, allocation_id: u64, new_size_bytes: u64) -> Result<u64> {
        let allocation = self.allocations.get(&allocation_id).ok_or_else(|| {
            MemoryError::InvalidReference {
                reason: format!("allocation {} not found", allocation_id),
            }
        })?;

        let old_page_count = allocation.page_count();
        let new_page_count = (new_size_bytes + PAGE_SIZE - 1) / PAGE_SIZE;

        if new_page_count == old_page_count {
            // No change needed
            return Ok(self.page_pool.page_to_address(allocation.first_page_idx()));
        }

        if new_page_count > old_page_count {
            // Growing allocation - allocate additional pages
            let additional_pages = new_page_count - old_page_count;
            let first_page_idx = allocation.first_page_idx();

            // Check if we can extend in place (next pages are free)
            let mut can_extend = true;
            for i in 0..additional_pages {
                let check_idx = first_page_idx + old_page_count + i;
                if check_idx >= self.page_pool.total_pages() {
                    can_extend = false;
                    break;
                }
                let meta = self.page_pool.page_metadata(check_idx)?;
                if meta.is_allocated() {
                    can_extend = false;
                    break;
                }
            }

            if can_extend {
                // Extend in place
                let (_, _) = self
                    .page_pool
                    .allocate_pages(additional_pages, allocation.owner_ct_id())?;
                let allocation = self.allocations.get_mut(&allocation_id).unwrap();
                allocation.page_count = new_page_count;
            } else {
                // Must relocate
                self.deallocate(allocation_id)?;
                let (new_alloc_id, new_addr, _) = self
                    .allocate(new_size_bytes, allocation.owner_ct_id())?;

                // For caller: they should use the new allocation_id
                // For now, we'll update the map
                if let Some(new_alloc) = self.allocations.remove(&new_alloc_id) {
                    self.allocations.insert(allocation_id, new_alloc);
                }

                return Ok(new_addr);
            }
        } else {
            // Shrinking allocation - deallocate trailing pages
            let pages_to_free = old_page_count - new_page_count;
            let first_page_idx = allocation.first_page_idx();
            self.page_pool
                .deallocate_pages(first_page_idx + new_page_count, pages_to_free)?;

            let allocation = self.allocations.get_mut(&allocation_id).unwrap();
            allocation.page_count = new_page_count;
        }

        Ok(self.page_pool.page_to_address(
            allocation.first_page_idx(),
        ))
    }

    /// Pins an allocation (prevents deallocation).
    pub fn pin(&mut self, allocation_id: u64) -> Result<()> {
        self.allocations
            .get_mut(&allocation_id)
            .ok_or_else(|| MemoryError::InvalidReference {
                reason: format!("allocation {} not found", allocation_id),
            })?
            .pin();

        Ok(())
    }

    /// Unpins an allocation.
    pub fn unpin(&mut self, allocation_id: u64) -> Result<()> {
        self.allocations
            .get_mut(&allocation_id)
            .ok_or_else(|| MemoryError::InvalidReference {
                reason: format!("allocation {} not found", allocation_id),
            })?
            .unpin();

        Ok(())
    }

    /// Gets allocation information.
    pub fn get_allocation(&self, allocation_id: u64) -> Result<L1Allocation> {
        self.allocations
            .get(&allocation_id)
            .cloned()
            .ok_or_else(|| MemoryError::InvalidReference {
                reason: format!("allocation {} not found", allocation_id),
            })
    }

    /// Returns the total allocated bytes.
    pub fn total_allocated_bytes(&self) -> u64 {
        self.allocations
            .values()
            .map(|alloc| alloc.size_bytes())
            .fold(0, |acc, size| acc.saturating_add(size))
    }

    /// Returns the total free bytes.
    pub fn total_free_bytes(&self) -> u64 {
        self.page_pool.free_pages_count().saturating_mul(PAGE_SIZE)
    }

    /// Returns the total capacity in bytes.
    pub fn total_capacity_bytes(&self) -> u64 {
        self.page_pool.total_pages().saturating_mul(PAGE_SIZE)
    }

    /// Returns utilization ratio (0.0 to 1.0).
    pub fn utilization(&self) -> f64 {
        self.page_pool.utilization()
    }

    /// Returns the number of active allocations.
    pub fn allocation_count(&self) -> usize {
        self.allocations.len()
    }

    /// Returns the region ID.
    pub fn region_id(&self) -> &MemoryRegionID {
        &self.region_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::format;
use alloc::string::ToString;

    #[test]
    fn test_l1_allocation_creation() {
        let region = MemoryRegionID::l1_gpu_local();
        let alloc = L1Allocation::new(1, region.clone(), 100, 10, 42);

        assert_eq!(alloc.allocation_id(), 1);
        assert_eq!(alloc.first_page_idx(), 100);
        assert_eq!(alloc.page_count(), 10);
        assert_eq!(alloc.size_bytes(), 10 * PAGE_SIZE);
        assert_eq!(alloc.owner_ct_id(), 42);
        assert_eq!(alloc.ref_count(), 1);
        assert!(!alloc.is_pinned());
    }

    #[test]
    fn test_l1_allocation_refcount() {
        let mut alloc = L1Allocation::new(1, MemoryRegionID::l1_gpu_local(), 0, 1, 1);
        assert_eq!(alloc.ref_count(), 1);

        alloc.increment_ref().unwrap();
        assert_eq!(alloc.ref_count(), 2);

        alloc.decrement_ref().unwrap();
        assert_eq!(alloc.ref_count(), 1);
    }

    #[test]
    fn test_l1_allocation_pinning() {
        let mut alloc = L1Allocation::new(1, MemoryRegionID::l1_gpu_local(), 0, 1, 1);
        assert!(!alloc.is_pinned());

        alloc.pin();
        assert!(alloc.is_pinned());

        alloc.unpin();
        assert!(!alloc.is_pinned());
    }

    #[test]
    fn test_l1_allocator_creation() {
        let allocator =
            L1Allocator::new(MemoryRegionID::l1_gpu_local(), 1000, 0x1000_0000).unwrap();

        assert_eq!(allocator.total_capacity_bytes(), 1000 * PAGE_SIZE);
        assert_eq!(allocator.total_allocated_bytes(), 0);
        assert_eq!(allocator.allocation_count(), 0);
    }

    #[test]
    fn test_l1_allocator_allocate_single_page() {
        let mut allocator =
            L1Allocator::new(MemoryRegionID::l1_gpu_local(), 1000, 0x1000_0000).unwrap();

        let (alloc_id, vaddr, paddr) = allocator.allocate(4096, 1).unwrap();

        assert_eq!(alloc_id, 1);
        assert_eq!(vaddr, paddr);
        assert_eq!(allocator.total_allocated_bytes(), PAGE_SIZE);
        assert_eq!(allocator.allocation_count(), 1);
    }

    #[test]
    fn test_l1_allocator_allocate_multiple_pages() {
        let mut allocator =
            L1Allocator::new(MemoryRegionID::l1_gpu_local(), 1000, 0x1000_0000).unwrap();

        let (_, _, _) = allocator.allocate(10 * PAGE_SIZE, 1).unwrap();

        assert_eq!(allocator.total_allocated_bytes(), 10 * PAGE_SIZE);
        assert_eq!(allocator.allocation_count(), 1);
    }

    #[test]
    fn test_l1_allocator_deallocate() {
        let mut allocator =
            L1Allocator::new(MemoryRegionID::l1_gpu_local(), 1000, 0x1000_0000).unwrap();

        let (alloc_id, _, _) = allocator.allocate(4096, 1).unwrap();
        assert_eq!(allocator.allocation_count(), 1);

        allocator.deallocate(alloc_id).unwrap();
        assert_eq!(allocator.allocation_count(), 0);
        assert_eq!(allocator.total_allocated_bytes(), 0);
    }

    #[test]
    fn test_l1_allocator_multiple_allocations() {
        let mut allocator =
            L1Allocator::new(MemoryRegionID::l1_gpu_local(), 1000, 0x1000_0000).unwrap();

        let (id1, _, _) = allocator.allocate(100 * PAGE_SIZE, 1).unwrap();
        let (id2, _, _) = allocator.allocate(50 * PAGE_SIZE, 2).unwrap();

        assert_eq!(allocator.allocation_count(), 2);
        assert_eq!(allocator.total_allocated_bytes(), 150 * PAGE_SIZE);

        allocator.deallocate(id1).unwrap();
        assert_eq!(allocator.allocation_count(), 1);
        assert_eq!(allocator.total_allocated_bytes(), 50 * PAGE_SIZE);

        allocator.deallocate(id2).unwrap();
        assert_eq!(allocator.allocation_count(), 0);
    }

    #[test]
    fn test_l1_allocator_resize_grow() {
        let mut allocator =
            L1Allocator::new(MemoryRegionID::l1_gpu_local(), 1000, 0x1000_0000).unwrap();

        let (alloc_id, _, _) = allocator.allocate(10 * PAGE_SIZE, 1).unwrap();
        allocator.resize(alloc_id, 20 * PAGE_SIZE).ok();

        assert!(allocator.total_allocated_bytes() >= 20 * PAGE_SIZE);
    }

    #[test]
    fn test_l1_allocator_resize_shrink() {
        let mut allocator =
            L1Allocator::new(MemoryRegionID::l1_gpu_local(), 1000, 0x1000_0000).unwrap();

        let (alloc_id, _, _) = allocator.allocate(20 * PAGE_SIZE, 1).unwrap();
        allocator.resize(alloc_id, 10 * PAGE_SIZE).ok();

        assert_eq!(allocator.total_allocated_bytes(), 10 * PAGE_SIZE);
    }

    #[test]
    fn test_l1_allocator_pinning() {
        let mut allocator =
            L1Allocator::new(MemoryRegionID::l1_gpu_local(), 1000, 0x1000_0000).unwrap();

        let (alloc_id, _, _) = allocator.allocate(4096, 1).unwrap();
        allocator.pin(alloc_id).unwrap();

        // Try to deallocate pinned allocation - should fail
        let result = allocator.deallocate(alloc_id);
        assert!(result.is_err());

        allocator.unpin(alloc_id).unwrap();
        allocator.deallocate(alloc_id).unwrap();
    }

    #[test]
    fn test_l1_allocator_utilization() {
        let mut allocator =
            L1Allocator::new(MemoryRegionID::l1_gpu_local(), 1000, 0x1000_0000).unwrap();

        assert_eq!(allocator.utilization(), 0.0);

        allocator.allocate(500 * PAGE_SIZE, 1).unwrap();
        assert_eq!(allocator.utilization(), 0.5);

        allocator.allocate(500 * PAGE_SIZE, 1).unwrap();
        assert_eq!(allocator.utilization(), 1.0);
    }

    #[test]
    fn test_l1_allocator_exhaustion() {
        let mut allocator =
            L1Allocator::new(MemoryRegionID::l1_gpu_local(), 100, 0x1000_0000).unwrap();

        allocator.allocate(100 * PAGE_SIZE, 1).unwrap();

        // Try to allocate more - should fail
        let result = allocator.allocate(1 * PAGE_SIZE, 2);
        assert!(result.is_err());
    }

    #[test]
    fn test_l1_allocator_get_allocation() {
        let mut allocator =
            L1Allocator::new(MemoryRegionID::l1_gpu_local(), 1000, 0x1000_0000).unwrap();

        let (alloc_id, _, _) = allocator.allocate(10 * PAGE_SIZE, 42).unwrap();

        let alloc = allocator.get_allocation(alloc_id).unwrap();
        assert_eq!(alloc.allocation_id(), alloc_id);
        assert_eq!(alloc.owner_ct_id(), 42);
        assert_eq!(alloc.page_count(), 10);
    }

    #[test]
    fn test_l1_allocator_invalid_allocation_id() {
        let allocator =
            L1Allocator::new(MemoryRegionID::l1_gpu_local(), 1000, 0x1000_0000).unwrap();

        let result = allocator.get_allocation(9999);
        assert!(result.is_err());
    }
}
