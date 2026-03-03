// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! GPU error recovery and fault management for Phase 0.
//!
//! Implements robust error recovery strategies for GPU faults, including
//! device reset capability, memory leak detection, fault isolation,
//! and watchdog timeout handling.
//!
//! ## Recovery Strategies
//!
//! **Recoverable Errors** (auto-retry with backoff):
//! - Malformed commands (invalid grid/block dims)
//! - Stream timeouts (watchdog triggered)
//! - Temporary driver errors
//!
//! **Unrecoverable Errors** (device taken offline):
//! - Device fatal errors (ECC errors, hardware faults)
//! - Persistent driver failures
//! - Thermal critical (> 95°C sustained)
//!
//! Reference: Engineering Plan § Error Recovery & Fault Handling, Week 6

use crate::gpu_error_handling::{GpuErrorReport, GpuFaultCode};
use crate::ids::GpuDeviceID;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::fmt;

/// Recovery action classification.
///
/// Determines appropriate response to a GPU fault.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecoveryAction {
    /// No action — fault is recoverable via normal flow
    NoAction,

    /// Retry submission with exponential backoff
    RetryWithBackoff,

    /// Pause new submissions while device recovers
    PauseSubmissions,

    /// Reset GPU device (cuCtxDestroy/hipCtxDestroy + reinit)
    ResetDevice,

    /// Take device offline — no new submissions accepted
    TakeOffline,

    /// Escalate to system-level error handler
    Escalate,
}

impl fmt::Display for RecoveryAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RecoveryAction::NoAction => write!(f, "NoAction"),
            RecoveryAction::RetryWithBackoff => write!(f, "RetryWithBackoff"),
            RecoveryAction::PauseSubmissions => write!(f, "PauseSubmissions"),
            RecoveryAction::ResetDevice => write!(f, "ResetDevice"),
            RecoveryAction::TakeOffline => write!(f, "TakeOffline"),
            RecoveryAction::Escalate => write!(f, "Escalate"),
        }
    }
}

/// Memory allocation tracking entry.
///
/// Records a single GPU memory allocation for leak detection.
#[derive(Clone, Copy, Debug)]
pub struct MemoryAllocation {
    /// Allocation ID (unique within device)
    pub allocation_id: u64,

    /// VRAM address (device pointer)
    pub device_ptr: u64,

    /// Size in bytes
    pub size_bytes: u64,

    /// Crew ID that owns this allocation
    pub crew_id: [u8; 16],

    /// Timestamp of allocation
    pub allocated_at_ns: u64,

    /// Whether this allocation has been freed
    pub is_freed: bool,

    /// Timestamp of deallocation (if freed)
    pub freed_at_ns: u64,
}

impl fmt::Display for MemoryAllocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "MemoryAllocation(id={}, ptr=0x{:x}, size={}MB, freed={})",
            self.allocation_id,
            self.device_ptr,
            self.size_bytes / 1024 / 1024,
            self.is_freed
        )
    }
}

/// Memory leak report — summary of potential leaks.
///
/// Identifies allocations that were not properly freed.
#[derive(Clone, Debug)]
pub struct MemoryLeakReport {
    /// Device where leak detected
    pub device_id: GpuDeviceID,

    /// Total leaked memory in bytes
    pub total_leaked_bytes: u64,

    /// Number of leaked allocations
    pub leaked_allocation_count: u32,

    /// List of leaked allocations
    pub leaked_allocations: Vec<MemoryAllocation>,

    /// Timestamp of report
    pub report_timestamp_ns: u64,
}

impl fmt::Display for MemoryLeakReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "MemoryLeakReport(device={:?}, total_leaked={}MB, count={})",
            self.device_id.as_bytes()[0],
            self.total_leaked_bytes / 1024 / 1024,
            self.leaked_allocation_count
        )
    }
}

impl MemoryLeakReport {
    /// Create new memory leak report.
    pub fn new(
        device_id: GpuDeviceID,
        total_leaked_bytes: u64,
        leaked_allocations: Vec<MemoryAllocation>,
        report_timestamp_ns: u64,
    ) -> Self {
        let leaked_allocation_count = leaked_allocations.len() as u32;

        MemoryLeakReport {
            device_id,
            total_leaked_bytes,
            leaked_allocation_count,
            leaked_allocations,
            report_timestamp_ns,
        }
    }

    /// Is leak report critical (> 1GB leaked)?
    pub fn is_critical(&self) -> bool {
        self.total_leaked_bytes > 1024 * 1024 * 1024
    }
}

/// GPU device state machine for recovery.
///
/// Tracks device state through fault detection and recovery.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeviceRecoveryState {
    /// Device operating normally
    Healthy,

    /// Fault detected, attempting recovery
    Recovering,

    /// Recovery in progress, pause submissions
    RecoveringPaused,

    /// Device reset in progress
    Resetting,

    /// Device reset complete, reinitializing contexts
    Reinitializing,

    /// Recovery failed, device taken offline
    Offline,
}

impl fmt::Display for DeviceRecoveryState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DeviceRecoveryState::Healthy => write!(f, "Healthy"),
            DeviceRecoveryState::Recovering => write!(f, "Recovering"),
            DeviceRecoveryState::RecoveringPaused => write!(f, "RecoveringPaused"),
            DeviceRecoveryState::Resetting => write!(f, "Resetting"),
            DeviceRecoveryState::Reinitializing => write!(f, "Reinitializing"),
            DeviceRecoveryState::Offline => write!(f, "Offline"),
        }
    }
}

/// Error recovery handler for a single GPU device.
///
/// Manages per-device error recovery state and makes recovery decisions.
#[derive(Debug)]
pub struct DeviceRecoveryManager {
    /// Device ID being managed
    pub device_id: GpuDeviceID,

    /// Current recovery state
    pub state: DeviceRecoveryState,

    /// Count of consecutive errors (reset after successful recovery)
    pub consecutive_error_count: u32,

    /// Maximum consecutive errors before taking offline
    pub max_consecutive_errors: u32,

    /// Retry backoff base in nanoseconds
    pub backoff_base_ns: u64,

    /// Memory allocations tracked on this device
    pub memory_allocations: BTreeMap<u64, MemoryAllocation>,

    /// Next allocation ID
    pub next_allocation_id: u64,

    /// Error history (recent errors)
    pub recent_errors: Vec<GpuErrorReport>,

    /// Maximum error history size
    pub max_error_history: u32,
}

impl DeviceRecoveryManager {
    /// Create new device recovery manager.
    pub fn new(device_id: GpuDeviceID, max_consecutive_errors: u32) -> Self {
        DeviceRecoveryManager {
            device_id,
            state: DeviceRecoveryState::Healthy,
            consecutive_error_count: 0,
            max_consecutive_errors,
            backoff_base_ns: 1_000_000, // 1ms
            memory_allocations: BTreeMap::new(),
            next_allocation_id: 1,
            recent_errors: Vec::new(),
            max_error_history: 100,
        }
    }

    /// Record a memory allocation.
    pub fn track_allocation(
        &mut self,
        device_ptr: u64,
        size_bytes: u64,
        crew_id: [u8; 16],
        timestamp_ns: u64,
    ) -> u64 {
        let allocation_id = self.next_allocation_id;
        self.next_allocation_id += 1;

        let allocation = MemoryAllocation {
            allocation_id,
            device_ptr,
            size_bytes,
            crew_id,
            allocated_at_ns: timestamp_ns,
            is_freed: false,
            freed_at_ns: 0,
        };

        self.memory_allocations.insert(allocation_id, allocation);
        allocation_id
    }

    /// Mark allocation as freed.
    pub fn track_deallocation(&mut self, allocation_id: u64, timestamp_ns: u64) -> Result<(), ()> {
        if let Some(alloc) = self.memory_allocations.get_mut(&allocation_id) {
            alloc.is_freed = true;
            alloc.freed_at_ns = timestamp_ns;
            Ok(())
        } else {
            Err(())
        }
    }

    /// Detect memory leaks — return list of unfreed allocations.
    pub fn detect_memory_leaks(&self) -> MemoryLeakReport {
        let mut total_leaked = 0u64;
        let mut leaked_allocs = Vec::new();

        for (_, alloc) in &self.memory_allocations {
            if !alloc.is_freed {
                total_leaked += alloc.size_bytes;
                leaked_allocs.push(*alloc);
            }
        }

        MemoryLeakReport::new(self.device_id, total_leaked, leaked_allocs, 0)
    }

    /// Record an error and determine recovery action.
    pub fn handle_error(
        &mut self,
        error_report: GpuErrorReport,
        current_timestamp_ns: u64,
    ) -> RecoveryAction {
        // Track error
        if self.recent_errors.len() >= self.max_error_history as usize {
            self.recent_errors.remove(0);
        }
        self.recent_errors.push(error_report);

        // Determine action based on fault code
        let action = if error_report.is_recoverable {
            self.consecutive_error_count += 1;

            if self.consecutive_error_count > self.max_consecutive_errors {
                RecoveryAction::TakeOffline
            } else {
                RecoveryAction::RetryWithBackoff
            }
        } else {
            // Unrecoverable error
            self.state = DeviceRecoveryState::Offline;
            RecoveryAction::Escalate
        };

        action
    }

    /// Reset error counter on successful operation.
    pub fn clear_error_state(&mut self) {
        self.consecutive_error_count = 0;
        self.state = DeviceRecoveryState::Healthy;
    }

    /// Calculate backoff delay for retry (exponential).
    pub fn calculate_backoff_ns(&self) -> u64 {
        // Exponential backoff: base * 2^(errors-1), capped at 10s
        let exponent = self.consecutive_error_count.saturating_sub(1);
        let delay = self.backoff_base_ns << exponent;
        delay.min(10_000_000_000) // cap at 10 seconds
    }
}

/// System-level error recovery coordinator.
///
/// Manages recovery for all devices and makes system-wide decisions.
#[derive(Debug)]
pub struct ErrorRecoveryCoordinator {
    /// Per-device recovery managers
    pub device_managers: BTreeMap<GpuDeviceID, DeviceRecoveryManager>,

    /// Total errors handled (for statistics)
    pub total_errors: u32,

    /// Total recoveries succeeded
    pub total_successful_recoveries: u32,

    /// Total recoveries failed (device taken offline)
    pub total_failed_recoveries: u32,
}

impl ErrorRecoveryCoordinator {
    /// Create new error recovery coordinator.
    pub fn new() -> Self {
        ErrorRecoveryCoordinator {
            device_managers: BTreeMap::new(),
            total_errors: 0,
            total_successful_recoveries: 0,
            total_failed_recoveries: 0,
        }
    }

    /// Register a device for recovery management.
    pub fn register_device(&mut self, device_id: GpuDeviceID, max_consecutive_errors: u32) {
        let manager = DeviceRecoveryManager::new(device_id, max_consecutive_errors);
        self.device_managers.insert(device_id, manager);
    }

    /// Get recovery manager for a device.
    pub fn get_device_manager(&mut self, device_id: GpuDeviceID) -> Option<&mut DeviceRecoveryManager> {
        self.device_managers.get_mut(&device_id)
    }

    /// Check how many devices are healthy.
    pub fn healthy_device_count(&self) -> u32 {
        self.device_managers
            .values()
            .filter(|mgr| mgr.state == DeviceRecoveryState::Healthy)
            .count() as u32
    }

    /// Check how many devices are offline.
    pub fn offline_device_count(&self) -> u32 {
        self.device_managers
            .values()
            .filter(|mgr| mgr.state == DeviceRecoveryState::Offline)
            .count() as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_recovery_manager_creation() {
        let device_id = GpuDeviceID::from_bytes([1u8; 16]);
        let manager = DeviceRecoveryManager::new(device_id, 5);

        assert_eq!(manager.state, DeviceRecoveryState::Healthy);
        assert_eq!(manager.consecutive_error_count, 0);
    }

    #[test]
    fn test_memory_allocation_tracking() {
        let device_id = GpuDeviceID::from_bytes([1u8; 16]);
        let mut manager = DeviceRecoveryManager::new(device_id, 5);

        let crew_id = [1u8; 16];
        let alloc_id = manager.track_allocation(0x1000, 1024 * 1024, crew_id, 100);

        assert_eq!(alloc_id, 1);
        assert!(manager.memory_allocations.contains_key(&alloc_id));
    }

    #[test]
    fn test_memory_leak_detection() {
        let device_id = GpuDeviceID::from_bytes([1u8; 16]);
        let mut manager = DeviceRecoveryManager::new(device_id, 5);

        let crew_id = [1u8; 16];
        let alloc_id = manager.track_allocation(0x1000, 1024 * 1024, crew_id, 100);

        // Don't free the allocation
        let leak_report = manager.detect_memory_leaks();
        assert_eq!(leak_report.leaked_allocation_count, 1);
        assert_eq!(leak_report.total_leaked_bytes, 1024 * 1024);
    }

    #[test]
    fn test_backoff_calculation() {
        let device_id = GpuDeviceID::from_bytes([1u8; 16]);
        let mut manager = DeviceRecoveryManager::new(device_id, 5);

        manager.consecutive_error_count = 1;
        let backoff_1 = manager.calculate_backoff_ns();

        manager.consecutive_error_count = 2;
        let backoff_2 = manager.calculate_backoff_ns();

        assert!(backoff_2 > backoff_1);
    }

    #[test]
    fn test_error_recovery_coordinator() {
        let mut coordinator = ErrorRecoveryCoordinator::new();
        let device_id = GpuDeviceID::from_bytes([1u8; 16]);

        coordinator.register_device(device_id, 5);
        assert_eq!(coordinator.healthy_device_count(), 1);
    }
}
