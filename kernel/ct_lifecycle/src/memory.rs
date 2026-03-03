// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 XKernal Contributors
//! Memory management with arena allocator and memory pools

use alloc::vec;
use alloc::vec::Vec;
use thiserror::Error;

/// Memory management errors
#[derive(Debug, Clone, Error)]
pub enum MemoryError {
    /// Allocation failed
    #[error("allocation failed: {0}")]
    AllocationFailed(alloc::string::String),
    /// Invalid address
    #[error("invalid address: {0:x}")]
    InvalidAddress(usize),
    /// Out of memory
    #[error("out of memory")]
    OutOfMemory,
    /// Deallocation failed
    #[error("deallocation failed: {0}")]
    DeallocationFailed(alloc::string::String),
}

pub type Result<T> = core::result::Result<T, MemoryError>;

/// Represents a memory block in the arena
#[derive(Debug, Clone)]
struct MemoryBlock {
    address: usize,
    size: usize,
    is_free: bool,
}

impl MemoryBlock {
    fn new(address: usize, size: usize) -> Self {
        Self {
            address,
            size,
            is_free: true,
        }
    }

    fn is_occupied(&self) -> bool {
        !self.is_free
    }

    fn contains(&self, address: usize) -> bool {
        address >= self.address && address < self.address + self.size
    }
}

/// Arena allocator for fixed-size memory regions
#[derive(Debug)]
pub struct ArenaAllocator {
    arena: Vec<u8>,
    blocks: Vec<MemoryBlock>,
    base_address: usize,
}

impl ArenaAllocator {
    /// Create a new arena allocator with the specified size
    pub fn new(size: usize, base_address: usize) -> Self {
        let mut allocator = Self {
            arena: Vec::with_capacity(size),
            blocks: Vec::new(),
            base_address,
        };

        allocator.blocks.push(MemoryBlock::new(base_address, size));
        allocator
    }

    /// Allocate memory from the arena
    pub fn allocate(&mut self, size: usize) -> Result<usize> {
        for block in &mut self.blocks {
            if block.is_free && block.size >= size {
                block.is_free = false;
                let allocated_address = block.address;

                // Split block if necessary
                if block.size > size {
                    let remaining_size = block.size - size;
                    let remaining_address = block.address + size;
                    block.size = size;
                    self.blocks.push(MemoryBlock::new(remaining_address, remaining_size));
                }

                return Ok(allocated_address);
            }
        }

        Err(MemoryError::OutOfMemory)
    }

    /// Deallocate memory from the arena
    pub fn deallocate(&mut self, address: usize) -> Result<usize> {
        let mut freed_size = 0;

        // Find and mark block as free
        for block in &mut self.blocks {
            if block.address == address {
                block.is_free = true;
                freed_size = block.size;
                break;
            }
        }

        if freed_size == 0 {
            return Err(MemoryError::InvalidAddress(address));
        }

        // Coalesce adjacent free blocks
        self.coalesce();

        Ok(freed_size)
    }

    /// Coalesce adjacent free blocks
    fn coalesce(&mut self) {
        let mut i = 0;
        while i < self.blocks.len() - 1 {
            if self.blocks[i].is_free && self.blocks[i + 1].is_free {
                let next_block = self.blocks.remove(i + 1);
                self.blocks[i].size += next_block.size;
            } else {
                i += 1;
            }
        }
    }

    /// Get statistics about the arena
    pub fn stats(&self) -> ArenaStats {
        let total_size: usize = self.blocks.iter().map(|b| b.size).sum();
        let free_size: usize = self.blocks.iter().filter(|b| b.is_free).map(|b| b.size).sum();
        let used_size = total_size - free_size;

        ArenaStats {
            total_size,
            used_size,
            free_size,
            block_count: self.blocks.len(),
        }
    }

    /// Get the number of blocks
    pub fn block_count(&self) -> usize {
        self.blocks.len()
    }
}

/// Arena statistics
#[derive(Debug, Clone)]
pub struct ArenaStats {
    /// Total arena size
    pub total_size: usize,
    /// Currently used size
    pub used_size: usize,
    /// Currently free size
    pub free_size: usize,
    /// Number of blocks
    pub block_count: usize,
}

/// Memory pool for object allocation
#[derive(Debug)]
pub struct MemoryPool {
    object_size: usize,
    free_objects: Vec<usize>,
    allocated: Vec<bool>,
}

impl MemoryPool {
    /// Create a new memory pool with specified object size and capacity
    pub fn new(object_size: usize, capacity: usize) -> Self {
        let mut pool = Self {
            object_size,
            free_objects: Vec::with_capacity(capacity),
            allocated: vec![false; capacity],
        };

        for i in 0..capacity {
            pool.free_objects.push(i);
        }

        pool
    }

    /// Allocate an object from the pool
    pub fn allocate_object(&mut self) -> Result<usize> {
        self.free_objects
            .pop()
            .ok_or(MemoryError::OutOfMemory)
    }

    /// Return an object to the pool
    pub fn deallocate_object(&mut self, object_id: usize) -> Result<()> {
        if object_id >= self.allocated.len() {
            return Err(MemoryError::InvalidAddress(object_id));
        }

        if self.allocated[object_id] {
            self.allocated[object_id] = false;
            self.free_objects.push(object_id);
            Ok(())
        } else {
            Err(MemoryError::DeallocationFailed(
                alloc::format!("object {} already free", object_id),
            ))
        }
    }

    /// Get the object size
    pub fn object_size(&self) -> usize {
        self.object_size
    }

    /// Get the number of free objects
    pub fn free_count(&self) -> usize {
        self.free_objects.len()
    }

    /// Get the number of allocated objects
    pub fn allocated_count(&self) -> usize {
        self.allocated.iter().filter(|&&a| a).count()
    }

    /// Get the total capacity
    pub fn capacity(&self) -> usize {
        self.allocated.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arena_allocate() {
        let mut arena = ArenaAllocator::new(1024, 0x1000);
        let addr = arena.allocate(256).unwrap();
        assert_eq!(addr, 0x1000);
    }

    #[test]
    fn test_arena_coalesce() {
        let mut arena = ArenaAllocator::new(1024, 0x1000);
        let a1 = arena.allocate(256).unwrap();
        let a2 = arena.allocate(256).unwrap();
        arena.deallocate(a1).unwrap();
        arena.deallocate(a2).unwrap();
        assert_eq!(arena.block_count(), 1);
    }

    #[test]
    fn test_memory_pool() {
        let mut pool = MemoryPool::new(64, 10);
        let obj = pool.allocate_object().unwrap();
        assert!(obj < 10);
        pool.deallocate_object(obj).unwrap();
    }
}
