// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! VRAM isolation and management.
//!
//! Implements crew-level VRAM regions with configurable isolation modes.
//! VRAM is the primary mechanism for ensuring crews cannot observe or corrupt
//! each other's data (KV-cache, attention tensors, intermediate activations).
//!
//! Reference: Engineering Plan § VRAM Isolation, Memory Safety

use crate::ids::{GpuDeviceID, VramRegionID};
use alloc::vec::Vec;
use core::fmt;

/// VRAM isolation mode for a region.
///
/// Defines the level of isolation enforced within a VRAM region.
/// Trade-off between security and memory efficiency.
///
/// Reference: Engineering Plan § Isolation Modes
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VramIsolationMode {
    /// Strict isolation: each crew gets dedicated physical pages.
    ///
    /// Ensures zero information leakage via memory state.
    /// Highest security, highest memory overhead (page boundaries limit packing).
    /// Suitable for multi-tenant scenarios with untrusted crews.
    Strict,

    /// Selective isolation: isolation by default, but controlled sharing allowed.
    ///
    /// Most crew memory is isolated, but specific shared tensors
    /// (e.g., model weights) can be explicitly shared with zero-copy semantics.
    /// Moderate security, balanced memory efficiency.
    Selective,

    /// Open mode: single-tenant, no isolation.
    ///
    /// All crews share the same VRAM address space.
    /// Lowest security (all crews see all data), lowest overhead.
    /// Suitable for single-crew deployments or trusted internal crews.
    Open,
}

impl fmt::Display for VramIsolationMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VramIsolationMode::Strict => write!(f, "Strict"),
            VramIsolationMode::Selective => write!(f, "Selective"),
            VramIsolationMode::Open => write!(f, "Open"),
        }
    }
}

/// VRAM region descriptor.
///
/// Represents a contiguous VRAM allocation owned by a crew.
/// The GPU Manager maintains an inventory of regions and validates
/// all memory access against them.
///
/// Reference: Engineering Plan § VRAM Allocation
#[derive(Clone, Debug)]
pub struct VramRegion {
    /// Unique region identifier.
    pub id: VramRegionID,

    /// GPU device this region is on.
    pub device_id: GpuDeviceID,

    /// Byte offset within device VRAM.
    pub offset_bytes: u64,

    /// Region size in bytes.
    pub size_bytes: u64,

    /// Crew that owns this region.
    pub owner_crew: [u8; 16],

    /// Isolation mode for this region.
    pub isolation_mode: VramIsolationMode,

    /// Is this region currently allocated? (True = allocated, False = free)
    pub is_allocated: bool,
}

impl VramRegion {
    /// Create a new VRAM region.
    pub fn new(
        id: VramRegionID,
        device_id: GpuDeviceID,
        offset_bytes: u64,
        size_bytes: u64,
        owner_crew: [u8; 16],
        isolation_mode: VramIsolationMode,
    ) -> Self {
        VramRegion {
            id,
            device_id,
            offset_bytes,
            size_bytes,
            owner_crew,
            isolation_mode,
            is_allocated: true,
        }
    }

    /// Check if an address range is within this region.
    ///
    /// # Arguments
    ///
    /// * `offset` - Byte offset to check
    /// * `size` - Byte size of the access
    ///
    /// Returns true if [offset, offset + size) is fully contained in this region.
    pub fn contains_range(&self, offset: u64, size: u64) -> bool {
        offset >= self.offset_bytes
            && size <= self.size_bytes.saturating_sub(offset.saturating_sub(self.offset_bytes))
            && offset.checked_add(size).map_or(false, |end| {
                end <= self.offset_bytes + self.size_bytes
            })
    }

    /// Get available (unallocated) space in this region.
    ///
    /// For simple regions, this is size_bytes (the entire region is available).
    /// Subdivided regions require higher-level tracking.
    pub fn available_bytes(&self) -> u64 {
        if self.is_allocated {
            0
        } else {
            self.size_bytes
        }
    }

    /// Mark this region as deallocated.
    pub fn deallocate(&mut self) {
        self.is_allocated = false;
    }
}

impl fmt::Display for VramRegion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "VramRegion({}, offset={}, size={}B, owner={:?}, mode={}, allocated={})",
            self.id,
            self.offset_bytes,
            self.size_bytes,
            &self.owner_crew[..4],
            self.isolation_mode,
            self.is_allocated
        )
    }
}

/// KV-cache pool descriptor.
///
/// The KV-cache (key-value tensors for attention) is a significant per-crew
/// memory consumer in LLM inference. This structure tracks KV-cache allocation
/// and enables isolation policies specific to cache data.
///
/// Reference: Engineering Plan § KV-Cache Isolation, Memory Efficiency
#[derive(Clone, Debug)]
pub struct KvCachePool {
    /// Unique pool identifier (part of a VRAM region).
    pub id: VramRegionID,

    /// Parent VRAM region this pool is part of.
    pub parent_region_id: VramRegionID,

    /// Owner crew.
    pub owner_crew: [u8; 16],

    /// Total cache capacity in bytes.
    pub capacity_bytes: u64,

    /// Currently used cache in bytes.
    pub used_bytes: u64,

    /// Maximum token sequence length this pool can support.
    pub max_tokens: u32,

    /// Current token count (used space / token_size_bytes).
    pub current_tokens: u32,

    /// Estimated bytes per token (determined by model hidden size).
    pub token_size_bytes: u32,
}

impl KvCachePool {
    /// Create a new KV-cache pool.
    pub fn new(
        id: VramRegionID,
        parent_region_id: VramRegionID,
        owner_crew: [u8; 16],
        capacity_bytes: u64,
        token_size_bytes: u32,
    ) -> Self {
        let max_tokens = (capacity_bytes / token_size_bytes as u64) as u32;

        KvCachePool {
            id,
            parent_region_id,
            owner_crew,
            capacity_bytes,
            used_bytes: 0,
            max_tokens,
            current_tokens: 0,
            token_size_bytes,
        }
    }

    /// Allocate tokens in the KV-cache.
    ///
    /// Returns true if successful, false if insufficient space.
    pub fn allocate_tokens(&mut self, token_count: u32) -> bool {
        let required_bytes = (token_count as u64) * (self.token_size_bytes as u64);

        if required_bytes + self.used_bytes > self.capacity_bytes {
            return false;
        }

        self.used_bytes += required_bytes;
        self.current_tokens += token_count;
        true
    }

    /// Deallocate tokens from the KV-cache.
    pub fn deallocate_tokens(&mut self, token_count: u32) {
        let freed_bytes = (token_count as u64) * (self.token_size_bytes as u64);
        self.used_bytes = self.used_bytes.saturating_sub(freed_bytes);
        self.current_tokens = self.current_tokens.saturating_sub(token_count);
    }

    /// Get available cache capacity in tokens.
    pub fn available_tokens(&self) -> u32 {
        let available_bytes = self.capacity_bytes.saturating_sub(self.used_bytes);
        (available_bytes / self.token_size_bytes as u64) as u32
    }

    /// Get utilization percentage (0-100).
    pub fn utilization_percent(&self) -> u32 {
        if self.capacity_bytes == 0 {
            return 0;
        }
        ((self.used_bytes as f64 / self.capacity_bytes as f64) * 100.0) as u32
    }

    /// Check if cache is full.
    pub fn is_full(&self) -> bool {
        self.current_tokens >= self.max_tokens
    }
}

impl fmt::Display for KvCachePool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "KvCachePool({}, capacity={}B, used={}B, tokens={}/{}, utilization={}%)",
            self.id,
            self.capacity_bytes,
            self.used_bytes,
            self.current_tokens,
            self.max_tokens,
            self.utilization_percent()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::format;

    #[test]
    fn test_vram_region_creation() {
        let region_id = VramRegionID::from_bytes([0u8; 16]);
        let device_id = GpuDeviceID::from_bytes([1u8; 16]);
        let crew_id = [2u8; 16];

        let region = VramRegion::new(
            region_id,
            device_id,
            1024,
            8192,
            crew_id,
            VramIsolationMode::Selective,
        );

        assert_eq!(region.id, region_id);
        assert_eq!(region.device_id, device_id);
        assert_eq!(region.offset_bytes, 1024);
        assert_eq!(region.size_bytes, 8192);
        assert_eq!(region.owner_crew, crew_id);
        assert_eq!(region.isolation_mode, VramIsolationMode::Selective);
        assert!(region.is_allocated);
    }

    #[test]
    fn test_vram_region_contains_range() {
        let region_id = VramRegionID::from_bytes([0u8; 16]);
        let device_id = GpuDeviceID::from_bytes([1u8; 16]);
        let crew_id = [2u8; 16];

        let region = VramRegion::new(
            region_id,
            device_id,
            1000,
            10000,
            crew_id,
            VramIsolationMode::Strict,
        );

        // Within bounds
        assert!(region.contains_range(1000, 1000));
        assert!(region.contains_range(1500, 5000));

        // Out of bounds
        assert!(!region.contains_range(999, 1));
        assert!(!region.contains_range(10999, 1));
    }

    #[test]
    fn test_vram_region_deallocate() {
        let region_id = VramRegionID::from_bytes([0u8; 16]);
        let device_id = GpuDeviceID::from_bytes([1u8; 16]);
        let crew_id = [2u8; 16];

        let mut region = VramRegion::new(
            region_id,
            device_id,
            0,
            1000,
            crew_id,
            VramIsolationMode::Open,
        );

        assert_eq!(region.available_bytes(), 0); // allocated
        region.deallocate();
        assert_eq!(region.available_bytes(), 1000);
    }

    #[test]
    fn test_kv_cache_pool_creation() {
        let pool_id = VramRegionID::from_bytes([0u8; 16]);
        let parent_id = VramRegionID::from_bytes([1u8; 16]);
        let crew_id = [2u8; 16];

        let pool = KvCachePool::new(
            pool_id,
            parent_id,
            crew_id,
            1_000_000,
            1024, // 1024 bytes per token
        );

        assert_eq!(pool.id, pool_id);
        assert_eq!(pool.capacity_bytes, 1_000_000);
        assert_eq!(pool.used_bytes, 0);
        assert_eq!(pool.token_size_bytes, 1024);
        assert_eq!(pool.max_tokens, 976); // 1_000_000 / 1024
    }

    #[test]
    fn test_kv_cache_pool_allocate_tokens() {
        let pool_id = VramRegionID::from_bytes([0u8; 16]);
        let parent_id = VramRegionID::from_bytes([1u8; 16]);
        let crew_id = [2u8; 16];

        let mut pool = KvCachePool::new(pool_id, parent_id, crew_id, 10000, 100);

        assert!(pool.allocate_tokens(50));
        assert_eq!(pool.current_tokens, 50);
        assert_eq!(pool.used_bytes, 5000);
        assert_eq!(pool.available_tokens(), 50); // (10000 - 5000) / 100

        // Try to allocate more than available
        assert!(!pool.allocate_tokens(100));
    }

    #[test]
    fn test_kv_cache_pool_deallocate_tokens() {
        let pool_id = VramRegionID::from_bytes([0u8; 16]);
        let parent_id = VramRegionID::from_bytes([1u8; 16]);
        let crew_id = [2u8; 16];

        let mut pool = KvCachePool::new(pool_id, parent_id, crew_id, 10000, 100);

        pool.allocate_tokens(50);
        assert_eq!(pool.current_tokens, 50);

        pool.deallocate_tokens(20);
        assert_eq!(pool.current_tokens, 30);
        assert_eq!(pool.used_bytes, 3000);
    }

    #[test]
    fn test_kv_cache_pool_utilization() {
        let pool_id = VramRegionID::from_bytes([0u8; 16]);
        let parent_id = VramRegionID::from_bytes([1u8; 16]);
        let crew_id = [2u8; 16];

        let mut pool = KvCachePool::new(pool_id, parent_id, crew_id, 10000, 100);

        assert_eq!(pool.utilization_percent(), 0);

        pool.allocate_tokens(50);
        assert_eq!(pool.utilization_percent(), 50); // 5000 / 10000

        pool.allocate_tokens(50);
        assert_eq!(pool.utilization_percent(), 100);
        assert!(pool.is_full());
    }

    #[test]
    fn test_isolation_mode_display() {
        assert_eq!(format!("{}", VramIsolationMode::Strict), "Strict");
        assert_eq!(format!("{}", VramIsolationMode::Selective), "Selective");
        assert_eq!(format!("{}", VramIsolationMode::Open), "Open");
    }
}
