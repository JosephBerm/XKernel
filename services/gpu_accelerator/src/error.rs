// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! GPU Manager error types.
//!
//! Comprehensive error enumeration covering device management, memory allocation,
//! kernel launch, isolation violations, and driver errors.
//!
//! Reference: Engineering Plan § Error Handling & Safety

/// Error type for GPU Manager operations.
///
/// Covers all failure modes in device management, memory allocation,
/// kernel launch, checkpoint/restore, and driver interaction.
///
/// Reference: Engineering Plan § Device Registry, VRAM Isolation
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GpuError {
    /// Requested GPU device not found or enumeration failed.
    ///
    /// May indicate hardware disconnect, driver failure, or invalid device ID.
    DeviceNotFound,

    /// Insufficient VRAM available for allocation request.
    ///
    /// Occurs when a crew's memory request exceeds available free VRAM
    /// or remaining allocation in a VRAM region.
    VramExhausted,

    /// Memory allocation (host or device) failed.
    ///
    /// Lower-level allocation failure; contrasts with VramExhausted
    /// which is semantic exhaustion.
    AllocationFailed,

    /// Kernel launch failed (grid too large, invalid parameters, etc.).
    ///
    /// Indicates invalid kernel configuration or resource constraints.
    KernelLaunchFailed,

    /// Checkpoint operation failed.
    ///
    /// May indicate I/O error, memory pressure, or driver issue during
    /// GPU state snapshot.
    CheckpointFailed,

    /// Restore operation failed.
    ///
    /// May indicate state corruption, incompatible device, or driver error
    /// during GPU state restoration.
    RestoreFailed,

    /// Driver error (CUDA or ROCm).
    ///
    /// Indicates lower-level driver issue. Wrapping crates may extend
    /// this with driver-specific error codes.
    DriverError,

    /// VRAM isolation boundary violation attempted.
    ///
    /// Crew attempted access to VRAM region outside its allocation.
    /// Represents security/safety violation in Strict or Selective mode.
    IsolationViolation,

    /// Requested TPC(s) unavailable for allocation.
    ///
    /// All TPCs are already allocated or in fault state.
    TpcUnavailable,
}

impl GpuError {
    /// Check if this is a recoverable error (true) or fatal (false).
    ///
    /// Recoverable errors (VramExhausted, TpcUnavailable) may resolve
    /// with resource cleanup or workload migration.
    /// Fatal errors (DriverError, IsolationViolation) require higher-level
    /// intervention.
    pub fn is_recoverable(&self) -> bool {
        matches!(self, GpuError::VramExhausted | GpuError::TpcUnavailable)
    }

    /// Human-readable error message.
    pub fn message(&self) -> &'static str {
        match self {
            GpuError::DeviceNotFound => "GPU device not found",
            GpuError::VramExhausted => "VRAM exhausted",
            GpuError::AllocationFailed => "Memory allocation failed",
            GpuError::KernelLaunchFailed => "Kernel launch failed",
            GpuError::CheckpointFailed => "Checkpoint operation failed",
            GpuError::RestoreFailed => "Restore operation failed",
            GpuError::DriverError => "Driver error",
            GpuError::IsolationViolation => "VRAM isolation violation",
            GpuError::TpcUnavailable => "TPC unavailable",
        }
    }
}

impl core::fmt::Display for GpuError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.message())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::format;

    #[test]
    fn test_recoverable_errors() {
        assert!(GpuError::VramExhausted.is_recoverable());
        assert!(GpuError::TpcUnavailable.is_recoverable());
    }

    #[test]
    fn test_fatal_errors() {
        assert!(!GpuError::DriverError.is_recoverable());
        assert!(!GpuError::IsolationViolation.is_recoverable());
        assert!(!GpuError::DeviceNotFound.is_recoverable());
    }

    #[test]
    fn test_error_messages() {
        assert_eq!(GpuError::DeviceNotFound.message(), "GPU device not found");
        assert_eq!(GpuError::VramExhausted.message(), "VRAM exhausted");
        assert_eq!(GpuError::IsolationViolation.message(), "VRAM isolation violation");
    }

    #[test]
    fn test_error_display() {
        let err = GpuError::AllocationFailed;
        let display_str = format!("{}", err);
        assert_eq!(display_str, "Memory allocation failed");
    }

    #[test]
    fn test_error_equality() {
        let err1 = GpuError::DeviceNotFound;
        let err2 = GpuError::DeviceNotFound;
        assert_eq!(err1, err2);
    }
}
