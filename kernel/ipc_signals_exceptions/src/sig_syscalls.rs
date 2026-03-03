// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! # Signal Syscall Handlers
//!
//! This module implements the syscall handlers for signal registration and
//! unregistration operations. These are the kernel entry points for
//! controlling signal delivery.
//!
//! ## Syscalls
//!
//! ### sig_register(signal_type, handler_fn_ptr) -> Result<()>
//!
//! Registers a signal handler for the given signal type. The handler is
//! invoked when the signal is delivered at a safe preemption point.
//!
//! Constraints:
//! - signal_type must be valid (0-7)
//! - handler_fn_ptr must be a valid user-space function pointer
//! - SIG_TERMINATE (type 0) cannot be registered (returns PermissionDenied)
//! - Handler must not panic or corrupt state
//!
//! ### sig_unregister(signal_type) -> Result<()>
//!
//! Unregisters a signal handler for the given signal type. Any pending
//! signals of this type are discarded.
//!
//! Constraints:
//! - signal_type must be valid (0-7)
//! - Pending signals are safely removed
//!
//! ## SIG_TERMINATE Special Handling
//!
//! SIG_TERMINATE (index 0) is uncatchable and cannot be registered. Attempts
//! to register it return PermissionDenied. The signal is delivered synchronously
//! by the kernel without consulting any registered handler, and always terminates
//! the CT.
//!
//! ## References
//!
//! - Engineering Plan § 6.1 (Signal System)
//! - Week 4 Objective: sig_register and sig_unregister syscall implementations

#![allow(dead_code)]

use crate::error::{CsError, IpcError, Result};
use crate::signal_dispatch::{SignalDispatchTable, SignalHandler, SignalType};

/// Signal syscall error types.
///
/// Additional error variants specific to signal syscalls.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SignalSyscallError {
    /// Invalid signal type (must be 0-7)
    InvalidSignalType,
    /// Invalid handler function pointer (null or misaligned)
    InvalidHandlerPointer,
    /// SIG_TERMINATE cannot be registered or unregistered
    TerminateNotAllowed,
    /// Handler validation failed
    HandlerValidationFailed,
}

impl SignalSyscallError {
    /// Convert to CsError
    pub fn to_cs_error(self) -> CsError {
        CsError::Ipc(IpcError::Other(alloc::format!("{:?}", self)))
    }
}

/// Signal registration syscall handler.
///
/// Validates the signal type and handler pointer, then registers the handler
/// with the CT's signal dispatch table.
///
/// # Arguments
///
/// * `dispatch_table` - The CT's signal dispatch table (mutable)
/// * `signal_type` - The signal type to register (0-7)
/// * `handler_fn_ptr` - User-space function pointer to the handler
///
/// # Returns
///
/// - Ok(()) if handler registered successfully
/// - Err(PermissionDenied) if trying to register SIG_TERMINATE
/// - Err(InvalidValue) if signal_type is out of range
/// - Err(BadPointer) if handler pointer is invalid
///
/// See Engineering Plan § 6.1 (Signal System)
pub fn sig_register(
    dispatch_table: &mut SignalDispatchTable,
    signal_type: u8,
    handler_fn_ptr: usize,
) -> Result<()> {
    // Validate signal type (0-7)
    if signal_type > 7 {
        return Err(CsError::Ipc(IpcError::Other(
            "signal_type must be 0-7".to_string(),
        )));
    }

    // Validate handler pointer
    if handler_fn_ptr == 0 || (handler_fn_ptr & 0x01) != 0 {
        return Err(CsError::Ipc(IpcError::Other(
            "invalid handler pointer".to_string(),
        )));
    }

    // Convert to SignalType
    let sig_type = match signal_type {
        0 => SignalType::Terminate,
        1 => SignalType::DeadlineWarn,
        2 => SignalType::Checkpoint,
        3 => SignalType::BudgetWarn,
        4 => SignalType::ContextLow,
        5 => SignalType::IpcFailed,
        6 => SignalType::Preempt,
        7 => SignalType::Resume,
        _ => unreachable!(),
    };

    // SIG_TERMINATE cannot be registered
    if sig_type == SignalType::Terminate {
        return Err(CsError::Ipc(IpcError::Other(
            "SIG_TERMINATE cannot be registered".to_string(),
        )));
    }

    // SAFETY: Handler pointer validation should be done by the kernel
    // before calling this function. Here we assume it's a valid function
    // pointer in user-space that has been validated by the kernel's
    // capability system.
    let handler: SignalHandler = unsafe { core::mem::transmute(handler_fn_ptr) };

    // Register the handler
    dispatch_table.register(sig_type, handler)
}

/// Signal unregistration syscall handler.
///
/// Removes the handler for the given signal type. Any pending signals
/// of this type are discarded.
///
/// # Arguments
///
/// * `dispatch_table` - The CT's signal dispatch table (mutable)
/// * `signal_type` - The signal type to unregister (0-7)
///
/// # Returns
///
/// - Ok(()) if handler unregistered successfully
/// - Err(...) if signal_type is invalid or unregistration failed
///
/// See Engineering Plan § 6.1 (Signal System)
pub fn sig_unregister(dispatch_table: &mut SignalDispatchTable, signal_type: u8) -> Result<()> {
    // Validate signal type (0-7)
    if signal_type > 7 {
        return Err(CsError::Ipc(IpcError::Other(
            "signal_type must be 0-7".to_string(),
        )));
    }

    // Convert to SignalType
    let sig_type = match signal_type {
        0 => SignalType::Terminate,
        1 => SignalType::DeadlineWarn,
        2 => SignalType::Checkpoint,
        3 => SignalType::BudgetWarn,
        4 => SignalType::ContextLow,
        5 => SignalType::IpcFailed,
        6 => SignalType::Preempt,
        7 => SignalType::Resume,
        _ => unreachable!(),
    };

    // Unregister the handler
    dispatch_table.unregister(sig_type)
}

/// Signal mask syscall handler.
///
/// Sets the signal mask for a signal type. Masked signals remain in the
/// pending queue but are not delivered.
///
/// # Arguments
///
/// * `dispatch_table` - The CT's signal dispatch table (mutable)
/// * `signal_type` - The signal type to mask (0-7)
/// * `masked` - True to mask, false to unmask
///
/// # Returns
///
/// - Ok(()) if mask was set successfully
/// - Err(...) if signal_type is invalid
pub fn sig_mask(
    dispatch_table: &mut SignalDispatchTable,
    signal_type: u8,
    masked: bool,
) -> Result<()> {
    // Validate signal type (0-7)
    if signal_type > 7 {
        return Err(CsError::Ipc(IpcError::Other(
            "signal_type must be 0-7".to_string(),
        )));
    }

    // Convert to SignalType
    let sig_type = match signal_type {
        0 => SignalType::Terminate,
        1 => SignalType::DeadlineWarn,
        2 => SignalType::Checkpoint,
        3 => SignalType::BudgetWarn,
        4 => SignalType::ContextLow,
        5 => SignalType::IpcFailed,
        6 => SignalType::Preempt,
        7 => SignalType::Resume,
        _ => unreachable!(),
    };

    // Set the mask (SIG_TERMINATE cannot be masked, but this is enforced
    // in the SignalDispatchTable)
    dispatch_table.set_signal_mask(sig_type, masked);
    Ok(())
}

/// Get signal mask syscall handler.
///
/// Retrieves the current signal mask for a signal type.
///
/// # Arguments
///
/// * `dispatch_table` - The CT's signal dispatch table
/// * `signal_type` - The signal type to query (0-7)
///
/// # Returns
///
/// - Ok(true) if signal is masked
/// - Ok(false) if signal is unmasked
/// - Err(...) if signal_type is invalid
pub fn sig_get_mask(dispatch_table: &SignalDispatchTable, signal_type: u8) -> Result<bool> {
    // Validate signal type (0-7)
    if signal_type > 7 {
        return Err(CsError::Ipc(IpcError::Other(
            "signal_type must be 0-7".to_string(),
        )));
    }

    // Convert to SignalType
    let sig_type = match signal_type {
        0 => SignalType::Terminate,
        1 => SignalType::DeadlineWarn,
        2 => SignalType::Checkpoint,
        3 => SignalType::BudgetWarn,
        4 => SignalType::ContextLow,
        5 => SignalType::IpcFailed,
        6 => SignalType::Preempt,
        7 => SignalType::Resume,
        _ => unreachable!(),
    };

    Ok(dispatch_table.is_signal_masked(sig_type))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signal::CognitiveSignal;
    use crate::signal_dispatch::SignalHandlerResult;
use alloc::format;
use alloc::string::ToString;

    // ============================================================================
    // sig_register Tests
    // ============================================================================

    #[test]
    fn test_sig_register_valid_handler() {
        let mut table = SignalDispatchTable::new(1);
        let handler: SignalHandler = |_sig: &CognitiveSignal| SignalHandlerResult::Continue;
        let handler_ptr = handler as usize;

        let result = sig_register(&mut table, 1, handler_ptr);
        assert!(result.is_ok());
        assert!(table.has_handler(SignalType::DeadlineWarn));
    }

    #[test]
    fn test_sig_register_all_signal_types() {
        let mut table = SignalDispatchTable::new(1);
        let handler: SignalHandler = |_sig: &CognitiveSignal| SignalHandlerResult::Continue;
        let handler_ptr = handler as usize;

        // Register all signal types except SIG_TERMINATE
        for sig_type in 1..=7 {
            let result = sig_register(&mut table, sig_type, handler_ptr);
            assert!(result.is_ok());
        }

        assert_eq!(table.handler_count(), 7);
    }

    #[test]
    fn test_sig_register_invalid_signal_type() {
        let mut table = SignalDispatchTable::new(1);
        let handler: SignalHandler = |_sig: &CognitiveSignal| SignalHandlerResult::Continue;
        let handler_ptr = handler as usize;

        let result = sig_register(&mut table, 8, handler_ptr);
        assert!(result.is_err());
    }

    #[test]
    fn test_sig_register_invalid_handler_pointer_null() {
        let mut table = SignalDispatchTable::new(1);
        let result = sig_register(&mut table, 1, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_sig_register_invalid_handler_pointer_misaligned() {
        let mut table = SignalDispatchTable::new(1);
        let result = sig_register(&mut table, 1, 0x1001);
        assert!(result.is_err());
    }

    #[test]
    fn test_sig_register_terminate_not_allowed() {
        let mut table = SignalDispatchTable::new(1);
        let handler: SignalHandler = |_sig: &CognitiveSignal| SignalHandlerResult::Continue;
        let handler_ptr = handler as usize;

        let result = sig_register(&mut table, 0, handler_ptr);
        assert!(result.is_err());
    }

    #[test]
    fn test_sig_register_overwrites_previous() {
        let mut table = SignalDispatchTable::new(1);
        let handler1: SignalHandler = |_sig: &CognitiveSignal| SignalHandlerResult::Continue;
        let handler2: SignalHandler = |_sig: &CognitiveSignal| SignalHandlerResult::Restart;

        let ptr1 = handler1 as usize;
        let ptr2 = handler2 as usize;

        sig_register(&mut table, 2, ptr1).unwrap();
        assert!(table.has_handler(SignalType::Checkpoint));

        sig_register(&mut table, 2, ptr2).unwrap();
        assert!(table.has_handler(SignalType::Checkpoint));
    }

    // ============================================================================
    // sig_unregister Tests
    // ============================================================================

    #[test]
    fn test_sig_unregister_valid() {
        let mut table = SignalDispatchTable::new(1);
        let handler: SignalHandler = |_sig: &CognitiveSignal| SignalHandlerResult::Continue;
        let handler_ptr = handler as usize;

        sig_register(&mut table, 1, handler_ptr).unwrap();
        assert!(table.has_handler(SignalType::DeadlineWarn));

        let result = sig_unregister(&mut table, 1);
        assert!(result.is_ok());
        assert!(!table.has_handler(SignalType::DeadlineWarn));
    }

    #[test]
    fn test_sig_unregister_all_signal_types() {
        let mut table = SignalDispatchTable::new(1);
        let handler: SignalHandler = |_sig: &CognitiveSignal| SignalHandlerResult::Continue;
        let handler_ptr = handler as usize;

        // Register all signal types
        for sig_type in 1..=7 {
            sig_register(&mut table, sig_type, handler_ptr).unwrap();
        }

        // Unregister all
        for sig_type in 1..=7 {
            sig_unregister(&mut table, sig_type).unwrap();
        }

        assert_eq!(table.handler_count(), 0);
    }

    #[test]
    fn test_sig_unregister_invalid_signal_type() {
        let mut table = SignalDispatchTable::new(1);
        let result = sig_unregister(&mut table, 8);
        assert!(result.is_err());
    }

    #[test]
    fn test_sig_unregister_nonexistent_handler() {
        let mut table = SignalDispatchTable::new(1);
        let result = sig_unregister(&mut table, 3);
        assert!(result.is_ok());
    }

    #[test]
    fn test_sig_unregister_clears_pending() {
        let mut table = SignalDispatchTable::new(1);
        let handler: SignalHandler = |_sig: &CognitiveSignal| SignalHandlerResult::Continue;
        let handler_ptr = handler as usize;

        sig_register(&mut table, 2, handler_ptr).unwrap();

        let sig = CognitiveSignal::SigCheckpoint {
            reason: "test".into(),
            timestamp_ms: 1000,
        };
        table.queue_signal(sig.clone(), 1000).unwrap();
        table.queue_signal(sig, 1001).unwrap();

        assert_eq!(table.pending_count(), 2);

        sig_unregister(&mut table, 2).unwrap();
        assert_eq!(table.pending_count(), 0);
    }

    // ============================================================================
    // sig_mask Tests
    // ============================================================================

    #[test]
    fn test_sig_mask_set() {
        let mut table = SignalDispatchTable::new(1);

        let result = sig_mask(&mut table, 1, true);
        assert!(result.is_ok());
        assert!(table.is_signal_masked(SignalType::DeadlineWarn));
    }

    #[test]
    fn test_sig_mask_unset() {
        let mut table = SignalDispatchTable::new(1);
        sig_mask(&mut table, 1, true).unwrap();
        assert!(table.is_signal_masked(SignalType::DeadlineWarn));

        let result = sig_mask(&mut table, 1, false);
        assert!(result.is_ok());
        assert!(!table.is_signal_masked(SignalType::DeadlineWarn));
    }

    #[test]
    fn test_sig_mask_invalid_signal_type() {
        let mut table = SignalDispatchTable::new(1);
        let result = sig_mask(&mut table, 8, true);
        assert!(result.is_err());
    }

    #[test]
    fn test_sig_mask_terminate_ignored() {
        let mut table = SignalDispatchTable::new(1);
        sig_mask(&mut table, 0, true).unwrap();
        assert!(!table.is_signal_masked(SignalType::Terminate));
    }

    #[test]
    fn test_sig_mask_multiple_signals() {
        let mut table = SignalDispatchTable::new(1);

        for sig_type in 1..=7 {
            sig_mask(&mut table, sig_type, true).unwrap();
        }

        for sig_type in 1..=7 {
            assert!(table.is_signal_masked(match sig_type {
                1 => SignalType::DeadlineWarn,
                2 => SignalType::Checkpoint,
                3 => SignalType::BudgetWarn,
                4 => SignalType::ContextLow,
                5 => SignalType::IpcFailed,
                6 => SignalType::Preempt,
                7 => SignalType::Resume,
                _ => unreachable!(),
            }));
        }
    }

    // ============================================================================
    // sig_get_mask Tests
    // ============================================================================

    #[test]
    fn test_sig_get_mask_unmasked() {
        let table = SignalDispatchTable::new(1);
        let result = sig_get_mask(&table, 1).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_sig_get_mask_masked() {
        let mut table = SignalDispatchTable::new(1);
        sig_mask(&mut table, 1, true).unwrap();
        let result = sig_get_mask(&table, 1).unwrap();
        assert!(result);
    }

    #[test]
    fn test_sig_get_mask_invalid_signal_type() {
        let table = SignalDispatchTable::new(1);
        let result = sig_get_mask(&table, 8);
        assert!(result.is_err());
    }

    #[test]
    fn test_sig_get_mask_terminate() {
        let table = SignalDispatchTable::new(1);
        let result = sig_get_mask(&table, 0).unwrap();
        assert!(!result);
    }

    // ============================================================================
    // Integration Tests
    // ============================================================================

    #[test]
    fn test_register_mask_unregister_flow() {
        let mut table = SignalDispatchTable::new(1);
        let handler: SignalHandler = |_sig: &CognitiveSignal| SignalHandlerResult::Continue;
        let handler_ptr = handler as usize;

        // Register
        sig_register(&mut table, 3, handler_ptr).unwrap();
        assert!(table.has_handler(SignalType::BudgetWarn));

        // Mask
        sig_mask(&mut table, 3, true).unwrap();
        assert!(sig_get_mask(&table, 3).unwrap());

        // Queue signal
        let sig = CognitiveSignal::SigBudgetWarn {
            budget_type: "tokens".into(),
            remaining: 100,
            allocated: 1000,
        };
        table.queue_signal(sig, 1000).unwrap();

        // Signal should be pending but not deliverable
        assert!(!table.has_pending_signals());

        // Unmask
        sig_mask(&mut table, 3, false).unwrap();
        assert!(!sig_get_mask(&table, 3).unwrap());
        assert!(table.has_pending_signals());

        // Unregister
        sig_unregister(&mut table, 3).unwrap();
        assert!(!table.has_handler(SignalType::BudgetWarn));
        assert_eq!(table.pending_count(), 0);
    }

    #[test]
    fn test_stress_register_unregister_cycles() {
        let mut table = SignalDispatchTable::new(1);
        let handler: SignalHandler = |_sig: &CognitiveSignal| SignalHandlerResult::Continue;
        let handler_ptr = handler as usize;

        for _cycle in 0..100 {
            for sig_type in 1..=7 {
                sig_register(&mut table, sig_type, handler_ptr).unwrap();
            }
            assert_eq!(table.handler_count(), 7);

            for sig_type in 1..=7 {
                sig_unregister(&mut table, sig_type).unwrap();
            }
            assert_eq!(table.handler_count(), 0);
        }
    }

    #[test]
    fn test_stress_rapid_masking() {
        let mut table = SignalDispatchTable::new(1);

        for _iter in 0..1000 {
            sig_mask(&mut table, 1, true).unwrap();
            sig_mask(&mut table, 1, false).unwrap();
        }

        assert!(!sig_get_mask(&table, 1).unwrap());
    }
}
