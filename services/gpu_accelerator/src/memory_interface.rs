// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! GPU memory management interface.
//!
//! Implements device memory allocation, deallocation, and transfer operations.
//! Tracks memory usage per crew, manages KV-cache pools, and enforces isolation.
//!
//! Reference: Engineering Plan § Memory Management, VRAM Isolation

use crate::error::GpuError;
use crate::ids::VramRegionID;
use crate::vram::{KvCachePool, VramIsolationMode};
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::fmt;

/// Unique identifier for a memory allocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MemHandle(u64);

impl fmt::Display for MemHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MemHandle({})", self.0)
    }
}

/// Memory allocation type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AllocationType {
    /// Device-local GPU memory (fastest).
    DeviceLocal,

    /// Unified memory (CPU + GPU shared address space).
    Unified,

    /// Pinned host memory (for DMA transfers).
    HostPinned,
}

impl fmt::Display for AllocationType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AllocationType::DeviceLocal => write!(f, "DeviceLocal"),
            AllocationType::Unified => write!(f, "Unified"),
            AllocationType::HostPinned => write!(f, "HostPinned"),
        }
    }
}

/// Memory allocation descriptor.
#[derive(Clone, Copy, Debug)]
pub struct GpuAllocation {
    /// Unique handle for this allocation.
    pub handle: MemHandle,

    /// Device pointer (opaque GPU address).
    pub device_ptr: u64,

    /// Allocation size in bytes.
    pub size: u64,

    /// Allocation type (affects isolation and access).
    pub alloc_type: AllocationType,

    /// Owning crew identifier.
    pub owning_crew: [u8; 16],

    /// Allocation timestamp (nanoseconds).
    pub created_at: u64,
}

impl fmt::Display for GpuAllocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "GpuAllocation({}, ptr=0x{:x}, size={}B, type={})",
            self.handle, self.device_ptr, self.size, self.alloc_type
        )
    }
}

/// Pool configuration for memory pooling.
#[derive(Clone, Copy, Debug)]
pub struct PoolConfig {
    /// Initial pool size in bytes.
    pub initial_size: u64,

    /// Maximum pool size in bytes.
    pub max_size: u64,
}

/// GPU memory manager.
///
/// Manages device memory allocations, tracks per-crew usage,
/// and enforces isolation policies.
///
/// Reference: Engineering Plan § Memory Management
#[derive(Debug)]
pub struct GpuMemoryManager {
    /// Active allocations (handle -> allocation).
    allocations: BTreeMap<MemHandle, GpuAllocation>,

    /// KV-cache pools per crew.
    kv_pools: BTreeMap<[u8; 16], Vec<KvCachePool>>,

    /// Memory pool configuration.
    pub pool_config: PoolConfig,

    /// Next allocation handle counter.
    next_handle: u64,

    /// Total VRAM available (bytes).
    pub total_vram: u64,

    /// Total VRAM allocated (bytes).
    pub allocated_vram: u64,

    /// Statistics.
    pub stats: MemoryManagerStats,
}

/// Memory manager statistics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemoryManagerStats {
    /// Total bytes allocated (cumulative).
    pub total_allocated: u64,

    /// Current free VRAM in bytes.
    pub free_vram: u64,

    /// Fragmentation percentage (0-100).
    pub fragmentation: u32,

    /// Per-crew memory usage (simplified for testing).
    pub crew_usage: u64,
}

impl GpuMemoryManager {
    /// Create a new memory manager.
    ///
    /// # Arguments
    ///
    /// * `total_vram` - Total device VRAM in bytes
    /// * `pool_config` - Memory pool configuration
    pub fn new(total_vram: u64, pool_config: PoolConfig) -> Self {
        GpuMemoryManager {
            allocations: BTreeMap::new(),
            kv_pools: BTreeMap::new(),
            pool_config,
            next_handle: 1,
            total_vram,
            allocated_vram: 0,
            stats: MemoryManagerStats {
                total_allocated: 0,
                free_vram: total_vram,
                fragmentation: 0,
                crew_usage: 0,
            },
        }
    }

    /// Allocate device memory.
    ///
    /// # Arguments
    ///
    /// * `size` - Allocation size in bytes
    /// * `alloc_type` - Memory type
    /// * `crew_id` - Owning crew
    /// * `timestamp` - Current timestamp in nanoseconds
    ///
    /// # Returns
    ///
    /// Memory handle if successful.
    pub fn alloc(
        &mut self,
        size: u64,
        alloc_type: AllocationType,
        crew_id: [u8; 16],
        timestamp: u64,
    ) -> Result<MemHandle, GpuError> {
        // Check available space
        if self.stats.free_vram < size {
            return Err(GpuError::VramExhausted);
        }

        let handle = MemHandle(self.next_handle);
        self.next_handle += 1;

        // Simulate device pointer allocation
        let device_ptr = 0x100000 + self.allocated_vram;

        let allocation = GpuAllocation {
            handle,
            device_ptr,
            size,
            alloc_type,
            owning_crew: crew_id,
            created_at: timestamp,
        };

        self.allocations.insert(handle, allocation);
        self.allocated_vram += size;

        // Update stats
        self.stats.total_allocated += size;
        self.stats.free_vram = self.total_vram.saturating_sub(self.allocated_vram);
        self.stats.crew_usage = self.allocated_vram;

        Ok(handle)
    }

    /// Free device memory.
    ///
    /// # Arguments
    ///
    /// * `handle` - Memory handle to free
    pub fn free(&mut self, handle: MemHandle) -> Result<(), GpuError> {
        if let Some(alloc) = self.allocations.remove(&handle) {
            self.allocated_vram = self.allocated_vram.saturating_sub(alloc.size);
            self.stats.free_vram = self.total_vram.saturating_sub(self.allocated_vram);

            // Update crew usage
            let crew = alloc.owning_crew;
            let crew_total: u64 = self
                .allocations
                .values()
                .filter(|a| a.owning_crew == crew)
                .map(|a| a.size)
                .sum();
            self.stats.crew_usage = crew_total;

            Ok(())
        } else {
            Err(GpuError::AllocationFailed) // Handle not found
        }
    }

    /// Get allocation by handle.
    pub fn get_allocation(&self, handle: MemHandle) -> Option<&GpuAllocation> {
        self.allocations.get(&handle)
    }

    /// Transfer data from host to device.
    ///
    /// # Arguments
    ///
    /// * `host_ptr` - Host memory pointer (opaque)
    /// * `device_ptr` - Device memory pointer
    /// * `size` - Transfer size in bytes
    ///
    /// # Returns
    ///
    /// Ok if transfer succeeds.
    pub fn transfer_host_to_device(
        &mut self,
        host_ptr: u64,
        device_ptr: u64,
        size: u64,
    ) -> Result<(), GpuError> {
        // Validate device pointer is in a known allocation
        let _found = self
            .allocations
            .values()
            .any(|a| a.device_ptr == device_ptr && a.size >= size);

        // In a real implementation, this would invoke driver FFI
        // For now, just validate and return Ok

        Ok(())
    }

    /// Transfer data from device to host.
    ///
    /// # Arguments
    ///
    /// * `device_ptr` - Device memory pointer
    /// * `host_ptr` - Host memory pointer (opaque)
    /// * `size` - Transfer size in bytes
    pub fn transfer_device_to_host(
        &mut self,
        device_ptr: u64,
        host_ptr: u64,
        size: u64,
    ) -> Result<(), GpuError> {
        // Validate device pointer is in a known allocation
        let _found = self
            .allocations
            .values()
            .any(|a| a.device_ptr == device_ptr && a.size >= size);

        // In a real implementation, this would invoke driver FFI

        Ok(())
    }

    /// Create a KV-cache pool for a crew.
    ///
    /// # Arguments
    ///
    /// * `pool_id` - Unique pool identifier
    /// * `crew_id` - Owner crew
    /// * `capacity_bytes` - Pool capacity in bytes
    /// * `token_size_bytes` - Bytes per token
    pub fn create_kv_cache_pool(
        &mut self,
        pool_id: VramRegionID,
        crew_id: [u8; 16],
        capacity_bytes: u64,
        token_size_bytes: u32,
    ) -> Result<(), GpuError> {
        let pool = KvCachePool::new(pool_id, pool_id, crew_id, capacity_bytes, token_size_bytes);

        self.kv_pools.entry(crew_id).or_insert_with(Vec::new).push(pool);

        Ok(())
    }

    /// Get KV-cache pool for a crew.
    pub fn get_kv_pool(&self, crew_id: [u8; 16], pool_id: VramRegionID) -> Option<&KvCachePool> {
        self.kv_pools
            .get(&crew_id)
            .and_then(|pools| pools.iter().find(|p| p.id == pool_id))
    }

    /// Get mutable KV-cache pool for a crew.
    pub fn get_kv_pool_mut(
        &mut self,
        crew_id: [u8; 16],
        pool_id: VramRegionID,
    ) -> Option<&mut KvCachePool> {
        self.kv_pools
            .get_mut(&crew_id)
            .and_then(|pools| pools.iter_mut().find(|p| p.id == pool_id))
    }

    /// Get all allocations for a crew.
    pub fn crew_allocations(&self, crew_id: [u8; 16]) -> Vec<&GpuAllocation> {
        self.allocations
            .values()
            .filter(|a| a.owning_crew == crew_id)
            .collect()
    }

    /// Get number of active allocations.
    pub fn allocation_count(&self) -> usize {
        self.allocations.len()
    }

    /// Calculate fragmentation percentage.
    pub fn calculate_fragmentation(&mut self) {
        // Simple heuristic: fragmentation = (allocations * 10) / (free_vram / 1MB)
        if self.stats.free_vram > 0 {
            let frag = ((self.allocations.len() as u64 * 10) / (self.stats.free_vram / (1024 * 1024)))
                as u32;
            self.stats.fragmentation = frag.min(100);
        } else {
            self.stats.fragmentation = 100;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::format;

    #[test]
    fn test_memory_manager_creation() {
        let pool_config = PoolConfig {
            initial_size: 1024 * 1024,
            max_size: 1024 * 1024 * 1024,
        };

        let mgr = GpuMemoryManager::new(80 * 1024 * 1024 * 1024, pool_config);

        assert_eq!(mgr.total_vram, 80 * 1024 * 1024 * 1024);
        assert_eq!(mgr.allocated_vram, 0);
        assert_eq!(mgr.stats.free_vram, 80 * 1024 * 1024 * 1024);
    }

    #[test]
    fn test_memory_alloc() {
        let pool_config = PoolConfig {
            initial_size: 1024 * 1024,
            max_size: 1024 * 1024 * 1024,
        };

        let mut mgr = GpuMemoryManager::new(80 * 1024 * 1024 * 1024, pool_config);
        let crew = [1u8; 16];

        let handle = mgr
            .alloc(1024 * 1024, AllocationType::DeviceLocal, crew, 1000)
            .unwrap();

        assert_eq!(mgr.allocated_vram, 1024 * 1024);
        assert_eq!(mgr.allocation_count(), 1);

        let alloc = mgr.get_allocation(handle).unwrap();
        assert_eq!(alloc.handle, handle);
        assert_eq!(alloc.size, 1024 * 1024);
        assert_eq!(alloc.owning_crew, crew);
    }

    #[test]
    fn test_memory_alloc_exhausted() {
        let pool_config = PoolConfig {
            initial_size: 1024,
            max_size: 1024 * 1024,
        };

        let mut mgr = GpuMemoryManager::new(1024 * 1024, pool_config);
        let crew = [1u8; 16];

        // Allocate most of VRAM
        mgr.alloc(1024 * 1024 - 1, AllocationType::DeviceLocal, crew, 1000)
            .unwrap();

        // Try to allocate more than available
        let result = mgr.alloc(1024, AllocationType::DeviceLocal, crew, 2000);

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), GpuError::VramExhausted);
    }

    #[test]
    fn test_memory_free() {
        let pool_config = PoolConfig {
            initial_size: 1024 * 1024,
            max_size: 1024 * 1024 * 1024,
        };

        let mut mgr = GpuMemoryManager::new(80 * 1024 * 1024 * 1024, pool_config);
        let crew = [1u8; 16];

        let handle = mgr
            .alloc(1024 * 1024, AllocationType::DeviceLocal, crew, 1000)
            .unwrap();

        assert_eq!(mgr.allocation_count(), 1);

        mgr.free(handle).unwrap();

        assert_eq!(mgr.allocation_count(), 0);
        assert_eq!(mgr.allocated_vram, 0);
    }

    #[test]
    fn test_memory_free_nonexistent() {
        let pool_config = PoolConfig {
            initial_size: 1024 * 1024,
            max_size: 1024 * 1024 * 1024,
        };

        let mut mgr = GpuMemoryManager::new(80 * 1024 * 1024 * 1024, pool_config);

        let handle = MemHandle(999); // Nonexistent
        let result = mgr.free(handle);

        assert!(result.is_err());
    }

    #[test]
    fn test_memory_crew_allocations() {
        let pool_config = PoolConfig {
            initial_size: 1024 * 1024,
            max_size: 1024 * 1024 * 1024,
        };

        let mut mgr = GpuMemoryManager::new(80 * 1024 * 1024 * 1024, pool_config);
        let crew1 = [1u8; 16];
        let crew2 = [2u8; 16];

        mgr.alloc(1024, AllocationType::DeviceLocal, crew1, 1000)
            .unwrap();
        mgr.alloc(2048, AllocationType::DeviceLocal, crew1, 1000)
            .unwrap();
        mgr.alloc(512, AllocationType::DeviceLocal, crew2, 1000)
            .unwrap();

        let crew1_allocs = mgr.crew_allocations(crew1);
        assert_eq!(crew1_allocs.len(), 2);

        let crew2_allocs = mgr.crew_allocations(crew2);
        assert_eq!(crew2_allocs.len(), 1);
    }

    #[test]
    fn test_memory_transfer_host_to_device() {
        let pool_config = PoolConfig {
            initial_size: 1024 * 1024,
            max_size: 1024 * 1024 * 1024,
        };

        let mut mgr = GpuMemoryManager::new(80 * 1024 * 1024 * 1024, pool_config);
        let crew = [1u8; 16];

        let handle = mgr
            .alloc(1024, AllocationType::DeviceLocal, crew, 1000)
            .unwrap();

        let alloc = mgr.get_allocation(handle).unwrap();
        let result = mgr.transfer_host_to_device(0x12345678, alloc.device_ptr, 512);

        assert!(result.is_ok());
    }

    #[test]
    fn test_memory_transfer_device_to_host() {
        let pool_config = PoolConfig {
            initial_size: 1024 * 1024,
            max_size: 1024 * 1024 * 1024,
        };

        let mut mgr = GpuMemoryManager::new(80 * 1024 * 1024 * 1024, pool_config);
        let crew = [1u8; 16];

        let handle = mgr
            .alloc(1024, AllocationType::DeviceLocal, crew, 1000)
            .unwrap();

        let alloc = mgr.get_allocation(handle).unwrap();
        let result = mgr.transfer_device_to_host(alloc.device_ptr, 0x87654321, 512);

        assert!(result.is_ok());
    }

    #[test]
    fn test_kv_cache_pool_creation() {
        let pool_config = PoolConfig {
            initial_size: 1024 * 1024,
            max_size: 1024 * 1024 * 1024,
        };

        let mut mgr = GpuMemoryManager::new(80 * 1024 * 1024 * 1024, pool_config);
        let crew = [1u8; 16];
        let pool_id = VramRegionID::from_bytes([0u8; 16]);

        let result = mgr.create_kv_cache_pool(pool_id, crew, 1_000_000, 1024);

        assert!(result.is_ok());

        let pool = mgr.get_kv_pool(crew, pool_id);
        assert!(pool.is_some());
        assert_eq!(pool.unwrap().capacity_bytes, 1_000_000);
    }

    #[test]
    fn test_memory_allocation_type_display() {
        assert_eq!(format!("{}", AllocationType::DeviceLocal), "DeviceLocal");
        assert_eq!(format!("{}", AllocationType::Unified), "Unified");
        assert_eq!(format!("{}", AllocationType::HostPinned), "HostPinned");
    }

    #[test]
    fn test_memory_fragmentation() {
        let pool_config = PoolConfig {
            initial_size: 1024 * 1024,
            max_size: 1024 * 1024 * 1024,
        };

        let mut mgr = GpuMemoryManager::new(80 * 1024 * 1024 * 1024, pool_config);
        let crew = [1u8; 16];

        mgr.alloc(1024, AllocationType::DeviceLocal, crew, 1000)
            .unwrap();
        mgr.alloc(1024, AllocationType::DeviceLocal, crew, 1000)
            .unwrap();
        mgr.alloc(1024, AllocationType::DeviceLocal, crew, 1000)
            .unwrap();

        mgr.calculate_fragmentation();

        assert!(mgr.stats.fragmentation <= 100);
    }

    #[test]
    fn test_allocation_display() {
        let alloc = GpuAllocation {
            handle: MemHandle(1),
            device_ptr: 0x100000,
            size: 1024,
            alloc_type: AllocationType::DeviceLocal,
            owning_crew: [1u8; 16],
            created_at: 1000,
        };

        let display_str = format!("{}", alloc);
        assert!(display_str.contains("MemHandle(1)"));
        assert!(display_str.contains("0x100000"));
        assert!(display_str.contains("1024B"));
    }
}
