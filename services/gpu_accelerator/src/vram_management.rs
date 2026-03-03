// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! VRAM management: pool, isolation, and KV-cache handling.

use alloc::vec::Vec;
use core::fmt;

/// VRAM pool for memory allocation
#[derive(Debug, Clone)]
pub struct VramPool {
    pub device_id: u32,
    pub total_bytes: u64,
    pub allocated_bytes: u64,
    allocations: Vec<VramAllocation>,
}

#[derive(Debug, Clone, Copy)]
pub struct VramAllocation {
    pub alloc_id: u64,
    pub offset: u64,
    pub size: u64,
}

impl VramPool {
    pub fn new(device_id: u32, total_bytes: u64) -> Self {
        Self {
            device_id,
            total_bytes,
            allocated_bytes: 0,
            allocations: Vec::new(),
        }
    }

    /// Allocate VRAM memory
    pub fn allocate(&mut self, size: u64) -> Result<VramAllocation, VramError> {
        if self.allocated_bytes + size > self.total_bytes {
            return Err(VramError::OutOfMemory {
                requested: size,
                available: self.total_bytes - self.allocated_bytes,
            });
        }

        let offset = self.allocated_bytes;
        let alloc_id = self.allocations.len() as u64;

        let alloc = VramAllocation {
            alloc_id,
            offset,
            size,
        };

        self.allocated_bytes += size;
        self.allocations.push(alloc);

        Ok(alloc)
    }

    /// Free a VRAM allocation
    pub fn free(&mut self, alloc_id: u64) -> Result<(), VramError> {
        let idx = self
            .allocations
            .iter()
            .position(|a| a.alloc_id == alloc_id)
            .ok_or(VramError::AllocationNotFound)?;

        let alloc = self.allocations.remove(idx);
        self.allocated_bytes = self.allocated_bytes.saturating_sub(alloc.size);
        Ok(())
    }

    pub fn utilization(&self) -> f64 {
        if self.total_bytes == 0 {
            0.0
        } else {
            self.allocated_bytes as f64 / self.total_bytes as f64
        }
    }

    pub fn free_bytes(&self) -> u64 {
        self.total_bytes.saturating_sub(self.allocated_bytes)
    }

    pub fn allocation_count(&self) -> usize {
        self.allocations.len()
    }
}

impl Default for VramPool {
    fn default() -> Self {
        Self::new(0, 16_000_000_000) // 16GB default
    }
}

/// Isolation level for VRAM regions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsolationLevel {
    /// No isolation (shared access)
    None,
    /// Logical isolation (separate allocations)
    Logical,
    /// Physical isolation (separate memory pages)
    Physical,
}

/// Isolated VRAM region for crew isolation
#[derive(Debug, Clone)]
pub struct IsolatedVramRegion {
    pub region_id: u64,
    pub crew_id: u64,
    pub pool: VramPool,
    pub isolation_level: IsolationLevel,
}

impl IsolatedVramRegion {
    pub fn new(
        region_id: u64,
        crew_id: u64,
        device_id: u32,
        size: u64,
        isolation_level: IsolationLevel,
    ) -> Self {
        Self {
            region_id,
            crew_id,
            pool: VramPool::new(device_id, size),
            isolation_level,
        }
    }

    pub fn is_physically_isolated(&self) -> bool {
        self.isolation_level == IsolationLevel::Physical
    }
}

/// KV-cache block for transformer attention
#[derive(Debug, Clone, Copy)]
pub struct KvcacheBlock {
    pub block_id: u64,
    pub offset: u64,
    pub capacity_tokens: u32,
    pub token_count: u32,
    pub allocated: bool,
}

impl KvcacheBlock {
    pub fn new(block_id: u64, offset: u64, capacity_tokens: u32) -> Self {
        Self {
            block_id,
            offset,
            capacity_tokens,
            token_count: 0,
            allocated: false,
        }
    }

    pub fn available_tokens(&self) -> u32 {
        self.capacity_tokens.saturating_sub(self.token_count)
    }

    pub fn is_full(&self) -> bool {
        self.token_count >= self.capacity_tokens
    }

    pub fn utilization(&self) -> f64 {
        if self.capacity_tokens == 0 {
            0.0
        } else {
            self.token_count as f64 / self.capacity_tokens as f64
        }
    }
}

/// KV-cache allocator
#[derive(Debug, Clone)]
pub struct KvcacheAllocator {
    blocks: Vec<KvcacheBlock>,
    max_blocks: usize,
}

impl KvcacheAllocator {
    pub fn new(max_blocks: usize) -> Self {
        Self {
            blocks: Vec::new(),
            max_blocks,
        }
    }

    /// Create a new KV-cache block
    pub fn create_block(&mut self, capacity_tokens: u32) -> Result<KvcacheBlock, VramError> {
        if self.blocks.len() >= self.max_blocks {
            return Err(VramError::KvcacheExhausted {
                max_blocks: self.max_blocks,
            });
        }

        let block_id = self.blocks.len() as u64;
        let offset = self.blocks.iter().map(|b| b.capacity_tokens).sum::<u32>() as u64;

        let block = KvcacheBlock::new(block_id, offset, capacity_tokens);
        self.blocks.push(block);

        Ok(block)
    }

    /// Allocate tokens in a block
    pub fn allocate_tokens(
        &mut self,
        block_id: u64,
        tokens: u32,
    ) -> Result<(), VramError> {
        let block = self
            .blocks
            .iter_mut()
            .find(|b| b.block_id == block_id)
            .ok_or(VramError::KvcacheNotFound)?;

        if block.token_count + tokens > block.capacity_tokens {
            return Err(VramError::KvcacheFull {
                block_id,
                available: block.available_tokens(),
                requested: tokens,
            });
        }

        block.token_count += tokens;
        Ok(())
    }

    pub fn block_count(&self) -> usize {
        self.blocks.len()
    }

    pub fn total_tokens_allocated(&self) -> u32 {
        self.blocks.iter().map(|b| b.token_count).sum()
    }

    pub fn total_capacity(&self) -> u32 {
        self.blocks.iter().map(|b| b.capacity_tokens).sum()
    }

    pub fn utilization(&self) -> f64 {
        let capacity = self.total_capacity();
        if capacity == 0 {
            0.0
        } else {
            self.total_tokens_allocated() as f64 / capacity as f64
        }
    }
}

impl Default for KvcacheAllocator {
    fn default() -> Self {
        Self::new(128) // 128 blocks
    }
}

/// VRAM errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VramError {
    OutOfMemory { requested: u64, available: u64 },
    AllocationNotFound,
    KvcacheExhausted { max_blocks: usize },
    KvcacheNotFound,
    KvcacheFull {
        block_id: u64,
        available: u32,
        requested: u32,
    },
}

impl fmt::Display for VramError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutOfMemory { requested, available } => {
                write!(f, "VRAM out of memory: requested {}, available {}", requested, available)
            }
            Self::AllocationNotFound => write!(f, "VRAM allocation not found"),
            Self::KvcacheExhausted { max_blocks } => {
                write!(f, "KV-cache exhausted (max blocks: {})", max_blocks)
            }
            Self::KvcacheNotFound => write!(f, "KV-cache block not found"),
            Self::KvcacheFull {
                block_id,
                available,
                requested,
            } => {
                write!(
                    f,
                    "KV-cache block {} full: available {}, requested {}",
                    block_id, available, requested
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vram_pool() {
        let mut pool = VramPool::new(0, 1_000_000);
        let alloc = pool.allocate(100_000).unwrap();
        assert_eq!(alloc.size, 100_000);
        assert_eq!(pool.allocated_bytes, 100_000);

        pool.free(alloc.alloc_id).unwrap();
        assert_eq!(pool.allocated_bytes, 0);
    }

    #[test]
    fn test_vram_out_of_memory() {
        let mut pool = VramPool::new(0, 100);
        pool.allocate(50).unwrap();
        assert!(matches!(
            pool.allocate(100),
            Err(VramError::OutOfMemory { .. })
        ));
    }

    #[test]
    fn test_isolated_vram() {
        let region = IsolatedVramRegion::new(1, 1, 0, 1_000_000, IsolationLevel::Physical);
        assert!(region.is_physically_isolated());
    }

    #[test]
    fn test_kvcache_allocator() {
        let mut alloc = KvcacheAllocator::new(10);
        let block = alloc.create_block(256).unwrap();
        assert_eq!(block.capacity_tokens, 256);

        alloc.allocate_tokens(block.block_id, 100).unwrap();
        assert_eq!(block.block_id, 0);
    }

    #[test]
    fn test_kvcache_full() {
        let mut alloc = KvcacheAllocator::new(10);
        let block = alloc.create_block(100).unwrap();

        alloc.allocate_tokens(block.block_id, 100).unwrap();

        assert!(matches!(
            alloc.allocate_tokens(block.block_id, 1),
            Err(VramError::KvcacheFull { .. })
        ));
    }
}
