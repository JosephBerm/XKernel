// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! GPU Manager performance telemetry and monitoring hooks.
//!
//! Provides metrics collection, performance tracking, and observability
//! for GPU operations. Integrates with the Cognitive Substrate's observability
//! pipeline for monitoring and alerting.
//!
//! Reference: Engineering Plan § Observability, Performance Monitoring

use crate::ids::GpuDeviceID;
use alloc::vec::Vec;
use core::fmt;

/// GPU performance metric types.
///
/// Enumerates the different metrics collected by the GPU Manager.
/// Each metric provides insight into device operation and health.
///
/// Reference: Engineering Plan § Telemetry
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GpuMetric {
    /// TPC (Tensor Processing Cluster) utilization percentage (0-100).
    Utilization,

    /// VRAM usage in bytes.
    VramUsage,

    /// TPC activity count (number of active TPCs).
    TpcActivity,

    /// Kernel execution duration in nanoseconds.
    KernelDuration,

    /// Memory bandwidth utilization in GB/s.
    MemoryBandwidth,

    /// Power consumption in watts.
    PowerConsumption,

    /// Thermal state (temperature in Celsius).
    ThermalState,
}

impl fmt::Display for GpuMetric {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GpuMetric::Utilization => write!(f, "Utilization"),
            GpuMetric::VramUsage => write!(f, "VramUsage"),
            GpuMetric::TpcActivity => write!(f, "TpcActivity"),
            GpuMetric::KernelDuration => write!(f, "KernelDuration"),
            GpuMetric::MemoryBandwidth => write!(f, "MemoryBandwidth"),
            GpuMetric::PowerConsumption => write!(f, "PowerConsumption"),
            GpuMetric::ThermalState => write!(f, "ThermalState"),
        }
    }
}

/// Single metric sample collected at a point in time.
///
/// Represents one observation of a metric with associated metadata.
///
/// Reference: Engineering Plan § Metric Sampling
#[derive(Clone, Copy, Debug)]
pub struct MetricSample {
    /// Metric type
    pub metric: GpuMetric,

    /// Measured value (units depend on metric type)
    pub value: u64,

    /// Sample timestamp in nanoseconds since boot
    pub timestamp_ns: u64,

    /// GPU device this sample is from
    pub device_id: GpuDeviceID,
}

impl fmt::Display for MetricSample {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "MetricSample({}, value={}, ts={}ns, device={:?})",
            self.metric,
            self.value,
            self.timestamp_ns,
            self.device_id.as_bytes()[0]
        )
    }
}

/// GPU telemetry collector (trait-based interface).
///
/// Defines the operations for collecting GPU metrics and performance data.
/// Implementations can aggregate metrics, apply filters, and export data.
///
/// Reference: Engineering Plan § Telemetry Interface
pub trait GpuTelemetryCollector: core::fmt::Debug {
    /// Collect a single metric sample.
    ///
    /// Records a metric observation for later analysis.
    ///
    /// # Arguments
    ///
    /// * `sample` - Metric sample to record
    fn collect_metrics(&self, sample: MetricSample) -> Result<(), ()>;

    /// Get utilization history for a device.
    ///
    /// Returns recent utilization measurements (e.g., last 100 samples).
    ///
    /// # Arguments
    ///
    /// * `device_id` - GPU device to query
    fn get_utilization_history(&self, device_id: GpuDeviceID) -> Result<Vec<u64>, ()>;

    /// Get power profile (power consumption over time).
    ///
    /// Returns power consumption samples for detailed analysis.
    ///
    /// # Arguments
    ///
    /// * `device_id` - GPU device to query
    fn get_power_profile(&self, device_id: GpuDeviceID) -> Result<Vec<MetricSample>, ()>;
}

/// Latency breakdown for a single kernel execution.
///
/// Breaks down kernel execution time into phases for performance debugging.
/// Identifies bottlenecks in the GPU pipeline.
///
/// Reference: Engineering Plan § Latency Breakdown
#[derive(Clone, Copy, Debug)]
pub struct LatencyBreakdown {
    /// Time waiting in queue before execution (nanoseconds).
    pub queue_wait_ns: u64,

    /// Time spent allocating/preparing resources (nanoseconds).
    pub allocation_ns: u64,

    /// Time from allocation to kernel launch on GPU (nanoseconds).
    pub kernel_launch_ns: u64,

    /// Actual GPU kernel execution time (nanoseconds).
    pub execution_ns: u64,

    /// Time copying results back to host (nanoseconds).
    pub result_copy_ns: u64,
}

impl LatencyBreakdown {
    /// Get total latency (sum of all phases).
    pub fn total_ns(&self) -> u64 {
        self.queue_wait_ns + self.allocation_ns + self.kernel_launch_ns + self.execution_ns + self.result_copy_ns
    }

    /// Get dominant bottleneck (the phase taking most time).
    pub fn dominant_bottleneck(&self) -> &'static str {
        let mut max = self.queue_wait_ns;
        let mut bottleneck = "queue_wait";

        if self.allocation_ns > max {
            max = self.allocation_ns;
            bottleneck = "allocation";
        }
        if self.kernel_launch_ns > max {
            max = self.kernel_launch_ns;
            bottleneck = "kernel_launch";
        }
        if self.execution_ns > max {
            max = self.execution_ns;
            bottleneck = "execution";
        }
        if self.result_copy_ns > max {
            bottleneck = "result_copy";
        }

        bottleneck
    }
}

impl fmt::Display for LatencyBreakdown {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "LatencyBreakdown(queue={}ns, alloc={}ns, launch={}ns, exec={}ns, copy={}ns, total={}ns, bottleneck={})",
            self.queue_wait_ns,
            self.allocation_ns,
            self.kernel_launch_ns,
            self.execution_ns,
            self.result_copy_ns,
            self.total_ns(),
            self.dominant_bottleneck()
        )
    }
}

/// Performance alert condition.
///
/// Indicates problematic conditions detected during GPU operation.
/// Used for monitoring and alerting to scheduler.
///
/// Reference: Engineering Plan § Performance Alerts
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PerformanceAlert {
    /// Device is thermally throttling (temperature-limited performance).
    ///
    /// Indicates device is reducing frequency due to thermal constraints.
    /// May indicate insufficient cooling or excessive sustained load.
    ThermalThrottle,

    /// VRAM is under pressure (available memory low).
    ///
    /// Device is running low on available VRAM.
    /// New allocations may fail; existing workloads may be at risk.
    VramPressure,

    /// TPC starvation: TPCs allocated but no kernels to execute.
    ///
    /// Resources allocated but not utilized (opportunity cost).
    /// Suggests scheduler is over-provisioning for current workload.
    TpcStarvation,

    /// High kernel execution latency.
    ///
    /// Request latency exceeds expected threshold.
    /// May indicate GPU contention or resource constraints.
    HighLatency,

    /// Device health degradation (performance decline).
    ///
    /// Device performance dropping over time.
    /// May indicate hardware fault or degradation.
    HealthDegradation,
}

impl fmt::Display for PerformanceAlert {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PerformanceAlert::ThermalThrottle => write!(f, "ThermalThrottle"),
            PerformanceAlert::VramPressure => write!(f, "VramPressure"),
            PerformanceAlert::TpcStarvation => write!(f, "TpcStarvation"),
            PerformanceAlert::HighLatency => write!(f, "HighLatency"),
            PerformanceAlert::HealthDegradation => write!(f, "HealthDegradation"),
        }
    }
}

/// Performance alert with context.
///
/// Alert with associated metadata for debugging and response.
///
/// Reference: Engineering Plan § Alert Context
#[derive(Clone, Copy, Debug)]
pub struct PerformanceAlertContext {
    /// Alert type
    pub alert: PerformanceAlert,

    /// Device triggering alert
    pub device_id: GpuDeviceID,

    /// Alert timestamp in nanoseconds since boot
    pub timestamp_ns: u64,

    /// Severity (0=info, 1=warning, 2=critical)
    pub severity: u8,

    /// Associated metric value (context-dependent)
    pub metric_value: u64,
}

impl fmt::Display for PerformanceAlertContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Alert({}, device={:?}, severity={}, value={}, ts={}ns)",
            self.alert,
            self.device_id.as_bytes()[0],
            self.severity,
            self.metric_value,
            self.timestamp_ns
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::format;

    #[test]
    fn test_gpu_metric_display() {
        assert_eq!(format!("{}", GpuMetric::Utilization), "Utilization");
        assert_eq!(format!("{}", GpuMetric::PowerConsumption), "PowerConsumption");
        assert_eq!(format!("{}", GpuMetric::ThermalState), "ThermalState");
    }

    #[test]
    fn test_metric_sample_creation() {
        let device_id = GpuDeviceID::from_bytes([1u8; 16]);
        let sample = MetricSample {
            metric: GpuMetric::Utilization,
            value: 75,
            timestamp_ns: 1_000_000_000,
            device_id,
        };

        assert_eq!(sample.metric, GpuMetric::Utilization);
        assert_eq!(sample.value, 75);
    }

    #[test]
    fn test_metric_sample_display() {
        let device_id = GpuDeviceID::from_bytes([1u8; 16]);
        let sample = MetricSample {
            metric: GpuMetric::VramUsage,
            value: 40_000_000_000,
            timestamp_ns: 1_000_000_000,
            device_id,
        };

        let display_str = format!("{}", sample);
        assert!(display_str.contains("MetricSample"));
    }

    #[test]
    fn test_latency_breakdown_creation() {
        let breakdown = LatencyBreakdown {
            queue_wait_ns: 100_000,
            allocation_ns: 50_000,
            kernel_launch_ns: 75_000,
            execution_ns: 5_000_000,
            result_copy_ns: 25_000,
        };

        assert_eq!(breakdown.total_ns(), 5_250_000);
    }

    #[test]
    fn test_latency_breakdown_dominant_bottleneck() {
        let breakdown = LatencyBreakdown {
            queue_wait_ns: 100_000,
            allocation_ns: 50_000,
            kernel_launch_ns: 75_000,
            execution_ns: 5_000_000,
            result_copy_ns: 25_000,
        };

        assert_eq!(breakdown.dominant_bottleneck(), "execution");
    }

    #[test]
    fn test_latency_breakdown_allocation_bottleneck() {
        let breakdown = LatencyBreakdown {
            queue_wait_ns: 100_000,
            allocation_ns: 10_000_000,
            kernel_launch_ns: 75_000,
            execution_ns: 500_000,
            result_copy_ns: 25_000,
        };

        assert_eq!(breakdown.dominant_bottleneck(), "allocation");
    }

    #[test]
    fn test_latency_breakdown_queue_bottleneck() {
        let breakdown = LatencyBreakdown {
            queue_wait_ns: 20_000_000,
            allocation_ns: 50_000,
            kernel_launch_ns: 75_000,
            execution_ns: 500_000,
            result_copy_ns: 25_000,
        };

        assert_eq!(breakdown.dominant_bottleneck(), "queue_wait");
    }

    #[test]
    fn test_latency_breakdown_display() {
        let breakdown = LatencyBreakdown {
            queue_wait_ns: 100_000,
            allocation_ns: 50_000,
            kernel_launch_ns: 75_000,
            execution_ns: 5_000_000,
            result_copy_ns: 25_000,
        };

        let display_str = format!("{}", breakdown);
        assert!(display_str.contains("LatencyBreakdown"));
        assert!(display_str.contains("execution"));
    }

    #[test]
    fn test_performance_alert_display() {
        assert_eq!(format!("{}", PerformanceAlert::ThermalThrottle), "ThermalThrottle");
        assert_eq!(format!("{}", PerformanceAlert::VramPressure), "VramPressure");
        assert_eq!(format!("{}", PerformanceAlert::TpcStarvation), "TpcStarvation");
    }

    #[test]
    fn test_performance_alert_context_creation() {
        let device_id = GpuDeviceID::from_bytes([1u8; 16]);
        let alert_context = PerformanceAlertContext {
            alert: PerformanceAlert::ThermalThrottle,
            device_id,
            timestamp_ns: 1_000_000_000,
            severity: 2,
            metric_value: 85,
        };

        assert_eq!(alert_context.alert, PerformanceAlert::ThermalThrottle);
        assert_eq!(alert_context.severity, 2);
    }

    #[test]
    fn test_performance_alert_context_display() {
        let device_id = GpuDeviceID::from_bytes([1u8; 16]);
        let alert_context = PerformanceAlertContext {
            alert: PerformanceAlert::VramPressure,
            device_id,
            timestamp_ns: 1_000_000_000,
            severity: 1,
            metric_value: 95,
        };

        let display_str = format!("{}", alert_context);
        assert!(display_str.contains("Alert"));
        assert!(display_str.contains("VramPressure"));
    }

    #[test]
    fn test_metric_sample_various_metrics() {
        let device_id = GpuDeviceID::from_bytes([1u8; 16]);

        let sample1 = MetricSample {
            metric: GpuMetric::PowerConsumption,
            value: 450,
            timestamp_ns: 1_000_000_000,
            device_id,
        };

        let sample2 = MetricSample {
            metric: GpuMetric::MemoryBandwidth,
            value: 800, // GB/s
            timestamp_ns: 1_000_001_000,
            device_id,
        };

        assert_eq!(sample1.metric, GpuMetric::PowerConsumption);
        assert_eq!(sample2.metric, GpuMetric::MemoryBandwidth);
    }

    #[test]
    fn test_latency_breakdown_zero_latencies() {
        let breakdown = LatencyBreakdown {
            queue_wait_ns: 0,
            allocation_ns: 0,
            kernel_launch_ns: 0,
            execution_ns: 0,
            result_copy_ns: 0,
        };

        assert_eq!(breakdown.total_ns(), 0);
    }

    #[test]
    fn test_performance_alerts_equality() {
        assert_eq!(PerformanceAlert::ThermalThrottle, PerformanceAlert::ThermalThrottle);
        assert_ne!(PerformanceAlert::ThermalThrottle, PerformanceAlert::VramPressure);
    }
}
