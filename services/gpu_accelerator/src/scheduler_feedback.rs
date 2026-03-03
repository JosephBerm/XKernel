// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! GPU Manager feedback to Cognitive Scheduler.
//!
//! Implements the feedback channel from GPU Manager to Cognitive Scheduler,
//! enabling informed CT (Computational Thread) placement decisions based on
//! real-time GPU resource utilization and thermal state.
//!
//! ## Feedback Loop
//!
//! ```
//! GPU Manager (during kernel execution)
//!     ├─ Monitor TPC utilization
//!     ├─ Track VRAM usage
//!     ├─ Sample thermal state
//!     ├─ Measure power consumption
//!     ↓
//! GpuUtilizationReport (emitted periodically)
//!     ↓
//! Cognitive Scheduler
//!     ├─ Receive report
//!     ├─ Update device state
//!     ├─ Adjust CT placement policy
//!     └─ May defer new CT submissions if GPU is at capacity
//! ```
//!
//! Reference: Engineering Plan § GPU → Scheduler Feedback, Week 6

use crate::ids::GpuDeviceID;
use alloc::vec::Vec;
use core::fmt;

/// Thermal state classification.
///
/// Categorizes device temperature for scheduler decision-making.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThermalState {
    /// Temperature < 50°C — Normal operation
    Normal,

    /// 50°C ≤ Temperature < 70°C — Elevated but safe
    Elevated,

    /// 70°C ≤ Temperature < 85°C — Hot, approaching throttle
    Hot,

    /// Temperature ≥ 85°C — Thermal throttling active
    Throttling,

    /// Temperature ≥ 95°C — Critical, performance severely reduced
    Critical,
}

impl fmt::Display for ThermalState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ThermalState::Normal => write!(f, "Normal"),
            ThermalState::Elevated => write!(f, "Elevated"),
            ThermalState::Hot => write!(f, "Hot"),
            ThermalState::Throttling => write!(f, "Throttling"),
            ThermalState::Critical => write!(f, "Critical"),
        }
    }
}

impl ThermalState {
    /// Determine thermal state from temperature in Celsius.
    pub fn from_temperature(temp_celsius: f64) -> Self {
        if temp_celsius < 50.0 {
            ThermalState::Normal
        } else if temp_celsius < 70.0 {
            ThermalState::Elevated
        } else if temp_celsius < 85.0 {
            ThermalState::Hot
        } else if temp_celsius < 95.0 {
            ThermalState::Throttling
        } else {
            ThermalState::Critical
        }
    }

    /// Is this state considered healthy for full performance?
    pub fn is_healthy(&self) -> bool {
        matches!(self, ThermalState::Normal | ThermalState::Elevated)
    }

    /// Get recommended submission rate adjustment (multiplier).
    /// 1.0 = normal rate, 0.5 = half rate, 0.0 = no submissions
    pub fn submission_rate_multiplier(&self) -> f64 {
        match self {
            ThermalState::Normal => 1.0,
            ThermalState::Elevated => 1.0,
            ThermalState::Hot => 0.75,
            ThermalState::Throttling => 0.5,
            ThermalState::Critical => 0.0,
        }
    }
}

/// GPU utilization snapshot — reported to Cognitive Scheduler.
///
/// Provides comprehensive view of device resource state for scheduler
/// decision-making on CT placement and prioritization.
///
/// Reference: Engineering Plan § Utilization Reporting
#[derive(Clone, Copy, Debug)]
pub struct GpuUtilizationReport {
    /// GPU device ID that generated this report
    pub device_id: GpuDeviceID,

    /// TPC (Tensor Processing Cluster) utilization (0-100%)
    pub tpc_utilization_percent: u32,

    /// Memory bandwidth utilization (0-100%)
    pub memory_bandwidth_percent: u32,

    /// VRAM usage in bytes
    pub vram_used_bytes: u64,

    /// VRAM capacity in bytes
    pub vram_capacity_bytes: u64,

    /// GPU temperature in Celsius
    pub temperature_celsius: f64,

    /// Thermal state classification
    pub thermal_state: ThermalState,

    /// Power consumption in watts
    pub power_consumption_w: f64,

    /// Number of active kernels on GPU
    pub active_kernel_count: u32,

    /// Number of idle TPCs
    pub idle_tpc_count: u32,

    /// Timestamp of report in nanoseconds since boot
    pub timestamp_ns: u64,

    /// Report sequence number (monotonically increasing)
    pub report_sequence: u64,
}

impl fmt::Display for GpuUtilizationReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "GpuUtilizationReport(device={:?}, tpc={}%, vram={}MB/{}, thermal={}, active_kernels={})",
            self.device_id.as_bytes()[0],
            self.tpc_utilization_percent,
            self.vram_used_bytes / 1024 / 1024,
            self.vram_capacity_bytes / 1024 / 1024,
            self.thermal_state,
            self.active_kernel_count
        )
    }
}

impl GpuUtilizationReport {
    /// Create new utilization report.
    pub fn new(
        device_id: GpuDeviceID,
        tpc_utilization_percent: u32,
        memory_bandwidth_percent: u32,
        vram_used_bytes: u64,
        vram_capacity_bytes: u64,
        temperature_celsius: f64,
        power_consumption_w: f64,
        active_kernel_count: u32,
        idle_tpc_count: u32,
        timestamp_ns: u64,
        report_sequence: u64,
    ) -> Self {
        let thermal_state = ThermalState::from_temperature(temperature_celsius);

        GpuUtilizationReport {
            device_id,
            tpc_utilization_percent,
            memory_bandwidth_percent,
            vram_used_bytes,
            vram_capacity_bytes,
            temperature_celsius,
            thermal_state,
            power_consumption_w,
            active_kernel_count,
            idle_tpc_count,
            timestamp_ns,
            report_sequence,
        }
    }

    /// VRAM utilization percentage
    pub fn vram_utilization_percent(&self) -> u32 {
        if self.vram_capacity_bytes == 0 {
            return 0;
        }
        ((self.vram_used_bytes as f64 / self.vram_capacity_bytes as f64) * 100.0) as u32
    }

    /// Is device available for new submissions?
    pub fn can_accept_submissions(&self) -> bool {
        // Device is oversubscribed if:
        // - TPC utilization > 95% AND
        // - VRAM utilization > 90% AND
        // - thermal state is critical
        if self.tpc_utilization_percent > 95
            && self.vram_utilization_percent() > 90
            && self.thermal_state == ThermalState::Critical
        {
            return false;
        }

        true
    }

    /// Recommended priority adjustment for new submissions.
    /// 1.0 = normal, 0.5 = lower priority, 2.0 = higher priority
    pub fn priority_adjustment(&self) -> f64 {
        // Reduce priority if device is hot
        match self.thermal_state {
            ThermalState::Normal | ThermalState::Elevated => 1.0,
            ThermalState::Hot => 0.75,
            ThermalState::Throttling => 0.5,
            ThermalState::Critical => 0.25,
        }
    }
}

/// Scheduler feedback message from GPU Manager.
///
/// Container for all feedback data sent to Cognitive Scheduler.
#[derive(Clone, Debug)]
pub struct SchedulerFeedbackMessage {
    /// Device reports (one per GPU)
    pub device_reports: Vec<GpuUtilizationReport>,

    /// Timestamp of message in nanoseconds
    pub timestamp_ns: u64,

    /// Sequence number of this feedback batch
    pub batch_sequence: u64,

    /// Requested action by GPU Manager (None = no urgent action needed)
    pub requested_action: Option<SchedulerAction>,
}

impl fmt::Display for SchedulerFeedbackMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SchedulerFeedbackMessage(devices={}, action={:?})",
            self.device_reports.len(),
            self.requested_action
        )
    }
}

impl SchedulerFeedbackMessage {
    /// Create new feedback message.
    pub fn new(
        device_reports: Vec<GpuUtilizationReport>,
        timestamp_ns: u64,
        batch_sequence: u64,
    ) -> Self {
        SchedulerFeedbackMessage {
            device_reports,
            timestamp_ns,
            batch_sequence,
            requested_action: None,
        }
    }

    /// Set requested action.
    pub fn with_action(mut self, action: SchedulerAction) -> Self {
        self.requested_action = Some(action);
        self
    }
}

/// Action requested by GPU Manager to Cognitive Scheduler.
#[derive(Clone, Copy, Debug)]
pub enum SchedulerAction {
    /// No action required — normal operation
    NoAction,

    /// Device is overheating — reduce submission rate or defer CTs
    ThermalThrottle,

    /// Device VRAM is exhausted — defer new CT allocations
    VramExhausted,

    /// Device experienced recoverable error — may retry submissions
    RecoverableError,

    /// Device experienced unrecoverable error — take offline
    FatalError,
}

impl fmt::Display for SchedulerAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SchedulerAction::NoAction => write!(f, "NoAction"),
            SchedulerAction::ThermalThrottle => write!(f, "ThermalThrottle"),
            SchedulerAction::VramExhausted => write!(f, "VramExhausted"),
            SchedulerAction::RecoverableError => write!(f, "RecoverableError"),
            SchedulerAction::FatalError => write!(f, "FatalError"),
        }
    }
}

/// GPU Manager feedback generator — emits reports to Scheduler.
///
/// Periodically samples GPU state and generates utilization reports
/// for consumption by Cognitive Scheduler.
#[derive(Debug)]
pub struct FeedbackGenerator {
    /// Last report sequence number sent
    last_report_sequence: u64,

    /// Last batch sequence number
    last_batch_sequence: u64,

    /// Reporting interval in nanoseconds (e.g., 100ms)
    report_interval_ns: u64,

    /// Last report timestamp
    last_report_timestamp_ns: u64,
}

impl FeedbackGenerator {
    /// Create new feedback generator.
    pub fn new(report_interval_ns: u64) -> Self {
        FeedbackGenerator {
            last_report_sequence: 0,
            last_batch_sequence: 0,
            report_interval_ns,
            last_report_timestamp_ns: 0,
        }
    }

    /// Check if it's time to generate a report (interval elapsed).
    pub fn should_report(&self, current_ns: u64) -> bool {
        current_ns - self.last_report_timestamp_ns >= self.report_interval_ns
    }

    /// Generate next report sequence number.
    pub fn next_report_sequence(&mut self) -> u64 {
        self.last_report_sequence += 1;
        self.last_report_sequence
    }

    /// Generate next batch sequence number.
    pub fn next_batch_sequence(&mut self) -> u64 {
        self.last_batch_sequence += 1;
        self.last_batch_sequence
    }

    /// Update last report timestamp.
    pub fn update_timestamp(&mut self, timestamp_ns: u64) {
        self.last_report_timestamp_ns = timestamp_ns;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thermal_state_from_temperature() {
        assert_eq!(ThermalState::from_temperature(40.0), ThermalState::Normal);
        assert_eq!(ThermalState::from_temperature(60.0), ThermalState::Elevated);
        assert_eq!(ThermalState::from_temperature(80.0), ThermalState::Hot);
        assert_eq!(ThermalState::from_temperature(90.0), ThermalState::Throttling);
        assert_eq!(ThermalState::from_temperature(100.0), ThermalState::Critical);
    }

    #[test]
    fn test_thermal_state_health_check() {
        assert!(ThermalState::Normal.is_healthy());
        assert!(ThermalState::Elevated.is_healthy());
        assert!(!ThermalState::Hot.is_healthy());
        assert!(!ThermalState::Throttling.is_healthy());
        assert!(!ThermalState::Critical.is_healthy());
    }

    #[test]
    fn test_submission_rate_multiplier() {
        assert_eq!(ThermalState::Normal.submission_rate_multiplier(), 1.0);
        assert_eq!(ThermalState::Elevated.submission_rate_multiplier(), 1.0);
        assert_eq!(ThermalState::Hot.submission_rate_multiplier(), 0.75);
        assert_eq!(ThermalState::Throttling.submission_rate_multiplier(), 0.5);
        assert_eq!(ThermalState::Critical.submission_rate_multiplier(), 0.0);
    }

    #[test]
    fn test_gpu_utilization_report_creation() {
        let device_id = GpuDeviceID::from_bytes([1u8; 16]);
        let report = GpuUtilizationReport::new(
            device_id,
            85,
            75,
            8 * 1024 * 1024 * 1024,
            16 * 1024 * 1024 * 1024,
            75.0,
            250.0,
            32,
            8,
            1000,
            1,
        );

        assert_eq!(report.vram_utilization_percent(), 50);
        assert!(report.can_accept_submissions());
        assert_eq!(report.priority_adjustment(), 0.75);
    }

    #[test]
    fn test_feedback_generator() {
        let mut gen_val = FeedbackGenerator::new(100_000_000); // 100ms interval

        assert!(gen_val.should_report(200_000_000));
        assert!(!gen_val.should_report(150_000_000));

        let seq1 = gen_val.next_report_sequence();
        let seq2 = gen_val.next_report_sequence();
        assert_eq!(seq2, seq1 + 1);
    }
}
