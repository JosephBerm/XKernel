// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! GPU error detection, reporting, and recovery.
//!
//! Implements malformed command detection, stream query timeouts, GPU fault reporting,
//! and error recovery mechanisms. Enables robust handling of GPU failures and invalid
//! operations while maintaining system stability.
//!
//! ## Error Categories
//!
//! - **Malformed Commands**: Invalid grid/block dims, invalid kernel function handles
//! - **Stream Timeouts**: Kernels exceeding deadline, stream hangs
//! - **GPU Faults**: Device errors, ECC errors, memory errors
//! - **Driver Errors**: CUDA/HIP API failures
//! - **Isolation Violations**: Unauthorized VRAM access attempts
//!
//! Reference: Engineering Plan § Error Handling, Week 5 Addendum v2.5.1

use crate::command_queue::SubmissionId;
use crate::error::GpuError;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::fmt;

/// Error code for GPU fault detection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GpuFaultCode {
    /// Invalid grid dimensions (0 or exceeds device limits).
    InvalidGridDims,

    /// Invalid block dimensions (0, exceeds 1024, or >3 dimensions used).
    InvalidBlockDims,

    /// Kernel function handle not found in registry.
    InvalidFunctionHandle,

    /// Shared memory exceeds device limit.
    ExcessiveSharedMemory,

    /// Kernel execution exceeded deadline.
    DeadlineExceeded,

    /// GPU stream query timeout (kernel hang).
    StreamTimeout,

    /// GPU device error (unrecoverable device fault).
    DeviceError,

    /// ECC error detected in VRAM.
    EccError,

    /// Memory access violation (isolation boundary crossed).
    MemoryAccessViolation,

    /// Driver error (CUDA/HIP API failure).
    DriverError,

    /// Unknown error (driver-specific or undocumented).
    UnknownError,
}

impl fmt::Display for GpuFaultCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GpuFaultCode::InvalidGridDims => write!(f, "InvalidGridDims"),
            GpuFaultCode::InvalidBlockDims => write!(f, "InvalidBlockDims"),
            GpuFaultCode::InvalidFunctionHandle => write!(f, "InvalidFunctionHandle"),
            GpuFaultCode::ExcessiveSharedMemory => write!(f, "ExcessiveSharedMemory"),
            GpuFaultCode::DeadlineExceeded => write!(f, "DeadlineExceeded"),
            GpuFaultCode::StreamTimeout => write!(f, "StreamTimeout"),
            GpuFaultCode::DeviceError => write!(f, "DeviceError"),
            GpuFaultCode::EccError => write!(f, "EccError"),
            GpuFaultCode::MemoryAccessViolation => write!(f, "MemoryAccessViolation"),
            GpuFaultCode::DriverError => write!(f, "DriverError"),
            GpuFaultCode::UnknownError => write!(f, "UnknownError"),
        }
    }
}

impl GpuFaultCode {
    /// Check if this fault is recoverable.
    ///
    /// Recoverable faults: malformed commands, timeouts (with retry)
    /// Unrecoverable: device errors, ECC errors
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            GpuFaultCode::InvalidGridDims
                | GpuFaultCode::InvalidBlockDims
                | GpuFaultCode::InvalidFunctionHandle
                | GpuFaultCode::ExcessiveSharedMemory
                | GpuFaultCode::DeadlineExceeded
                | GpuFaultCode::StreamTimeout
        )
    }

    /// Get human-readable error message.
    pub fn message(&self) -> &'static str {
        match self {
            GpuFaultCode::InvalidGridDims => "Invalid grid dimensions",
            GpuFaultCode::InvalidBlockDims => "Invalid block dimensions",
            GpuFaultCode::InvalidFunctionHandle => "Kernel function not found",
            GpuFaultCode::ExcessiveSharedMemory => "Shared memory exceeds device limit",
            GpuFaultCode::DeadlineExceeded => "Kernel deadline exceeded",
            GpuFaultCode::StreamTimeout => "GPU stream timeout (kernel hang)",
            GpuFaultCode::DeviceError => "GPU device error",
            GpuFaultCode::EccError => "ECC error in VRAM",
            GpuFaultCode::MemoryAccessViolation => "Memory access violation (isolation)",
            GpuFaultCode::DriverError => "GPU driver error",
            GpuFaultCode::UnknownError => "Unknown GPU error",
        }
    }
}

/// GPU error report.
///
/// Detailed error information for logging, monitoring, and recovery decisions.
#[derive(Clone, Copy, Debug)]
pub struct GpuErrorReport {
    /// Fault code classification.
    pub fault_code: GpuFaultCode,

    /// Submission ID that triggered the error (if applicable).
    pub submission_id: Option<SubmissionId>,

    /// Crew identifier.
    pub crew_id: [u8; 16],

    /// GPU device ordinal.
    pub device_ordinal: u32,

    /// Timestamp of error in nanoseconds.
    pub error_time_ns: u64,

    /// Recoverable flag (from fault code).
    pub is_recoverable: bool,

    /// Additional context (kernel name, function handle, etc.).
    pub context: [u8; 256],
}

impl fmt::Display for GpuErrorReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "GpuErrorReport(fault={}, crew={:?}, device={}, recoverable={})",
            self.fault_code, &self.crew_id[..4], self.device_ordinal, self.is_recoverable
        )
    }
}

/// Recovery action for a failed GPU operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecoveryAction {
    /// Retry the operation (up to max retries).
    Retry,

    /// Quarantine the crew (disable future submissions).
    QuarantineCrew,

    /// Reset the GPU device.
    ResetDevice,

    /// Migrate workload to different GPU.
    MigrateToDevice,

    /// Terminate the crew and cleanup.
    TerminateCrew,

    /// No recovery (log only).
    NoRecovery,
}

impl fmt::Display for RecoveryAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RecoveryAction::Retry => write!(f, "Retry"),
            RecoveryAction::QuarantineCrew => write!(f, "QuarantineCrew"),
            RecoveryAction::ResetDevice => write!(f, "ResetDevice"),
            RecoveryAction::MigrateToDevice => write!(f, "MigrateToDevice"),
            RecoveryAction::TerminateCrew => write!(f, "TerminateCrew"),
            RecoveryAction::NoRecovery => write!(f, "NoRecovery"),
        }
    }
}

/// Fault recovery strategy.
#[derive(Clone, Debug)]
pub struct RecoveryStrategy {
    /// Action to take.
    pub action: RecoveryAction,

    /// Maximum number of retries (for Retry action).
    pub max_retries: u32,

    /// Target device for migration (for MigrateToDevice action).
    pub target_device: u32,

    /// Additional context for recovery.
    pub context: [u8; 128],
}

impl RecoveryStrategy {
    /// Create a recovery strategy for a fault.
    pub fn for_fault(fault_code: GpuFaultCode) -> Self {
        let action = match fault_code {
            GpuFaultCode::InvalidGridDims
            | GpuFaultCode::InvalidBlockDims
            | GpuFaultCode::InvalidFunctionHandle
            | GpuFaultCode::ExcessiveSharedMemory => RecoveryAction::NoRecovery,

            GpuFaultCode::DeadlineExceeded | GpuFaultCode::StreamTimeout => {
                RecoveryAction::Retry
            }

            GpuFaultCode::DeviceError => RecoveryAction::ResetDevice,
            GpuFaultCode::EccError => RecoveryAction::QuarantineCrew,
            GpuFaultCode::MemoryAccessViolation => RecoveryAction::TerminateCrew,
            GpuFaultCode::DriverError => RecoveryAction::ResetDevice,
            GpuFaultCode::UnknownError => RecoveryAction::Retry,
        };

        RecoveryStrategy {
            action,
            max_retries: if action == RecoveryAction::Retry { 3 } else { 0 },
            target_device: 0,
            context: [0u8; 128],
        }
    }
}

impl fmt::Display for RecoveryStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RecoveryStrategy(action={}, retries={})", self.action, self.max_retries)
    }
}

/// GPU error handler and recovery manager.
///
/// Detects, reports, and manages recovery from GPU errors.
/// Maintains error history and coordinates with GPU Manager for recovery.
///
/// **Responsibilities:**
/// - Validate kernel configurations (malformed command detection)
/// - Monitor stream timeouts (cuStreamQuery / hipStreamQuery)
/// - Report GPU faults (device errors, ECC errors)
/// - Determine recovery actions
/// - Coordinate recovery execution
///
/// Reference: Engineering Plan § Error Handling & Recovery
#[derive(Debug)]
pub struct GpuErrorHandler {
    /// Error history (recent errors for analysis).
    error_history: Vec<GpuErrorReport>,

    /// Quarantined crews (disabled due to repeated errors).
    quarantined_crews: Vec<[u8; 16]>,

    /// Failed submissions awaiting retry.
    retry_queue: BTreeMap<SubmissionId, (u32, RecoveryStrategy)>, // (retry_count, strategy)

    /// Statistics.
    pub stats: ErrorHandlingStats,

    /// Maximum error history size.
    max_history: u32,
}

/// Statistics for error handling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ErrorHandlingStats {
    /// Total errors detected.
    pub total_errors: u64,

    /// Errors by category.
    pub malformed_commands: u64,
    pub stream_timeouts: u64,
    pub device_errors: u64,
    pub ecc_errors: u64,
    pub driver_errors: u64,

    /// Recovery statistics.
    pub total_retries: u64,
    pub successful_recoveries: u64,
    pub failed_recoveries: u64,

    /// Quarantined crews.
    pub quarantined_crew_count: u64,
}

impl GpuErrorHandler {
    /// Create a new GPU error handler.
    pub fn new() -> Self {
        GpuErrorHandler {
            error_history: Vec::new(),
            quarantined_crews: Vec::new(),
            retry_queue: BTreeMap::new(),
            stats: ErrorHandlingStats {
                total_errors: 0,
                malformed_commands: 0,
                stream_timeouts: 0,
                device_errors: 0,
                ecc_errors: 0,
                driver_errors: 0,
                total_retries: 0,
                successful_recoveries: 0,
                failed_recoveries: 0,
                quarantined_crew_count: 0,
            },
            max_history: 1000,
        }
    }

    /// Validate a kernel command before submission.
    ///
    /// Detects malformed commands (invalid grid/block dims, etc.)
    ///
    /// # Arguments
    ///
    /// * `grid_dims` - Grid dimensions (x, y, z)
    /// * `block_dims` - Block dimensions (x, y, z)
    /// * `shared_mem_bytes` - Shared memory size
    /// * `max_shared_mem_bytes` - Device limit for shared memory
    ///
    /// # Returns
    ///
    /// Ok(()) if valid, GpuError if malformed.
    pub fn validate_kernel_command(
        &mut self,
        grid_dims: (u32, u32, u32),
        block_dims: (u32, u32, u32),
        shared_mem_bytes: u32,
        max_shared_mem_bytes: u32,
        crew_id: [u8; 16],
        device_ordinal: u32,
        current_time_ns: u64,
    ) -> Result<(), GpuError> {
        // Check grid dimensions
        if grid_dims.0 == 0 || grid_dims.1 == 0 || grid_dims.2 == 0 {
            let report = GpuErrorReport {
                fault_code: GpuFaultCode::InvalidGridDims,
                submission_id: None,
                crew_id,
                device_ordinal,
                error_time_ns: current_time_ns,
                is_recoverable: false,
                context: [0u8; 256],
            };

            self._report_error(report)?;
            self.stats.malformed_commands += 1;
            return Err(GpuError::KernelLaunchFailed);
        }

        // Check block dimensions
        if block_dims.0 == 0 || block_dims.1 == 0 || block_dims.2 == 0 {
            let report = GpuErrorReport {
                fault_code: GpuFaultCode::InvalidBlockDims,
                submission_id: None,
                crew_id,
                device_ordinal,
                error_time_ns: current_time_ns,
                is_recoverable: false,
                context: [0u8; 256],
            };

            self._report_error(report)?;
            self.stats.malformed_commands += 1;
            return Err(GpuError::KernelLaunchFailed);
        }

        let total_threads = (block_dims.0 as u64)
            * (block_dims.1 as u64)
            * (block_dims.2 as u64);
        if total_threads > 1024 {
            let report = GpuErrorReport {
                fault_code: GpuFaultCode::InvalidBlockDims,
                submission_id: None,
                crew_id,
                device_ordinal,
                error_time_ns: current_time_ns,
                is_recoverable: false,
                context: [0u8; 256],
            };

            self._report_error(report)?;
            self.stats.malformed_commands += 1;
            return Err(GpuError::KernelLaunchFailed);
        }

        // Check shared memory
        if shared_mem_bytes > max_shared_mem_bytes {
            let report = GpuErrorReport {
                fault_code: GpuFaultCode::ExcessiveSharedMemory,
                submission_id: None,
                crew_id,
                device_ordinal,
                error_time_ns: current_time_ns,
                is_recoverable: false,
                context: [0u8; 256],
            };

            self._report_error(report)?;
            self.stats.malformed_commands += 1;
            return Err(GpuError::KernelLaunchFailed);
        }

        Ok(())
    }

    /// Report a GPU error and determine recovery action.
    ///
    /// # Arguments
    ///
    /// * `report` - Error report
    ///
    /// # Returns
    ///
    /// Recovery strategy to apply.
    pub fn report_error(&mut self, report: GpuErrorReport) -> Result<RecoveryStrategy, GpuError> {
        self._report_error(report)?;

        match report.fault_code {
            GpuFaultCode::StreamTimeout => self.stats.stream_timeouts += 1,
            GpuFaultCode::DeviceError => self.stats.device_errors += 1,
            GpuFaultCode::EccError => self.stats.ecc_errors += 1,
            GpuFaultCode::DriverError => self.stats.driver_errors += 1,
            _ => {}
        }

        Ok(RecoveryStrategy::for_fault(report.fault_code))
    }

    /// Internal error reporting (common path).
    fn _report_error(&mut self, report: GpuErrorReport) -> Result<(), GpuError> {
        self.stats.total_errors += 1;

        // Store in history
        if self.error_history.len() >= self.max_history as usize {
            self.error_history.remove(0);
        }
        self.error_history.push(report);

        Ok(())
    }

    /// Queue a submission for retry.
    ///
    /// # Arguments
    ///
    /// * `submission_id` - Submission to retry
    /// * `strategy` - Recovery strategy
    pub fn queue_retry(
        &mut self,
        submission_id: SubmissionId,
        strategy: RecoveryStrategy,
    ) -> Result<(), GpuError> {
        self.retry_queue.insert(submission_id, (0, strategy));
        self.stats.total_retries += 1;
        Ok(())
    }

    /// Get next retry submission.
    ///
    /// Returns a submission ready to retry, removing it from the queue.
    pub fn dequeue_retry(
        &mut self,
    ) -> Option<(SubmissionId, u32, RecoveryStrategy)> {
        if let Some((submission_id, (retry_count, strategy))) = self.retry_queue.iter().next() {
            if *retry_count < strategy.max_retries {
                let id = *submission_id;
                let count = *retry_count + 1;
                let strat = strategy.clone();
                self.retry_queue.remove(&id);
                return Some((id, count, strat));
            }
        }
        None
    }

    /// Quarantine a crew (disable future submissions).
    ///
    /// Called after repeated errors or isolation violations.
    pub fn quarantine_crew(&mut self, crew_id: [u8; 16]) -> Result<(), GpuError> {
        if !self.quarantined_crews.contains(&crew_id) {
            self.quarantined_crews.push(crew_id);
            self.stats.quarantined_crew_count += 1;
        }
        Ok(())
    }

    /// Check if a crew is quarantined.
    pub fn is_crew_quarantined(&self, crew_id: [u8; 16]) -> bool {
        self.quarantined_crews.contains(&crew_id)
    }

    /// Get error history (most recent first).
    pub fn get_error_history(&self, count: u32) -> Vec<GpuErrorReport> {
        let start = self.error_history.len().saturating_sub(count as usize);
        self.error_history[start..]
            .iter()
            .rev()
            .copied()
            .collect()
    }

    /// Get error handling statistics.
    pub fn stats(&self) -> ErrorHandlingStats {
        self.stats
    }

    /// Clear error history (used on shutdown).
    pub fn clear(&mut self) {
        self.error_history.clear();
        self.retry_queue.clear();
    }
}

impl Default for GpuErrorHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::format;

    #[test]
    fn test_gpu_fault_code_display() {
        assert_eq!(format!("{}", GpuFaultCode::InvalidGridDims), "InvalidGridDims");
        assert_eq!(format!("{}", GpuFaultCode::StreamTimeout), "StreamTimeout");
        assert_eq!(format!("{}", GpuFaultCode::DeviceError), "DeviceError");
    }

    #[test]
    fn test_gpu_fault_code_recoverable() {
        assert!(!GpuFaultCode::InvalidGridDims.is_recoverable());
        assert!(GpuFaultCode::StreamTimeout.is_recoverable());
        assert!(!GpuFaultCode::DeviceError.is_recoverable());
    }

    #[test]
    fn test_recovery_strategy_for_fault() {
        let strategy = RecoveryStrategy::for_fault(GpuFaultCode::StreamTimeout);
        assert_eq!(strategy.action, RecoveryAction::Retry);
        assert_eq!(strategy.max_retries, 3);
    }

    #[test]
    fn test_gpu_error_handler_creation() {
        let handler = GpuErrorHandler::new();

        assert_eq!(handler.stats.total_errors, 0);
        assert_eq!(handler.quarantined_crews.len(), 0);
    }

    #[test]
    fn test_validate_kernel_command() {
        let mut handler = GpuErrorHandler::new();

        let result = handler.validate_kernel_command(
            (16, 1, 1),
            (256, 1, 1),
            0,
            49152,
            [1u8; 16],
            0,
            1000,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_kernel_invalid_grid() {
        let mut handler = GpuErrorHandler::new();

        let result = handler.validate_kernel_command(
            (0, 1, 1), // Invalid
            (256, 1, 1),
            0,
            49152,
            [1u8; 16],
            0,
            1000,
        );

        assert!(result.is_err());
        assert_eq!(handler.stats.malformed_commands, 1);
    }

    #[test]
    fn test_validate_kernel_invalid_block() {
        let mut handler = GpuErrorHandler::new();

        let result = handler.validate_kernel_command(
            (16, 1, 1),
            (512, 512, 1), // Invalid: 512*512 > 1024
            0,
            49152,
            [1u8; 16],
            0,
            1000,
        );

        assert!(result.is_err());
        assert_eq!(handler.stats.malformed_commands, 1);
    }

    #[test]
    fn test_validate_kernel_excessive_shared_mem() {
        let mut handler = GpuErrorHandler::new();

        let result = handler.validate_kernel_command(
            (16, 1, 1),
            (256, 1, 1),
            100000, // Exceeds limit
            49152,
            [1u8; 16],
            0,
            1000,
        );

        assert!(result.is_err());
        assert_eq!(handler.stats.malformed_commands, 1);
    }

    #[test]
    fn test_queue_retry() {
        let mut handler = GpuErrorHandler::new();

        let strategy = RecoveryStrategy::for_fault(GpuFaultCode::StreamTimeout);
        let result = handler.queue_retry(SubmissionId::new(1), strategy);

        assert!(result.is_ok());
        assert_eq!(handler.stats.total_retries, 1);
    }

    #[test]
    fn test_dequeue_retry() {
        let mut handler = GpuErrorHandler::new();

        let strategy = RecoveryStrategy::for_fault(GpuFaultCode::StreamTimeout);
        handler.queue_retry(SubmissionId::new(1), strategy).unwrap();

        let retry = handler.dequeue_retry();
        assert!(retry.is_some());

        let (submission_id, retry_count, _) = retry.unwrap();
        assert_eq!(submission_id, SubmissionId::new(1));
        assert_eq!(retry_count, 1);
    }

    #[test]
    fn test_quarantine_crew() {
        let mut handler = GpuErrorHandler::new();

        let crew_id = [1u8; 16];
        let result = handler.quarantine_crew(crew_id);

        assert!(result.is_ok());
        assert!(handler.is_crew_quarantined(crew_id));
        assert_eq!(handler.stats.quarantined_crew_count, 1);
    }

    #[test]
    fn test_recovery_action_display() {
        assert_eq!(format!("{}", RecoveryAction::Retry), "Retry");
        assert_eq!(format!("{}", RecoveryAction::ResetDevice), "ResetDevice");
        assert_eq!(format!("{}", RecoveryAction::TerminateCrew), "TerminateCrew");
    }
}
