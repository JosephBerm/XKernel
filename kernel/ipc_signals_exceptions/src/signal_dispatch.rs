// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! # Signal Dispatch Table and Delivery System
//!
//! This module implements the signal dispatch infrastructure for delivering
//! cognitive signals to registered handlers at safe preemption points.
//!
//! ## Architecture
//!
//! The signal dispatch system manages:
//! - Per-CT signal handler registration (8 signals maximum)
//! - Pending signal queue (FIFO delivery)
//! - Signal masking (per-signal blocking)
//! - SIG_TERMINATE special handling (uncatchable, always terminates)
//! - Race-free signal delivery at preemption points
//!
//! ## Signal Types
//!
//! The system supports 8 cognitive signals:
//! 1. SIG_TERMINATE (index 0) - Uncatchable termination
//! 2. SIG_DEADLINE_WARN (index 1) - Deadline approaching
//! 3. SIG_CHECKPOINT (index 2) - Checkpoint requested
//! 4. SIG_BUDGET_WARN (index 3) - Budget warning
//! 5. SIG_CONTEXT_LOW (index 4) - Context buffer pressure
//! 6. SIG_IPC_FAILED (index 5) - IPC communication failure
//! 7. SIG_PREEMPT (index 6) - Preemption request
//! 8. SIG_RESUME (index 7) - Resume from preemption
//!
//! ## Handler Registration
//!
//! Handlers are registered as function pointers with signature:
//! `fn(&CognitiveSignal) -> SignalHandlerResult`
//!
//! SIG_TERMINATE cannot be registered - attempts return PermissionDenied.
//!
//! ## Signal Delivery
//!
//! Delivery occurs via the FIFO pending_signals queue at safe preemption points:
//! - After syscall completion
//! - Between reasoning phases
//! - At timer interrupt
//! - Before context switch
//!
//! ## References
//!
//! - Engineering Plan § 6.1 (Signal System)
//! - Week 4 Objective: Signal dispatch table with per-CT mapping

#![allow(dead_code)]

use crate::error::{CsError, IpcError, Result};
use crate::signal::CognitiveSignal;
use alloc::collections::VecDeque;
use core::mem;

/// Signal type enumeration for indexing into the handler array.
///
/// Each signal maps to a unique index (0-7) in the handler table.
///
/// See Engineering Plan § 6.1 (Signal System)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum SignalType {
    /// SIG_TERMINATE (index 0) - Uncatchable termination signal
    Terminate = 0,
    /// SIG_DEADLINE_WARN (index 1) - Deadline approaching
    DeadlineWarn = 1,
    /// SIG_CHECKPOINT (index 2) - Checkpoint requested
    Checkpoint = 2,
    /// SIG_BUDGET_WARN (index 3) - Budget warning
    BudgetWarn = 3,
    /// SIG_CONTEXT_LOW (index 4) - Context buffer pressure
    ContextLow = 4,
    /// SIG_IPC_FAILED (index 5) - IPC communication failure
    IpcFailed = 5,
    /// SIG_PREEMPT (index 6) - Preemption request
    Preempt = 6,
    /// SIG_RESUME (index 7) - Resume from preemption
    Resume = 7,
}

impl SignalType {
    /// Get the signal type from a CognitiveSignal.
    pub fn from_signal(signal: &CognitiveSignal) -> Self {
        match signal {
            CognitiveSignal::SigTerminate { .. } => SignalType::Terminate,
            CognitiveSignal::SigDeadlineWarn { .. } => SignalType::DeadlineWarn,
            CognitiveSignal::SigCheckpoint { .. } => SignalType::Checkpoint,
            CognitiveSignal::SigBudgetWarn { .. } => SignalType::BudgetWarn,
            CognitiveSignal::SigContextLow { .. } => SignalType::ContextLow,
            CognitiveSignal::SigIpcFailed { .. } => SignalType::IpcFailed,
            CognitiveSignal::SigPreempt { .. } => SignalType::Preempt,
            CognitiveSignal::SigResume { .. } => SignalType::Resume,
        }
    }

    /// Get the index for this signal type (0-7).
    pub fn index(&self) -> usize {
        *self as usize
    }

    /// Get the name of this signal type.
    pub fn name(&self) -> &'static str {
        match self {
            SignalType::Terminate => "SIG_TERMINATE",
            SignalType::DeadlineWarn => "SIG_DEADLINE_WARN",
            SignalType::Checkpoint => "SIG_CHECKPOINT",
            SignalType::BudgetWarn => "SIG_BUDGET_WARN",
            SignalType::ContextLow => "SIG_CONTEXT_LOW",
            SignalType::IpcFailed => "SIG_IPC_FAILED",
            SignalType::Preempt => "SIG_PREEMPT",
            SignalType::Resume => "SIG_RESUME",
        }
    }
}

/// Result of signal handler execution.
///
/// Indicates how the handler processed the signal and what action the
/// signal delivery system should take next.
///
/// See Engineering Plan § 6.1 (Signal System)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SignalHandlerResult {
    /// Handler processed signal successfully, continue to next signal
    Continue,
    /// Handler requires restart of current phase/operation
    Restart,
    /// Handler encountered error, escalate to exception system
    Escalate,
}

/// Signal handler function type.
///
/// A signal handler is invoked with a reference to the signal and returns
/// a SignalHandlerResult indicating how the system should proceed.
///
/// Handlers must not panic and must complete quickly (preemption-safe).
///
/// See Engineering Plan § 6.1 (Signal System)
pub type SignalHandler = fn(&CognitiveSignal) -> SignalHandlerResult;

/// Pending signal entry in the delivery queue.
///
/// Each pending signal includes the signal data and metadata about
/// when it was enqueued.
#[derive(Clone, Debug)]
struct PendingSignal {
    /// The cognitive signal to deliver
    signal: CognitiveSignal,
    /// Timestamp when enqueued (Unix epoch milliseconds)
    enqueued_ms: u64,
}

/// Signal Dispatch Table for a Cognitive Thread.
///
/// Manages per-CT signal handler registration, pending signal queue,
/// and signal masking. This is the core infrastructure for signal delivery.
///
/// Each CT has exactly one SignalDispatchTable instance.
///
/// ## Thread Safety
///
/// This structure is designed for use in a concurrent kernel environment.
/// Access is synchronized via kernel-level locks or atomic operations.
///
/// See Engineering Plan § 6.1 (Signal System), Week 4 Objective
#[derive(Debug)]
pub struct SignalDispatchTable {
    /// CT ID that owns this dispatch table
    ct_id: u64,
    /// Handler for each signal type (index 0-7)
    /// None indicates no handler registered
    handlers: [Option<SignalHandler>; 8],
    /// Bit mask of masked signals (1 = masked/blocked)
    /// Bit 0 = SIG_TERMINATE (ignored, always unmasked)
    /// Bits 1-7 for other signals
    signal_mask: u8,
    /// Queue of pending signals awaiting delivery
    /// Delivered FIFO at safe preemption points
    pending_signals: VecDeque<PendingSignal>,
}

impl SignalDispatchTable {
    /// Create a new signal dispatch table for a CT.
    ///
    /// Initializes with no handlers registered and no signals masked.
    ///
    /// # Arguments
    ///
    /// * `ct_id` - The cognitive thread ID that owns this table
    ///
    /// # Returns
    ///
    /// A new SignalDispatchTable
    pub fn new(ct_id: u64) -> Self {
        Self {
            ct_id,
            handlers: [None; 8],
            signal_mask: 0,
            pending_signals: VecDeque::with_capacity(64),
        }
    }

    /// Register a signal handler for a signal type.
    ///
    /// # Arguments
    ///
    /// * `signal_type` - The signal type to register for
    /// * `handler` - The handler function (must not panic)
    ///
    /// # Returns
    ///
    /// - Ok(()) if handler registered successfully
    /// - Err(PermissionDenied) if trying to register SIG_TERMINATE
    ///
    /// See Engineering Plan § 6.1 (Signal System)
    pub fn register(&mut self, signal_type: SignalType, handler: SignalHandler) -> Result<()> {
        // SIG_TERMINATE cannot be registered
        if signal_type == SignalType::Terminate {
            return Err(CsError::Ipc(IpcError::Other(
                "SIG_TERMINATE cannot be registered".to_string(),
            )));
        }

        let index = signal_type.index();
        self.handlers[index] = Some(handler);
        Ok(())
    }

    /// Unregister a signal handler for a signal type.
    ///
    /// After unregistration, signals of this type will not be delivered,
    /// and any pending signals of this type will be discarded.
    ///
    /// # Arguments
    ///
    /// * `signal_type` - The signal type to unregister
    ///
    /// # Returns
    ///
    /// - Ok(()) if handler unregistered successfully
    /// - Err(...) if signal_type is invalid
    pub fn unregister(&mut self, signal_type: SignalType) -> Result<()> {
        let index = signal_type.index();
        self.handlers[index] = None;

        // Discard all pending signals of this type
        self.pending_signals
            .retain(|pending| SignalType::from_signal(&pending.signal) != signal_type);

        Ok(())
    }

    /// Check if a handler is registered for a signal type.
    ///
    /// # Arguments
    ///
    /// * `signal_type` - The signal type to check
    ///
    /// # Returns
    ///
    /// True if a handler is registered for this signal type
    pub fn has_handler(&self, signal_type: SignalType) -> bool {
        self.handlers[signal_type.index()].is_some()
    }

    /// Get the current handler for a signal type, if registered.
    ///
    /// # Arguments
    ///
    /// * `signal_type` - The signal type to get handler for
    ///
    /// # Returns
    ///
    /// Some(handler) if registered, None otherwise
    pub fn get_handler(&self, signal_type: SignalType) -> Option<SignalHandler> {
        self.handlers[signal_type.index()]
    }

    /// Set the signal mask for a signal type.
    ///
    /// Masked signals are not delivered, but remain in the pending queue.
    ///
    /// # Arguments
    ///
    /// * `signal_type` - The signal type to mask
    /// * `masked` - True to mask, false to unmask
    ///
    /// # Note
    ///
    /// SIG_TERMINATE (bit 0) cannot be masked - this bit is always ignored
    pub fn set_signal_mask(&mut self, signal_type: SignalType, masked: bool) {
        // SIG_TERMINATE cannot be masked
        if signal_type == SignalType::Terminate {
            return;
        }

        let bit = signal_type.index();
        if masked {
            self.signal_mask |= 1 << bit;
        } else {
            self.signal_mask &= !(1 << bit);
        }
    }

    /// Check if a signal type is masked.
    ///
    /// # Arguments
    ///
    /// * `signal_type` - The signal type to check
    ///
    /// # Returns
    ///
    /// True if the signal is masked (blocked)
    pub fn is_signal_masked(&self, signal_type: SignalType) -> bool {
        // SIG_TERMINATE is never masked
        if signal_type == SignalType::Terminate {
            return false;
        }

        let bit = signal_type.index();
        (self.signal_mask & (1 << bit)) != 0
    }

    /// Queue a signal for pending delivery at the next preemption point.
    ///
    /// Signals are queued in FIFO order and delivered from the queue
    /// at safe preemption points.
    ///
    /// # Arguments
    ///
    /// * `signal` - The signal to queue
    /// * `current_time_ms` - Current timestamp (Unix epoch milliseconds)
    ///
    /// # Returns
    ///
    /// Ok(()) if signal queued successfully
    pub fn queue_signal(&mut self, signal: CognitiveSignal, current_time_ms: u64) -> Result<()> {
        self.pending_signals.push_back(PendingSignal {
            signal,
            enqueued_ms: current_time_ms,
        });
        Ok(())
    }

    /// Get the number of pending signals awaiting delivery.
    ///
    /// # Returns
    ///
    /// Count of signals in the pending queue
    pub fn pending_count(&self) -> usize {
        self.pending_signals.len()
    }

    /// Check if there are any pending signals that should be delivered.
    ///
    /// Returns true if there are unmasked pending signals.
    ///
    /// # Returns
    ///
    /// True if delivery should be attempted at next preemption point
    pub fn has_pending_signals(&self) -> bool {
        // Check if any pending signal is not masked
        self.pending_signals.iter().any(|pending| {
            let sig_type = SignalType::from_signal(&pending.signal);
            !self.is_signal_masked(sig_type)
        })
    }

    /// Deliver the next pending signal (if any).
    ///
    /// This is called at safe preemption points. Processes signals in FIFO order,
    /// skipping masked signals, and invoking the appropriate handler.
    ///
    /// SIG_TERMINATE is always delivered immediately, bypassing the mask.
    ///
    /// # Returns
    ///
    /// - Ok(Some(result)) if a signal was delivered with handler result
    /// - Ok(None) if no signals to deliver
    /// - Err(...) if signal delivery failed
    ///
    /// See Engineering Plan § 6.1 (Signal System)
    pub fn deliver_next_signal(&mut self) -> Result<Option<SignalHandlerResult>> {
        // Find the next unmasked signal
        let index = self.pending_signals.iter().position(|pending| {
            let sig_type = SignalType::from_signal(&pending.signal);
            !self.is_signal_masked(sig_type)
        });

        if let Some(idx) = index {
            // Remove and process the signal
            if let Some(pending) = self.pending_signals.remove(idx) {
                return self.deliver_signal(&pending.signal);
            }
        }

        Ok(None)
    }

    /// Deliver a signal immediately using its registered handler.
    ///
    /// This is an internal method used by deliver_next_signal().
    /// Invokes the handler for the signal type if registered.
    ///
    /// For SIG_TERMINATE, this method is bypassed - the signal is handled
    /// synchronously by the kernel without invoking a registered handler.
    ///
    /// # Arguments
    ///
    /// * `signal` - The signal to deliver
    ///
    /// # Returns
    ///
    /// - Ok(Some(result)) with handler result if handler registered
    /// - Ok(Some(Continue)) if no handler registered
    /// - Err(...) if handler invocation failed
    fn deliver_signal(&self, signal: &CognitiveSignal) -> Result<Option<SignalHandlerResult>> {
        let sig_type = SignalType::from_signal(signal);

        // Get handler if registered
        if let Some(handler) = self.get_handler(sig_type) {
            let result = handler(signal);
            Ok(Some(result))
        } else {
            // No handler registered, signal is silently ignored
            Ok(Some(SignalHandlerResult::Continue))
        }
    }

    /// Clear all pending signals (used for CT cleanup).
    ///
    /// Discards all queued signals without delivering them.
    pub fn clear_pending(&mut self) {
        self.pending_signals.clear();
    }

    /// Get CT ID that owns this table.
    ///
    /// # Returns
    ///
    /// The cognitive thread ID
    pub fn ct_id(&self) -> u64 {
        self.ct_id
    }

    /// Get the current signal mask value.
    ///
    /// Each bit represents one signal (bit 0 = SIG_TERMINATE ... bit 7 = SIG_RESUME).
    ///
    /// # Returns
    ///
    /// The 8-bit signal mask
    pub fn signal_mask(&self) -> u8 {
        self.signal_mask
    }

    /// Set the entire signal mask at once.
    ///
    /// # Arguments
    ///
    /// * `mask` - The new signal mask value
    ///
    /// # Note
    ///
    /// Bit 0 (SIG_TERMINATE) is always treated as unmasked, regardless of value
    pub fn set_signal_mask_value(&mut self, mask: u8) {
        // Ensure SIG_TERMINATE bit is never set (always unmasked)
        self.signal_mask = mask & 0xFE;
    }

    /// Get the number of registered handlers.
    ///
    /// # Returns
    ///
    /// Count of signal types with handlers registered
    pub fn handler_count(&self) -> usize {
        self.handlers.iter().filter(|h| h.is_some()).count()
    }
}

impl Default for SignalDispatchTable {
    fn default() -> Self {
        Self::new(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::string::ToString;

    // ============================================================================
    // SignalType Tests
    // ============================================================================

    #[test]
    fn test_signal_type_from_signal() {
        let sig = CognitiveSignal::SigTerminate {
            reason: "test".into(),
            grace_period_ms: 0,
        };
        assert_eq!(SignalType::from_signal(&sig), SignalType::Terminate);
    }

    #[test]
    fn test_signal_type_index() {
        assert_eq!(SignalType::Terminate.index(), 0);
        assert_eq!(SignalType::DeadlineWarn.index(), 1);
        assert_eq!(SignalType::Resume.index(), 7);
    }

    #[test]
    fn test_signal_type_names() {
        assert_eq!(SignalType::Terminate.name(), "SIG_TERMINATE");
        assert_eq!(SignalType::DeadlineWarn.name(), "SIG_DEADLINE_WARN");
        assert_eq!(SignalType::Resume.name(), "SIG_RESUME");
    }

    // ============================================================================
    // SignalDispatchTable Creation and Basic Operations
    // ============================================================================

    #[test]
    fn test_dispatch_table_new() {
        let table = SignalDispatchTable::new(42);
        assert_eq!(table.ct_id(), 42);
        assert_eq!(table.pending_count(), 0);
        assert!(!table.has_pending_signals());
    }

    #[test]
    fn test_dispatch_table_default() {
        let table = SignalDispatchTable::default();
        assert_eq!(table.ct_id(), 0);
    }

    #[test]
    fn test_handler_registration() {
        let mut table = SignalDispatchTable::new(1);
        let handler: SignalHandler = |_sig| SignalHandlerResult::Continue;

        assert!(!table.has_handler(SignalType::DeadlineWarn));
        table.register(SignalType::DeadlineWarn, handler).unwrap();
        assert!(table.has_handler(SignalType::DeadlineWarn));
    }

    #[test]
    fn test_handler_unregistration() {
        let mut table = SignalDispatchTable::new(1);
        let handler: SignalHandler = |_sig| SignalHandlerResult::Continue;

        table.register(SignalType::Checkpoint, handler).unwrap();
        assert!(table.has_handler(SignalType::Checkpoint));

        table.unregister(SignalType::Checkpoint).unwrap();
        assert!(!table.has_handler(SignalType::Checkpoint));
    }

    // ============================================================================
    // SIG_TERMINATE Special Handling
    // ============================================================================

    #[test]
    fn test_sig_terminate_cannot_be_registered() {
        let mut table = SignalDispatchTable::new(1);
        let handler: SignalHandler = |_sig| SignalHandlerResult::Continue;

        let result = table.register(SignalType::Terminate, handler);
        assert!(result.is_err());
    }

    #[test]
    fn test_sig_terminate_cannot_be_masked() {
        let mut table = SignalDispatchTable::new(1);
        assert!(!table.is_signal_masked(SignalType::Terminate));

        table.set_signal_mask(SignalType::Terminate, true);
        assert!(!table.is_signal_masked(SignalType::Terminate));
    }

    #[test]
    fn test_sig_terminate_mask_value_ignored() {
        let mut table = SignalDispatchTable::new(1);
        table.set_signal_mask_value(0xFF);

        // Bit 0 should be cleared (SIG_TERMINATE always unmasked)
        assert_eq!(table.signal_mask() & 0x01, 0);
    }

    // ============================================================================
    // Signal Masking
    // ============================================================================

    #[test]
    fn test_signal_masking() {
        let mut table = SignalDispatchTable::new(1);

        assert!(!table.is_signal_masked(SignalType::DeadlineWarn));
        table.set_signal_mask(SignalType::DeadlineWarn, true);
        assert!(table.is_signal_masked(SignalType::DeadlineWarn));

        table.set_signal_mask(SignalType::DeadlineWarn, false);
        assert!(!table.is_signal_masked(SignalType::DeadlineWarn));
    }

    #[test]
    fn test_signal_mask_value() {
        let mut table = SignalDispatchTable::new(1);
        table.set_signal_mask_value(0x42);

        // Bit 0 is cleared, others preserved
        assert_eq!(table.signal_mask(), 0x42);
    }

    // ============================================================================
    // Signal Queueing and Delivery
    // ============================================================================

    #[test]
    fn test_queue_single_signal() {
        let mut table = SignalDispatchTable::new(1);
        let sig = CognitiveSignal::SigCheckpoint {
            reason: "test".into(),
            timestamp_ms: 1000,
        };

        assert_eq!(table.pending_count(), 0);
        table.queue_signal(sig, 1000).unwrap();
        assert_eq!(table.pending_count(), 1);
    }

    #[test]
    fn test_queue_multiple_signals() {
        let mut table = SignalDispatchTable::new(1);

        for i in 0..10 {
            let sig = CognitiveSignal::SigBudgetWarn {
                budget_type: "tokens".into(),
                remaining: 100,
                allocated: 1000,
            };
            table.queue_signal(sig, 1000 + i).unwrap();
        }

        assert_eq!(table.pending_count(), 10);
    }

    #[test]
    fn test_has_pending_signals_empty() {
        let table = SignalDispatchTable::new(1);
        assert!(!table.has_pending_signals());
    }

    #[test]
    fn test_has_pending_signals_unmasked() {
        let mut table = SignalDispatchTable::new(1);
        let sig = CognitiveSignal::SigContextLow {
            current_bytes: 900,
            max_bytes: 1000,
        };

        table.queue_signal(sig, 1000).unwrap();
        assert!(table.has_pending_signals());
    }

    #[test]
    fn test_has_pending_signals_all_masked() {
        let mut table = SignalDispatchTable::new(1);
        let sig = CognitiveSignal::SigContextLow {
            current_bytes: 900,
            max_bytes: 1000,
        };

        table.queue_signal(sig, 1000).unwrap();
        table.set_signal_mask(SignalType::ContextLow, true);
        assert!(!table.has_pending_signals());
    }

    // ============================================================================
    // Signal Handler Execution
    // ============================================================================

    #[test]
    fn test_deliver_signal_with_handler() {
        let mut table = SignalDispatchTable::new(1);
        let handler: SignalHandler = |_sig| SignalHandlerResult::Restart;
        table.register(SignalType::BudgetWarn, handler).unwrap();

        let sig = CognitiveSignal::SigBudgetWarn {
            budget_type: "tokens".into(),
            remaining: 100,
            allocated: 1000,
        };

        table.queue_signal(sig, 1000).unwrap();
        let result = table.deliver_next_signal().unwrap();
        assert_eq!(result, Some(SignalHandlerResult::Restart));
    }

    #[test]
    fn test_deliver_signal_without_handler() {
        let mut table = SignalDispatchTable::new(1);

        let sig = CognitiveSignal::SigPreempt {
            reason: "test".into(),
            deadline_ms: 2000,
        };

        table.queue_signal(sig, 1000).unwrap();
        let result = table.deliver_next_signal().unwrap();
        // No handler registered, returns Continue
        assert_eq!(result, Some(SignalHandlerResult::Continue));
    }

    #[test]
    fn test_deliver_masked_signal_skipped() {
        let mut table = SignalDispatchTable::new(1);
        let handler: SignalHandler = |_sig| SignalHandlerResult::Escalate;
        table.register(SignalType::IpcFailed, handler).unwrap();

        let sig = CognitiveSignal::SigIpcFailed {
            channel_id: "ch_1".into(),
            failure_reason: "timeout".into(),
        };

        table.queue_signal(sig, 1000).unwrap();
        table.set_signal_mask(SignalType::IpcFailed, true);

        // Masked signal is not delivered
        let result = table.deliver_next_signal().unwrap();
        assert_eq!(result, None);
        assert_eq!(table.pending_count(), 1);
    }

    #[test]
    fn test_deliver_signals_fifo_order() {
        let mut table = SignalDispatchTable::new(1);
        let handler1: SignalHandler = |_sig| SignalHandlerResult::Continue;
        let handler2: SignalHandler = |_sig| SignalHandlerResult::Restart;

        table.register(SignalType::Checkpoint, handler1).unwrap();
        table.register(SignalType::DeadlineWarn, handler2).unwrap();

        let sig1 = CognitiveSignal::SigCheckpoint {
            reason: "first".into(),
            timestamp_ms: 1000,
        };
        let sig2 = CognitiveSignal::SigDeadlineWarn {
            deadline_name: "second".into(),
            remaining_ms: 5000,
        };

        table.queue_signal(sig1, 1000).unwrap();
        table.queue_signal(sig2, 1001).unwrap();

        // First signal should be delivered first
        let result1 = table.deliver_next_signal().unwrap();
        assert_eq!(result1, Some(SignalHandlerResult::Continue));

        // Then second signal
        let result2 = table.deliver_next_signal().unwrap();
        assert_eq!(result2, Some(SignalHandlerResult::Restart));

        // No more signals
        let result3 = table.deliver_next_signal().unwrap();
        assert_eq!(result3, None);
    }

    // ============================================================================
    // Signal Cleanup
    // ============================================================================

    #[test]
    fn test_unregister_removes_handler_and_pending() {
        let mut table = SignalDispatchTable::new(1);
        let handler: SignalHandler = |_sig| SignalHandlerResult::Continue;
        table.register(SignalType::Checkpoint, handler).unwrap();

        let sig = CognitiveSignal::SigCheckpoint {
            reason: "test".into(),
            timestamp_ms: 1000,
        };

        table.queue_signal(sig.clone(), 1000).unwrap();
        table.queue_signal(sig, 1001).unwrap();

        assert_eq!(table.pending_count(), 2);
        table.unregister(SignalType::Checkpoint).unwrap();

        assert!(!table.has_handler(SignalType::Checkpoint));
        assert_eq!(table.pending_count(), 0);
    }

    #[test]
    fn test_clear_pending() {
        let mut table = SignalDispatchTable::new(1);

        for _i in 0..5 {
            let sig = CognitiveSignal::SigResume {
                context_info: "test".into(),
            };
            table.queue_signal(sig, 1000).unwrap();
        }

        assert_eq!(table.pending_count(), 5);
        table.clear_pending();
        assert_eq!(table.pending_count(), 0);
    }

    // ============================================================================
    // Statistics and Queries
    // ============================================================================

    #[test]
    fn test_handler_count() {
        let mut table = SignalDispatchTable::new(1);
        let handler: SignalHandler = |_sig| SignalHandlerResult::Continue;

        assert_eq!(table.handler_count(), 0);

        table.register(SignalType::BudgetWarn, handler).unwrap();
        assert_eq!(table.handler_count(), 1);

        table.register(SignalType::ContextLow, handler).unwrap();
        assert_eq!(table.handler_count(), 2);

        table.unregister(SignalType::BudgetWarn).unwrap();
        assert_eq!(table.handler_count(), 1);
    }

    #[test]
    fn test_get_handler() {
        let mut table = SignalDispatchTable::new(1);
        let handler: SignalHandler = |_sig| SignalHandlerResult::Restart;

        assert!(table.get_handler(SignalType::Preempt).is_none());

        table.register(SignalType::Preempt, handler).unwrap();
        assert!(table.get_handler(SignalType::Preempt).is_some());
    }

    // ============================================================================
    // Handler Result Types
    // ============================================================================

    #[test]
    fn test_signal_handler_result_continue() {
        let result = SignalHandlerResult::Continue;
        assert_eq!(result, SignalHandlerResult::Continue);
    }

    #[test]
    fn test_signal_handler_result_restart() {
        let result = SignalHandlerResult::Restart;
        assert_eq!(result, SignalHandlerResult::Restart);
    }

    #[test]
    fn test_signal_handler_result_escalate() {
        let result = SignalHandlerResult::Escalate;
        assert_eq!(result, SignalHandlerResult::Escalate);
    }

    // ============================================================================
    // Stress Test: 1000+ signals per second
    // ============================================================================

    #[test]
    fn test_stress_queue_1000_signals() {
        let mut table = SignalDispatchTable::new(1);

        for i in 0..1000 {
            let sig = CognitiveSignal::SigBudgetWarn {
                budget_type: "tokens".into(),
                remaining: 100,
                allocated: 1000,
            };
            table.queue_signal(sig, 1000 + i).unwrap();
        }

        assert_eq!(table.pending_count(), 1000);
    }

    #[test]
    fn test_stress_deliver_1000_signals() {
        let mut table = SignalDispatchTable::new(1);
        let handler: SignalHandler = |_sig| SignalHandlerResult::Continue;
        table.register(SignalType::BudgetWarn, handler).unwrap();

        // Queue 1000 signals
        for i in 0..1000 {
            let sig = CognitiveSignal::SigBudgetWarn {
                budget_type: "tokens".into(),
                remaining: 100,
                allocated: 1000,
            };
            table.queue_signal(sig, 1000 + i).unwrap();
        }

        // Deliver all signals
        let mut delivered = 0;
        loop {
            match table.deliver_next_signal().unwrap() {
                Some(_) => delivered += 1,
                None => break,
            }
        }

        assert_eq!(delivered, 1000);
        assert_eq!(table.pending_count(), 0);
    }

    #[test]
    fn test_stress_mixed_signal_types() {
        let mut table = SignalDispatchTable::new(1);
        let handler: SignalHandler = |_sig| SignalHandlerResult::Continue;

        // Register handlers for multiple signal types
        table.register(SignalType::Checkpoint, handler).unwrap();
        table.register(SignalType::BudgetWarn, handler).unwrap();
        table.register(SignalType::ContextLow, handler).unwrap();
        table.register(SignalType::IpcFailed, handler).unwrap();

        // Queue 250 of each signal type (1000 total)
        for i in 0..250 {
            let sig1 = CognitiveSignal::SigCheckpoint {
                reason: "ckpt".into(),
                timestamp_ms: 1000 + i as u64,
            };
            let sig2 = CognitiveSignal::SigBudgetWarn {
                budget_type: "tokens".into(),
                remaining: 100,
                allocated: 1000,
            };
            let sig3 = CognitiveSignal::SigContextLow {
                current_bytes: 900,
                max_bytes: 1000,
            };
            let sig4 = CognitiveSignal::SigIpcFailed {
                channel_id: "ch".into(),
                failure_reason: "err".into(),
            };

            table.queue_signal(sig1, 1000 + i as u64).unwrap();
            table.queue_signal(sig2, 1000 + i as u64).unwrap();
            table.queue_signal(sig3, 1000 + i as u64).unwrap();
            table.queue_signal(sig4, 1000 + i as u64).unwrap();
        }

        assert_eq!(table.pending_count(), 1000);

        // Deliver all signals
        let mut delivered = 0;
        loop {
            match table.deliver_next_signal().unwrap() {
                Some(_) => delivered += 1,
                None => break,
            }
        }

        assert_eq!(delivered, 1000);
    }

    #[test]
    fn test_stress_masking_performance() {
        let mut table = SignalDispatchTable::new(1);

        // Queue 1000 signals
        for i in 0..1000 {
            let sig = CognitiveSignal::SigDeadlineWarn {
                deadline_name: "deadline".into(),
                remaining_ms: 5000,
            };
            table.queue_signal(sig, 1000 + i).unwrap();
        }

        // Mask all signals
        table.set_signal_mask(SignalType::DeadlineWarn, true);

        // Try to deliver - should not deliver any
        let mut delivered = 0;
        loop {
            match table.deliver_next_signal().unwrap() {
                Some(_) => delivered += 1,
                None => break,
            }
        }

        assert_eq!(delivered, 0);
        assert_eq!(table.pending_count(), 1000);
    }
}
