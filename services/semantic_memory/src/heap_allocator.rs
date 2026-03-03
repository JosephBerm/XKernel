// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Basic heap allocator for Memory Manager's own data structures.
//!
//! Provides a simple bump allocator for the Memory Manager service's internal
//! data structures. Phase 0: No fragmentation management, no defragmentation.
//!
//! See Engineering Plan § 4.1.0: Heap Allocation.

use crate::error::{MemoryError, Result};

/// Simple bump allocator for Memory Manager internals.
///
/// Allocates memory in a linear fashion, advancing a pointer.
/// No deallocation or fragmentation management in Phase 0.
///
/// See Engineering Plan § 4.1.0: Heap Allocator.
pub struct HeapAllocator {
    /// Base address of the heap
    base_address: u64,
    /// Current position (allocation pointer)
    current_offset: u64,
    /// Total heap size
    total_size: u64,
}

impl HeapAllocator {
    /// Creates a new heap allocator.
    ///
    /// # Arguments
    ///
    /// * `base_address` - Starting address of the heap
    /// * `total_size` - Total heap size in bytes
    pub fn new(base_address: u64, total_size: u64) -> Self {
        HeapAllocator {
            base_address,
            current_offset: 0,
            total_size,
        }
    }

    /// Allocates memory from the heap.
    ///
    /// # Arguments
    ///
    /// * `size` - Bytes to allocate
    /// * `alignment` - Alignment requirement (must be power of 2)
    ///
    /// # Returns
    ///
    /// `Result<u64>` - Allocated address
    pub fn allocate(&mut self, size: u64, alignment: u64) -> Result<u64> {
        // Validate alignment is power of 2
        if alignment == 0 || (alignment & (alignment - 1)) != 0 {
            return Err(MemoryError::Other(
                "alignment must be power of 2".to_string(),
            ));
        }

        // Align current offset
        let aligned_offset = (self.current_offset + alignment - 1) & !(alignment - 1);

        // Check if we have space
        if aligned_offset.saturating_add(size) > self.total_size {
            return Err(MemoryError::AllocationFailed {
                requested: size,
                available: self.total_size.saturating_sub(aligned_offset),
            });
        }

        let address = self.base_address.saturating_add(aligned_offset);
        self.current_offset = aligned_offset.saturating_add(size);

        Ok(address)
    }

    /// Allocates with 8-byte alignment (common default).
    pub fn allocate_aligned8(&mut self, size: u64) -> Result<u64> {
        self.allocate(size, 8)
    }

    /// Allocates with 16-byte alignment (SIMD-friendly).
    pub fn allocate_aligned16(&mut self, size: u64) -> Result<u64> {
        self.allocate(size, 16)
    }

    /// Allocates with no alignment requirements.
    pub fn allocate_unaligned(&mut self, size: u64) -> Result<u64> {
        self.allocate(size, 1)
    }

    /// Returns the current free space.
    pub fn free_space(&self) -> u64 {
        self.total_size.saturating_sub(self.current_offset)
    }

    /// Returns the utilization ratio (0.0 to 1.0).
    pub fn utilization(&self) -> f64 {
        if self.total_size == 0 {
            0.0
        } else {
            self.current_offset as f64 / self.total_size as f64
        }
    }

    /// Returns the current offset.
    pub fn current_offset(&self) -> u64 {
        self.current_offset
    }

    /// Returns the total heap size.
    pub fn total_size(&self) -> u64 {
        self.total_size
    }

    /// Returns the base address.
    pub fn base_address(&self) -> u64 {
        self.base_address
    }

    /// Resets the allocator (clears all allocations).
    /// Use with caution - invalidates all previous pointers.
    pub fn reset(&mut self) {
        self.current_offset = 0;
    }
}

/// Statistics about heap usage.
#[derive(Clone, Debug)]
pub struct HeapStats {
    /// Total heap size
    pub total_size: u64,
    /// Allocated bytes
    pub allocated_bytes: u64,
    /// Free bytes
    pub free_bytes: u64,
    /// Utilization ratio (0.0-1.0)
    pub utilization: f64,
}

impl HeapStats {
    /// Creates stats from a heap allocator.
    pub fn from_allocator(allocator: &HeapAllocator) -> Self {
        let allocated_bytes = allocator.current_offset();
        let total_size = allocator.total_size();
        let free_bytes = allocator.free_space();

        HeapStats {
            total_size,
            allocated_bytes,
            free_bytes,
            utilization: allocator.utilization(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::string::ToString;

    #[test]
    fn test_heap_allocator_creation() {
        let allocator = HeapAllocator::new(0x1000_0000, 1024 * 1024);
        assert_eq!(allocator.base_address(), 0x1000_0000);
        assert_eq!(allocator.total_size(), 1024 * 1024);
        assert_eq!(allocator.current_offset(), 0);
        assert_eq!(allocator.free_space(), 1024 * 1024);
    }

    #[test]
    fn test_heap_allocator_single_allocation() {
        let mut allocator = HeapAllocator::new(0x1000_0000, 1024 * 1024);
        let addr = allocator.allocate(256, 1).unwrap();

        assert_eq!(addr, 0x1000_0000);
        assert_eq!(allocator.current_offset(), 256);
        assert_eq!(allocator.free_space(), 1024 * 1024 - 256);
    }

    #[test]
    fn test_heap_allocator_multiple_allocations() {
        let mut allocator = HeapAllocator::new(0x1000_0000, 1024 * 1024);

        let addr1 = allocator.allocate(256, 1).unwrap();
        let addr2 = allocator.allocate(512, 1).unwrap();

        assert_eq!(addr1, 0x1000_0000);
        assert_eq!(addr2, 0x1000_0100);
        assert_eq!(allocator.current_offset(), 768);
    }

    #[test]
    fn test_heap_allocator_alignment() {
        let mut allocator = HeapAllocator::new(0x1000_0000, 1024 * 1024);

        let addr1 = allocator.allocate(5, 1).unwrap();
        let addr2 = allocator.allocate(100, 8).unwrap();

        assert_eq!(addr1, 0x1000_0000);
        // addr2 should be 8-byte aligned
        assert_eq!(addr2 & 0x7, 0);
        assert!(addr2 > addr1.saturating_add(5));
    }

    #[test]
    fn test_heap_allocator_aligned8() {
        let mut allocator = HeapAllocator::new(0x1000_0000, 1024 * 1024);
        let addr = allocator.allocate_aligned8(256).unwrap();

        assert_eq!(addr & 0x7, 0); // 8-byte aligned
    }

    #[test]
    fn test_heap_allocator_aligned16() {
        let mut allocator = HeapAllocator::new(0x1000_0000, 1024 * 1024);
        let addr = allocator.allocate_aligned16(256).unwrap();

        assert_eq!(addr & 0xF, 0); // 16-byte aligned
    }

    #[test]
    fn test_heap_allocator_exhaustion() {
        let mut allocator = HeapAllocator::new(0x1000_0000, 256);

        // Allocate entire heap
        allocator.allocate(256, 1).unwrap();

        // Next allocation should fail
        let result = allocator.allocate(1, 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_heap_allocator_invalid_alignment() {
        let mut allocator = HeapAllocator::new(0x1000_0000, 1024 * 1024);

        let result = allocator.allocate(256, 3); // Not power of 2
        assert!(result.is_err());

        let result = allocator.allocate(256, 0); // Zero alignment
        assert!(result.is_err());
    }

    #[test]
    fn test_heap_allocator_utilization() {
        let mut allocator = HeapAllocator::new(0x1000_0000, 1000);

        assert_eq!(allocator.utilization(), 0.0);

        allocator.allocate(500, 1).unwrap();
        assert_eq!(allocator.utilization(), 0.5);

        allocator.allocate(500, 1).unwrap();
        assert_eq!(allocator.utilization(), 1.0);
    }

    #[test]
    fn test_heap_allocator_reset() {
        let mut allocator = HeapAllocator::new(0x1000_0000, 1024 * 1024);

        allocator.allocate(256, 1).unwrap();
        allocator.allocate(512, 1).unwrap();

        assert_eq!(allocator.current_offset(), 768);

        allocator.reset();

        assert_eq!(allocator.current_offset(), 0);
        assert_eq!(allocator.free_space(), 1024 * 1024);
    }

    #[test]
    fn test_heap_stats() {
        let mut allocator = HeapAllocator::new(0x1000_0000, 1000);
        allocator.allocate(400, 1).unwrap();

        let stats = HeapStats::from_allocator(&allocator);

        assert_eq!(stats.total_size, 1000);
        assert_eq!(stats.allocated_bytes, 400);
        assert_eq!(stats.free_bytes, 600);
        assert_eq!(stats.utilization, 0.4);
    }

    #[test]
    fn test_heap_allocator_large_allocation() {
        let mut allocator = HeapAllocator::new(0x1000_0000, 1024 * 1024 * 1024);

        let addr = allocator.allocate(256 * 1024 * 1024, 1).unwrap();
        assert_eq!(addr, 0x1000_0000);
    }

    #[test]
    fn test_heap_allocator_unaligned() {
        let mut allocator = HeapAllocator::new(0x1000_0000, 1024 * 1024);
        
        let addr1 = allocator.allocate_unaligned(7).unwrap();
        let addr2 = allocator.allocate_unaligned(3).unwrap();

        assert_eq!(addr1, 0x1000_0000);
        assert_eq!(addr2, 0x1000_0007);
    }
}
