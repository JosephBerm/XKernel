// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! GPU scheduling strategies: TPC allocation, spatial scheduling, and right-sizing.

use alloc::vec::Vec;
use core::fmt;

/// TPC (Texture Processing Cluster) allocation for GPU kernels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TpcAllocation {
    /// Number of TPCs allocated
    pub tpc_count: u32,
    /// Clock frequency in MHz
    pub clock_mhz: u32,
    /// Power budget in watts
    pub power_watts: u32,
}

impl TpcAllocation {
    pub fn new(tpc_count: u32, clock_mhz: u32, power_watts: u32) -> Self {
        Self {
            tpc_count,
            clock_mhz,
            power_watts,
        }
    }

    /// Estimate throughput in teraops based on allocation
    pub fn estimated_throughput_tflops(&self) -> f64 {
        // Rough: 2 ops per clock per CUDA core, ~128 cores per TPC
        self.tpc_count as f64 * 128.0 * 2.0 * self.clock_mhz as f64 / 1000.0
    }

    /// Power efficiency in TFLOPS per watt
    pub fn power_efficiency(&self) -> f64 {
        if self.power_watts == 0 {
            0.0
        } else {
            self.estimated_throughput_tflops() / self.power_watts as f64
        }
    }
}

impl Default for TpcAllocation {
    fn default() -> Self {
        Self::new(40, 1500, 250) // Mid-range allocation
    }
}

/// Spatial scheduler for kernel placement
#[derive(Debug, Clone)]
pub struct SpatialScheduler {
    /// Available TPC clusters
    available_tpcs: u32,
    /// Scheduled kernels with their TPC ranges
    kernels: Vec<(u64, u32, u32)>, // (kernel_id, start_tpc, count)
}

impl SpatialScheduler {
    pub fn new(total_tpcs: u32) -> Self {
        Self {
            available_tpcs: total_tpcs,
            kernels: Vec::new(),
        }
    }

    /// Schedule kernel with specified TPC count
    pub fn schedule_kernel(
        &mut self,
        kernel_id: u64,
        tpc_needed: u32,
    ) -> Result<TpcAllocation, SchedulingError> {
        if tpc_needed == 0 || tpc_needed > self.available_tpcs {
            return Err(SchedulingError::InsufficientTpcs {
                requested: tpc_needed,
                available: self.available_tpcs,
            });
        }

        // Find first fit
        let start_tpc = self
            .kernels
            .iter()
            .map(|(_, start, count)| start + count)
            .max()
            .unwrap_or(0);

        if start_tpc + tpc_needed > self.available_tpcs {
            return Err(SchedulingError::SchedulingConflict);
        }

        self.kernels.push((kernel_id, start_tpc, tpc_needed));

        Ok(TpcAllocation::new(tpc_needed, 1500, 250))
    }

    /// Unschedule a kernel
    pub fn unschedule_kernel(&mut self, kernel_id: u64) -> bool {
        let old_len = self.kernels.len();
        self.kernels.retain(|(id, _, _)| *id != kernel_id);
        self.kernels.len() < old_len
    }

    pub fn scheduled_count(&self) -> usize {
        self.kernels.len()
    }

    pub fn free_tpcs(&self) -> u32 {
        let used: u32 = self
            .kernels
            .iter()
            .map(|(_, _, count)| count)
            .sum();
        self.available_tpcs.saturating_sub(used)
    }
}

impl Default for SpatialScheduler {
    fn default() -> Self {
        Self::new(80) // Typical: 80 TPCs on modern GPU
    }
}

/// Right-sizer for dynamic GPU resource allocation
#[derive(Debug, Clone)]
pub struct RightSizer {
    /// Historical kernel metrics for sizing
    kernel_profiles: Vec<KernelProfile>,
    /// Min/max TPC bounds
    min_tpc: u32,
    max_tpc: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct KernelProfile {
    pub kernel_id: u64,
    pub flops: u64,
    pub memory_bytes: u64,
    pub latency_ms: u32,
}

impl RightSizer {
    pub fn new(min_tpc: u32, max_tpc: u32) -> Self {
        Self {
            kernel_profiles: Vec::new(),
            min_tpc,
            max_tpc,
        }
    }

    /// Profile a kernel execution
    pub fn profile_kernel(&mut self, profile: KernelProfile) {
        self.kernel_profiles.push(profile);
    }

    /// Right-size TPC allocation based on kernel profile
    pub fn right_size(&self, kernel_id: u64, desired_latency_ms: u32) -> u32 {
        // Find similar kernel in history
        let similar = self
            .kernel_profiles
            .iter()
            .filter(|p| p.kernel_id == kernel_id)
            .last();

        if let Some(profile) = similar {
            // Estimate TPCs needed: scale based on latency requirement
            let ratio = profile.latency_ms as f64 / desired_latency_ms as f64;
            let estimated_tpcs = ((ratio * self.max_tpc as f64) as u32)
                .clamp(self.min_tpc, self.max_tpc);
            estimated_tpcs
        } else {
            // Default to mid-range
            (self.min_tpc + self.max_tpc) / 2
        }
    }

    pub fn kernel_count(&self) -> usize {
        self.kernel_profiles.len()
    }
}

impl Default for RightSizer {
    fn default() -> Self {
        Self::new(8, 80)
    }
}

/// GPU scheduler trait for extensibility
pub trait GpuScheduler: Send + Sync {
    /// Schedule a kernel execution
    fn schedule(&mut self, kernel_id: u64) -> Result<TpcAllocation, SchedulingError>;

    /// Unschedule a kernel
    fn unschedule(&mut self, kernel_id: u64) -> bool;

    /// Get current utilization
    fn utilization(&self) -> f64;
}

/// Scheduling error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulingError {
    InsufficientTpcs { requested: u32, available: u32 },
    SchedulingConflict,
    InvalidKernelId,
    AllocationFailed,
}

impl fmt::Display for SchedulingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InsufficientTpcs { requested, available } => {
                write!(
                    f,
                    "insufficient TPCs: requested {}, available {}",
                    requested, available
                )
            }
            Self::SchedulingConflict => write!(f, "scheduling conflict"),
            Self::InvalidKernelId => write!(f, "invalid kernel ID"),
            Self::AllocationFailed => write!(f, "allocation failed"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tpc_allocation() {
        let alloc = TpcAllocation::new(40, 1500, 250);
        assert_eq!(alloc.tpc_count, 40);
        assert!(alloc.estimated_throughput_tflops() > 0.0);
        assert!(alloc.power_efficiency() > 0.0);
    }

    #[test]
    fn test_spatial_scheduler() {
        let mut scheduler = SpatialScheduler::new(80);
        let alloc = scheduler.schedule_kernel(1, 20).unwrap();
        assert_eq!(alloc.tpc_count, 20);

        let alloc2 = scheduler.schedule_kernel(2, 30).unwrap();
        assert_eq!(alloc2.tpc_count, 30);

        assert_eq!(scheduler.scheduled_count(), 2);
        assert_eq!(scheduler.free_tpcs(), 30);
    }

    #[test]
    fn test_scheduler_overflow() {
        let mut scheduler = SpatialScheduler::new(80);
        scheduler.schedule_kernel(1, 40).unwrap();
        scheduler.schedule_kernel(2, 30).unwrap();

        assert!(matches!(
            scheduler.schedule_kernel(3, 20),
            Err(SchedulingError::InsufficientTpcs { .. })
        ));
    }

    #[test]
    fn test_scheduler_unschedule() {
        let mut scheduler = SpatialScheduler::new(80);
        scheduler.schedule_kernel(1, 40).unwrap();

        assert!(scheduler.unschedule_kernel(1));
        assert_eq!(scheduler.free_tpcs(), 80);
        assert!(!scheduler.unschedule_kernel(1)); // Already removed
    }

    #[test]
    fn test_right_sizer() {
        let mut sizer = RightSizer::new(8, 80);

        sizer.profile_kernel(KernelProfile {
            kernel_id: 1,
            flops: 1_000_000_000,
            memory_bytes: 1_000_000,
            latency_ms: 100,
        });

        let sized = sizer.right_size(1, 50); // Desired 50ms
        assert!(sized > 8 && sized <= 80);
    }
}
