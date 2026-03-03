// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Memory Manager metrics collection and observability.
//!
//! This module provides comprehensive metrics collection for the Memory Manager,
//! enabling observability into tier utilization, eviction activity, memory pressure,
//! and request latency. Metrics are emitted as CEF (Common Event Format) events
//! for integration with security and observability systems.
//!
//! See Engineering Plan § 4.1.0: Observability & Metrics.

use alloc::string::String;
use alloc::vec::Vec;

/// Per-tier metrics - tracks usage and performance for a single tier.
///
/// Provides detailed visibility into L1, L2, or L3 usage patterns.
/// See Engineering Plan § 4.1.0: Tier Metrics.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TierMetrics {
    /// Tier name (L1, L2, or L3)
    pub tier_name: String,
    /// Bytes currently allocated in this tier
    pub allocated_bytes: u64,
    /// Bytes free in this tier
    pub free_bytes: u64,
    /// Fragmentation ratio (0.0 to 1.0) - percentage of free space that's fragmented
    pub fragmentation_ratio: u64, // 0-100 (percentage)
    /// Average latency in nanoseconds for operations on this tier
    pub avg_latency_ns: u64,
    /// Number of eviction operations performed
    pub eviction_count: u64,
    /// Number of allocation operations
    pub allocation_count: u64,
    /// Cache hit rate (0-100 percentage)
    pub hit_rate_percent: u64,
}

impl TierMetrics {
    /// Creates new tier metrics.
    pub fn new(tier_name: impl Into<String>) -> Self {
        TierMetrics {
            tier_name: tier_name.into(),
            allocated_bytes: 0,
            free_bytes: 0,
            fragmentation_ratio: 0,
            avg_latency_ns: 0,
            eviction_count: 0,
            allocation_count: 0,
            hit_rate_percent: 0,
        }
    }

    /// Returns utilization as a percentage.
    pub fn utilization_percent(&self) -> u64 {
        let total = self.allocated_bytes + self.free_bytes;
        if total == 0 {
            0
        } else {
            (self.allocated_bytes * 100) / total
        }
    }

    /// Returns available space in bytes.
    pub fn available_bytes(&self) -> u64 {
        self.free_bytes
    }

    /// Checks if tier is above a utilization threshold.
    pub fn is_above_threshold(&self, threshold_percent: u64) -> bool {
        self.utilization_percent() > threshold_percent
    }
}

/// Overall Memory Manager metrics snapshot.
///
/// Captures the complete state of memory tier usage, allocation rates,
/// eviction activity, and pressure indicators.
///
/// See Engineering Plan § 4.1.0: System Metrics.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryMetrics {
    /// Timestamp when snapshot was taken (milliseconds since epoch)
    pub timestamp_ms: u64,
    /// Per-tier metrics for L1, L2, L3
    pub tier_metrics: Vec<TierMetrics>,
    /// Current memory pressure level (0-100)
    pub pressure_level: u8,
    /// Allocation rate (allocations per second)
    pub allocation_rate_per_sec: u64,
    /// Eviction rate (evictions per second)
    pub eviction_rate_per_sec: u64,
    /// Average request latency in nanoseconds
    pub avg_request_latency_ns: u64,
    /// Total requests processed since startup
    pub total_requests: u64,
    /// Total bytes allocated since startup
    pub total_bytes_allocated: u64,
}

impl MemoryMetrics {
    /// Creates new memory metrics snapshot.
    pub fn new(timestamp_ms: u64, pressure: u8) -> Self {
        MemoryMetrics {
            timestamp_ms,
            tier_metrics: Vec::new(),
            pressure_level: pressure,
            allocation_rate_per_sec: 0,
            eviction_rate_per_sec: 0,
            avg_request_latency_ns: 0,
            total_requests: 0,
            total_bytes_allocated: 0,
        }
    }

    /// Adds a tier metrics snapshot.
    pub fn add_tier_metrics(&mut self, tier: TierMetrics) {
        self.tier_metrics.push(tier);
    }

    /// Returns total bytes allocated across all tiers.
    pub fn total_allocated_bytes(&self) -> u64 {
        self.tier_metrics
            .iter()
            .map(|t| t.allocated_bytes)
            .sum()
    }

    /// Returns total bytes free across all tiers.
    pub fn total_free_bytes(&self) -> u64 {
        self.tier_metrics
            .iter()
            .map(|t| t.free_bytes)
            .sum()
    }

    /// Returns overall system utilization (0-100).
    pub fn overall_utilization_percent(&self) -> u64 {
        let total_allocated = self.total_allocated_bytes();
        let total_free = self.total_free_bytes();
        let total = total_allocated + total_free;

        if total == 0 {
            0
        } else {
            (total_allocated * 100) / total
        }
    }

    /// Returns total number of evictions across all tiers.
    pub fn total_evictions(&self) -> u64 {
        self.tier_metrics
            .iter()
            .map(|t| t.eviction_count)
            .sum()
    }

    /// Returns total number of allocations across all tiers.
    pub fn total_allocations(&self) -> u64 {
        self.tier_metrics
            .iter()
            .map(|t| t.allocation_count)
            .sum()
    }

    /// Computes average hit rate across all tiers (weighted by allocation).
    pub fn weighted_avg_hit_rate(&self) -> u64 {
        let total_allocated = self.total_allocated_bytes();
        if total_allocated == 0 {
            0
        } else {
            let weighted_sum: u64 = self
                .tier_metrics
                .iter()
                .map(|t| t.hit_rate_percent * t.allocated_bytes)
                .sum();
            weighted_sum / total_allocated
        }
    }
}

/// CEF Event for memory access operations (for observability/security).
///
/// CEF (Common Event Format) is a standard for security event reporting.
/// This structure emits memory operations as security events.
///
/// See Engineering Plan § E06: CEF Event Format.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CefMemoryAccessEvent {
    /// Event version (CEF:0)
    pub version: u8,
    /// Device vendor name
    pub vendor: String,
    /// Device product name
    pub product: String,
    /// Device version
    pub device_version: String,
    /// Signature ID (unique event type)
    pub signature_id: String,
    /// Event name
    pub event_name: String,
    /// Event severity (0=Low, 1=Medium, 2=High, 3=Critical)
    pub severity: u8,
    /// Operation type (Allocate, Read, Write, Mount, Query, Evict)
    pub operation: String,
    /// Target region ID
    pub region_id: String,
    /// CT ID performing operation
    pub ct_id: String,
    /// Bytes affected
    pub bytes: u64,
    /// Operation latency in nanoseconds
    pub latency_ns: u64,
    /// Operation success (true) or failure (false)
    pub success: bool,
}

impl CefMemoryAccessEvent {
    /// Creates a new CEF memory access event.
    pub fn new(
        operation: impl Into<String>,
        region_id: impl Into<String>,
        ct_id: impl Into<String>,
        bytes: u64,
        latency_ns: u64,
        success: bool,
    ) -> Self {
        CefMemoryAccessEvent {
            version: 0,
            vendor: "CognitiveSubstrate".to_string(),
            product: "MemoryManager".to_string(),
            device_version: "1.0".to_string(),
            signature_id: "MEM_ACCESS".to_string(),
            event_name: "Memory Access".to_string(),
            severity: if success { 0 } else { 2 },
            operation: operation.into(),
            region_id: region_id.into(),
            ct_id: ct_id.into(),
            bytes,
            latency_ns,
            success,
        }
    }

    /// Formats this event as a CEF string for logging.
    pub fn to_cef_string(&self) -> String {
        alloc::format!(
            "CEF:{}|{}|{}|{}|{}|{}|{}|operation={} region_id={} ct_id={} bytes={} latency_ns={} success={}",
            self.version,
            self.vendor,
            self.product,
            self.device_version,
            self.signature_id,
            self.event_name,
            self.severity,
            self.operation,
            self.region_id,
            self.ct_id,
            self.bytes,
            self.latency_ns,
            self.success,
        )
    }
}

/// Metrics collector - accumulates metrics and produces snapshots.
///
/// Tracks memory operations and generates periodic metrics snapshots
/// for observability.
///
/// See Engineering Plan § 4.1.0: Metrics Collection.
pub struct MetricsCollector {
    /// L1 tier metrics
    pub l1_metrics: TierMetrics,
    /// L2 tier metrics
    pub l2_metrics: TierMetrics,
    /// L3 tier metrics
    pub l3_metrics: TierMetrics,
    /// Recent CEF events (last 100 or so)
    pub recent_events: Vec<CefMemoryAccessEvent>,
    /// Total requests since startup
    pub total_requests_ever: u64,
}

impl MetricsCollector {
    /// Creates a new metrics collector.
    pub fn new() -> Self {
        MetricsCollector {
            l1_metrics: TierMetrics::new("L1"),
            l2_metrics: TierMetrics::new("L2"),
            l3_metrics: TierMetrics::new("L3"),
            recent_events: Vec::new(),
            total_requests_ever: 0,
        }
    }

    /// Records a memory access event.
    pub fn record_event(&mut self, event: CefMemoryAccessEvent) {
        self.recent_events.push(event);
        self.total_requests_ever = self.total_requests_ever.saturating_add(1);

        // Keep only last 100 events
        if self.recent_events.len() > 100 {
            self.recent_events.remove(0);
        }
    }

    /// Collects a metrics snapshot.
    pub fn collect_snapshot(&self, timestamp_ms: u64, pressure: u8) -> MemoryMetrics {
        let mut metrics = MemoryMetrics::new(timestamp_ms, pressure);

        metrics.add_tier_metrics(self.l1_metrics.clone());
        metrics.add_tier_metrics(self.l2_metrics.clone());
        metrics.add_tier_metrics(self.l3_metrics.clone());

        metrics.total_requests = self.total_requests_ever;

        metrics
    }

    /// Resets all metrics to zero.
    pub fn reset(&mut self) {
        self.l1_metrics = TierMetrics::new("L1");
        self.l2_metrics = TierMetrics::new("L2");
        self.l3_metrics = TierMetrics::new("L3");
        self.recent_events.clear();
        self.total_requests_ever = 0;
    }

    /// Returns the count of events in the buffer.
    pub fn event_count(&self) -> usize {
        self.recent_events.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::format;
use alloc::string::ToString;

    #[test]
    fn test_tier_metrics_creation() {
        let metrics = TierMetrics::new("L1");
        assert_eq!(metrics.tier_name, "L1");
        assert_eq!(metrics.allocated_bytes, 0);
        assert_eq!(metrics.free_bytes, 0);
    }

    #[test]
    fn test_tier_metrics_utilization() {
        let mut metrics = TierMetrics::new("L1");
        metrics.allocated_bytes = 5000;
        metrics.free_bytes = 5000;
        assert_eq!(metrics.utilization_percent(), 50);

        let mut metrics_full = TierMetrics::new("L2");
        metrics_full.allocated_bytes = 10000;
        metrics_full.free_bytes = 0;
        assert_eq!(metrics_full.utilization_percent(), 100);

        let mut metrics_empty = TierMetrics::new("L3");
        metrics_empty.allocated_bytes = 0;
        metrics_empty.free_bytes = 10000;
        assert_eq!(metrics_empty.utilization_percent(), 0);
    }

    #[test]
    fn test_tier_metrics_available_bytes() {
        let mut metrics = TierMetrics::new("L1");
        metrics.free_bytes = 8192;
        assert_eq!(metrics.available_bytes(), 8192);
    }

    #[test]
    fn test_tier_metrics_is_above_threshold() {
        let mut metrics = TierMetrics::new("L1");
        metrics.allocated_bytes = 80;
        metrics.free_bytes = 20;

        assert!(metrics.is_above_threshold(75)); // 80% > 75%
        assert!(!metrics.is_above_threshold(85)); // 80% < 85%
    }

    #[test]
    fn test_memory_metrics_creation() {
        let metrics = MemoryMetrics::new(1000, 50);
        assert_eq!(metrics.timestamp_ms, 1000);
        assert_eq!(metrics.pressure_level, 50);
        assert_eq!(metrics.tier_metrics.len(), 0);
    }

    #[test]
    fn test_memory_metrics_add_tier() {
        let mut metrics = MemoryMetrics::new(1000, 50);
        let mut tier = TierMetrics::new("L1");
        tier.allocated_bytes = 5000;

        metrics.add_tier_metrics(tier);
        assert_eq!(metrics.tier_metrics.len(), 1);
        assert_eq!(metrics.total_allocated_bytes(), 5000);
    }

    #[test]
    fn test_memory_metrics_total_allocated() {
        let mut metrics = MemoryMetrics::new(1000, 50);

        let mut l1 = TierMetrics::new("L1");
        l1.allocated_bytes = 1000;
        metrics.add_tier_metrics(l1);

        let mut l2 = TierMetrics::new("L2");
        l2.allocated_bytes = 2000;
        metrics.add_tier_metrics(l2);

        assert_eq!(metrics.total_allocated_bytes(), 3000);
    }

    #[test]
    fn test_memory_metrics_overall_utilization() {
        let mut metrics = MemoryMetrics::new(1000, 50);

        let mut l1 = TierMetrics::new("L1");
        l1.allocated_bytes = 5000;
        l1.free_bytes = 5000;
        metrics.add_tier_metrics(l1);

        let mut l2 = TierMetrics::new("L2");
        l2.allocated_bytes = 5000;
        l2.free_bytes = 5000;
        metrics.add_tier_metrics(l2);

        assert_eq!(metrics.overall_utilization_percent(), 50);
    }

    #[test]
    fn test_memory_metrics_total_evictions() {
        let mut metrics = MemoryMetrics::new(1000, 50);

        let mut l1 = TierMetrics::new("L1");
        l1.eviction_count = 10;
        metrics.add_tier_metrics(l1);

        let mut l2 = TierMetrics::new("L2");
        l2.eviction_count = 20;
        metrics.add_tier_metrics(l2);

        assert_eq!(metrics.total_evictions(), 30);
    }

    #[test]
    fn test_memory_metrics_weighted_avg_hit_rate() {
        let mut metrics = MemoryMetrics::new(1000, 50);

        let mut l1 = TierMetrics::new("L1");
        l1.allocated_bytes = 1000;
        l1.hit_rate_percent = 100; // Perfect hit rate
        metrics.add_tier_metrics(l1);

        let mut l2 = TierMetrics::new("L2");
        l2.allocated_bytes = 1000;
        l2.hit_rate_percent = 50; // 50% hit rate
        metrics.add_tier_metrics(l2);

        assert_eq!(metrics.weighted_avg_hit_rate(), 75); // (100 + 50) / 2
    }

    #[test]
    fn test_cef_memory_access_event_creation() {
        let event = CefMemoryAccessEvent::new("Allocate", "region-001", "ct-001", 1024, 100, true);

        assert_eq!(event.operation, "Allocate");
        assert_eq!(event.region_id, "region-001");
        assert_eq!(event.ct_id, "ct-001");
        assert_eq!(event.bytes, 1024);
        assert_eq!(event.latency_ns, 100);
        assert!(event.success);
    }

    #[test]
    fn test_cef_memory_access_event_severity() {
        let success_event = CefMemoryAccessEvent::new("Read", "region-001", "ct-001", 512, 50, true);
        assert_eq!(success_event.severity, 0); // Low severity on success

        let failure_event = CefMemoryAccessEvent::new("Write", "region-001", "ct-001", 512, 50, false);
        assert_eq!(failure_event.severity, 2); // High severity on failure
    }

    #[test]
    fn test_cef_memory_access_event_to_cef_string() {
        let event = CefMemoryAccessEvent::new("Allocate", "region-001", "ct-001", 1024, 100, true);
        let cef_str = event.to_cef_string();

        assert!(cef_str.contains("CEF:0"));
        assert!(cef_str.contains("Allocate"));
        assert!(cef_str.contains("region-001"));
        assert!(cef_str.contains("ct-001"));
        assert!(cef_str.contains("1024"));
    }

    #[test]
    fn test_metrics_collector_creation() {
        let collector = MetricsCollector::new();
        assert_eq!(collector.l1_metrics.tier_name, "L1");
        assert_eq!(collector.l2_metrics.tier_name, "L2");
        assert_eq!(collector.l3_metrics.tier_name, "L3");
        assert_eq!(collector.recent_events.len(), 0);
    }

    #[test]
    fn test_metrics_collector_record_event() {
        let mut collector = MetricsCollector::new();
        let event = CefMemoryAccessEvent::new("Allocate", "region-001", "ct-001", 1024, 100, true);

        collector.record_event(event);
        assert_eq!(collector.event_count(), 1);
        assert_eq!(collector.total_requests_ever, 1);
    }

    #[test]
    fn test_metrics_collector_event_buffer_limit() {
        let mut collector = MetricsCollector::new();

        // Add 110 events
        for i in 0..110 {
            let event = CefMemoryAccessEvent::new(
                "Allocate",
                &alloc::format!("region-{:03}", i),
                "ct-001",
                1024,
                100,
                true,
            );
            collector.record_event(event);
        }

        // Should only keep last 100
        assert_eq!(collector.event_count(), 100);
        assert_eq!(collector.total_requests_ever, 110);
    }

    #[test]
    fn test_metrics_collector_collect_snapshot() {
        let mut collector = MetricsCollector::new();
        collector.l1_metrics.allocated_bytes = 2000;
        collector.l1_metrics.free_bytes = 2000;

        let snapshot = collector.collect_snapshot(1000, 50);
        assert_eq!(snapshot.timestamp_ms, 1000);
        assert_eq!(snapshot.pressure_level, 50);
        assert_eq!(snapshot.tier_metrics.len(), 3);
        assert_eq!(snapshot.total_allocated_bytes(), 2000);
    }

    #[test]
    fn test_metrics_collector_reset() {
        let mut collector = MetricsCollector::new();
        collector.l1_metrics.allocated_bytes = 2000;
        let event = CefMemoryAccessEvent::new("Allocate", "region-001", "ct-001", 1024, 100, true);
        collector.record_event(event);
        collector.total_requests_ever = 100;

        collector.reset();
        assert_eq!(collector.l1_metrics.allocated_bytes, 0);
        assert_eq!(collector.event_count(), 0);
        assert_eq!(collector.total_requests_ever, 0);
    }
}
