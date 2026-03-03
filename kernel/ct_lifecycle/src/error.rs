// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! # Error Types for CT Lifecycle
//!
//! This module defines all error types that can occur during CT lifecycle management.
//! Errors are organized by subsystem for better debugging and error handling.
//!
//! ## References
//!
//! - Engineering Plan S 4.3 (Error Handling & Recovery)
//! - Engineering Plan S 5.2 (CT Invariants & Type-Safety)

use core::fmt;
use alloc::string::String;

/// Unified error type for the CT lifecycle subsystem.
#[derive(Debug, Clone)]
pub enum CsError {
    /// Invalid phase transition attempted.
    InvalidPhaseTransition {
        /// The current phase name.
        current: String,
        /// The target phase name.
        target: String,
        /// Reason the transition is invalid.
        reason: String,
    },
    /// Cyclic dependency detected at spawn time.
    CyclicDependency {
        /// The CT that triggered the cycle.
        ct_id: String,
        /// A description of the cycle path.
        cycle: String,
    },
    /// Resource budget exceeded.
    BudgetExceeded {
        /// The resource type.
        resource: String,
        /// The amount requested.
        requested: u64,
        /// The amount available.
        available: u64,
    },
    /// Watchdog timeout exceeded.
    WatchdogTimeout {
        /// The timeout type (e.g. "deadline", "heartbeat").
        timeout_type: String,
        /// The configured limit.
        limit: u64,
        /// The actual elapsed time.
        actual: u64,
    },
    /// Loop detected during execution.
    LoopDetected {
        /// Number of repeated iterations observed.
        repeat_count: u64,
        /// The configured threshold.
        threshold: u64,
    },
    /// Context window is full.
    ContextWindowFull {
        /// Current context size.
        current_size: u64,
        /// Maximum capacity.
        max_capacity: u64,
        /// Requested additional space.
        requested: u64,
    },
    /// Internal error.
    InternalError {
        /// Error message.
        message: String,
    },
    /// Capability violation.
    CapabilityViolation {
        /// The capability ID.
        capability_id: String,
        /// Reason for the violation.
        reason: String,
    },
    /// CT not found.
    CtNotFound {
        /// The CT identifier.
        ct_id: String,
    },
    /// Duplicate CT.
    DuplicateCt {
        /// The CT identifier.
        ct_id: String,
    },
}

impl fmt::Display for CsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CsError::InvalidPhaseTransition { current, target, reason } => {
                write!(f, "Invalid phase transition from {} to {}: {}", current, target, reason)
            }
            CsError::CyclicDependency { ct_id, cycle } => {
                write!(f, "Cyclic dependency detected for CT {}: {}", ct_id, cycle)
            }
            CsError::BudgetExceeded { resource, requested, available } => {
                write!(f, "Budget exceeded for {}: requested {}, available {}", resource, requested, available)
            }
            CsError::WatchdogTimeout { timeout_type, limit, actual } => {
                write!(f, "Watchdog {} timeout: limit {}, actual {}", timeout_type, limit, actual)
            }
            CsError::LoopDetected { repeat_count, threshold } => {
                write!(f, "Loop detected: {} repeats exceeds threshold {}", repeat_count, threshold)
            }
            CsError::ContextWindowFull { current_size, max_capacity, requested } => {
                write!(f, "Context window full: size {}/{}, requested {}", current_size, max_capacity, requested)
            }
            CsError::InternalError { message } => {
                write!(f, "Internal error: {}", message)
            }
            CsError::CapabilityViolation { capability_id, reason } => {
                write!(f, "Capability violation for {}: {}", capability_id, reason)
            }
            CsError::CtNotFound { ct_id } => {
                write!(f, "CT not found: {}", ct_id)
            }
            CsError::DuplicateCt { ct_id } => {
                write!(f, "Duplicate CT: {}", ct_id)
            }
        }
    }
}

/// Result type alias for the CT lifecycle subsystem.
pub type Result<T> = core::result::Result<T, CsError>;

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;

    #[test]
    fn test_invalid_phase_transition_display() {
        let err = CsError::InvalidPhaseTransition {
            current: "Plan".to_string(),
            target: "Spawn".to_string(),
            reason: "Cannot transition backwards".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("Plan"));
        assert!(msg.contains("Spawn"));
    }

    #[test]
    fn test_cyclic_dependency_display() {
        let err = CsError::CyclicDependency {
            ct_id: "ct-001".to_string(),
            cycle: "ct-001 -> ct-002 -> ct-001".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("Cyclic"));
    }

    #[test]
    fn test_budget_exceeded_display() {
        let err = CsError::BudgetExceeded {
            resource: "GPU".to_string(),
            requested: 1000,
            available: 500,
        };
        let msg = err.to_string();
        assert!(msg.contains("GPU"));
        assert!(msg.contains("1000"));
    }

    #[test]
    fn test_watchdog_timeout_display() {
        let err = CsError::WatchdogTimeout {
            timeout_type: "deadline".to_string(),
            limit: 5000,
            actual: 5100,
        };
        let msg = err.to_string();
        assert!(msg.contains("deadline"));
    }

    #[test]
    fn test_loop_detected_display() {
        let err = CsError::LoopDetected {
            repeat_count: 50,
            threshold: 30,
        };
        let msg = err.to_string();
        assert!(msg.contains("Loop"));
    }

    #[test]
    fn test_context_window_full_display() {
        let err = CsError::ContextWindowFull {
            current_size: 8000,
            max_capacity: 8192,
            requested: 500,
        };
        let msg = err.to_string();
        assert!(msg.contains("Context"));
    }

    #[test]
    fn test_error_clone() {
        let err = CsError::InternalError {
            message: "test error".to_string(),
        };
        let _cloned = err.clone();
    }

    #[test]
    fn test_capability_violation_display() {
        let err = CsError::CapabilityViolation {
            capability_id: "CAP-123".to_string(),
            reason: "Not in parent capability set".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("Capability"));
    }
}
