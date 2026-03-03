// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! # Cognitive Substrate Profiler (cs-profile)
//!
//! The cs-profile crate provides performance profiling and analysis for Cognitive Substrate,
//! including CPU profiling, memory analysis, and flamegraph generation.
//!
//! ## Architecture
//!
//! This crate defines:
//! - **ProfileSession**: Profiling session management
//! - **ProfileMetrics**: Performance metric collection
//! - **Flamegraph**: Call stack visualization data

#![no_std]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use serde::{Deserialize, Serialize};
use alloc::format;
use alloc::string::ToString;

/// Profile metric type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MetricType {
    /// CPU cycles
    CpuCycles,
    /// CPU time in nanoseconds
    CpuTime,
    /// Memory allocated in bytes
    MemoryAllocated,
    /// Memory freed in bytes
    MemoryFreed,
    /// Cache hits
    CacheHits,
    /// Cache misses
    CacheMisses,
    /// Branch predictions
    BranchPredictions,
    /// Branch mispredictions
    BranchMispredictions,
}

/// Individual metric sample
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricSample {
    /// Metric type
    pub metric_type: MetricType,
    /// Timestamp in nanoseconds
    pub timestamp_ns: u64,
    /// Value
    pub value: u64,
    /// Associated task ID
    pub task_id: Option<String>,
}

/// Performance metrics collection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileMetrics {
    /// Collected samples
    pub samples: Vec<MetricSample>,
    /// Aggregated statistics
    pub stats: BTreeMap<String, u64>,
}

impl ProfileMetrics {
    /// Create new metrics collection
    pub fn new() -> Self {
        ProfileMetrics {
            samples: Vec::new(),
            stats: BTreeMap::new(),
        }
    }

    /// Add sample
    pub fn add_sample(&mut self, sample: MetricSample) {
        self.samples.push(sample);
    }

    /// Get total CPU time in nanoseconds
    pub fn get_total_cpu_time(&self) -> u64 {
        self.samples.iter()
            .filter(|s| s.metric_type == MetricType::CpuTime)
            .map(|s| s.value)
            .sum()
    }

    /// Get peak memory allocation
    pub fn get_peak_memory(&self) -> u64 {
        self.samples.iter()
            .filter(|s| s.metric_type == MetricType::MemoryAllocated)
            .map(|s| s.value)
            .max()
            .unwrap_or(0)
    }

    /// Calculate statistics
    pub fn calculate_stats(&mut self) {
        self.stats.insert("total_cpu_time_ns".to_string(), self.get_total_cpu_time());
        self.stats.insert("peak_memory_bytes".to_string(), self.get_peak_memory());
        self.stats.insert("sample_count".to_string(), self.samples.len() as u64);

        let cache_hits: u64 = self.samples.iter()
            .filter(|s| s.metric_type == MetricType::CacheHits)
            .map(|s| s.value)
            .sum();
        self.stats.insert("cache_hits".to_string(), cache_hits);

        let cache_misses: u64 = self.samples.iter()
            .filter(|s| s.metric_type == MetricType::CacheMisses)
            .map(|s| s.value)
            .sum();
        self.stats.insert("cache_misses".to_string(), cache_misses);
    }
}

impl Default for ProfileMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Stack frame in call stack
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackFrame {
    /// Function or syscall name
    pub name: String,
    /// File location
    pub file: Option<String>,
    /// Line number
    pub line: Option<u32>,
    /// Sample count for this frame
    pub samples: u64,
}

/// Flamegraph data for call stack visualization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Flamegraph {
    /// Root frames
    pub root_frames: Vec<StackFrame>,
    /// Call tree depth
    pub depth: u32,
    /// Total samples
    pub total_samples: u64,
}

impl Flamegraph {
    /// Create new flamegraph
    pub fn new() -> Self {
        Flamegraph {
            root_frames: Vec::new(),
            depth: 0,
            total_samples: 0,
        }
    }

    /// Add frame
    pub fn add_frame(&mut self, frame: StackFrame) {
        if self.root_frames.is_empty() {
            self.depth = 1;
        }
        self.total_samples += frame.samples;
        self.root_frames.push(frame);
    }

    /// Get frame count
    pub fn frame_count(&self) -> usize {
        self.root_frames.len()
    }

    /// Get hot frames (sorted by sample count)
    pub fn get_hot_frames(&self, limit: usize) -> Vec<&StackFrame> {
        let mut frames: Vec<_> = self.root_frames.iter().collect();
        frames.sort_by(|a, b| b.samples.cmp(&a.samples));
        frames.into_iter().take(limit).collect()
    }
}

impl Default for Flamegraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Profiling session
#[derive(Debug)]
pub struct ProfileSession {
    /// Session ID
    pub session_id: String,
    /// Is profiling
    pub profiling: bool,
    /// Metrics
    pub metrics: ProfileMetrics,
    /// Flamegraph data
    pub flamegraph: Flamegraph,
}

impl ProfileSession {
    /// Create new profile session
    pub fn new(session_id: String) -> Self {
        ProfileSession {
            session_id,
            profiling: false,
            metrics: ProfileMetrics::new(),
            flamegraph: Flamegraph::new(),
        }
    }

    /// Start profiling
    pub fn start(&mut self) {
        self.profiling = true;
    }

    /// Stop profiling
    pub fn stop(&mut self) {
        self.profiling = false;
        self.metrics.calculate_stats();
    }

    /// Record metric
    pub fn record_metric(&mut self, sample: MetricSample) {
        if self.profiling {
            self.metrics.add_sample(sample);
        }
    }

    /// Add stack frame to flamegraph
    pub fn add_flamegraph_frame(&mut self, frame: StackFrame) {
        self.flamegraph.add_frame(frame);
    }

    /// Get CPU time percentage
    pub fn get_cpu_percentage(&self, task_id: &str) -> f32 {
        let task_time: u64 = self.metrics.samples.iter()
            .filter(|s| s.metric_type == MetricType::CpuTime && s.task_id.as_ref().map_or(false, |id| id == task_id))
            .map(|s| s.value)
            .sum();

        let total_time = self.metrics.get_total_cpu_time();
        if total_time == 0 {
            0.0
        } else {
            (task_time as f32 / total_time as f32) * 100.0
        }
    }

    /// Get report summary
    pub fn get_report_summary(&self) -> String {
        let mut report = String::new();
        report.push_str("Profile Report Summary\n");
        report.push_str("======================\n");

        if let Some(&total_cpu) = self.metrics.stats.get("total_cpu_time_ns") {
            report.push_str(&format!("Total CPU Time: {} ns\n", total_cpu));
        }

        if let Some(&peak_mem) = self.metrics.stats.get("peak_memory_bytes") {
            report.push_str(&format!("Peak Memory: {} bytes\n", peak_mem));
        }

        if let Some(&samples) = self.metrics.stats.get("sample_count") {
            report.push_str(&format!("Sample Count: {}\n", samples));
        }

        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_session_creation() {
        let session = ProfileSession::new("test_session".to_string());
        assert_eq!(session.session_id, "test_session");
        assert!(!session.profiling);
    }

    #[test]
    fn test_profile_start_stop() {
        let mut session = ProfileSession::new("test".to_string());
        session.start();
        assert!(session.profiling);

        session.stop();
        assert!(!session.profiling);
    }

    #[test]
    fn test_record_metric() {
        let mut session = ProfileSession::new("test".to_string());
        session.start();

        let sample = MetricSample {
            metric_type: MetricType::CpuTime,
            timestamp_ns: 1000,
            value: 1000,
            task_id: Some("task1".to_string()),
        };

        session.record_metric(sample);
        assert_eq!(session.metrics.samples.len(), 1);
    }

    #[test]
    fn test_no_record_when_stopped() {
        let mut session = ProfileSession::new("test".to_string());

        let sample = MetricSample {
            metric_type: MetricType::CpuTime,
            timestamp_ns: 1000,
            value: 1000,
            task_id: Some("task1".to_string()),
        };

        session.record_metric(sample);
        assert_eq!(session.metrics.samples.len(), 0);
    }

    #[test]
    fn test_get_total_cpu_time() {
        let mut metrics = ProfileMetrics::new();

        metrics.add_sample(MetricSample {
            metric_type: MetricType::CpuTime,
            timestamp_ns: 1000,
            value: 1000,
            task_id: None,
        });

        metrics.add_sample(MetricSample {
            metric_type: MetricType::CpuTime,
            timestamp_ns: 2000,
            value: 2000,
            task_id: None,
        });

        assert_eq!(metrics.get_total_cpu_time(), 3000);
    }

    #[test]
    fn test_get_peak_memory() {
        let mut metrics = ProfileMetrics::new();

        metrics.add_sample(MetricSample {
            metric_type: MetricType::MemoryAllocated,
            timestamp_ns: 1000,
            value: 1000,
            task_id: None,
        });

        metrics.add_sample(MetricSample {
            metric_type: MetricType::MemoryAllocated,
            timestamp_ns: 2000,
            value: 2000,
            task_id: None,
        });

        assert_eq!(metrics.get_peak_memory(), 2000);
    }

    #[test]
    fn test_calculate_stats() {
        let mut metrics = ProfileMetrics::new();

        metrics.add_sample(MetricSample {
            metric_type: MetricType::CpuTime,
            timestamp_ns: 1000,
            value: 1000,
            task_id: None,
        });

        metrics.calculate_stats();
        assert!(metrics.stats.contains_key("total_cpu_time_ns"));
        assert!(metrics.stats.contains_key("sample_count"));
    }

    #[test]
    fn test_flamegraph_creation() {
        let fg = Flamegraph::new();
        assert_eq!(fg.root_frames.len(), 0);
        assert_eq!(fg.total_samples, 0);
    }

    #[test]
    fn test_flamegraph_add_frame() {
        let mut fg = Flamegraph::new();

        let frame = StackFrame {
            name: "function_a".to_string(),
            file: Some("lib.rs".to_string()),
            line: Some(42),
            samples: 100,
        };

        fg.add_frame(frame);
        assert_eq!(fg.root_frames.len(), 1);
        assert_eq!(fg.total_samples, 100);
    }

    #[test]
    fn test_flamegraph_get_hot_frames() {
        let mut fg = Flamegraph::new();

        for i in 0..5 {
            let frame = StackFrame {
                name: format!("function_{}", i),
                file: None,
                line: None,
                samples: (i as u64 + 1) * 100,
            };
            fg.add_frame(frame);
        }

        let hot = fg.get_hot_frames(3);
        assert_eq!(hot.len(), 3);
        assert!(hot[0].samples >= hot[1].samples);
    }

    #[test]
    fn test_get_cpu_percentage() {
        let mut session = ProfileSession::new("test".to_string());
        session.start();

        session.record_metric(MetricSample {
            metric_type: MetricType::CpuTime,
            timestamp_ns: 1000,
            value: 1000,
            task_id: Some("task1".to_string()),
        });

        session.record_metric(MetricSample {
            metric_type: MetricType::CpuTime,
            timestamp_ns: 2000,
            value: 1000,
            task_id: Some("task2".to_string()),
        });

        session.stop();

        let pct = session.get_cpu_percentage("task1");
        assert!(pct > 0.0 && pct <= 100.0);
    }

    #[test]
    fn test_get_report_summary() {
        let mut session = ProfileSession::new("test".to_string());
        session.start();

        session.record_metric(MetricSample {
            metric_type: MetricType::CpuTime,
            timestamp_ns: 1000,
            value: 1000,
            task_id: None,
        });

        session.stop();

        let report = session.get_report_summary();
        assert!(report.contains("Profile Report Summary"));
        assert!(report.contains("Total CPU Time"));
    }

    #[test]
    fn test_metric_types() {
        let mt1 = MetricType::CpuTime;
        let mt2 = MetricType::CpuCycles;
        assert_ne!(mt1, mt2);
    }

    #[test]
    fn test_stack_frame() {
        let frame = StackFrame {
            name: "main".to_string(),
            file: Some("main.rs".to_string()),
            line: Some(1),
            samples: 500,
        };
        assert_eq!(frame.name, "main");
        assert_eq!(frame.samples, 500);
    }
}
