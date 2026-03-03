// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Capability engine performance benchmarking suite.
//!
//! Measures latency distribution (p50, p95, p99), cache hit rates, and contention profiles.
//! Validates <100ns p99 across 1-16 cores.
//! See Engineering Plan § 3.2.0 and Week 6 § 6.

use alloc::string::String;

use core::fmt::Write;

#![forbid(unsafe_code)]

use alloc::vec::Vec;
use core::fmt::{self, Debug};

use crate::error::CapError;

/// A latency measurement in nanoseconds.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct LatencyNs(pub u64);

impl LatencyNs {
    /// Creates a new latency measurement.
    pub const fn new(nanos: u64) -> Self {
        LatencyNs(nanos)
    }

    /// Returns the latency value.
    pub const fn nanos(&self) -> u64 {
        self.0
    }

    /// Checks if latency is within budget (<100ns).
    pub const fn within_budget(&self) -> bool {
        self.0 < 100
    }
}

impl fmt::Display for LatencyNs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}ns", self.0)
    }
}

/// Latency percentile measurements.
/// See Week 6 § 6: Benchmarking Suite.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LatencyPercentiles {
    /// 50th percentile (median)
    pub p50: LatencyNs,

    /// 95th percentile
    pub p95: LatencyNs,

    /// 99th percentile (SLO target: <100ns)
    pub p99: LatencyNs,

    /// Maximum observed latency
    pub max: LatencyNs,

    /// Minimum observed latency
    pub min: LatencyNs,

    /// Mean (average) latency
    pub mean: LatencyNs,
}

impl LatencyPercentiles {
    /// Creates percentiles from sorted latency data.
    /// Data must be sorted in ascending order.
    pub fn from_sorted_data(data: &[LatencyNs]) -> Result<Self, CapError> {
        if data.is_empty() {
            return Err(CapError::Other("no latency data provided".into()));
        }

        let min = data[0];
        let max = data[data.len() - 1];

        let p50_idx = (data.len() as f64 * 0.50) as usize;
        let p95_idx = (data.len() as f64 * 0.95) as usize;
        let p99_idx = (data.len() as f64 * 0.99) as usize;

        let p50 = data[p50_idx.min(data.len() - 1)];
        let p95 = data[p95_idx.min(data.len() - 1)];
        let p99 = data[p99_idx.min(data.len() - 1)];

        let sum: u64 = data.iter().map(|l| l.nanos()).sum();
        let mean = LatencyNs::new(sum / data.len() as u64);

        Ok(LatencyPercentiles {
            p50,
            p95,
            p99,
            max,
            min,
            mean,
        })
    }

    /// Checks if p99 latency is within SLO (<100ns).
    pub const fn p99_within_slo(&self) -> bool {
        self.p99.within_budget()
    }
}

impl fmt::Display for LatencyPercentiles {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "p50={}, p95={}, p99={}, min={}, max={}, mean={}",
            self.p50, self.p95, self.p99, self.min, self.max, self.mean
        )
    }
}

/// Cache statistics from a benchmark run.
/// See Week 6 § 6.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CacheStats {
    /// Total number of cache lookups
    pub total_lookups: u64,

    /// Number of cache hits
    pub hits: u64,

    /// Number of cache misses
    pub misses: u64,

    /// Hit rate (0.0 to 1.0)
    pub hit_rate: f64,
}

impl CacheStats {
    /// Creates cache statistics from hit/miss counts.
    pub fn new(hits: u64, misses: u64) -> Self {
        let total = hits + misses;
        let hit_rate = if total == 0 {
            0.0
        } else {
            hits as f64 / total as f64
        };

        CacheStats {
            total_lookups: total,
            hits,
            misses,
            hit_rate,
        }
    }

    /// Checks if hit rate meets target (>95%).
    pub fn meets_target(&self) -> bool {
        self.hit_rate > 0.95
    }
}

impl fmt::Display for CacheStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "hits={}, misses={}, total={}, hit_rate={:.2}%",
            self.hits,
            self.misses,
            self.total_lookups,
            self.hit_rate * 100.0
        )
    }
}

/// Contention statistics (for multi-core testing).
/// See Week 6 § 6.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ContentionStats {
    /// Number of cores in the test
    pub core_count: u32,

    /// Total lock acquisitions
    pub lock_acquisitions: u64,

    /// Number of lock contentions (failed acquisitions)
    pub contentions: u64,

    /// Average contention rate (0.0 to 1.0)
    pub contention_rate: f64,

    /// Maximum contention on any single lock
    pub max_contention: f64,
}

impl ContentionStats {
    /// Creates contention statistics.
    pub fn new(core_count: u32, acquisitions: u64, contentions: u64, max_contention: f64) -> Self {
        let contention_rate = if acquisitions == 0 {
            0.0
        } else {
            contentions as f64 / acquisitions as f64
        };

        ContentionStats {
            core_count,
            lock_acquisitions: acquisitions,
            contentions,
            contention_rate,
            max_contention,
        }
    }

    /// Checks if contention is low (<5%).
    pub fn low_contention(&self) -> bool {
        self.contention_rate < 0.05
    }
}

impl fmt::Display for ContentionStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "cores={}, acquisitions={}, contentions={}, rate={:.2}%, max={:.2}%",
            self.core_count,
            self.lock_acquisitions,
            self.contentions,
            self.contention_rate * 100.0,
            self.max_contention * 100.0
        )
    }
}

/// Complete benchmark result for a capability check scenario.
/// See Week 6 § 6.
#[derive(Clone, Debug)]
pub struct BenchmarkResult {
    /// Name of the benchmark
    pub name: alloc::string::String,

    /// Latency percentiles
    pub latencies: LatencyPercentiles,

    /// Cache statistics
    pub cache_stats: CacheStats,

    /// Contention statistics (if multi-core)
    pub contention: Option<ContentionStats>,

    /// Operations per second
    pub ops_per_second: u64,

    /// Whether benchmark passed SLO
    pub passed_slo: bool,
}

impl BenchmarkResult {
    /// Creates a new benchmark result.
    pub fn new(
        name: impl Into<alloc::string::String>,
        latencies: LatencyPercentiles,
        cache_stats: CacheStats,
        contention: Option<ContentionStats>,
        ops_per_second: u64,
    ) -> Self {
        let passed_slo = latencies.p99_within_slo();

        BenchmarkResult {
            name: name.into(),
            latencies,
            cache_stats,
            contention,
            ops_per_second,
            passed_slo,
        }
    }

    /// Generates a human-readable summary report.
    pub fn summary(&self) -> alloc::string::String {

        let mut report = String::new();
        let _ = writeln!(report, "=== {} ===", self.name);
        let _ = writeln!(report, "Latencies: {}", self.latencies);
        let _ = writeln!(report, "Cache: {}", self.cache_stats);
        if let Some(contention) = self.contention {
            let _ = writeln!(report, "Contention: {}", contention);
        }
        let _ = writeln!(report, "Ops/sec: {}", self.ops_per_second);
        let _ = writeln!(report, "SLO Passed: {}", self.passed_slo);

        report
    }
}

impl fmt::Display for BenchmarkResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}(p99={}ns, hit_rate={:.2}%, ops/s={}, slo={})",
            self.name,
            self.latencies.p99.nanos(),
            self.cache_stats.hit_rate * 100.0,
            self.ops_per_second,
            if self.passed_slo { "PASS" } else { "FAIL" }
        )
    }
}

/// Benchmark suite runner.
/// See Week 6 § 6.
pub struct BenchmarkSuite {
    /// Results from all benchmark runs
    results: Vec<BenchmarkResult>,
}

impl BenchmarkSuite {
    /// Creates a new benchmark suite.
    pub fn new() -> Self {
        BenchmarkSuite {
            results: Vec::new(),
        }
    }

    /// Adds a benchmark result.
    pub fn add_result(&mut self, result: BenchmarkResult) {
        self.results.push(result);
    }

    /// Returns the number of benchmarks.
    pub fn count(&self) -> usize {
        self.results.len()
    }

    /// Returns all results.
    pub fn results(&self) -> &[BenchmarkResult] {
        &self.results
    }

    /// Checks if all benchmarks passed SLO.
    pub fn all_passed_slo(&self) -> bool {
        self.results.iter().all(|r| r.passed_slo)
    }

    /// Computes average p99 latency across all benchmarks.
    pub fn avg_p99(&self) -> u64 {
        if self.results.is_empty() {
            return 0;
        }
        let sum: u64 = self.results.iter().map(|r| r.latencies.p99.nanos()).sum();
        sum / self.results.len() as u64
    }

    /// Computes average cache hit rate across all benchmarks.
    pub fn avg_hit_rate(&self) -> f64 {
        if self.results.is_empty() {
            return 0.0;
        }
        let sum: f64 = self.results.iter().map(|r| r.cache_stats.hit_rate).sum();
        sum / self.results.len() as f64
    }

    /// Generates a summary report for all benchmarks.
    pub fn summary_report(&self) -> alloc::string::String {

        let mut report = String::new();
        let _ = writeln!(report, "\n=== Benchmark Suite Summary ===");
        let _ = writeln!(report, "Total benchmarks: {}", self.count());
        let _ = writeln!(report, "All passed SLO: {}", self.all_passed_slo());
        let _ = writeln!(report, "Average p99: {}ns", self.avg_p99());
        let _ = writeln!(report, "Average hit rate: {:.2}%", self.avg_hit_rate() * 100.0);
        let _ = writeln!(report, "\nDetailed Results:");

        for result in &self.results {
            let _ = writeln!(report, "  {}", result);
        }

        report
    }
}

impl Default for BenchmarkSuite {
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for BenchmarkSuite {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BenchmarkSuite")
            .field("result_count", &self.count())
            .field("all_passed_slo", &self.all_passed_slo())
            .field("avg_p99", &self.avg_p99())
            .field("avg_hit_rate", &self.avg_hit_rate())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::format;
use alloc::vec;

    #[test]
    fn test_latency_ns_creation() {
        let latency = LatencyNs::new(50);
        assert_eq!(latency.nanos(), 50);
        assert!(latency.within_budget());
    }

    #[test]
    fn test_latency_within_budget() {
        let ok = LatencyNs::new(99);
        let bad = LatencyNs::new(101);

        assert!(ok.within_budget());
        assert!(!bad.within_budget());
    }

    #[test]
    fn test_latency_percentiles_from_data() {
        let data = vec![
            LatencyNs::new(10),
            LatencyNs::new(20),
            LatencyNs::new(30),
            LatencyNs::new(40),
            LatencyNs::new(50),
            LatencyNs::new(60),
            LatencyNs::new(70),
            LatencyNs::new(80),
            LatencyNs::new(90),
            LatencyNs::new(100),
        ];

        let percentiles = LatencyPercentiles::from_sorted_data(&data).expect("percentiles");
        assert_eq!(percentiles.min, LatencyNs::new(10));
        assert_eq!(percentiles.max, LatencyNs::new(100));
        assert!(percentiles.p50.nanos() >= 40);
        assert!(percentiles.p95.nanos() >= 85);
    }

    #[test]
    fn test_latency_percentiles_p99_slo() {
        let data = vec![
            LatencyNs::new(10),
            LatencyNs::new(20),
            LatencyNs::new(30),
            LatencyNs::new(40),
            LatencyNs::new(50),
        ];

        let percentiles = LatencyPercentiles::from_sorted_data(&data).expect("percentiles");
        assert!(percentiles.p99_within_slo());
    }

    #[test]
    fn test_cache_stats_hit_rate() {
        let stats = CacheStats::new(950, 50);
        assert!(stats.hit_rate > 0.9);
        assert!(stats.meets_target());
    }

    #[test]
    fn test_cache_stats_low_hit_rate() {
        let stats = CacheStats::new(50, 950);
        assert!(stats.hit_rate < 0.1);
        assert!(!stats.meets_target());
    }

    #[test]
    fn test_contention_stats_low_contention() {
        let stats = ContentionStats::new(8, 10000, 100, 0.02);
        assert!(stats.low_contention());
    }

    #[test]
    fn test_contention_stats_high_contention() {
        let stats = ContentionStats::new(8, 10000, 2000, 0.25);
        assert!(!stats.low_contention());
    }

    #[test]
    fn test_benchmark_result_creation() {
        let latencies = LatencyPercentiles::from_sorted_data(&[LatencyNs::new(50)])
            .expect("percentiles");
        let cache = CacheStats::new(95, 5);

        let result =
            BenchmarkResult::new("test-bench", latencies, cache, None, 10_000_000);

        assert_eq!(result.name, "test-bench");
        assert!(result.passed_slo);
        assert!(result.summary().contains("test-bench"));
    }

    #[test]
    fn test_benchmark_suite_aggregation() {
        let mut suite = BenchmarkSuite::new();

        for i in 0..5 {
            let latencies =
                LatencyPercentiles::from_sorted_data(&[LatencyNs::new(50 + i * 5)])
                    .expect("percentiles");
            let cache = CacheStats::new(95, 5);
            let result = BenchmarkResult::new(
                &format!("bench-{}", i),
                latencies,
                cache,
                None,
                10_000_000,
            );
            suite.add_result(result);
        }

        assert_eq!(suite.count(), 5);
        assert!(suite.all_passed_slo());
        assert!(suite.summary_report().contains("Benchmark Suite Summary"));
    }

    #[test]
    fn test_benchmark_suite_summary() {
        let mut suite = BenchmarkSuite::new();

        let latencies = LatencyPercentiles::from_sorted_data(&[LatencyNs::new(50)])
            .expect("percentiles");
        let cache = CacheStats::new(950, 50);

        let result = BenchmarkResult::new("bench-1", latencies, cache, None, 10_000_000);
        suite.add_result(result);

        let summary = suite.summary_report();
        assert!(summary.contains("Average p99"));
        assert!(summary.contains("Average hit rate"));
    }
}
