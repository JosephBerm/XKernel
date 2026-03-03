// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Memory allocation strategies: arena, pool, and compactor patterns.

use alloc::vec::Vec;
use core::fmt;

/// Arena allocator for batch allocations
#[derive(Debug, Clone)]
pub struct ArenaAllocator {
    /// Total capacity in bytes
    capacity: u64,
    /// Currently allocated bytes
    allocated: u64,
    /// Allocation blocks with their sizes
    blocks: Vec<(u64, u64)>, // (offset, size)
}

impl ArenaAllocator {
    pub fn new(capacity: u64) -> Self {
        Self {
            capacity,
            allocated: 0,
            blocks: Vec::new(),
        }
    }

    /// Allocate a block of given size
    pub fn allocate(&mut self, size: u64) -> Result<u64, AllocationError> {
        if self.allocated + size > self.capacity {
            return Err(AllocationError::OutOfMemory {
                requested: size,
                available: self.capacity - self.allocated,
            });
        }

        let offset = self.allocated;
        self.allocated += size;
        self.blocks.push((offset, size));

        Ok(offset)
    }

    /// Free all allocations (reset arena)
    pub fn reset(&mut self) {
        self.allocated = 0;
        self.blocks.clear();
    }

    /// Get utilization percentage
    pub fn utilization(&self) -> f64 {
        if self.capacity == 0 {
            0.0
        } else {
            self.allocated as f64 / self.capacity as f64
        }
    }

    pub fn free_space(&self) -> u64 {
        self.capacity.saturating_sub(self.allocated)
    }
}

impl Default for ArenaAllocator {
    fn default() -> Self {
        Self::new(1_000_000)
    }
}

/// Object pool for frequent allocations/deallocations
#[derive(Debug, Clone)]
pub struct MemoryPool {
    /// Free objects ready to reuse
    free_list: Vec<u64>,
    /// Allocated objects
    allocated: Vec<u64>,
    /// Object size in bytes
    object_size: u64,
    /// Maximum pool size
    max_objects: usize,
}

impl MemoryPool {
    pub fn new(object_size: u64, max_objects: usize) -> Self {
        Self {
            free_list: Vec::with_capacity(max_objects),
            allocated: Vec::new(),
            object_size,
            max_objects,
        }
    }

    /// Acquire an object from the pool
    pub fn acquire(&mut self) -> Result<u64, AllocationError> {
        if let Some(obj_id) = self.free_list.pop() {
            // Reuse from free list
            self.allocated.push(obj_id);
            Ok(obj_id)
        } else if self.allocated.len() < self.max_objects {
            // Create new object
            let obj_id = self.allocated.len() as u64;
            self.allocated.push(obj_id);
            Ok(obj_id)
        } else {
            Err(AllocationError::PoolExhausted {
                max_size: self.max_objects,
            })
        }
    }

    /// Release an object back to the pool
    pub fn release(&mut self, obj_id: u64) {
        self.allocated.retain(|&id| id != obj_id);
        if self.free_list.len() < self.max_objects {
            self.free_list.push(obj_id);
        }
    }

    pub fn available(&self) -> usize {
        self.free_list.len()
    }

    pub fn allocated_count(&self) -> usize {
        self.allocated.len()
    }

    pub fn utilization(&self) -> f64 {
        if self.max_objects == 0 {
            0.0
        } else {
            self.allocated_count() as f64 / self.max_objects as f64
        }
    }
}

impl Default for MemoryPool {
    fn default() -> Self {
        Self::new(4096, 1000)
    }
}

/// Compactor for defragmentation
#[derive(Debug, Clone)]
pub struct MemoryCompactor {
    /// Total moved bytes in current session
    bytes_moved: u64,
    /// Fragmentation ratio before compaction
    fragmentation_threshold: f64,
    /// Whether compaction is in progress
    in_progress: bool,
}

impl MemoryCompactor {
    pub fn new(fragmentation_threshold: f64) -> Self {
        Self {
            bytes_moved: 0,
            fragmentation_threshold,
            in_progress: false,
        }
    }

    /// Check if compaction is needed
    pub fn should_compact(&self, fragmented_bytes: u64, total_bytes: u64) -> bool {
        if total_bytes == 0 {
            return false;
        }
        let fragmentation_ratio = fragmented_bytes as f64 / total_bytes as f64;
        fragmentation_ratio > self.fragmentation_threshold
    }

    /// Start compaction pass
    pub fn start_compact(&mut self) -> Result<(), CompactionError> {
        if self.in_progress {
            return Err(CompactionError::AlreadyInProgress);
        }
        self.in_progress = true;
        self.bytes_moved = 0;
        Ok(())
    }

    /// Record bytes moved during compaction
    pub fn record_move(&mut self, bytes: u64) {
        self.bytes_moved += bytes;
    }

    /// Finish compaction pass
    pub fn finish_compact(&mut self) {
        self.in_progress = false;
    }

    pub fn total_moved(&self) -> u64 {
        self.bytes_moved
    }
}

impl Default for MemoryCompactor {
    fn default() -> Self {
        Self::new(0.3) // Trigger at 30% fragmentation
    }
}

/// Allocation error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllocationError {
    OutOfMemory { requested: u64, available: u64 },
    PoolExhausted { max_size: usize },
    InvalidSize,
    AlignmentError,
}

impl fmt::Display for AllocationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutOfMemory { requested, available } => {
                write!(
                    f,
                    "out of memory: requested {}, available {}",
                    requested, available
                )
            }
            Self::PoolExhausted { max_size } => {
                write!(f, "pool exhausted (max size: {})", max_size)
            }
            Self::InvalidSize => write!(f, "invalid allocation size"),
            Self::AlignmentError => write!(f, "alignment error"),
        }
    }
}

/// Compaction error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompactionError {
    AlreadyInProgress,
    NotInProgress,
    CompactionFailed,
}

impl fmt::Display for CompactionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyInProgress => write!(f, "compaction already in progress"),
            Self::NotInProgress => write!(f, "no compaction in progress"),
            Self::CompactionFailed => write!(f, "compaction failed"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arena_allocator() {
        let mut arena = ArenaAllocator::new(1000);
        let offset1 = arena.allocate(100).unwrap();
        let offset2 = arena.allocate(200).unwrap();

        assert_eq!(offset1, 0);
        assert_eq!(offset2, 100);
        assert_eq!(arena.allocated, 300);
    }

    #[test]
    fn test_arena_out_of_memory() {
        let mut arena = ArenaAllocator::new(100);
        assert!(arena.allocate(50).is_ok());
        assert!(matches!(
            arena.allocate(100),
            Err(AllocationError::OutOfMemory { .. })
        ));
    }

    #[test]
    fn test_memory_pool() {
        let mut pool = MemoryPool::new(4096, 3);
        let obj1 = pool.acquire().unwrap();
        let obj2 = pool.acquire().unwrap();

        assert_eq!(pool.allocated_count(), 2);

        pool.release(obj1);
        assert_eq!(pool.available(), 1);

        let obj3 = pool.acquire().unwrap();
        assert_eq!(obj3, obj1); // Reused
    }

    #[test]
    fn test_pool_exhausted() {
        let mut pool = MemoryPool::new(4096, 2);
        pool.acquire().unwrap();
        pool.acquire().unwrap();

        assert!(matches!(
            pool.acquire(),
            Err(AllocationError::PoolExhausted { .. })
        ));
    }

    #[test]
    fn test_compactor() {
        let mut comp = MemoryCompactor::new(0.5);

        // Low fragmentation
        assert!(!comp.should_compact(100, 1000));

        // High fragmentation
        assert!(comp.should_compact(600, 1000));

        assert!(comp.start_compact().is_ok());
        comp.record_move(500);
        assert_eq!(comp.total_moved(), 500);
        comp.finish_compact();
    }
}
