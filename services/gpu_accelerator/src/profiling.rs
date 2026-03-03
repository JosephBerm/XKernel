// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! GPU profiling: millisecond-scale timing and utilization metrics.

use alloc::vec::Vec;
use core::fmt;

/// GPU execution timing in milliseconds
#[derive(Debug, Clone, Copy, Default)]
pub struct GpuMs {
    pub kernel_ms: u32,
    pub memory_ms: u32,
    pub overhead_ms: u32,
    pub total_ms: u32,
}

impl GpuMs {
    pub fn new(kernel_ms: u32, memory_ms: u32, overhead_ms: u32) -> Self {
        Self {
            kernel_ms,
            memory_ms,
            overhead_ms,
            total_ms: kernel_ms + memory_ms + overhead_ms,
        }
    }

    /// Compute kernel efficiency ratio
    pub fn kernel_efficiency(&self) -> f64 {
        if self.total_ms == 0 {
            0.0
        } else {
            self.kernel_ms as f64 / self.total_ms as f64
        }
    }

    /// Compute memory overhead ratio
    pub fn memory_overhead(&self) -> f64 {
        if self.total_ms == 0 {
            0.0
        } else {
            self.memory_ms as f64 / self.total_ms as f64
        }
    }

    /// Compute system overhead ratio
    pub fn system_overhead(&self) -> f64 {
        if self.total_ms == 0 {
            0.0
        } else {
            self.overhead_ms as f64 / self.total_ms as f64
        }
    }
}

impl fmt::Display for GpuMs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}ms (kernel: {}ms, memory: {}ms, overhead: {}ms)",
            self.total_ms, self.kernel_ms, self.memory_ms, self.overhead_ms
        )
    }
}

/// GPU utilization metrics
#[derive(Debug, Clone, Copy, Default)]
pub struct GpuUtilization {
    /// Compute utilization (0-100%)
    pub compute_percent: u8,
    /// Memory utilization (0-100%)
    pub memory_percent: u8,
    /// Cache hit rate (0-100%)
    pub cache_hit_percent: u8,
    /// Thermal limit percentage
    pub thermal_percent: u8,
}

impl GpuUtilization {
    pub fn new(
        compute_percent: u8,
        memory_percent: u8,
        cache_hit_percent: u8,
        thermal_percent: u8,
    ) -> Self {
        Self {
            compute_percent: compute_percent.min(100),
            memory_percent: memory_percent.min(100),
            cache_hit_percent: cache_hit_percent.min(100),
            thermal_percent: thermal_percent.min(100),
        }
    }

    /// Bottleneck analysis: which resource is limiting
    pub fn bottleneck(&self) -> Bottleneck {
        let max_util = self
            .compute_percent
            .max(self.memory_percent)
            .max(self.cache_hit_percent)
            .max(self.thermal_percent);

        if self.thermal_percent == max_util {
            Bottleneck::Thermal
        } else if self.compute_percent == max_util {
            Bottleneck::Compute
        } else if self.memory_percent == max_util {
            Bottleneck::Memory
        } else if self.cache_hit_percent < 50 {
            Bottleneck::Cache
        } else {
            Bottleneck::Other
        }
    }

    pub fn is_thermal_throttled(&self) -> bool {
        self.thermal_percent > 95
    }

    pub fn is_compute_bound(&self) -> bool {
        self.compute_percent > self.memory_percent + 20
    }

    pub fn is_memory_bound(&self) -> bool {
        self.memory_percent > self.compute_percent + 20
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bottleneck {
    Compute,
    Memory,
    Cache,
    Thermal,
    Other,
}

impl fmt::Display for Bottleneck {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Compute => write!(f, "Compute"),
            Self::Memory => write!(f, "Memory"),
            Self::Cache => write!(f, "Cache"),
            Self::Thermal => write!(f, "Thermal"),
            Self::Other => write!(f, "Other"),
        }
    }
}

/// GPU profile for a kernel execution
#[derive(Debug, Clone)]
pub struct GpuProfile {
    pub kernel_id: u64,
    pub execution_time: GpuMs,
    pub utilization: GpuUtilization,
    pub flops: u64,
    pub bandwidth_gbps: f64,
}

impl GpuProfile {
    pub fn new(kernel_id: u64, execution_time: GpuMs, utilization: GpuUtilization) -> Self {
        Self {
            kernel_id,
            execution_time,
            utilization,
            flops: 0,
            bandwidth_gbps: 0.0,
        }
    }

    /// Compute FLOPS based on metrics
    pub fn effective_flops(&self) -> u64 {
        // Estimate FLOPS from execution time and utilization
        let effective_compute = self.utilization.compute_percent as f64 / 100.0;
        let base_flops = 1_000_000; // Baseline
        (base_flops as f64 * effective_compute) as u64
    }

    /// Compute efficiency (FLOPS per watt)
    pub fn energy_efficiency(&self, power_watts: u32) -> f64 {
        if power_watts == 0 {
            0.0
        } else {
            self.effective_flops() as f64 / power_watts as f64
        }
    }
}

/// Profiling data collector
#[derive(Debug, Clone)]
pub struct ProfileCollector {
    profiles: Vec<GpuProfile>,
    max_profiles: usize,
}

impl ProfileCollector {
    pub fn new(max_profiles: usize) -> Self {
        Self {
            profiles: Vec::new(),
            max_profiles,
        }
    }

    /// Record a profile
    pub fn record(&mut self, profile: GpuProfile) {
        if self.profiles.len() >= self.max_profiles {
            self.profiles.remove(0); // FIFO eviction
        }
        self.profiles.push(profile);
    }

    /// Get average utilization
    pub fn average_utilization(&self) -> Option<GpuUtilization> {
        if self.profiles.is_empty() {
            return None;
        }

        let avg_compute = self
            .profiles
            .iter()
            .map(|p| p.utilization.compute_percent as u32)
            .sum::<u32>()
            / self.profiles.len() as u32;

        let avg_memory = self
            .profiles
            .iter()
            .map(|p| p.utilization.memory_percent as u32)
            .sum::<u32>()
            / self.profiles.len() as u32;

        let avg_cache = self
            .profiles
            .iter()
            .map(|p| p.utilization.cache_hit_percent as u32)
            .sum::<u32>()
            / self.profiles.len() as u32;

        Some(GpuUtilization::new(
            avg_compute as u8,
            avg_memory as u8,
            avg_cache as u8,
            0,
        ))
    }

    /// Get average execution time
    pub fn average_execution_time(&self) -> Option<GpuMs> {
        if self.profiles.is_empty() {
            return None;
        }

        let avg_kernel = self
            .profiles
            .iter()
            .map(|p| p.execution_time.kernel_ms as u64)
            .sum::<u64>()
            / self.profiles.len() as u64;

        let avg_memory = self
            .profiles
            .iter()
            .map(|p| p.execution_time.memory_ms as u64)
            .sum::<u64>()
            / self.profiles.len() as u64;

        Some(GpuMs::new(
            avg_kernel as u32,
            avg_memory as u32,
            0,
        ))
    }

    pub fn profile_count(&self) -> usize {
        self.profiles.len()
    }

    pub fn get_profile(&self, kernel_id: u64) -> Option<&GpuProfile> {
        self.profiles.iter().find(|p| p.kernel_id == kernel_id)
    }
}

impl Default for ProfileCollector {
    fn default() -> Self {
        Self::new(1000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpu_ms() {
        let timing = GpuMs::new(50, 20, 5);
        assert_eq!(timing.total_ms, 75);
        assert!(timing.kernel_efficiency() > 0.6);
    }

    #[test]
    fn test_gpu_utilization() {
        let util = GpuUtilization::new(80, 60, 90, 40);
        assert_eq!(util.compute_percent, 80);
        assert!(util.is_compute_bound());
        assert!(!util.is_thermal_throttled());
    }

    #[test]
    fn test_bottleneck_detection() {
        let compute_heavy = GpuUtilization::new(90, 30, 70, 20);
        assert_eq!(compute_heavy.bottleneck(), Bottleneck::Compute);

        let memory_heavy = GpuUtilization::new(30, 90, 50, 20);
        assert_eq!(memory_heavy.bottleneck(), Bottleneck::Memory);

        let thermal_heavy = GpuUtilization::new(70, 70, 70, 98);
        assert_eq!(thermal_heavy.bottleneck(), Bottleneck::Thermal);
    }

    #[test]
    fn test_gpu_profile() {
        let timing = GpuMs::new(50, 20, 5);
        let util = GpuUtilization::new(85, 70, 90, 50);
        let profile = GpuProfile::new(1, timing, util);

        assert!(profile.effective_flops() > 0);
        assert!(profile.energy_efficiency(250) > 0.0);
    }

    #[test]
    fn test_profile_collector() {
        let mut collector = ProfileCollector::new(10);

        let timing = GpuMs::new(50, 20, 5);
        let util = GpuUtilization::new(80, 70, 85, 50);

        for i in 0..5 {
            let profile = GpuProfile::new(i, timing, util);
            collector.record(profile);
        }

        assert_eq!(collector.profile_count(), 5);

        let avg = collector.average_utilization().unwrap();
        assert_eq!(avg.compute_percent, 80);
    }
}
