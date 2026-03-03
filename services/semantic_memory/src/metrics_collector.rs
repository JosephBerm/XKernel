// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Performance metrics collection for Memory Manager syscalls.
//!
//! This module provides comprehensive metrics collection:
//! - Latency tracking per syscall (p50, p95, p99)
//! - Throughput (allocations/sec, bytes/sec for reads/writes)
//! - Error rate tracking
//! - Memory Manager process footprint measurement
//!
//! See Engineering Plan § 4.1.1: Metrics Collection (Week 6).

use alloc::vec;
use alloc::vec::Vec;
use alloc::string::String;

/// Percentile latency data for a syscall.
///
/// Tracks latency distribution (p50, p95, p99) for detailed performance analysis.
/// See Engineering Plan § 4.1.1: Latency Metrics.
#[derive(Clone, Debug)]
pub struct LatencyPercentiles {
    /// Median latency (microseconds)
    pub p50_us: u64,
    /// 95th percentile latency (microseconds)
    pub p95_us: u64,
    /// 99th percentile latency (microseconds)
    pub p99_us: u64,
}

impl LatencyPercentiles {
    /// Creates new latency percentiles.
    pub fn new(p50_us: u64, p95_us: u64, p99_us: u64) -> Self {
        LatencyPercentiles {
            p50_us,
            p95_us,
            p99_us,
        }
    }

    /// Returns average of percentiles.
    pub fn average_us(&self) -> u64 {
        (self.p50_us + self.p95_us + self.p99_us) / 3
    }

    /// Formats as human-readable string.
    pub fn format(&self) -> String {
        alloc::format!(
            "p50={:.2}µs, p95={:.2}µs, p99={:.2}µs",
            self.p50_us, self.p95_us, self.p99_us
        )
    }
}

/// Per-syscall performance metrics.
///
/// Tracks performance characteristics of each CSCI syscall.
/// See Engineering Plan § 4.1.1: Syscall Metrics.
#[derive(Clone, Debug)]
pub struct SyscallMetrics {
    /// Syscall name (mem_alloc, mem_read, etc.)
    pub syscall_name: String,
    /// Total number of invocations
    pub total_calls: u64,
    /// Successful calls
    pub successful_calls: u64,
    /// Failed calls
    pub failed_calls: u64,
    /// Latency percentiles (microseconds)
    pub latency: LatencyPercentiles,
    /// Average data size per call (bytes)
    pub avg_data_size: u64,
    /// Throughput (operations per second)
    pub throughput_ops_per_sec: u64,
    /// Bytes per second (for read/write)
    pub throughput_bytes_per_sec: u64,
}

impl SyscallMetrics {
    /// Creates new syscall metrics.
    pub fn new(syscall_name: impl Into<String>) -> Self {
        SyscallMetrics {
            syscall_name: syscall_name.into(),
            total_calls: 0,
            successful_calls: 0,
            failed_calls: 0,
            latency: LatencyPercentiles::new(0, 0, 0),
            avg_data_size: 0,
            throughput_ops_per_sec: 0,
            throughput_bytes_per_sec: 0,
        }
    }

    /// Returns success rate as percentage.
    pub fn success_rate_percent(&self) -> f64 {
        if self.total_calls == 0 {
            0.0
        } else {
            (self.successful_calls as f64 / self.total_calls as f64) * 100.0
        }
    }

    /// Returns error rate as percentage.
    pub fn error_rate_percent(&self) -> f64 {
        100.0 - self.success_rate_percent()
    }

    /// Formats metrics as human-readable string.
    pub fn format(&self) -> String {
        alloc::format!(
            "{}: {} calls, {:.1}% success, {} ops/sec, {} B/sec, latency: {}",
            self.syscall_name,
            self.total_calls,
            self.success_rate_percent(),
            self.throughput_ops_per_sec,
            self.throughput_bytes_per_sec,
            self.latency.format(),
        )
    }
}

/// Memory Manager process footprint.
///
/// Tracks resource usage of the Memory Manager process itself.
/// See Engineering Plan § 4.1.1: Process Metrics.
#[derive(Clone, Debug)]
pub struct ProcessFootprint {
    /// Memory used by MM process (bytes)
    pub rss_bytes: u64,
    /// Virtual memory used (bytes)
    pub vms_bytes: u64,
    /// Number of allocations tracked
    pub allocation_count: u64,
    /// Number of threads
    pub thread_count: u64,
    /// CPU time used (milliseconds)
    pub cpu_time_ms: u64,
}

impl ProcessFootprint {
    /// Creates new process footprint.
    pub fn new() -> Self {
        ProcessFootprint {
            rss_bytes: 0,
            vms_bytes: 0,
            allocation_count: 0,
            thread_count: 1,
            cpu_time_ms: 0,
        }
    }

    /// Returns RSS in megabytes.
    pub fn rss_mb(&self) -> u64 {
        self.rss_bytes / (1024 * 1024)
    }

    /// Returns VMS in megabytes.
    pub fn vms_mb(&self) -> u64 {
        self.vms_bytes / (1024 * 1024)
    }

    /// Formats footprint as string.
    pub fn format(&self) -> String {
        alloc::format!(
            "RSS: {} MB, VMS: {} MB, Allocs: {}, Threads: {}, CPU: {} ms",
            self.rss_mb(),
            self.vms_mb(),
            self.allocation_count,
            self.thread_count,
            self.cpu_time_ms,
        )
    }
}

/// Comprehensive metrics collector for Memory Manager.
///
/// Collects performance data across all syscalls and provides detailed analysis.
/// See Engineering Plan § 4.1.1: Metrics Collection.
#[derive(Debug)]
pub struct MetricsCollector {
    /// Per-syscall metrics
    syscall_metrics: Vec<SyscallMetrics>,
    /// Process footprint
    process_footprint: ProcessFootprint,
    /// Total elapsed time (microseconds)
    total_time_us: u64,
    /// Collection start timestamp (microseconds since epoch)
    start_time_us: u64,
}

impl MetricsCollector {
    /// Creates a new metrics collector.
    pub fn new(start_time_us: u64) -> Self {
        MetricsCollector {
            syscall_metrics: vec![
                SyscallMetrics::new("mem_alloc"),
                SyscallMetrics::new("mem_read"),
                SyscallMetrics::new("mem_write"),
                SyscallMetrics::new("mem_mount"),
            ],
            process_footprint: ProcessFootprint::new(),
            total_time_us: 0,
            start_time_us,
        }
    }

    /// Records a syscall invocation.
    ///
    /// # Arguments
    ///
    /// * `syscall_name` - Name of syscall ("mem_alloc", etc.)
    /// * `success` - Whether the call succeeded
    /// * `latency_us` - Latency in microseconds
    /// * `data_size` - Size of data processed (bytes)
    pub fn record_syscall(
        &mut self,
        syscall_name: &str,
        success: bool,
        latency_us: u64,
        data_size: u64,
    ) {
        for metrics in self.syscall_metrics.iter_mut() {
            if metrics.syscall_name == syscall_name {
                metrics.total_calls += 1;
                if success {
                    metrics.successful_calls += 1;
                } else {
                    metrics.failed_calls += 1;
                }

                // Update latency percentiles (stub: use direct values)
                if metrics.latency.p50_us == 0 {
                    metrics.latency.p50_us = latency_us;
                    metrics.latency.p95_us = latency_us;
                    metrics.latency.p99_us = latency_us;
                } else {
                    // In real implementation, maintain sorted latency samples
                    metrics.latency.p50_us = (metrics.latency.p50_us + latency_us) / 2;
                    if latency_us > metrics.latency.p95_us {
                        metrics.latency.p95_us = latency_us;
                    }
                    if latency_us > metrics.latency.p99_us {
                        metrics.latency.p99_us = latency_us;
                    }
                }

                // Update data size
                if metrics.total_calls == 1 {
                    metrics.avg_data_size = data_size;
                } else {
                    metrics.avg_data_size = (metrics.avg_data_size + data_size) / 2;
                }

                break;
            }
        }
    }

    /// Finalizes metrics collection and calculates derived metrics.
    ///
    /// # Arguments
    ///
    /// * `end_time_us` - End timestamp in microseconds
    pub fn finalize(&mut self, end_time_us: u64) {
        self.total_time_us = end_time_us.saturating_sub(self.start_time_us);
        if self.total_time_us == 0 {
            self.total_time_us = 1; // Avoid division by zero
        }

        // Calculate throughput
        for metrics in self.syscall_metrics.iter_mut() {
            if metrics.total_calls > 0 {
                // Operations per second
                metrics.throughput_ops_per_sec = (metrics.total_calls * 1_000_000)
                    / self.total_time_us.max(1);

                // Bytes per second
                let total_bytes = metrics.total_calls * metrics.avg_data_size;
                metrics.throughput_bytes_per_sec = (total_bytes * 1_000_000)
                    / self.total_time_us.max(1);
            }
        }
    }

    /// Returns metrics for a specific syscall.
    pub fn get_syscall_metrics(&self, name: &str) -> Option<&SyscallMetrics> {
        self.syscall_metrics.iter().find(|m| m.syscall_name == name)
    }

    /// Returns all syscall metrics.
    pub fn syscall_metrics(&self) -> &[SyscallMetrics] {
        &self.syscall_metrics
    }

    /// Returns process footprint.
    pub fn process_footprint(&self) -> &ProcessFootprint {
        &self.process_footprint
    }

    /// Updates process footprint.
    pub fn set_process_footprint(&mut self, footprint: ProcessFootprint) {
        self.process_footprint = footprint;
    }

    /// Returns total elapsed time in microseconds.
    pub fn total_time_us(&self) -> u64 {
        self.total_time_us
    }

    /// Generates a detailed metrics report.
    pub fn report(&self) -> String {
        let mut report = alloc::format!(
            "METRICS REPORT\n\
            Elapsed Time: {:.3} seconds\n\
            Process Footprint: {}\n\
            \n\
            SYSCALL METRICS:\n",
            self.total_time_us as f64 / 1_000_000.0,
            self.process_footprint.format(),
        );

        for metrics in self.syscall_metrics.iter() {
            report.push_str(&alloc::format!("  {}\n", metrics.format()));
        }

        report
    }

    /// Returns summary statistics.
    pub fn summary(&self) -> String {
        let total_calls: u64 = self.syscall_metrics.iter().map(|m| m.total_calls).sum();
        let total_successful: u64 = self.syscall_metrics.iter().map(|m| m.successful_calls).sum();
        let total_failed: u64 = self.syscall_metrics.iter().map(|m| m.failed_calls).sum();

        let avg_success_rate = if total_calls == 0 {
            0.0
        } else {
            (total_successful as f64 / total_calls as f64) * 100.0
        };

        alloc::format!(
            "Summary: {} total calls, {:.1}% success rate, {} failures, {:.3}s elapsed",
            total_calls,
            avg_success_rate,
            total_failed,
            self.total_time_us as f64 / 1_000_000.0,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::format;

    #[test]
    fn test_latency_percentiles() {
        let latency = LatencyPercentiles::new(50, 95, 99);
        assert_eq!(latency.p50_us, 50);
        assert_eq!(latency.p95_us, 95);
        assert_eq!(latency.p99_us, 99);
        assert_eq!(latency.average_us(), 81);
    }

    #[test]
    fn test_syscall_metrics_success_rate() {
        let mut metrics = SyscallMetrics::new("mem_alloc");
        metrics.total_calls = 100;
        metrics.successful_calls = 95;
        metrics.failed_calls = 5;

        assert_eq!(metrics.success_rate_percent(), 95.0);
        assert_eq!(metrics.error_rate_percent(), 5.0);
    }

    #[test]
    fn test_process_footprint() {
        let mut footprint = ProcessFootprint::new();
        footprint.rss_bytes = 512 * 1024 * 1024;
        footprint.vms_bytes = 1024 * 1024 * 1024;
        footprint.allocation_count = 1000;

        assert_eq!(footprint.rss_mb(), 512);
        assert_eq!(footprint.vms_mb(), 1024);
    }

    #[test]
    fn test_metrics_collector_recording() {
        let mut collector = MetricsCollector::new(0);

        collector.record_syscall("mem_alloc", true, 50, 4096);
        collector.record_syscall("mem_alloc", true, 55, 4096);
        collector.record_syscall("mem_alloc", false, 100, 0);

        let alloc_metrics = collector.get_syscall_metrics("mem_alloc").unwrap();
        assert_eq!(alloc_metrics.total_calls, 3);
        assert_eq!(alloc_metrics.successful_calls, 2);
        assert_eq!(alloc_metrics.failed_calls, 1);
    }

    #[test]
    fn test_metrics_collector_finalize() {
        let mut collector = MetricsCollector::new(0);

        collector.record_syscall("mem_alloc", true, 50, 4096);
        collector.record_syscall("mem_alloc", true, 55, 4096);

        collector.finalize(1_000_000); // 1 second

        let alloc_metrics = collector.get_syscall_metrics("mem_alloc").unwrap();
        assert!(alloc_metrics.throughput_ops_per_sec > 0);
        assert!(alloc_metrics.throughput_bytes_per_sec > 0);
    }

    #[test]
    fn test_metrics_report_generation() {
        let mut collector = MetricsCollector::new(0);

        collector.record_syscall("mem_alloc", true, 50, 4096);
        collector.record_syscall("mem_read", true, 100, 1024);
        collector.finalize(500_000);

        let report = collector.report();
        assert!(report.contains("METRICS REPORT"));
        assert!(report.contains("mem_alloc"));
        assert!(report.contains("mem_read"));
    }

    #[test]
    fn test_metrics_summary() {
        let mut collector = MetricsCollector::new(0);

        collector.record_syscall("mem_alloc", true, 50, 4096);
        collector.record_syscall("mem_alloc", true, 55, 4096);
        collector.finalize(1_000_000);

        let summary = collector.summary();
        assert!(summary.contains("Summary"));
        assert!(summary.contains("100.0%"));
    }
}
