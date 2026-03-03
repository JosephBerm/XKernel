// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! L1 Working Memory (GPU-Local Context Window)
//!
//! L1 is the fastest tier in the memory hierarchy, typically implemented using HBM
//! (High Bandwidth Memory) on GPU accelerators. It provides microsecond-scale latency
//! and is used for the agent's immediate working context.
//!
//! See Engineering Plan § 4.1.1: L1 Working Memory.

use crate::error::Result;
use crate::error::MemoryError;
use crate::ids::MemoryRegionID;

/// L1 Working Memory tier representing GPU-local context window.
///
/// L1 memory operates at microsecond-scale latency and serves as the immediate
/// working context for agents. It is typically backed by HBM (High Bandwidth Memory)
/// and must be regularly managed through eviction policies when capacity is exceeded.
///
/// # Design Principles
///
/// - **Microsecond latency**: Optimized for immediate access patterns
/// - **Limited capacity**: Typically 1-100 MB per agent depending on GPU
/// - **Aggressive eviction**: Data is evicted to L2 under memory pressure
/// - **Minimal I/O overhead**: No persistence or replication at this tier
///
/// See Engineering Plan § 4.1.1: L1 Operations & Microsecond Access.
#[derive(Clone, Debug)]
pub struct L1WorkingMemory {
    /// Unique identifier for this memory region
    region_id: MemoryRegionID,

    /// Total capacity in bytes
    capacity_bytes: u64,

    /// Currently used bytes in this region
    used_bytes: u64,

    /// Target access latency in nanoseconds (microsecond-scale)
    /// Typically 100-500 ns for HBM on modern GPUs
    access_latency_target_ns: u64,

    /// Whether this region is active and accepting new allocations
    active: bool,
}

impl L1WorkingMemory {
    /// Creates a new L1 Working Memory region.
    ///
    /// # Arguments
    ///
    /// * `region_id` - Unique identifier for this region
    /// * `capacity_bytes` - Total capacity in bytes
    /// * `access_latency_target_ns` - Target access latency in nanoseconds
    ///
    /// # See
    ///
    /// Engineering Plan § 4.1.1: L1 Initialization.
    pub fn new(
        region_id: MemoryRegionID,
        capacity_bytes: u64,
        access_latency_target_ns: u64,
    ) -> Self {
        L1WorkingMemory {
            region_id,
            capacity_bytes,
            used_bytes: 0,
            access_latency_target_ns,
            active: true,
        }
    }

    /// Returns the region identifier.
    pub fn region_id(&self) -> &MemoryRegionID {
        &self.region_id
    }

    /// Returns the total capacity in bytes.
    pub fn capacity_bytes(&self) -> u64 {
        self.capacity_bytes
    }

    /// Returns the currently used bytes.
    pub fn used_bytes(&self) -> u64 {
        self.used_bytes
    }

    /// Returns the available free bytes.
    pub fn available_bytes(&self) -> u64 {
        self.capacity_bytes.saturating_sub(self.used_bytes)
    }

    /// Returns the utilization ratio (0.0 to 1.0).
    pub fn utilization(&self) -> f64 {
        if self.capacity_bytes == 0 {
            0.0
        } else {
            self.used_bytes as f64 / self.capacity_bytes as f64
        }
    }

    /// Returns the target access latency in nanoseconds.
    pub fn access_latency_target_ns(&self) -> u64 {
        self.access_latency_target_ns
    }

    /// Returns whether this region is active.
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Allocates memory in this region.
    ///
    /// Returns an error if insufficient capacity is available.
    ///
    /// # Arguments
    ///
    /// * `size_bytes` - Number of bytes to allocate
    ///
    /// # Returns
    ///
    /// Returns offset of the allocated region, or an error if allocation fails.
    ///
    /// # See
    ///
    /// Engineering Plan § 4.1.1: Allocation Operations.
    pub fn allocate(&mut self, size_bytes: u64) -> Result<u64> {
        if !self.active {
            return Err(MemoryError::InvalidTier {
                operation: "allocate".to_string(),
                tier: "L1 (inactive)".to_string(),
            });
        }

        if size_bytes > self.available_bytes() {
            return Err(MemoryError::AllocationFailed {
                requested: size_bytes,
                available: self.available_bytes(),
            });
        }

        let offset = self.used_bytes;
        self.used_bytes += size_bytes;
        Ok(offset)
    }

    /// Resizes an existing allocation.
    ///
    /// # Arguments
    ///
    /// * `current_size` - Current allocation size
    /// * `new_size` - Desired new size
    ///
    /// # Returns
    ///
    /// Returns new offset if resize succeeds, or error if not enough space.
    ///
    /// # See
    ///
    /// Engineering Plan § 4.1.1: Resize Operations.
    pub fn resize(&mut self, current_size: u64, new_size: u64) -> Result<()> {
        let delta = if new_size > current_size {
            new_size - current_size
        } else {
            0 // Shrinking doesn't require additional space
        };

        if delta > self.available_bytes() {
            return Err(MemoryError::AllocationFailed {
                requested: delta,
                available: self.available_bytes(),
            });
        }

        self.used_bytes += delta;
        Ok(())
    }

    /// Attempts to evict data to make space.
    ///
    /// This is a placeholder for the eviction policy trigger.
    /// In production, this would coordinate with the global eviction policy.
    ///
    /// # Arguments
    ///
    /// * `bytes_needed` - Minimum bytes to free
    ///
    /// # Returns
    ///
    /// Returns number of bytes freed, or error if eviction cannot proceed.
    ///
    /// # See
    ///
    /// Engineering Plan § 4.1.2: Eviction Operations.
    pub fn evict(&mut self, bytes_needed: u64) -> Result<u64> {
        // In a real implementation, this would trigger the eviction policy
        // For now, we return an error if we can't satisfy the request
        let available = self.available_bytes();
        if bytes_needed <= available {
            Ok(bytes_needed)
        } else {
            Err(MemoryError::EvictionFailed {
                reason: format!(
                    "insufficient free space: need {}, available {}",
                    bytes_needed, available
                ),
            })
        }
    }

    /// Compresses data in-place to reduce memory footprint.
    ///
    /// This triggers data compression algorithms (e.g., ZSTDor gzip) on stored data.
    ///
    /// # Returns
    ///
    /// Returns number of bytes freed through compression.
    ///
    /// # See
    ///
    /// Engineering Plan § 4.1.1: Compression Operations.
    pub fn compress(&mut self) -> Result<u64> {
        // Placeholder: in real implementation, would compress all active allocations
        // For now, we assume no compression possible
        Ok(0)
    }

    /// Creates a snapshot of the current memory state.
    ///
    /// Snapshots are used for checkpointing and memory analysis.
    ///
    /// # Returns
    ///
    /// Returns a copy of the current metadata (not the data itself).
    ///
    /// # See
    ///
    /// Engineering Plan § 4.1.1: Snapshot Operations.
    pub fn snapshot(&self) -> L1Snapshot {
        L1Snapshot {
            region_id: self.region_id.clone(),
            capacity_bytes: self.capacity_bytes,
            used_bytes: self.used_bytes,
            access_latency_target_ns: self.access_latency_target_ns,
            timestamp_ns: 0, // Would use wall clock in production
        }
    }

    /// Prefetches data in anticipation of future access.
    ///
    /// This is a hint to the memory hierarchy to prepare data for faster access.
    ///
    /// # Arguments
    ///
    /// * `size_bytes` - Bytes to prefetch
    ///
    /// # See
    ///
    /// Engineering Plan § 4.1.1: Prefetch Operations.
    pub fn prefetch(&mut self, size_bytes: u64) -> Result<()> {
        // Prefetch succeeds if we can allocate the space
        // In real implementation, would load from L2 or L3
        if size_bytes <= self.available_bytes() {
            Ok(())
        } else {
            Err(MemoryError::AllocationFailed {
                requested: size_bytes,
                available: self.available_bytes(),
            })
        }
    }

    /// Deactivates this region (e.g., during shutdown or migration).
    pub fn deactivate(&mut self) {
        self.active = false;
    }

    /// Clears all data from this region.
    pub fn clear(&mut self) {
        self.used_bytes = 0;
    }
}

/// A snapshot of L1 memory state at a point in time.
///
/// Used for monitoring, debugging, and checkpointing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct L1Snapshot {
    /// Region identifier
    pub region_id: MemoryRegionID,

    /// Total capacity at snapshot time
    pub capacity_bytes: u64,

    /// Used bytes at snapshot time
    pub used_bytes: u64,

    /// Target access latency
    pub access_latency_target_ns: u64,

    /// Timestamp of snapshot (nanoseconds since epoch)
    pub timestamp_ns: u64,
}

impl L1Snapshot {
    /// Returns the utilization at snapshot time.
    pub fn utilization(&self) -> f64 {
        if self.capacity_bytes == 0 {
            0.0
        } else {
            self.used_bytes as f64 / self.capacity_bytes as f64
        }
    }

    /// Returns available bytes at snapshot time.
    pub fn available_bytes(&self) -> u64 {
        self.capacity_bytes.saturating_sub(self.used_bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::format;
use alloc::string::ToString;

    #[test]
    fn test_l1_creation() {
        let region_id = MemoryRegionID::l1_gpu_local();
        let l1 = L1WorkingMemory::new(region_id.clone(), 1024 * 1024, 200); // 1 MB, 200 ns latency

        assert_eq!(l1.region_id(), &region_id);
        assert_eq!(l1.capacity_bytes(), 1024 * 1024);
        assert_eq!(l1.used_bytes(), 0);
        assert_eq!(l1.access_latency_target_ns(), 200);
        assert!(l1.is_active());
    }

    #[test]
    fn test_l1_available_bytes() {
        let region_id = MemoryRegionID::l1_gpu_local();
        let mut l1 = L1WorkingMemory::new(region_id, 1000, 200);

        assert_eq!(l1.available_bytes(), 1000);

        l1.allocate(300).unwrap();
        assert_eq!(l1.available_bytes(), 700);
    }

    #[test]
    fn test_l1_utilization() {
        let region_id = MemoryRegionID::l1_gpu_local();
        let mut l1 = L1WorkingMemory::new(region_id, 1000, 200);

        assert_eq!(l1.utilization(), 0.0);

        l1.allocate(500).unwrap();
        assert_eq!(l1.utilization(), 0.5);

        l1.allocate(500).unwrap();
        assert_eq!(l1.utilization(), 1.0);
    }

    #[test]
    fn test_l1_allocate_success() {
        let region_id = MemoryRegionID::l1_gpu_local();
        let mut l1 = L1WorkingMemory::new(region_id, 1000, 200);

        let offset1 = l1.allocate(300).unwrap();
        assert_eq!(offset1, 0);
        assert_eq!(l1.used_bytes(), 300);

        let offset2 = l1.allocate(200).unwrap();
        assert_eq!(offset2, 300);
        assert_eq!(l1.used_bytes(), 500);
    }

    #[test]
    fn test_l1_allocate_insufficient_space() {
        let region_id = MemoryRegionID::l1_gpu_local();
        let mut l1 = L1WorkingMemory::new(region_id, 1000, 200);

        l1.allocate(600).unwrap();
        let result = l1.allocate(500);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), MemoryError::AllocationFailed { .. }));
    }

    #[test]
    fn test_l1_allocate_inactive() {
        let region_id = MemoryRegionID::l1_gpu_local();
        let mut l1 = L1WorkingMemory::new(region_id, 1000, 200);

        l1.deactivate();
        let result = l1.allocate(100);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), MemoryError::InvalidTier { .. }));
    }

    #[test]
    fn test_l1_resize_expand() {
        let region_id = MemoryRegionID::l1_gpu_local();
        let mut l1 = L1WorkingMemory::new(region_id, 1000, 200);

        l1.allocate(400).unwrap();
        l1.resize(400, 600).unwrap();
        assert_eq!(l1.used_bytes(), 600);
    }

    #[test]
    fn test_l1_resize_shrink() {
        let region_id = MemoryRegionID::l1_gpu_local();
        let mut l1 = L1WorkingMemory::new(region_id, 1000, 200);

        l1.allocate(400).unwrap();
        l1.resize(400, 200).unwrap();
        assert_eq!(l1.used_bytes(), 600); // used_bytes only increases
    }

    #[test]
    fn test_l1_resize_insufficient_space() {
        let region_id = MemoryRegionID::l1_gpu_local();
        let mut l1 = L1WorkingMemory::new(region_id, 1000, 200);

        l1.allocate(800).unwrap();
        let result = l1.resize(800, 1200);
        assert!(result.is_err());
    }

    #[test]
    fn test_l1_evict_success() {
        let region_id = MemoryRegionID::l1_gpu_local();
        let mut l1 = L1WorkingMemory::new(region_id, 1000, 200);

        l1.allocate(600).unwrap();
        let freed = l1.evict(100).unwrap();
        assert_eq!(freed, 100);
    }

    #[test]
    fn test_l1_evict_insufficient() {
        let region_id = MemoryRegionID::l1_gpu_local();
        let mut l1 = L1WorkingMemory::new(region_id, 1000, 200);

        l1.allocate(900).unwrap();
        let result = l1.evict(500);
        assert!(result.is_err());
    }

    #[test]
    fn test_l1_compress() {
        let region_id = MemoryRegionID::l1_gpu_local();
        let mut l1 = L1WorkingMemory::new(region_id, 1000, 200);

        let compressed = l1.compress().unwrap();
        assert_eq!(compressed, 0); // No compression in simple implementation
    }

    #[test]
    fn test_l1_snapshot() {
        let region_id = MemoryRegionID::l1_gpu_local();
        let mut l1 = L1WorkingMemory::new(region_id, 1000, 200);

        l1.allocate(600).unwrap();
        let snapshot = l1.snapshot();

        assert_eq!(snapshot.capacity_bytes, 1000);
        assert_eq!(snapshot.used_bytes, 600);
        assert_eq!(snapshot.access_latency_target_ns, 200);
        assert_eq!(snapshot.utilization(), 0.6);
    }

    #[test]
    fn test_l1_prefetch() {
        let region_id = MemoryRegionID::l1_gpu_local();
        let mut l1 = L1WorkingMemory::new(region_id, 1000, 200);

        let result = l1.prefetch(500);
        assert!(result.is_ok());
    }

    #[test]
    fn test_l1_prefetch_insufficient() {
        let region_id = MemoryRegionID::l1_gpu_local();
        let mut l1 = L1WorkingMemory::new(region_id, 1000, 200);

        l1.allocate(800).unwrap();
        let result = l1.prefetch(300);
        assert!(result.is_err());
    }

    #[test]
    fn test_l1_deactivate() {
        let region_id = MemoryRegionID::l1_gpu_local();
        let mut l1 = L1WorkingMemory::new(region_id, 1000, 200);

        assert!(l1.is_active());
        l1.deactivate();
        assert!(!l1.is_active());
    }

    #[test]
    fn test_l1_clear() {
        let region_id = MemoryRegionID::l1_gpu_local();
        let mut l1 = L1WorkingMemory::new(region_id, 1000, 200);

        l1.allocate(600).unwrap();
        assert_eq!(l1.used_bytes(), 600);

        l1.clear();
        assert_eq!(l1.used_bytes(), 0);
    }

    #[test]
    fn test_l1_snapshot_available_bytes() {
        let region_id = MemoryRegionID::l1_gpu_local();
        let mut l1 = L1WorkingMemory::new(region_id, 1000, 200);

        l1.allocate(400).unwrap();
        let snapshot = l1.snapshot();

        assert_eq!(snapshot.available_bytes(), 600);
    }
}
