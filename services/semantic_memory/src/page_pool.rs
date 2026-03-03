// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Simple page pool management with free list and allocation bitmap.
//!
//! This module implements a basic page pool allocator for L1 Working Memory,
//! supporting allocation, deallocation, and page metadata tracking.
//! Phase 0: No eviction, no migration, stub implementation.
//!
//! See Engineering Plan § 4.1.1: Page Pool Management.

use alloc::vec::Vec;
use crate::error::{MemoryError, Result};

/// Page size constant (4KB pages).
pub const PAGE_SIZE: u64 = 4096;

/// Page metadata tracking allocation status and refcount.
///
/// Tracks whether a page is free, allocated, or pinned.
/// Supports reference counting for shared pages in crew scenarios.
///
/// See Engineering Plan § 4.1.1: Page Metadata.
#[derive(Clone, Debug, Copy, PartialEq, Eq)]
pub struct PageMetadata {
    /// Whether this page is allocated
    allocated: bool,
    /// Reference count (for shared pages)
    ref_count: u32,
    /// Whether this page is pinned (cannot be evicted)
    pinned: bool,
    /// Owner CT ID (for per-thread tracking)
    owner_ct_id: u32,
}

impl PageMetadata {
    /// Creates a new free page metadata.
    pub fn new() -> Self {
        PageMetadata {
            allocated: false,
            ref_count: 0,
            pinned: false,
            owner_ct_id: u32::MAX, // Unowned
        }
    }

    /// Creates allocated page metadata for a specific owner.
    pub fn allocated(owner_ct_id: u32) -> Self {
        PageMetadata {
            allocated: true,
            ref_count: 1,
            pinned: false,
            owner_ct_id,
        }
    }

    /// Creates a pinned page metadata.
    pub fn pinned(owner_ct_id: u32) -> Self {
        PageMetadata {
            allocated: true,
            ref_count: 1,
            pinned: true,
            owner_ct_id,
        }
    }

    /// Marks this page as free.
    pub fn free(&mut self) {
        self.allocated = false;
        self.ref_count = 0;
        self.owner_ct_id = u32::MAX;
    }

    /// Increments the reference count (for shared access).
    pub fn increment_refcount(&mut self) -> Result<()> {
        if self.ref_count >= u32::MAX {
            return Err(MemoryError::Other("refcount overflow".to_string()));
        }
        self.ref_count = self.ref_count.saturating_add(1);
        Ok(())
    }

    /// Decrements the reference count.
    pub fn decrement_refcount(&mut self) -> Result<()> {
        if self.ref_count == 0 {
            return Err(MemoryError::Other("cannot decrement refcount below 0".to_string()));
        }
        self.ref_count = self.ref_count.saturating_sub(1);
        Ok(())
    }

    /// Returns true if this page is allocated.
    pub fn is_allocated(&self) -> bool {
        self.allocated
    }

    /// Returns true if this page is pinned.
    pub fn is_pinned(&self) -> bool {
        self.pinned
    }

    /// Returns the reference count.
    pub fn ref_count(&self) -> u32 {
        self.ref_count
    }

    /// Returns the owner CT ID.
    pub fn owner_ct_id(&self) -> u32 {
        self.owner_ct_id
    }
}

impl Default for PageMetadata {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple page pool allocator.
///
/// Manages a pool of 4KB pages with free list and allocation bitmap.
/// Supports allocation, deallocation, and resize operations.
///
/// See Engineering Plan § 4.1.1: Page Pool Allocator.
pub struct PagePool {
    /// Total number of pages in this pool
    total_pages: u64,
    /// Metadata for each page
    metadata: Vec<PageMetadata>,
    /// Free page indices (free list)
    free_pages: Vec<u64>,
    /// Base address of the pool in memory
    base_address: u64,
}

impl PagePool {
    /// Creates a new page pool.
    ///
    /// # Arguments
    ///
    /// * `total_pages` - Number of pages to allocate in this pool
    /// * `base_address` - Starting address of the pool
    ///
    /// # Returns
    ///
    /// `Result<Self>` with initialized pool
    pub fn new(total_pages: u64, base_address: u64) -> Result<Self> {
        if total_pages == 0 {
            return Err(MemoryError::Other("pool size must be > 0".to_string()));
        }

        // Initialize metadata for all pages
        let metadata = alloc::vec![PageMetadata::new(); total_pages as usize];

        // Initialize free list with all page indices
        let mut free_pages = Vec::with_capacity(total_pages as usize);
        for i in 0..total_pages {
            free_pages.push(i);
        }

        Ok(PagePool {
            total_pages,
            metadata,
            free_pages,
            base_address,
        })
    }

    /// Allocates a single page.
    ///
    /// # Arguments
    ///
    /// * `owner_ct_id` - CT ID owning this page
    ///
    /// # Returns
    ///
    /// `Result<(u64, u64)>` - (page_index, physical_address)
    pub fn allocate_page(&mut self, owner_ct_id: u32) -> Result<(u64, u64)> {
        if let Some(page_idx) = self.free_pages.pop() {
            self.metadata[page_idx as usize] = PageMetadata::allocated(owner_ct_id);
            let phys_addr = self.base_address.saturating_add(page_idx.saturating_mul(PAGE_SIZE));
            Ok((page_idx, phys_addr))
        } else {
            Err(MemoryError::AllocationFailed {
                requested: PAGE_SIZE,
                available: 0,
            })
        }
    }

    /// Allocates multiple contiguous pages.
    ///
    /// # Arguments
    ///
    /// * `page_count` - Number of contiguous pages to allocate
    /// * `owner_ct_id` - CT ID owning these pages
    ///
    /// # Returns
    ///
    /// `Result<(u64, u64)>` - (first_page_index, starting_physical_address)
    pub fn allocate_pages(&mut self, page_count: u64, owner_ct_id: u32) -> Result<(u64, u64)> {
        if page_count == 0 {
            return Err(MemoryError::Other("page_count must be > 0".to_string()));
        }

        // Find contiguous free pages
        let mut allocated_pages = Vec::new();
        for _ in 0..page_count {
            if let Some(page_idx) = self.free_pages.pop() {
                allocated_pages.push(page_idx);
            } else {
                // Rollback: return allocated pages to free list
                for idx in allocated_pages {
                    self.free_pages.push(idx);
                    self.metadata[idx as usize].free();
                }
                return Err(MemoryError::AllocationFailed {
                    requested: page_count.saturating_mul(PAGE_SIZE),
                    available: self.free_pages.len() as u64 * PAGE_SIZE,
                });
            }
        }

        // Mark as allocated
        let first_page_idx = allocated_pages[0];
        for page_idx in allocated_pages {
            self.metadata[page_idx as usize] = PageMetadata::allocated(owner_ct_id);
        }

        let phys_addr = self
            .base_address
            .saturating_add(first_page_idx.saturating_mul(PAGE_SIZE));
        Ok((first_page_idx, phys_addr))
    }

    /// Deallocates a single page.
    ///
    /// # Arguments
    ///
    /// * `page_idx` - Index of the page to deallocate
    ///
    /// # Returns
    ///
    /// `Result<()>` on success
    pub fn deallocate_page(&mut self, page_idx: u64) -> Result<()> {
        if page_idx >= self.total_pages {
            return Err(MemoryError::InvalidReference {
                reason: format!("page_idx {} out of range", page_idx),
            });
        }

        let metadata = &mut self.metadata[page_idx as usize];

        if !metadata.is_allocated() {
            return Err(MemoryError::Other("page already free".to_string()));
        }

        if metadata.is_pinned() {
            return Err(MemoryError::Other("cannot deallocate pinned page".to_string()));
        }

        metadata.free();
        self.free_pages.push(page_idx);

        Ok(())
    }

    /// Deallocates multiple pages.
    ///
    /// # Arguments
    ///
    /// * `first_page_idx` - Index of first page
    /// * `page_count` - Number of pages to deallocate
    ///
    /// # Returns
    ///
    /// `Result<()>` on success
    pub fn deallocate_pages(&mut self, first_page_idx: u64, page_count: u64) -> Result<()> {
        if page_count == 0 {
            return Err(MemoryError::Other("page_count must be > 0".to_string()));
        }

        for i in 0..page_count {
            let page_idx = first_page_idx.saturating_add(i);
            if page_idx >= self.total_pages {
                return Err(MemoryError::InvalidReference {
                    reason: format!("page_idx {} out of range", page_idx),
                });
            }

            let metadata = &mut self.metadata[page_idx as usize];
            if !metadata.is_allocated() {
                return Err(MemoryError::Other(format!("page {} already free", page_idx)));
            }

            if metadata.is_pinned() {
                return Err(MemoryError::Other(format!("page {} is pinned", page_idx)));
            }

            metadata.free();
            self.free_pages.push(page_idx);
        }

        Ok(())
    }

    /// Gets the metadata for a page.
    pub fn page_metadata(&self, page_idx: u64) -> Result<PageMetadata> {
        if page_idx >= self.total_pages {
            return Err(MemoryError::InvalidReference {
                reason: format!("page_idx {} out of range", page_idx),
            });
        }
        Ok(self.metadata[page_idx as usize])
    }

    /// Pins a page (prevents eviction).
    pub fn pin_page(&mut self, page_idx: u64) -> Result<()> {
        if page_idx >= self.total_pages {
            return Err(MemoryError::InvalidReference {
                reason: format!("page_idx {} out of range", page_idx),
            });
        }

        self.metadata[page_idx as usize].pinned = true;
        Ok(())
    }

    /// Unpins a page.
    pub fn unpin_page(&mut self, page_idx: u64) -> Result<()> {
        if page_idx >= self.total_pages {
            return Err(MemoryError::InvalidReference {
                reason: format!("page_idx {} out of range", page_idx),
            });
        }

        self.metadata[page_idx as usize].pinned = false;
        Ok(())
    }

    /// Returns the total number of pages in this pool.
    pub fn total_pages(&self) -> u64 {
        self.total_pages
    }

    /// Returns the number of free pages.
    pub fn free_pages_count(&self) -> u64 {
        self.free_pages.len() as u64
    }

    /// Returns the number of allocated pages.
    pub fn allocated_pages_count(&self) -> u64 {
        self.total_pages.saturating_sub(self.free_pages.len() as u64)
    }

    /// Returns utilization ratio (0.0 to 1.0).
    pub fn utilization(&self) -> f64 {
        if self.total_pages == 0 {
            0.0
        } else {
            self.allocated_pages_count() as f64 / self.total_pages as f64
        }
    }

    /// Returns the base address of this pool.
    pub fn base_address(&self) -> u64 {
        self.base_address
    }

    /// Converts a page index to a physical address.
    pub fn page_to_address(&self, page_idx: u64) -> u64 {
        self.base_address
            .saturating_add(page_idx.saturating_mul(PAGE_SIZE))
    }

    /// Converts a physical address to a page index.
    pub fn address_to_page(&self, address: u64) -> Result<u64> {
        if address < self.base_address {
            return Err(MemoryError::InvalidReference {
                reason: "address below pool base".to_string(),
            });
        }

        let offset = address.saturating_sub(self.base_address);
        if offset % PAGE_SIZE != 0 {
            return Err(MemoryError::InvalidReference {
                reason: "address not page-aligned".to_string(),
            });
        }

        let page_idx = offset / PAGE_SIZE;
        if page_idx >= self.total_pages {
            return Err(MemoryError::InvalidReference {
                reason: "address above pool end".to_string(),
            });
        }

        Ok(page_idx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::format;
use alloc::string::ToString;
use alloc::vec;

    #[test]
    fn test_page_metadata_new() {
        let meta = PageMetadata::new();
        assert!(!meta.is_allocated());
        assert_eq!(meta.ref_count(), 0);
        assert!(!meta.is_pinned());
    }

    #[test]
    fn test_page_metadata_allocated() {
        let meta = PageMetadata::allocated(42);
        assert!(meta.is_allocated());
        assert_eq!(meta.ref_count(), 1);
        assert_eq!(meta.owner_ct_id(), 42);
    }

    #[test]
    fn test_page_metadata_pinned() {
        let meta = PageMetadata::pinned(42);
        assert!(meta.is_allocated());
        assert!(meta.is_pinned());
        assert_eq!(meta.owner_ct_id(), 42);
    }

    #[test]
    fn test_page_metadata_refcount() {
        let mut meta = PageMetadata::allocated(42);
        assert_eq!(meta.ref_count(), 1);

        meta.increment_refcount().unwrap();
        assert_eq!(meta.ref_count(), 2);

        meta.decrement_refcount().unwrap();
        assert_eq!(meta.ref_count(), 1);
    }

    #[test]
    fn test_page_pool_creation() {
        let pool = PagePool::new(1024, 0x1000_0000).unwrap();
        assert_eq!(pool.total_pages(), 1024);
        assert_eq!(pool.free_pages_count(), 1024);
        assert_eq!(pool.allocated_pages_count(), 0);
    }

    #[test]
    fn test_page_pool_allocate_single() {
        let mut pool = PagePool::new(100, 0x1000_0000).unwrap();
        let (page_idx, phys_addr) = pool.allocate_page(1).unwrap();

        assert_eq!(page_idx, 99); // Last in the list (stack behavior)
        assert_eq!(phys_addr, 0x1000_0000 + 99 * PAGE_SIZE);
        assert_eq!(pool.free_pages_count(), 99);
        assert_eq!(pool.allocated_pages_count(), 1);
    }

    #[test]
    fn test_page_pool_allocate_multiple() {
        let mut pool = PagePool::new(1000, 0x1000_0000).unwrap();
        let (first_idx, first_addr) = pool.allocate_pages(10, 1).unwrap();

        assert_eq!(pool.allocated_pages_count(), 10);
        assert_eq!(pool.free_pages_count(), 990);
        assert_eq!(first_addr, 0x1000_0000 + first_idx * PAGE_SIZE);
    }

    #[test]
    fn test_page_pool_deallocate_single() {
        let mut pool = PagePool::new(100, 0x1000_0000).unwrap();
        let (page_idx, _) = pool.allocate_page(1).unwrap();

        pool.deallocate_page(page_idx).unwrap();
        assert_eq!(pool.allocated_pages_count(), 0);
        assert_eq!(pool.free_pages_count(), 100);
    }

    #[test]
    fn test_page_pool_deallocate_multiple() {
        let mut pool = PagePool::new(1000, 0x1000_0000).unwrap();
        let (first_idx, _) = pool.allocate_pages(10, 1).unwrap();

        pool.deallocate_pages(first_idx, 10).unwrap();
        assert_eq!(pool.allocated_pages_count(), 0);
        assert_eq!(pool.free_pages_count(), 1000);
    }

    #[test]
    fn test_page_pool_allocation_failure() {
        let mut pool = PagePool::new(10, 0x1000_0000).unwrap();
        
        // Allocate all pages
        let _ = pool.allocate_pages(10, 1);
        
        // Try to allocate more - should fail
        let result = pool.allocate_page(2);
        assert!(result.is_err());
    }

    #[test]
    fn test_page_pool_double_free() {
        let mut pool = PagePool::new(100, 0x1000_0000).unwrap();
        let (page_idx, _) = pool.allocate_page(1).unwrap();

        pool.deallocate_page(page_idx).unwrap();
        // Try to free again
        let result = pool.deallocate_page(page_idx);
        assert!(result.is_err());
    }

    #[test]
    fn test_page_pool_pin_unpin() {
        let mut pool = PagePool::new(100, 0x1000_0000).unwrap();
        let (page_idx, _) = pool.allocate_page(1).unwrap();

        pool.pin_page(page_idx).unwrap();
        
        // Try to deallocate pinned page - should fail
        let result = pool.deallocate_page(page_idx);
        assert!(result.is_err());

        pool.unpin_page(page_idx).unwrap();
        pool.deallocate_page(page_idx).unwrap();
    }

    #[test]
    fn test_page_pool_utilization() {
        let mut pool = PagePool::new(1000, 0x1000_0000).unwrap();
        assert_eq!(pool.utilization(), 0.0);

        pool.allocate_pages(500, 1).unwrap();
        assert_eq!(pool.utilization(), 0.5);

        pool.allocate_pages(500, 1).unwrap();
        assert_eq!(pool.utilization(), 1.0);
    }

    #[test]
    fn test_page_pool_address_conversion() {
        let pool = PagePool::new(100, 0x1000_0000).unwrap();

        // Convert page index to address
        let addr = pool.page_to_address(10);
        assert_eq!(addr, 0x1000_0000 + 10 * PAGE_SIZE);

        // Convert address back to page index
        let page_idx = pool.address_to_page(addr).unwrap();
        assert_eq!(page_idx, 10);
    }

    #[test]
    fn test_page_pool_invalid_address() {
        let pool = PagePool::new(100, 0x1000_0000).unwrap();

        // Address too low
        let result = pool.address_to_page(0x0000_0000);
        assert!(result.is_err());

        // Address not aligned
        let result = pool.address_to_page(0x1000_0001);
        assert!(result.is_err());

        // Address too high
        let result = pool.address_to_page(0x1000_0000 + 100 * PAGE_SIZE);
        assert!(result.is_err());
    }

    #[test]
    fn test_page_pool_zero_pages() {
        let result = PagePool::new(0, 0x1000_0000);
        assert!(result.is_err());
    }

    #[test]
    fn test_page_pool_metadata_access() {
        let mut pool = PagePool::new(100, 0x1000_0000).unwrap();
        let (page_idx, _) = pool.allocate_page(42).unwrap();

        let meta = pool.page_metadata(page_idx).unwrap();
        assert!(meta.is_allocated());
        assert_eq!(meta.owner_ct_id(), 42);
    }
}
