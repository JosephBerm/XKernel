// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! GPU Manager performance profiling and baseline measurement.
//!
//! Implements comprehensive performance monitoring for Phase 0 GPU Manager,
//! measuring end-to-end latencies, throughput, and resource utilization.
//! Validates architectural performance targets and identifies bottlenecks.
//!
//! ## Performance Targets (Phase 0)
//!
//! | Metric | Target | Notes |
//! |--------|--------|-------|
//! | Model load latency | < 5s | 1GB model to GPU VRAM |
//! | Command submission latency | < 100µs | kernel → GPU queue |
//! | Kernel execution overhead | < 1% | async scheduling overhead |
//! | GPU utilization | > 80% | under typical inference load |
//! | Memory bandwidth utilization | > 70% | of device peak |
//!
//! Reference: Engineering Plan § Performance Targets, Week 6

use crate::kernel_submission::KernelSubmissionConfig;
use crate::model_loading::ModelLoadRequest;
use crate::telemetry_hooks::GpuMetric;
use alloc::vec::Vec;
use core::fmt;

/// Performance metric measurement.
///
/// Records a single performance observation with timing details.
#[derive(Clone, Copy, Debug)]
pub struct PerformanceMetric {
    /// Metric name (e.g., "model_load_latency", "submission_latency")
    pub name: [u8; 64],

    /// Measured value in nanoseconds
    pub value_ns: u64,

    /// Target threshold in nanoseconds (for pass/fail evaluation)
    pub target_ns: u64,

    /// Whether measurement meets target
    pub meets_target: bool,

    /// Sample count (for averaging)
    pub sample_count: u32,

    /// Timestamp of measurement
    pub timestamp_ns: u64,
}

impl fmt::Display for PerformanceMetric {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name_str = core::str::from_utf8(&self.name)
            .unwrap_or("(invalid)")
            .trim_end_matches('\0');

        write!(
            f,
            "PerformanceMetric({}, {}ns, target={}ns, pass={})",
            name_str, self.value_ns, self.target_ns, self.meets_target
        )
    }
}

/// Model load latency profiling.
///
/// Measures end-to-end model load performance:
/// 1. File I/O (read model from storage)
/// 2. VRAM allocation (cuMemAlloc/hipMemAlloc)
/// 3. Memory transfer (H2D transfer)
/// 4. Registry update
/// 5. Completion
///
/// Target: < 5 seconds for 1GB model
///
/// Reference: Engineering Plan § Model Load Performance
#[derive(Clone, Debug)]
pub struct ModelLoadPerformance {
    /// Model size in bytes
    pub model_size_bytes: u64,

    /// Total load time in nanoseconds
    pub total_load_ns: u64,

    /// File I/O time in nanoseconds
    pub file_io_ns: u64,

    /// VRAM allocation time in nanoseconds
    pub vram_alloc_ns: u64,

    /// Memory transfer (H2D) time in nanoseconds
    pub memory_transfer_ns: u64,

    /// Registry update time in nanoseconds
    pub registry_update_ns: u64,

    /// Achieved throughput in MB/s
    pub throughput_mbs: f64,

    /// Meets target (< 5 seconds)
    pub meets_target: bool,
}

impl fmt::Display for ModelLoadPerformance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ModelLoadPerformance(size={}MB, total={}ms, throughput={:.1}MB/s, target={})",
            self.model_size_bytes / 1024 / 1024,
            self.total_load_ns / 1_000_000,
            self.throughput_mbs,
            self.meets_target
        )
    }
}

impl ModelLoadPerformance {
    /// Create a new performance measurement.
    pub fn new(
        model_size_bytes: u64,
        total_load_ns: u64,
        file_io_ns: u64,
        vram_alloc_ns: u64,
        memory_transfer_ns: u64,
        registry_update_ns: u64,
    ) -> Self {
        let throughput_mbs = if total_load_ns > 0 {
            (model_size_bytes as f64) / ((total_load_ns as f64) / 1_000_000_000.0) / 1_000_000.0
        } else {
            0.0
        };

        let meets_target = total_load_ns < 5_000_000_000; // < 5 seconds

        ModelLoadPerformance {
            model_size_bytes,
            total_load_ns,
            file_io_ns,
            vram_alloc_ns,
            memory_transfer_ns,
            registry_update_ns,
            throughput_mbs,
            meets_target,
        }
    }

    /// Breakdown of load time by phase (percentage)
    pub fn time_breakdown(&self) -> (f64, f64, f64, f64, f64) {
        let total = self.total_load_ns as f64;
        if total == 0.0 {
            return (0.0, 0.0, 0.0, 0.0, 0.0);
        }

        (
            (self.file_io_ns as f64 / total) * 100.0,
            (self.vram_alloc_ns as f64 / total) * 100.0,
            (self.memory_transfer_ns as f64 / total) * 100.0,
            (self.registry_update_ns as f64 / total) * 100.0,
            ((self.total_load_ns - self.file_io_ns - self.vram_alloc_ns - self.memory_transfer_ns
                - self.registry_update_ns) as f64
                / total)
                * 100.0,
        )
    }
}

/// Command submission latency profiling.
///
/// Measures kernel submission overhead:
/// 1. Kernel config validation
/// 2. Command queue insertion
/// 3. CUDA/HIP API call (cuLaunchKernel)
/// 4. Stream scheduling
/// 5. Completion event recording
///
/// Target: < 100 microseconds per submission
///
/// Reference: Engineering Plan § Submission Latency
#[derive(Clone, Debug)]
pub struct SubmissionLatencyProfile {
    /// Number of submissions measured
    pub submission_count: u32,

    /// Minimum latency in nanoseconds
    pub min_ns: u64,

    /// Maximum latency in nanoseconds
    pub max_ns: u64,

    /// Average latency in nanoseconds
    pub avg_ns: u64,

    /// P99 latency in nanoseconds (99th percentile)
    pub p99_ns: u64,

    /// Meets target (< 100µs average)
    pub meets_target: bool,
}

impl fmt::Display for SubmissionLatencyProfile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SubmissionLatencyProfile(count={}, avg={}µs, p99={}µs, min={}µs, max={}µs, target={})",
            self.submission_count,
            self.avg_ns / 1000,
            self.p99_ns / 1000,
            self.min_ns / 1000,
            self.max_ns / 1000,
            self.meets_target
        )
    }
}

impl SubmissionLatencyProfile {
    /// Create a new submission latency profile.
    pub fn new(
        submission_count: u32,
        min_ns: u64,
        max_ns: u64,
        avg_ns: u64,
        p99_ns: u64,
    ) -> Self {
        let meets_target = avg_ns < 100_000; // < 100 microseconds

        SubmissionLatencyProfile {
            submission_count,
            min_ns,
            max_ns,
            avg_ns,
            p99_ns,
            meets_target,
        }
    }
}

/// Kernel execution overhead profiling.
///
/// Measures async execution overhead as percentage of kernel runtime:
/// 1. Submission queue time
/// 2. Scheduling/dispatch overhead
/// 3. Event recording overhead
/// 4. Polling/completion detection time
///
/// Target: < 1% overhead relative to kernel execution time
///
/// Reference: Engineering Plan § Execution Overhead
#[derive(Clone, Debug)]
pub struct ExecutionOverheadProfile {
    /// Actual GPU kernel execution time in nanoseconds
    pub kernel_execution_ns: u64,

    /// Submission queue wait time in nanoseconds
    pub queue_wait_ns: u64,

    /// Scheduling/dispatch overhead in nanoseconds
    pub dispatch_ns: u64,

    /// Event recording overhead in nanoseconds
    pub event_recording_ns: u64,

    /// Polling/completion detection time in nanoseconds
    pub completion_polling_ns: u64,

    /// Total overhead in nanoseconds
    pub total_overhead_ns: u64,

    /// Overhead as percentage of kernel execution time
    pub overhead_percent: f64,

    /// Meets target (< 1% overhead)
    pub meets_target: bool,
}

impl fmt::Display for ExecutionOverheadProfile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ExecutionOverheadProfile(kernel={}ms, overhead={}%, target={})",
            self.kernel_execution_ns / 1_000_000,
            self.overhead_percent,
            self.meets_target
        )
    }
}

impl ExecutionOverheadProfile {
    /// Create a new execution overhead profile.
    pub fn new(
        kernel_execution_ns: u64,
        queue_wait_ns: u64,
        dispatch_ns: u64,
        event_recording_ns: u64,
        completion_polling_ns: u64,
    ) -> Self {
        let total_overhead_ns = queue_wait_ns + dispatch_ns + event_recording_ns + completion_polling_ns;

        let overhead_percent = if kernel_execution_ns > 0 {
            (total_overhead_ns as f64 / kernel_execution_ns as f64) * 100.0
        } else {
            0.0
        };

        let meets_target = overhead_percent < 1.0;

        ExecutionOverheadProfile {
            kernel_execution_ns,
            queue_wait_ns,
            dispatch_ns,
            event_recording_ns,
            completion_polling_ns,
            total_overhead_ns,
            overhead_percent,
            meets_target,
        }
    }

    /// Breakdown of overhead sources
    pub fn overhead_breakdown(&self) -> (f64, f64, f64, f64) {
        if self.total_overhead_ns == 0 {
            return (0.0, 0.0, 0.0, 0.0);
        }

        let total = self.total_overhead_ns as f64;
        (
            (self.queue_wait_ns as f64 / total) * 100.0,
            (self.dispatch_ns as f64 / total) * 100.0,
            (self.event_recording_ns as f64 / total) * 100.0,
            (self.completion_polling_ns as f64 / total) * 100.0,
        )
    }
}

/// GPU utilization and bandwidth metrics.
///
/// Measures device-level resource utilization:
/// 1. TPC (Tensor Processing Cluster) utilization percentage
/// 2. Memory bandwidth utilization
/// 3. Power consumption
/// 4. Thermal state
///
/// Reference: Engineering Plan § GPU Utilization Metrics
#[derive(Clone, Copy, Debug)]
pub struct GpuUtilizationMetrics {
    /// TPC utilization percentage (0-100)
    pub tpc_utilization_percent: u32,

    /// Memory bandwidth utilization percentage (0-100)
    pub memory_bandwidth_percent: u32,

    /// VRAM usage in bytes
    pub vram_usage_bytes: u64,

    /// VRAM capacity in bytes
    pub vram_capacity_bytes: u64,

    /// Power consumption in watts
    pub power_consumption_w: f64,

    /// GPU temperature in Celsius
    pub temperature_celsius: f64,

    /// Timestamp of measurement
    pub timestamp_ns: u64,

    /// Meets target (> 80% TPC utilization)
    pub meets_target: bool,
}

impl fmt::Display for GpuUtilizationMetrics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "GpuUtilizationMetrics(tpc={}%, bandwidth={}%, vram={}MB/{}, temp={}°C, power={}W)",
            self.tpc_utilization_percent,
            self.memory_bandwidth_percent,
            self.vram_usage_bytes / 1024 / 1024,
            self.vram_capacity_bytes / 1024 / 1024,
            self.temperature_celsius,
            self.power_consumption_w
        )
    }
}

impl GpuUtilizationMetrics {
    /// Create new utilization metrics.
    pub fn new(
        tpc_utilization_percent: u32,
        memory_bandwidth_percent: u32,
        vram_usage_bytes: u64,
        vram_capacity_bytes: u64,
        power_consumption_w: f64,
        temperature_celsius: f64,
        timestamp_ns: u64,
    ) -> Self {
        let meets_target = tpc_utilization_percent > 80;

        GpuUtilizationMetrics {
            tpc_utilization_percent,
            memory_bandwidth_percent,
            vram_usage_bytes,
            vram_capacity_bytes,
            power_consumption_w,
            temperature_celsius,
            timestamp_ns,
            meets_target,
        }
    }

    /// VRAM utilization percentage
    pub fn vram_utilization_percent(&self) -> u32 {
        if self.vram_capacity_bytes == 0 {
            return 0;
        }
        ((self.vram_usage_bytes as f64 / self.vram_capacity_bytes as f64) * 100.0) as u32
    }
}

/// Throughput measurement for kernel submissions.
///
/// Measures submission throughput under load.
#[derive(Clone, Copy, Debug)]
pub struct ThroughputMetrics {
    /// Submissions per second
    pub submissions_per_sec: f64,

    /// Kernels per second completed
    pub completions_per_sec: f64,

    /// Total duration of measurement in seconds
    pub duration_sec: f64,

    /// Total submissions during measurement period
    pub total_submissions: u32,

    /// Total completions during measurement period
    pub total_completions: u32,
}

impl fmt::Display for ThroughputMetrics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ThroughputMetrics({:.0} subm/sec, {:.0} compl/sec, {} submissions, {} completions)",
            self.submissions_per_sec,
            self.completions_per_sec,
            self.total_submissions,
            self.total_completions
        )
    }
}

/// Performance profiling report.
///
/// Aggregates all performance metrics into a single comprehensive report
/// for analysis and validation against Phase 0 targets.
#[derive(Clone, Debug)]
pub struct PerformanceProfilingReport {
    /// Model load performance
    pub model_load: Option<ModelLoadPerformance>,

    /// Submission latency profile
    pub submission_latency: Option<SubmissionLatencyProfile>,

    /// Execution overhead profile
    pub execution_overhead: Option<ExecutionOverheadProfile>,

    /// GPU utilization metrics
    pub gpu_utilization: Option<GpuUtilizationMetrics>,

    /// Throughput metrics
    pub throughput: Option<ThroughputMetrics>,

    /// Overall pass/fail (all metrics meet targets)
    pub all_targets_met: bool,

    /// Report timestamp
    pub report_timestamp_ns: u64,
}

impl fmt::Display for PerformanceProfilingReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PerformanceProfilingReport(targets_met={})", self.all_targets_met)
    }
}

impl PerformanceProfilingReport {
    /// Create new performance report.
    pub fn new() -> Self {
        PerformanceProfilingReport {
            model_load: None,
            submission_latency: None,
            execution_overhead: None,
            gpu_utilization: None,
            throughput: None,
            all_targets_met: true,
            report_timestamp_ns: 0,
        }
    }

    /// Update all targets met flag based on individual metrics.
    pub fn update_targets_met(&mut self) {
        self.all_targets_met = true;

        if let Some(ref ml) = self.model_load {
            if !ml.meets_target {
                self.all_targets_met = false;
            }
        }

        if let Some(ref sl) = self.submission_latency {
            if !sl.meets_target {
                self.all_targets_met = false;
            }
        }

        if let Some(ref eo) = self.execution_overhead {
            if !eo.meets_target {
                self.all_targets_met = false;
            }
        }

        if let Some(ref gu) = self.gpu_utilization {
            if !gu.meets_target {
                self.all_targets_met = false;
            }
        }
    }

    /// Generate a summary string for reporting.
    pub fn summary(&self) -> [u8; 512] {
        let mut summary = [0u8; 512];
        // In a full implementation, would format all metrics into summary
        summary
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_load_performance_calculation() {
        let perf = ModelLoadPerformance::new(
            1024 * 1024 * 1024, // 1 GB
            4_000_000_000,      // 4 seconds (meets target)
            1_000_000_000,      // file I/O
            500_000_000,        // VRAM alloc
            2_000_000_000,      // memory transfer
            500_000_000,        // registry
        );

        assert!(perf.meets_target);
        assert!(perf.throughput_mbs > 0.0);
    }

    #[test]
    fn test_submission_latency_profile() {
        let profile = SubmissionLatencyProfile::new(
            1000,           // 1000 submissions
            10_000,         // min: 10µs
            50_000,         // max: 50µs
            30_000,         // avg: 30µs (meets target)
            45_000,         // p99: 45µs
        );

        assert!(profile.meets_target);
    }

    #[test]
    fn test_execution_overhead_profile() {
        let overhead = ExecutionOverheadProfile::new(
            100_000_000, // kernel: 100ms
            1_000_000,   // queue wait: 1ms
            500_000,     // dispatch: 0.5ms
            300_000,     // event: 0.3ms
            200_000,     // polling: 0.2ms
        );

        assert!(overhead.meets_target);
        assert!(overhead.overhead_percent < 1.0);
    }

    #[test]
    fn test_gpu_utilization_metrics() {
        let metrics = GpuUtilizationMetrics::new(
            85,                     // 85% TPC (meets target)
            75,                     // 75% bandwidth
            8 * 1024 * 1024 * 1024, // 8GB VRAM used
            16 * 1024 * 1024 * 1024,// 16GB total
            250.0,                  // 250W power
            65.0,                   // 65°C
            0,
        );

        assert!(metrics.meets_target);
        assert_eq!(metrics.vram_utilization_percent(), 50);
    }
}
