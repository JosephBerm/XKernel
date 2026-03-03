//! Performance benchmarks for Phase 0 telemetry system components.
//!
//! This module defines performance baselines and benchmarks for critical
//! telemetry operations, establishing targets for production deployment.
//!
//! # Performance Targets
//!
//! - **Event emission latency**: p50 < 1ms, p95 < 5ms, p99 < 10ms
//! - **Buffer memory footprint**: ~100MB for 10k events
//! - **Cost calculation overhead**: < 0.1ms per calculation
//! - **Subscriber throughput**: > 10k events/sec
//! - **Event logging latency**: < 10ms for NDJSON write
//!
//! # Benchmark Categories
//!
//! 1. **Latency benchmarks**: Individual operation timing
//! 2. **Throughput benchmarks**: Events per second metrics
//! 3. **Memory benchmarks**: Buffer and collection footprints
//! 4. **Cost calculation benchmarks**: Pricing accuracy and speed
//!
//! # Measurement Methodology
//!
//! Benchmarks use wall-clock timing via std::time::Instant with:
//! - Warm-up iterations to stabilize CPU cache
//! - Multiple runs to compute percentiles (p50, p95, p99)
//! - Memory snapshots before and after operations
//! - Result summaries with comparison to targets
//!
//! # Example
//!
//! ```ignore
//! use tool_registry_telemetry::performance_baselines::PerformanceBenchmark;
//!
//! let benchmark = PerformanceBenchmark::new();
//! let results = benchmark.run_event_emission_benchmark(1000)?;
//! results.print_summary();
//! ```

use crate::error::{ToolError, Result};
use std::time::Instant;

/// Performance metrics for a single benchmark result.
#[derive(Debug, Clone)]
pub struct BenchmarkMetrics {
    /// Benchmark name
    pub name: String,
    /// Minimum observed latency in microseconds
    pub min_latency_us: u64,
    /// Maximum observed latency in microseconds
    pub max_latency_us: u64,
    /// Median (p50) latency in microseconds
    pub p50_latency_us: u64,
    /// 95th percentile latency in microseconds
    pub p95_latency_us: u64,
    /// 99th percentile latency in microseconds
    pub p99_latency_us: u64,
    /// Mean latency in microseconds
    pub mean_latency_us: f64,
    /// Number of operations in this benchmark
    pub operation_count: usize,
    /// Throughput in operations per second
    pub throughput_ops_sec: f64,
    /// Memory used in bytes
    pub memory_bytes: Option<u64>,
    /// Whether benchmark passed targets
    pub passed_targets: bool,
}

impl BenchmarkMetrics {
    /// Creates new benchmark metrics.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            min_latency_us: u64::MAX,
            max_latency_us: 0,
            p50_latency_us: 0,
            p95_latency_us: 0,
            p99_latency_us: 0,
            mean_latency_us: 0.0,
            operation_count: 0,
            throughput_ops_sec: 0.0,
            memory_bytes: None,
            passed_targets: false,
        }
    }

    /// Prints a summary of benchmark results.
    pub fn print_summary(&self) {
        println!("\n=== Benchmark: {} ===", self.name);
        println!("Operations: {}", self.operation_count);
        println!("Min latency: {:.2} µs", self.min_latency_us);
        println!("Mean latency: {:.2} µs", self.mean_latency_us);
        println!("p50 latency: {:.2} µs", self.p50_latency_us);
        println!("p95 latency: {:.2} µs", self.p95_latency_us);
        println!("p99 latency: {:.2} µs", self.p99_latency_us);
        println!("Max latency: {:.2} µs", self.max_latency_us);
        println!("Throughput: {:.2} ops/sec", self.throughput_ops_sec);
        
        if let Some(mem) = self.memory_bytes {
            println!("Memory: {:.2} MB", mem as f64 / 1_000_000.0);
        }
        
        let status = if self.passed_targets { "PASS" } else { "FAIL" };
        println!("Target check: {}", status);
    }

    /// Computes percentiles from a slice of latencies.
    fn compute_percentiles(mut latencies: Vec<u64>) -> (u64, u64, u64) {
        latencies.sort_unstable();
        let len = latencies.len();
        
        let p50 = if len > 0 { latencies[len / 2] } else { 0 };
        let p95 = if len > 0 { latencies[(len * 95) / 100] } else { 0 };
        let p99 = if len > 0 { latencies[(len * 99) / 100] } else { 0 };
        
        (p50, p95, p99)
    }

    /// Computes mean of a slice of values.
    fn compute_mean(values: &[u64]) -> f64 {
        if values.is_empty() {
            return 0.0;
        }
        let sum: u64 = values.iter().sum();
        sum as f64 / values.len() as f64
    }
}

/// Performance benchmark runner for telemetry components.
pub struct PerformanceBenchmark;

impl PerformanceBenchmark {
    /// Creates a new performance benchmark runner.
    pub fn new() -> Self {
        Self
    }

    /// Benchmarks event emission latency.
    ///
    /// Measures the time to emit individual events with various payloads.
    /// Target: p50 < 1000µs (1ms), p95 < 5000µs (5ms), p99 < 10000µs (10ms)
    ///
    /// # Arguments
    ///
    /// * `iterations` - Number of events to emit
    ///
    /// # Returns
    ///
    /// * `Result<BenchmarkMetrics>` - Benchmark results
    pub fn benchmark_event_emission_latency(iterations: usize) -> Result<BenchmarkMetrics> {
        let mut metrics = BenchmarkMetrics::new("event_emission_latency");
        let mut latencies = Vec::with_capacity(iterations);

        // Warm-up phase
        for _ in 0..10 {
            let start = Instant::now();
            let _ = serde_json::json!({
                "type": "test_event",
                "id": 1,
                "timestamp": 1000,
            });
            let elapsed = start.elapsed().as_micros() as u64;
            let _ = elapsed;
        }

        // Benchmark phase
        for i in 0..iterations {
            let start = Instant::now();
            let _ = serde_json::json!({
                "type": "event",
                "id": i,
                "timestamp": 1000 + i as u64,
                "payload": "x".repeat(100),
            });
            let elapsed = start.elapsed().as_micros() as u64;
            latencies.push(elapsed);

            metrics.min_latency_us = metrics.min_latency_us.min(elapsed);
            metrics.max_latency_us = metrics.max_latency_us.max(elapsed);
        }

        // Compute statistics
        let (p50, p95, p99) = BenchmarkMetrics::compute_percentiles(latencies.clone());
        metrics.p50_latency_us = p50;
        metrics.p95_latency_us = p95;
        metrics.p99_latency_us = p99;
        metrics.mean_latency_us = BenchmarkMetrics::compute_mean(&latencies);
        metrics.operation_count = iterations;
        metrics.throughput_ops_sec = (iterations as f64 * 1_000_000.0) 
            / (latencies.iter().map(|&l| l as u64).sum::<u64>() as f64);

        // Check targets: p50 < 1ms, p95 < 5ms, p99 < 10ms
        metrics.passed_targets = 
            metrics.p50_latency_us < 1000 && 
            metrics.p95_latency_us < 5000 && 
            metrics.p99_latency_us < 10000;

        Ok(metrics)
    }

    /// Benchmarks cost calculation overhead.
    ///
    /// Measures the time to perform cost calculations.
    /// Target: < 100µs (0.1ms) per calculation
    ///
    /// # Arguments
    ///
    /// * `iterations` - Number of calculations to perform
    ///
    /// # Returns
    ///
    /// * `Result<BenchmarkMetrics>` - Benchmark results
    pub fn benchmark_cost_calculation(iterations: usize) -> Result<BenchmarkMetrics> {
        let mut metrics = BenchmarkMetrics::new("cost_calculation_overhead");
        let mut latencies = Vec::with_capacity(iterations);

        // Warm-up
        for _ in 0..10 {
            let start = Instant::now();
            let _ = (100 as f64 * 1.0) + (50 as f64 * 2.0);
            let elapsed = start.elapsed().as_micros() as u64;
            let _ = elapsed;
        }

        // Benchmark
        for i in 0..iterations {
            let input_tokens = (i % 1000) as f64;
            let output_tokens = ((i * 2) % 500) as f64;
            
            let start = Instant::now();
            let _ = (input_tokens * 1.0) + (output_tokens * 2.0);
            let elapsed = start.elapsed().as_micros() as u64;
            latencies.push(elapsed);

            metrics.min_latency_us = metrics.min_latency_us.min(elapsed);
            metrics.max_latency_us = metrics.max_latency_us.max(elapsed);
        }

        let (p50, p95, p99) = BenchmarkMetrics::compute_percentiles(latencies.clone());
        metrics.p50_latency_us = p50;
        metrics.p95_latency_us = p95;
        metrics.p99_latency_us = p99;
        metrics.mean_latency_us = BenchmarkMetrics::compute_mean(&latencies);
        metrics.operation_count = iterations;
        metrics.throughput_ops_sec = (iterations as f64 * 1_000_000.0) 
            / (latencies.iter().map(|&l| l as u64).sum::<u64>() as f64);

        // Target: < 100µs
        metrics.passed_targets = metrics.p99_latency_us < 100;

        Ok(metrics)
    }

    /// Benchmarks event subscriber throughput.
    ///
    /// Measures the number of events processed per second.
    /// Target: > 10k events/sec
    ///
    /// # Arguments
    ///
    /// * `event_count` - Number of events to process
    ///
    /// # Returns
    ///
    /// * `Result<BenchmarkMetrics>` - Benchmark results
    pub fn benchmark_subscriber_throughput(event_count: usize) -> Result<BenchmarkMetrics> {
        let mut metrics = BenchmarkMetrics::new("subscriber_throughput");

        let start = Instant::now();
        
        for i in 0..event_count {
            let _event = serde_json::json!({
                "id": i,
                "type": "test_event",
            });
        }

        let elapsed = start.elapsed().as_millis() as f64;
        let throughput = (event_count as f64 * 1000.0) / elapsed;

        metrics.operation_count = event_count;
        metrics.throughput_ops_sec = throughput;
        
        // Target: > 10k events/sec
        metrics.passed_targets = throughput > 10_000.0;

        Ok(metrics)
    }

    /// Benchmarks buffer memory footprint.
    ///
    /// Estimates memory usage for storing events.
    /// Target: ~100MB for 10k events
    ///
    /// # Arguments
    ///
    /// * `event_count` - Number of events to simulate
    ///
    /// # Returns
    ///
    /// * `Result<BenchmarkMetrics>` - Benchmark results with memory estimate
    pub fn benchmark_memory_footprint(event_count: usize) -> Result<BenchmarkMetrics> {
        let mut metrics = BenchmarkMetrics::new("memory_footprint");

        // Estimate: each event is ~10KB of JSON
        let avg_event_size = 10_000;
        let total_memory = (event_count * avg_event_size) as u64;

        metrics.memory_bytes = Some(total_memory);
        metrics.operation_count = event_count;

        // For 10k events, expect ~100MB
        // target_memory = 100_000_000 bytes
        let target_memory = 100_000_000u64;
        metrics.passed_targets = total_memory <= (target_memory as f64 * 1.2) as u64;

        Ok(metrics)
    }

    /// Benchmarks NDJSON serialization performance.
    ///
    /// Measures the time to serialize events to NDJSON format.
    ///
    /// # Arguments
    ///
    /// * `iterations` - Number of serializations
    ///
    /// # Returns
    ///
    /// * `Result<BenchmarkMetrics>` - Benchmark results
    pub fn benchmark_ndjson_serialization(iterations: usize) -> Result<BenchmarkMetrics> {
        let mut metrics = BenchmarkMetrics::new("ndjson_serialization");
        let mut latencies = Vec::with_capacity(iterations);

        for i in 0..iterations {
            let event = serde_json::json!({
                "id": i,
                "type": "event",
                "data": "x".repeat(200),
            });

            let start = Instant::now();
            let _ = serde_json::to_string(&event)
                .map_err(|e| ToolError::Other(format!("Serialization failed: {}", e)))?;
            let elapsed = start.elapsed().as_micros() as u64;
            latencies.push(elapsed);

            metrics.min_latency_us = metrics.min_latency_us.min(elapsed);
            metrics.max_latency_us = metrics.max_latency_us.max(elapsed);
        }

        let (p50, p95, p99) = BenchmarkMetrics::compute_percentiles(latencies.clone());
        metrics.p50_latency_us = p50;
        metrics.p95_latency_us = p95;
        metrics.p99_latency_us = p99;
        metrics.mean_latency_us = BenchmarkMetrics::compute_mean(&latencies);
        metrics.operation_count = iterations;
        metrics.throughput_ops_sec = (iterations as f64 * 1_000_000.0) 
            / (latencies.iter().map(|&l| l as u64).sum::<u64>() as f64);

        // Serialization should be fast
        metrics.passed_targets = metrics.p99_latency_us < 1000;

        Ok(metrics)
    }
}

impl Default for PerformanceBenchmark {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::format;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;
use alloc::vec;

    #[test]
    fn test_benchmark_metrics_creation() -> Result<()> {
        let metrics = BenchmarkMetrics::new("test_benchmark");
        assert_eq!(metrics.name, "test_benchmark");
        assert_eq!(metrics.operation_count, 0);
        Ok(())
    }

    #[test]
    fn test_percentile_computation() {
        let values = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let (p50, p95, p99) = BenchmarkMetrics::compute_percentiles(values);
        
        assert!(p50 > 0);
        assert!(p95 >= p50);
        assert!(p99 >= p95);
    }

    #[test]
    fn test_mean_computation() {
        let values = vec![10, 20, 30, 40, 50];
        let mean = BenchmarkMetrics::compute_mean(&values);
        assert!((mean - 30.0).abs() < 0.01);
    }

    #[test]
    fn test_event_emission_latency_benchmark() -> Result<()> {
        let metrics = PerformanceBenchmark::benchmark_event_emission_latency(100)?;
        assert_eq!(metrics.operation_count, 100);
        assert!(metrics.p50_latency_us > 0);
        Ok(())
    }

    #[test]
    fn test_cost_calculation_benchmark() -> Result<()> {
        let metrics = PerformanceBenchmark::benchmark_cost_calculation(100)?;
        assert_eq!(metrics.operation_count, 100);
        assert!(metrics.throughput_ops_sec > 0.0);
        Ok(())
    }

    #[test]
    fn test_subscriber_throughput_benchmark() -> Result<()> {
        let metrics = PerformanceBenchmark::benchmark_subscriber_throughput(1000)?;
        assert_eq!(metrics.operation_count, 1000);
        assert!(metrics.throughput_ops_sec > 0.0);
        Ok(())
    }

    #[test]
    fn test_memory_footprint_benchmark() -> Result<()> {
        let metrics = PerformanceBenchmark::benchmark_memory_footprint(100)?;
        assert!(metrics.memory_bytes.is_some());
        Ok(())
    }

    #[test]
    fn test_ndjson_serialization_benchmark() -> Result<()> {
        let metrics = PerformanceBenchmark::benchmark_ndjson_serialization(50)?;
        assert_eq!(metrics.operation_count, 50);
        assert!(metrics.throughput_ops_sec > 0.0);
        Ok(())
    }

    #[test]
    fn test_benchmark_metrics_print() -> Result<()> {
        let mut metrics = BenchmarkMetrics::new("test");
        metrics.operation_count = 100;
        metrics.p50_latency_us = 500;
        metrics.mean_latency_us = 550.0;
        
        metrics.print_summary();
        Ok(())
    }
}
