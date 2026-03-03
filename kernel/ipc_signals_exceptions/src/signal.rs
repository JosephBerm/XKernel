// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! # Cognitive Signal Types and Handling
//!
//! This module defines the cognitive signal system for inter-agent communication.
//! Signals provide a lightweight, high-priority notification mechanism for critical
//! events that require immediate attention from agents or crews.
//!
//! ## Signal Types
//!
//! The system defines 8 core signal types with priority and delivery guarantees:
//! 1. SigTerminate - Immediate shutdown (Unblockable)
//! 2. SigDeadlineWarn - Deadline approaching (High)
//! 3. SigCheckpoint - Checkpoint requested (Normal)
//! 4. SigBudgetWarn - Resource budget warning (Normal)
//! 5. SigContextLow - Context buffer pressure (High)
//! 6. SigIpcFailed - IPC communication failure (High)
//! 7. SigPreempt - Preemption request (High)
//! 8. SigResume - Resume from preemption (Normal)
//!
//! ## Signal Priority
//!
//! Signals have four priority levels:
//! - Unblockable: Cannot be masked or delayed
//! - High: Processed urgently, before normal work
//! - Normal: Standard priority processing
//! - Low: Processed when resources available
//!
//! ## Signal Delivery Guarantees
//!
//! - Immediate: Delivery cannot be delayed (for unblockable signals)
//! - Queued: Delivery at next safe preemption point
//!
//! ## References
//!
//! - Engineering Plan § 6.1 (Signal System)

use crate::ids::{CheckpointID, SignalID};
use alloc::string::String;
use serde::{Deserialize, Serialize};

/// Signal priority level.
///
/// Determines how urgently a signal should be processed relative to normal work.
///
/// See Engineering Plan § 6.1 (Signal System)
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum SignalPriority {
    /// Lowest priority - processed when resources permit.
    Low = 0,

    /// Normal priority - standard signal processing.
    Normal = 1,

    /// High priority - processed before normal work.
    High = 2,

    /// Unblockable priority - cannot be masked or delayed.
    /// Only SigTerminate uses this level.
    Unblockable = 3,
}

impl SignalPriority {
    /// Check if this priority can be masked (delayed) by the receiver.
    pub fn is_maskable(&self) -> bool {
        !matches!(self, SignalPriority::Unblockable)
    }
}

/// Signal delivery guarantee.
///
/// Determines when a signal is guaranteed to be delivered to its target.
///
/// See Engineering Plan § 6.1 (Signal System)
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum SignalDeliveryGuarantee {
    /// Immediate delivery - cannot be delayed.
    /// Used for unblockable signals and critical conditions.
    Immediate,

    /// Queued delivery - delivered at next safe preemption point.
    /// Allows safe state cleanup before handling the signal.
    Queued,
}

/// Cognitive signal type for inter-agent communication.
///
/// Signals provide a lightweight notification mechanism for critical events.
/// Each signal type has defined priority and maskability characteristics.
///
/// See Engineering Plan § 6.1 (Signal System)
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum CognitiveSignal {
    /// Immediate termination signal (Unblockable).
    ///
    /// Signals immediate shutdown. Cannot be masked or delayed.
    /// Receiver must stop execution and clean up immediately.
    ///
    /// See Engineering Plan § 6.1 (Signal System)
    SigTerminate {
        /// Reason for termination
        reason: String,
        /// Grace period in milliseconds (0 for immediate)
        grace_period_ms: u32,
    },

    /// Deadline approaching warning (High).
    ///
    /// Emitted when the deadline is approaching. Allows proactive completion.
    /// Can be masked but should not be ignored.
    ///
    /// See Engineering Plan § 6.1 (Signal System)
    SigDeadlineWarn {
        /// Deadline name/description
        deadline_name: String,
        /// Milliseconds remaining before deadline
        remaining_ms: u64,
    },

    /// Checkpoint request (Normal).
    ///
    /// Requests creation of a checkpoint for recovery/optimization.
    /// Can be deferred to a convenient time.
    ///
    /// See Engineering Plan § 6.1 (Signal System)
    SigCheckpoint {
        /// Checkpoint reason (e.g., "periodic", "before_tool_call", "manual")
        reason: String,
        /// Timestamp of request (Unix epoch milliseconds)
        timestamp_ms: u64,
    },

    /// Resource budget warning (Normal).
    ///
    /// Emitted when token budget, computational budget, or other resource limits
    /// are approaching exhaustion.
    ///
    /// See Engineering Plan § 6.1 (Signal System)
    SigBudgetWarn {
        /// Budget type (e.g., "tokens", "compute_seconds")
        budget_type: String,
        /// Remaining budget
        remaining: u64,
        /// Original allocation
        allocated: u64,
    },

    /// Context buffer pressure warning (High).
    ///
    /// Emitted when the context buffer capacity is approaching or has exceeded limits.
    /// Receiver should reduce cognitive load or clear context.
    ///
    /// See Engineering Plan § 6.1 (Signal System)
    SigContextLow {
        /// Current context buffer usage in bytes
        current_bytes: u64,
        /// Maximum capacity in bytes
        max_bytes: u64,
    },

    /// IPC communication failure notification (High).
    ///
    /// Emitted when inter-process communication fails (channel error, delivery failure).
    /// Receiver should escalate or attempt recovery.
    ///
    /// See Engineering Plan § 6.1 (Signal System)
    SigIpcFailed {
        /// Channel or endpoint identifier
        channel_id: String,
        /// Failure description
        failure_reason: String,
    },

    /// Preemption request (High).
    ///
    /// Requests the receiver to pause execution and yield control.
    /// Receiver should save state and wait for SigResume.
    ///
    /// See Engineering Plan § 6.1 (Signal System)
    SigPreempt {
        /// Reason for preemption
        reason: String,
        /// Preemption deadline (Unix epoch milliseconds)
        deadline_ms: u64,
    },

    /// Resume from preemption (Normal).
    ///
    /// Signals that preemption is lifted and execution can resume.
    ///
    /// See Engineering Plan § 6.1 (Signal System)
    SigResume {
        /// Information from preemption period (if any)
        context_info: String,
    },
}

impl CognitiveSignal {
    /// Get a human-readable signal type name.
    pub fn signal_type(&self) -> &'static str {
        match self {
            CognitiveSignal::SigTerminate { .. } => "SigTerminate",
            CognitiveSignal::SigDeadlineWarn { .. } => "SigDeadlineWarn",
            CognitiveSignal::SigCheckpoint { .. } => "SigCheckpoint",
            CognitiveSignal::SigBudgetWarn { .. } => "SigBudgetWarn",
            CognitiveSignal::SigContextLow { .. } => "SigContextLow",
            CognitiveSignal::SigIpcFailed { .. } => "SigIpcFailed",
            CognitiveSignal::SigPreempt { .. } => "SigPreempt",
            CognitiveSignal::SigResume { .. } => "SigResume",
        }
    }

    /// Get the priority of this signal.
    ///
    /// Determines how urgently this signal should be processed.
    ///
    /// See Engineering Plan § 6.1 (Signal System)
    pub fn priority(&self) -> SignalPriority {
        match self {
            CognitiveSignal::SigTerminate { .. } => SignalPriority::Unblockable,
            CognitiveSignal::SigDeadlineWarn { .. } => SignalPriority::High,
            CognitiveSignal::SigCheckpoint { .. } => SignalPriority::Normal,
            CognitiveSignal::SigBudgetWarn { .. } => SignalPriority::Normal,
            CognitiveSignal::SigContextLow { .. } => SignalPriority::High,
            CognitiveSignal::SigIpcFailed { .. } => SignalPriority::High,
            CognitiveSignal::SigPreempt { .. } => SignalPriority::High,
            CognitiveSignal::SigResume { .. } => SignalPriority::Normal,
        }
    }

    /// Check if this signal can be masked (deferred) by the receiver.
    ///
    /// Unblockable signals (like SigTerminate) cannot be masked and must be
    /// handled immediately. Other signals can be delayed to a safe point.
    ///
    /// See Engineering Plan § 6.1 (Signal System)
    pub fn is_maskable(&self) -> bool {
        self.priority().is_maskable()
    }

    /// Get the delivery guarantee for this signal.
    ///
    /// Determines when delivery is guaranteed.
    ///
    /// See Engineering Plan § 6.1 (Signal System)
    pub fn delivery_guarantee(&self) -> SignalDeliveryGuarantee {
        match self {
            CognitiveSignal::SigTerminate { .. } => SignalDeliveryGuarantee::Immediate,
            CognitiveSignal::SigDeadlineWarn { .. } => SignalDeliveryGuarantee::Queued,
            CognitiveSignal::SigCheckpoint { .. } => SignalDeliveryGuarantee::Queued,
            CognitiveSignal::SigBudgetWarn { .. } => SignalDeliveryGuarantee::Queued,
            CognitiveSignal::SigContextLow { .. } => SignalDeliveryGuarantee::Immediate,
            CognitiveSignal::SigIpcFailed { .. } => SignalDeliveryGuarantee::Immediate,
            CognitiveSignal::SigPreempt { .. } => SignalDeliveryGuarantee::Immediate,
            CognitiveSignal::SigResume { .. } => SignalDeliveryGuarantee::Queued,
        }
    }
}

/// Signal handler trait for processing signals.
///
/// Implementations handle specific signal types and define the response behavior.
pub trait SignalHandler: Send + Sync {
    /// Handle a signal and return success/failure.
    fn handle_signal(&self, signal: &CognitiveSignal) -> Result<(), String>;

    /// Get the signal type(s) this handler processes.
    fn handles(&self, signal_type: &str) -> bool;
}

/// Signal handler table mapping signal types to handlers.
///
/// Allows efficient dispatch of signals to appropriate handlers.
#[derive(Clone, Debug)]
pub struct SignalHandlerTable {
    handlers: alloc::vec::Vec<(String, alloc::boxed::Box<dyn SignalHandler>)>,
}

impl SignalHandlerTable {
    /// Create a new empty signal handler table.
    pub fn new() -> Self {
        Self {
            handlers: alloc::vec::Vec::new(),
        }
    }

    /// Register a handler for a signal type.
    pub fn register(
        &mut self,
        signal_type: String,
        handler: alloc::boxed::Box<dyn SignalHandler>,
    ) {
        self.handlers.push((signal_type, handler));
    }

    /// Dispatch a signal to all registered handlers.
    pub fn dispatch(&self, signal: &CognitiveSignal) -> Result<(), String> {
        for (_signal_type, handler) in &self.handlers {
            if handler.handles(signal.signal_type()) {
                handler.handle_signal(signal)?;
            }
        }
        Ok(())
    }

    /// Check if any handler is registered for a signal type.
    pub fn has_handler(&self, signal_type: &str) -> bool {
        self.handlers.iter().any(|(_type, handler)| {
            handler.handles(signal_type)
        })
    }
}

impl Default for SignalHandlerTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::boxed::Box;
use alloc::vec::Vec;

    // ============================================================================
    // Signal Priority Tests
    // ============================================================================

    #[test]
    fn test_signal_priority_ordering() {
        assert!(SignalPriority::Low < SignalPriority::Normal);
        assert!(SignalPriority::Normal < SignalPriority::High);
        assert!(SignalPriority::High < SignalPriority::Unblockable);
    }

    #[test]
    fn test_signal_priority_maskable() {
        assert!(SignalPriority::Low.is_maskable());
        assert!(SignalPriority::Normal.is_maskable());
        assert!(SignalPriority::High.is_maskable());
        assert!(!SignalPriority::Unblockable.is_maskable());
    }

    // ============================================================================
    // Signal Type Tests - Unblockable
    // ============================================================================

    #[test]
    fn test_sig_terminate() {
        let sig = CognitiveSignal::SigTerminate {
            reason: alloc::string::String::from("shutdown"),
            grace_period_ms: 1000,
        };
        assert_eq!(sig.signal_type(), "SigTerminate");
        assert_eq!(sig.priority(), SignalPriority::Unblockable);
        assert!(!sig.is_maskable());
        assert_eq!(sig.delivery_guarantee(), SignalDeliveryGuarantee::Immediate);
    }

    // ============================================================================
    // Signal Type Tests - High Priority
    // ============================================================================

    #[test]
    fn test_sig_deadline_warn() {
        let sig = CognitiveSignal::SigDeadlineWarn {
            deadline_name: alloc::string::String::from("task_timeout"),
            remaining_ms: 5000,
        };
        assert_eq!(sig.signal_type(), "SigDeadlineWarn");
        assert_eq!(sig.priority(), SignalPriority::High);
        assert!(sig.is_maskable());
        assert_eq!(sig.delivery_guarantee(), SignalDeliveryGuarantee::Queued);
    }

    #[test]
    fn test_sig_context_low() {
        let sig = CognitiveSignal::SigContextLow {
            current_bytes: 900,
            max_bytes: 1000,
        };
        assert_eq!(sig.signal_type(), "SigContextLow");
        assert_eq!(sig.priority(), SignalPriority::High);
        assert!(sig.is_maskable());
        assert_eq!(sig.delivery_guarantee(), SignalDeliveryGuarantee::Immediate);
    }

    #[test]
    fn test_sig_ipc_failed() {
        let sig = CognitiveSignal::SigIpcFailed {
            channel_id: alloc::string::String::from("ch_123"),
            failure_reason: alloc::string::String::from("delivery timeout"),
        };
        assert_eq!(sig.signal_type(), "SigIpcFailed");
        assert_eq!(sig.priority(), SignalPriority::High);
        assert_eq!(sig.delivery_guarantee(), SignalDeliveryGuarantee::Immediate);
    }

    #[test]
    fn test_sig_preempt() {
        let sig = CognitiveSignal::SigPreempt {
            reason: alloc::string::String::from("resource pressure"),
            deadline_ms: 5000,
        };
        assert_eq!(sig.signal_type(), "SigPreempt");
        assert_eq!(sig.priority(), SignalPriority::High);
        assert!(sig.is_maskable());
        assert_eq!(sig.delivery_guarantee(), SignalDeliveryGuarantee::Immediate);
    }

    // ============================================================================
    // Signal Type Tests - Normal Priority
    // ============================================================================

    #[test]
    fn test_sig_checkpoint() {
        let sig = CognitiveSignal::SigCheckpoint {
            reason: alloc::string::String::from("periodic"),
            timestamp_ms: 5000,
        };
        assert_eq!(sig.signal_type(), "SigCheckpoint");
        assert_eq!(sig.priority(), SignalPriority::Normal);
        assert!(sig.is_maskable());
        assert_eq!(sig.delivery_guarantee(), SignalDeliveryGuarantee::Queued);
    }

    #[test]
    fn test_sig_budget_warn() {
        let sig = CognitiveSignal::SigBudgetWarn {
            budget_type: alloc::string::String::from("tokens"),
            remaining: 100,
            allocated: 1000,
        };
        assert_eq!(sig.signal_type(), "SigBudgetWarn");
        assert_eq!(sig.priority(), SignalPriority::Normal);
        assert!(sig.is_maskable());
        assert_eq!(sig.delivery_guarantee(), SignalDeliveryGuarantee::Queued);
    }

    #[test]
    fn test_sig_resume() {
        let sig = CognitiveSignal::SigResume {
            context_info: alloc::string::String::from("state saved"),
        };
        assert_eq!(sig.signal_type(), "SigResume");
        assert_eq!(sig.priority(), SignalPriority::Normal);
        assert!(sig.is_maskable());
        assert_eq!(sig.delivery_guarantee(), SignalDeliveryGuarantee::Queued);
    }

    // ============================================================================
    // Signal Handler Table Tests
    // ============================================================================

    #[test]
    fn test_signal_handler_table_new() {
        let table = SignalHandlerTable::new();
        assert!(!table.has_handler("SigContextLow"));
    }

    #[test]
    fn test_signal_handler_table_default() {
        let table = SignalHandlerTable::default();
        assert!(!table.has_handler("SigTerminate"));
    }
}
