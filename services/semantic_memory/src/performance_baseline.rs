// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Performance baseline measurement and reporting for Memory Manager.
//!
//! This module provides:
//! - Target latency verification (mem_alloc < 100µs, mem_read < 100µs, mem_write < 100µs)
//! - Baseline data structure for comparison in Phase 1
//! - Markdown report generation
//!
//! See Engineering Plan § 4.1.1: Performance Baseline (Week 6).

use alloc::string::String;
use alloc::string::ToString;
use alloc::vec;
use alloc::vec::Vec;

/// Performance targets for Memory Manager syscalls.
///
/// Defines the latency and throughput targets that the system should achieve.
/// See Engineering Plan § 4.1.1: Performance Targets.
#[derive(Clone, Debug)]
pub struct PerformanceTargets {
    /// Maximum acceptable latency for mem_alloc (microseconds)
    pub mem_alloc_max_us: u64,
    /// Maximum acceptable latency for mem_read (microseconds)
    pub mem_read_max_us: u64,
    /// Maximum acceptable latency for mem_write (microseconds)
    pub mem_write_max_us: u64,
    /// Maximum acceptable latency for mem_mount (microseconds)
    pub mem_mount_max_us: u64,
    /// Minimum acceptable throughput (allocations per second)
    pub alloc_throughput_min: u64,
}

impl PerformanceTargets {
    /// Default Phase 0 targets.
    ///
    /// Phase 0 is stub-only, so targets are generous.
    pub fn phase0_default() -> Self {
        PerformanceTargets {
            mem_alloc_max_us: 100,
            mem_read_max_us: 100,
            mem_write_max_us: 100,
            mem_mount_max_us: 1000,
            alloc_throughput_min: 10_000, // 10K allocs/sec
        }
    }

    /// Phase 1 targets (real allocator).
    pub fn phase1_real_allocator() -> Self {
        PerformanceTargets {
            mem_alloc_max_us: 50,
            mem_read_max_us: 50,
            mem_write_max_us: 50,
            mem_mount_max_us: 500,
            alloc_throughput_min: 100_000, // 100K allocs/sec
        }
    }

    /// Phase 2+ targets (with L2/L3 optimization).
    pub fn phase2_optimized() -> Self {
        PerformanceTargets {
            mem_alloc_max_us: 20,
            mem_read_max_us: 20,
            mem_write_max_us: 20,
            mem_mount_max_us: 100,
            alloc_throughput_min: 1_000_000, // 1M allocs/sec
        }
    }
}

/// Measured performance data for a syscall.
///
/// Captures actual measured performance against targets.
/// See Engineering Plan § 4.1.1: Performance Measurement.
#[derive(Clone, Debug)]
pub struct SyscallPerformanceData {
    /// Syscall name (mem_alloc, mem_read, etc.)
    pub syscall_name: String,
    /// Measured median latency (microseconds)
    pub measured_p50_us: u64,
    /// Measured 95th percentile (microseconds)
    pub measured_p95_us: u64,
    /// Measured 99th percentile (microseconds)
    pub measured_p99_us: u64,
    /// Target maximum latency (microseconds)
    pub target_max_us: u64,
    /// Actual throughput (operations per second)
    pub measured_throughput: u64,
}

impl SyscallPerformanceData {
    /// Creates new performance data.
    pub fn new(
        syscall_name: impl Into<String>,
        p50: u64,
        p95: u64,
        p99: u64,
        target: u64,
        throughput: u64,
    ) -> Self {
        SyscallPerformanceData {
            syscall_name: syscall_name.into(),
            measured_p50_us: p50,
            measured_p95_us: p95,
            measured_p99_us: p99,
            target_max_us: target,
            measured_throughput: throughput,
        }
    }

    /// Returns true if this syscall meets its latency target.
    pub fn meets_target(&self) -> bool {
        self.measured_p99_us <= self.target_max_us
    }

    /// Returns status as percentage.
    pub fn target_percentage(&self) -> u64 {
        if self.target_max_us == 0 {
            100
        } else {
            let ratio = self.measured_p99_us as f64 / self.target_max_us as f64;
            ((1.0 / ratio) * 100.0) as u64
        }
    }

    /// Formats as human-readable string.
    pub fn format(&self) -> String {
        let status = if self.meets_target() { "PASS" } else { "FAIL" };
        alloc::format!(
            "{:12} | p50={:5}µs | p95={:5}µs | p99={:5}µs | target={:5}µs | {} | {:.0}% | {} ops/s",
            self.syscall_name,
            self.measured_p50_us,
            self.measured_p95_us,
            self.measured_p99_us,
            self.target_max_us,
            status,
            self.target_percentage(),
            self.measured_throughput,
        )
    }
}

/// Complete baseline measurement for a release/version.
///
/// Contains all performance data for reproducible comparison between versions.
/// See Engineering Plan § 4.1.1: Baseline Storage.
#[derive(Clone, Debug)]
pub struct PerformanceBaseline {
    /// Baseline version/release name
    pub version: String,
    /// Timestamp when baseline was measured (ISO 8601)
    pub timestamp: String,
    /// Phase (0, 1, 2, etc.)
    pub phase: u32,
    /// Per-syscall performance data
    pub syscalls: Vec<SyscallPerformanceData>,
    /// Process memory footprint (MB)
    pub process_memory_mb: u64,
    /// Overall pass/fail status
    pub status: String,
}

impl PerformanceBaseline {
    /// Creates a new baseline.
    pub fn new(version: impl Into<String>, phase: u32) -> Self {
        PerformanceBaseline {
            version: version.into(),
            timestamp: "2026-03-01T00:00:00Z".into(),
            phase,
            syscalls: Vec::new(),
            process_memory_mb: 0,
            status: "UNKNOWN".into(),
        }
    }

    /// Adds a syscall measurement.
    pub fn add_syscall(&mut self, data: SyscallPerformanceData) {
        self.syscalls.push(data);
    }

    /// Returns overall pass/fail status based on all syscalls.
    pub fn calculate_status(&mut self) {
        let all_pass = self.syscalls.iter().all(|s| s.meets_target());
        self.status = if all_pass { "PASS".into() } else { "FAIL".into() };
    }

    /// Returns number of passing syscalls.
    pub fn pass_count(&self) -> usize {
        self.syscalls.iter().filter(|s| s.meets_target()).count()
    }

    /// Returns number of failing syscalls.
    pub fn fail_count(&self) -> usize {
        self.syscalls.len() - self.pass_count()
    }

    /// Returns overall pass percentage.
    pub fn pass_percentage(&self) -> f64 {
        if self.syscalls.is_empty() {
            0.0
        } else {
            (self.pass_count() as f64 / self.syscalls.len() as f64) * 100.0
        }
    }

    /// Generates Markdown report.
    pub fn to_markdown(&self) -> String {
        let mut md = alloc::format!(
            "# Performance Baseline Report\n\
            \n\
            **Version:** {}\n\
            **Phase:** {}\n\
            **Timestamp:** {}\n\
            **Status:** {}\n\
            **Pass Rate:** {:.1}% ({}/{})\n\
            **Process Memory:** {} MB\n\
            \n\
            ## Syscall Performance\n\
            \n\
            | Syscall | P50 | P95 | P99 | Target | Status | %Target | Throughput |\n\
            |---------|-----|-----|-----|--------|--------|---------|------------|\n",
            self.version, self.phase, self.timestamp, self.status,
            self.pass_percentage(), self.pass_count(), self.syscalls.len(),
            self.process_memory_mb,
        );

        for data in self.syscalls.iter() {
            md.push_str(&alloc::format!(
                "| {} | {}µs | {}µs | {}µs | {}µs | {} | {:.0}% | {} |\n",
                data.syscall_name,
                data.measured_p50_us,
                data.measured_p95_us,
                data.measured_p99_us,
                data.target_max_us,
                if data.meets_target() { "✓ PASS" } else { "✗ FAIL" },
                data.target_percentage(),
                data.measured_throughput,
            ));
        }

        md.push_str("\n## Analysis\n\n");

        if self.fail_count() == 0 {
            md.push_str("All syscalls meet performance targets. Baseline is acceptable.\n");
        } else {
            md.push_str(&alloc::format!(
                "{} syscalls exceed latency targets:\n\n",
                self.fail_count()
            ));

            for data in self.syscalls.iter() {
                if !data.meets_target() {
                    md.push_str(&alloc::format!(
                        "- **{}**: {} µs (target: {} µs, {} ms over target)\n",
                        data.syscall_name,
                        data.measured_p99_us,
                        data.target_max_us,
                        data.measured_p99_us.saturating_sub(data.target_max_us) / 1000,
                    ));
                }
            }
        }

        md
    }

    /// Generates JSON-like output for machine parsing.
    pub fn to_json_like(&self) -> String {
        let mut json = alloc::format!(
            "{{ \"baseline\": {{ \
            \"version\": \"{}\", \
            \"phase\": {}, \
            \"timestamp\": \"{}\", \
            \"status\": \"{}\", \
            \"syscalls\": [",
            self.version, self.phase, self.timestamp, self.status,
        );

        for (i, data) in self.syscalls.iter().enumerate() {
            if i > 0 {
                json.push_str(", ");
            }
            json.push_str(&alloc::format!(
                "{{ \"name\": \"{}\", \"p50\": {}, \"p95\": {}, \"p99\": {}, \"target\": {}, \"throughput\": {} }}",
                data.syscall_name,
                data.measured_p50_us,
                data.measured_p95_us,
                data.measured_p99_us,
                data.target_max_us,
                data.measured_throughput,
            ));
        }

        json.push_str("] } }");
        json
    }
}

/// Performance baseline suite for comparing across phases.
#[derive(Debug)]
pub struct BaselineSuite {
    /// All baselines
    baselines: Vec<PerformanceBaseline>,
}

impl BaselineSuite {
    /// Creates a new baseline suite.
    pub fn new() -> Self {
        BaselineSuite {
            baselines: Vec::new(),
        }
    }

    /// Adds a baseline to the suite.
    pub fn add_baseline(&mut self, baseline: PerformanceBaseline) {
        self.baselines.push(baseline);
    }

    /// Returns baselines.
    pub fn baselines(&self) -> &[PerformanceBaseline] {
        &self.baselines
    }

    /// Generates comparison report (markdown).
    pub fn comparison_report(&self) -> String {
        let mut report = "# Performance Baseline Comparison\n\n".to_string();

        for baseline in self.baselines.iter() {
            report.push_str(&baseline.to_markdown());
            report.push_str("\n---\n\n");
        }

        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::format;

    #[test]
    fn test_performance_targets_phase0() {
        let targets = PerformanceTargets::phase0_default();
        assert_eq!(targets.mem_alloc_max_us, 100);
        assert_eq!(targets.alloc_throughput_min, 10_000);
    }

    #[test]
    fn test_performance_targets_phase1() {
        let targets = PerformanceTargets::phase1_real_allocator();
        assert_eq!(targets.mem_alloc_max_us, 50);
        assert_eq!(targets.alloc_throughput_min, 100_000);
    }

    #[test]
    fn test_syscall_performance_data_meets_target() {
        let data = SyscallPerformanceData::new("mem_alloc", 30, 60, 90, 100, 50_000);
        assert!(data.meets_target());
    }

    #[test]
    fn test_syscall_performance_data_exceeds_target() {
        let data = SyscallPerformanceData::new("mem_alloc", 50, 80, 150, 100, 50_000);
        assert!(!data.meets_target());
    }

    #[test]
    fn test_syscall_performance_target_percentage() {
        let data = SyscallPerformanceData::new("mem_alloc", 25, 50, 75, 100, 50_000);
        assert_eq!(data.target_percentage(), 133); // 100 / (75/100) = 133%
    }

    #[test]
    fn test_performance_baseline_creation() {
        let baseline = PerformanceBaseline::new("0.1.0", 0);
        assert_eq!(baseline.version, "0.1.0");
        assert_eq!(baseline.phase, 0);
        assert_eq!(baseline.syscalls.len(), 0);
    }

    #[test]
    fn test_performance_baseline_add_syscall() {
        let mut baseline = PerformanceBaseline::new("0.1.0", 0);
        let data = SyscallPerformanceData::new("mem_alloc", 30, 60, 90, 100, 50_000);
        baseline.add_syscall(data);

        assert_eq!(baseline.syscalls.len(), 1);
    }

    #[test]
    fn test_performance_baseline_status() {
        let mut baseline = PerformanceBaseline::new("0.1.0", 0);
        baseline.add_syscall(SyscallPerformanceData::new(
            "mem_alloc",
            30, 60, 90, 100, 50_000,
        ));
        baseline.add_syscall(SyscallPerformanceData::new(
            "mem_read",
            30, 60, 90, 100, 50_000,
        ));

        baseline.calculate_status();
        assert_eq!(baseline.status, "PASS");
        assert_eq!(baseline.pass_percentage(), 100.0);
    }

    #[test]
    fn test_performance_baseline_markdown() {
        let mut baseline = PerformanceBaseline::new("0.1.0", 0);
        baseline.add_syscall(SyscallPerformanceData::new(
            "mem_alloc",
            30, 60, 90, 100, 50_000,
        ));
        baseline.process_memory_mb = 256;

        baseline.calculate_status();

        let markdown = baseline.to_markdown();
        assert!(markdown.contains("Performance Baseline Report"));
        assert!(markdown.contains("0.1.0"));
        assert!(markdown.contains("mem_alloc"));
        assert!(markdown.contains("256 MB"));
    }

    #[test]
    fn test_baseline_suite() {
        let mut suite = BaselineSuite::new();

        let mut baseline0 = PerformanceBaseline::new("0.1.0", 0);
        baseline0.add_syscall(SyscallPerformanceData::new(
            "mem_alloc",
            30, 60, 90, 100, 50_000,
        ));

        suite.add_baseline(baseline0);
        assert_eq!(suite.baselines().len(), 1);
    }
}