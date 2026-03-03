// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Memory tier abstractions for L1 HBM, L2 DRAM, and L3 NVMe layers.

use alloc::vec::Vec;
use core::fmt;

/// Latency target for L1 HBM access in microseconds (87 us)
pub const L1_LATENCY_TARGET_US: u32 = 87;

/// Latency target for L2 DRAM access in milliseconds (48 ms)
pub const L2_LATENCY_TARGET_MS: u32 = 48;

/// Latency target for L3 NVMe access in milliseconds (92 ms)
pub const L3_LATENCY_TARGET_MS: u32 = 92;

/// Memory tier enumeration for 3-tier hierarchical model
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MemoryTier {
    /// L1: GPU-local HBM/SRAM (microsecond latency)
    L1Hbm,
    /// L2: Host DRAM (millisecond latency, episodic)
    L2Dram,
    /// L3: Persistent NVMe (millisecond+ latency, long-term)
    L3Nvme,
}

impl fmt::Display for MemoryTier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::L1Hbm => write!(f, "L1(HBM)"),
            Self::L2Dram => write!(f, "L2(DRAM)"),
            Self::L3Nvme => write!(f, "L3(NVMe)"),
        }
    }
}

impl MemoryTier {
    /// Get expected latency in microseconds for this tier
    pub fn latency_us(&self) -> u32 {
        match self {
            Self::L1Hbm => L1_LATENCY_TARGET_US,
            Self::L2Dram => L2_LATENCY_TARGET_MS * 1000,
            Self::L3Nvme => L3_LATENCY_TARGET_MS * 1000,
        }
    }

    /// Check if this tier is persistent (survives process restart)
    pub fn is_persistent(&self) -> bool {
        matches!(self, Self::L3Nvme)
    }

    /// Check if this tier is volatile (lost on shutdown)
    pub fn is_volatile(&self) -> bool {
        !self.is_persistent()
    }
}

/// Tier-specific configuration and state
#[derive(Debug, Clone)]
pub struct TierConfig {
    pub tier: MemoryTier,
    pub capacity_bytes: u64,
    pub block_size: usize,
    pub max_entries: usize,
}

impl TierConfig {
    /// Create L1 tier configuration
    pub fn new_l1(capacity_bytes: u64) -> Self {
        Self {
            tier: MemoryTier::L1Hbm,
            capacity_bytes,
            block_size: 256,
            max_entries: (capacity_bytes / 256) as usize,
        }
    }

    /// Create L2 tier configuration
    pub fn new_l2(capacity_bytes: u64) -> Self {
        Self {
            tier: MemoryTier::L2Dram,
            capacity_bytes,
            block_size: 4096,
            max_entries: (capacity_bytes / 4096) as usize,
        }
    }

    /// Create L3 tier configuration
    pub fn new_l3(capacity_bytes: u64) -> Self {
        Self {
            tier: MemoryTier::L3Nvme,
            capacity_bytes,
            block_size: 65536,
            max_entries: (capacity_bytes / 65536) as usize,
        }
    }
}

/// Track tier-specific metrics
#[derive(Debug, Clone, Default)]
pub struct TierMetrics {
    pub accesses: u64,
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub bytes_read: u64,
    pub bytes_written: u64,
    pub avg_latency_us: u32,
}

impl TierMetrics {
    pub fn hit_rate(&self) -> f64 {
        if self.accesses == 0 {
            0.0
        } else {
            self.hits as f64 / self.accesses as f64
        }
    }

    pub fn record_access(&mut self, hit: bool, latency_us: u32) {
        self.accesses += 1;
        if hit {
            self.hits += 1;
        } else {
            self.misses += 1;
        }
        // Simple moving average for latency
        self.avg_latency_us = (self.avg_latency_us + latency_us) / 2;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_tier_latency() {
        assert_eq!(MemoryTier::L1Hbm.latency_us(), 87);
        assert_eq!(MemoryTier::L2Dram.latency_us(), 48_000);
        assert_eq!(MemoryTier::L3Nvme.latency_us(), 92_000);
    }

    #[test]
    fn test_tier_persistence() {
        assert!(!MemoryTier::L1Hbm.is_persistent());
        assert!(!MemoryTier::L2Dram.is_persistent());
        assert!(MemoryTier::L3Nvme.is_persistent());
    }

    #[test]
    fn test_tier_config() {
        let l1_cfg = TierConfig::new_l1(1_000_000);
        assert_eq!(l1_cfg.tier, MemoryTier::L1Hbm);
        assert_eq!(l1_cfg.block_size, 256);

        let l3_cfg = TierConfig::new_l3(1_000_000);
        assert_eq!(l3_cfg.block_size, 65536);
    }

    #[test]
    fn test_tier_metrics() {
        let mut metrics = TierMetrics::default();
        metrics.record_access(true, 50);
        metrics.record_access(true, 60);
        metrics.record_access(false, 40);

        assert_eq!(metrics.accesses, 3);
        assert_eq!(metrics.hits, 2);
        assert_eq!(metrics.misses, 1);
        assert!(metrics.hit_rate() > 0.6 && metrics.hit_rate() < 0.7);
    }
}
