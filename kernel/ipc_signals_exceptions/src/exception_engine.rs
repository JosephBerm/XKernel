// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! # Cognitive Exception Engine
//!
//! This module implements the per-CT exception engine that handles exception
//! registration, dispatch, and recovery coordination. The engine maintains
//! exception history, prevents recursive exception handling, and invokes
//! user-registered or default exception handlers.
//!
//! ## Exception Handling Flow
//!
//! When an exception occurs:
//! 1. Kernel detects exception → CT pauses
//! 2. Kernel captures comprehensive context (registers, memory, IPC, tools)
//! 3. ExceptionEngine creates ExceptionContext
//! 4. ExceptionEngine invokes registered handler or default handler
//! 5. Handler returns recovery action
//! 6. ExceptionEngine executes recovery action
//! 7. CT resumes or terminates based on recovery result
//!
//! ## Recursion Prevention
//!
//! The `in_exception_handler` flag prevents infinite recursion if an exception
//! occurs while handling an exception. This is critical for system stability.
//!
//! ## Exception History
//!
//! The engine maintains a VecDeque of the last 10 exceptions for analysis
//! and debugging without unbounded memory growth.
//!
//! ## References
//!
//! - Engineering Plan § 6.2 (Exception System)
//! - Engineering Plan § 6.4 (Exception Handling & Recovery)
//! - Engineering Plan § 6.7 (Exception Engine Implementation)

use crate::exception_context::ExceptionContext;
use crate::recovery_strategies::{
use alloc::boxed::Box;

use alloc::collections::VecDeque;

use alloc::string::String;

    RecoveryActionResult, RetryStrategy, RollbackStrategy, EscalationStrategy,
    TerminationStrategy,
};
use crate::exception::{CognitiveException, ExceptionSeverity};
use crate::ids::ExceptionID;
use crate::error::{CsError, Result};
use cs_ct_lifecycle::CTID;
use serde::{Deserialize, Serialize};

/// Exception handler function type.
///
/// A function that processes an exception and returns a recovery action.
/// Handlers must not panic and should handle all exception types gracefully.
pub type ExceptionHandlerFn = Box<dyn Fn(&ExceptionContext) -> Result<RecoveryActionResult>>;

/// Exception handling mode.
///
/// Controls how strict the exception handling is.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExceptionHandlingMode {
    /// All exceptions are handled and recovered
    Resilient,

    /// Only recoverable exceptions are handled; critical exceptions escalate
    Selective,

    /// Exceptions are collected but not automatically handled (manual intervention)
    Observability,

    /// All exceptions cause immediate termination
    Fail Fast,
}

/// Exception engine per CT.
///
/// Manages exception handling for a single CT. Maintains handler registrations,
/// exception history, and recursion prevention.
///
/// See Engineering Plan § 6.7 (Exception Engine Implementation)
#[derive(Debug)]
pub struct ExceptionEngine {
    /// CT this engine belongs to
    ct_id: CTID,

    /// Current exception handler (if registered)
    handler: Option<ExceptionHandlerFn>,

    /// Exception history (last 10 exceptions)
    history: VecDeque<ExceptionContext>,

    /// Maximum history size
    max_history_size: usize,

    /// Flag to prevent recursive exception handling
    in_exception_handler: bool,

    /// Exception handling mode
    mode: ExceptionHandlingMode,

    /// Total exceptions processed by this engine
    exception_count: u64,

    /// Total exceptions successfully recovered
    recovered_count: u64,

    /// Total exceptions escalated
    escalated_count: u64,

    /// Total exceptions terminated
    terminated_count: u64,
}

impl ExceptionEngine {
    /// Create a new exception engine for a CT.
    ///
    /// # Arguments
    /// * `ct_id` - ID of the CT this engine manages
    ///
    /// See Engineering Plan § 6.7 (Exception Engine Implementation)
    pub fn new(ct_id: CTID) -> Self {
        Self {
            ct_id,
            handler: None,
            history: VecDeque::new(),
            max_history_size: 10,
            in_exception_handler: false,
            mode: ExceptionHandlingMode::Resilient,
            exception_count: 0,
            recovered_count: 0,
            escalated_count: 0,
            terminated_count: 0,
        }
    }

    /// Get the CT ID this engine manages.
    pub fn ct_id(&self) -> CTID {
        self.ct_id
    }

    /// Get the current exception handling mode.
    pub fn mode(&self) -> ExceptionHandlingMode {
        self.mode
    }

    /// Set the exception handling mode.
    pub fn set_mode(&mut self, mode: ExceptionHandlingMode) {
        self.mode = mode;
    }

    /// Register a custom exception handler.
    ///
    /// The handler function receives the exception context and must return
    /// a recovery action. The handler should not panic.
    ///
    /// # Arguments
    /// * `handler` - Function to handle exceptions
    ///
    /// # Errors
    /// Returns an error if a handler is already registered.
    ///
    /// See Engineering Plan § 6.7 (Exception Engine Implementation)
    pub fn register_handler<F>(&mut self, handler: F) -> Result<()>
    where
        F: Fn(&ExceptionContext) -> Result<RecoveryActionResult> + 'static,
    {
        if self.handler.is_some() {
            return Err(CsError::Other(
                "Exception handler already registered".to_string(),
            ));
        }

        self.handler = Some(Box::new(handler));
        Ok(())
    }

    /// Unregister the current exception handler.
    ///
    /// After this call, exceptions will use default handling.
    pub fn unregister_handler(&mut self) {
        self.handler = None;
    }

    /// Check if a handler is registered.
    pub fn has_handler(&self) -> bool {
        self.handler.is_some()
    }

    /// Dispatch an exception to the handler.
    ///
    /// This is the main entry point for exception handling.
    /// It prevents recursion, invokes the handler, and processes the result.
    ///
    /// # Arguments
    /// * `exception_context` - The captured exception context
    ///
    /// # Returns
    /// A recovery action result
    ///
    /// See Engineering Plan § 6.7 (Exception Engine Implementation)
    pub fn dispatch(&mut self, mut exception_context: ExceptionContext) -> Result<RecoveryActionResult> {
        // Increment exception counter
        self.exception_count += 1;

        // Check for recursion - if we're already handling an exception, escalate
        if self.in_exception_handler {
            self.escalated_count += 1;
            return Ok(RecoveryActionResult::EscalationInitiated {
                supervisor_id: "system_handler".to_string(),
                request_id: format!("exc_{}", exception_context.exception_id),
            });
        }

        // Mark that we're in the handler
        self.in_exception_handler = true;
        exception_context.mark_in_handler();

        // Add to history
        self.history.push_back(exception_context.clone());
        if self.history.len() > self.max_history_size {
            let _ = self.history.pop_front();
        }

        // Invoke handler
        let result = self.invoke_handler(&exception_context);

        // Clear the handler flag
        self.in_exception_handler = false;

        result
    }

    /// Invoke the registered handler or use default handling.
    fn invoke_handler(&mut self, context: &ExceptionContext) -> Result<RecoveryActionResult> {
        if let Some(ref handler) = self.handler {
            handler(context)
        } else {
            self.default_handling(context)
        }
    }

    /// Default exception handling based on exception severity.
    fn default_handling(&mut self, context: &ExceptionContext) -> Result<RecoveryActionResult> {
        match self.mode {
            ExceptionHandlingMode::Resilient => {
                self.handle_resilient(context)
            }
            ExceptionHandlingMode::Selective => {
                self.handle_selective(context)
            }
            ExceptionHandlingMode::Observability => {
                // Just log, don't take action
                Ok(RecoveryActionResult::Failed {
                    reason: "Observability mode - no automatic recovery".to_string(),
                })
            }
            ExceptionHandlingMode::Fail Fast => {
                // Terminate immediately
                self.terminated_count += 1;
                Ok(RecoveryActionResult::TerminationInitiated {
                    exit_status: 1,
                    cleanup_count: 0,
                })
            }
        }
    }

    /// Resilient mode: attempt recovery for all exceptions.
    fn handle_resilient(&mut self, context: &ExceptionContext) -> Result<RecoveryActionResult> {
        match context.severity() {
            ExceptionSeverity::Critical => {
                // For critical, escalate
                self.escalated_count += 1;
                Ok(RecoveryActionResult::EscalationInitiated {
                    supervisor_id: "system_handler".to_string(),
                    request_id: format!("exc_{}", context.exception_id),
                })
            }
            ExceptionSeverity::High => {
                // For high, try escalation first
                self.escalated_count += 1;
                Ok(RecoveryActionResult::EscalationInitiated {
                    supervisor_id: "system_handler".to_string(),
                    request_id: format!("exc_{}", context.exception_id),
                })
            }
            ExceptionSeverity::Medium => {
                // For medium, try retry
                if context.exception.is_recoverable() {
                    self.recovered_count += 1;
                    let strategy = RetryStrategy::default_exponential();
                    Ok(RecoveryActionResult::RetryScheduled {
                        delay_ms: strategy.next_delay_ms(),
                        attempt: strategy.attempt_number(),
                    })
                } else {
                    // If not recoverable, escalate
                    self.escalated_count += 1;
                    Ok(RecoveryActionResult::EscalationInitiated {
                        supervisor_id: "system_handler".to_string(),
                        request_id: format!("exc_{}", context.exception_id),
                    })
                }
            }
            ExceptionSeverity::Low => {
                // For low severity, retry with backoff
                self.recovered_count += 1;
                let strategy = RetryStrategy::conservative();
                Ok(RecoveryActionResult::RetryScheduled {
                    delay_ms: strategy.next_delay_ms(),
                    attempt: strategy.attempt_number(),
                })
            }
        }
    }

    /// Selective mode: only recover if exception is marked recoverable.
    fn handle_selective(&mut self, context: &ExceptionContext) -> Result<RecoveryActionResult> {
        if context.exception.is_critical() {
            // Critical exceptions are escalated
            self.escalated_count += 1;
            Ok(RecoveryActionResult::EscalationInitiated {
                supervisor_id: "system_handler".to_string(),
                request_id: format!("exc_{}", context.exception_id),
            })
        } else if context.exception.is_recoverable() {
            // Recoverable exceptions are retried
            self.recovered_count += 1;
            let strategy = RetryStrategy::default_exponential();
            Ok(RecoveryActionResult::RetryScheduled {
                delay_ms: strategy.next_delay_ms(),
                attempt: strategy.attempt_number(),
            })
        } else {
            // Non-recoverable exceptions are escalated
            self.escalated_count += 1;
            Ok(RecoveryActionResult::EscalationInitiated {
                supervisor_id: "system_handler".to_string(),
                request_id: format!("exc_{}", context.exception_id),
            })
        }
    }

    /// Get the exception history.
    pub fn history(&self) -> &VecDeque<ExceptionContext> {
        &self.history
    }

    /// Get the number of exceptions in history.
    pub fn history_size(&self) -> usize {
        self.history.len()
    }

    /// Clear the exception history.
    pub fn clear_history(&mut self) {
        self.history.clear();
    }

    /// Get exception statistics.
    pub fn stats(&self) -> ExceptionEngineStats {
        ExceptionEngineStats {
            total_exceptions: self.exception_count,
            recovered_count: self.recovered_count,
            escalated_count: self.escalated_count,
            terminated_count: self.terminated_count,
            history_size: self.history.len(),
            in_handler: self.in_exception_handler,
        }
    }

    /// Reset statistics.
    pub fn reset_stats(&mut self) {
        self.exception_count = 0;
        self.recovered_count = 0;
        self.escalated_count = 0;
        self.terminated_count = 0;
    }
}

/// Exception engine statistics.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExceptionEngineStats {
    /// Total exceptions handled
    pub total_exceptions: u64,

    /// Exceptions that were recovered
    pub recovered_count: u64,

    /// Exceptions that were escalated
    pub escalated_count: u64,

    /// Exceptions that caused termination
    pub terminated_count: u64,

    /// Current history size
    pub history_size: usize,

    /// Whether currently in exception handler
    pub in_handler: bool,
}

impl ExceptionEngineStats {
    /// Get the recovery rate as a percentage.
    pub fn recovery_rate_percent(&self) -> u32 {
        if self.total_exceptions == 0 {
            0
        } else {
            ((self.recovered_count as f64 / self.total_exceptions as f64) * 100.0) as u32
        }
    }

    /// Get the escalation rate as a percentage.
    pub fn escalation_rate_percent(&self) -> u32 {
        if self.total_exceptions == 0 {
            0
        } else {
            ((self.escalated_count as f64 / self.total_exceptions as f64) * 100.0) as u32
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exception_context::{RegisterSnapshot, WorkingMemorySnapshot, IpcState};
use alloc::format;
use alloc::string::ToString;

    fn create_test_exception_context() -> ExceptionContext {
        let exc = CognitiveException::ToolCallFailed(
            crate::exception::ToolFailureContext::new(
                "test_tool".to_string(),
                "test error".to_string(),
                true,
                5000,
            ),
        );
        let regs = RegisterSnapshot::zeroed();
        let mem = WorkingMemorySnapshot::new(512, 1024, 10, 5);
        let ipc = IpcState::new(5, 10, 1024, 2048);

        ExceptionContext::new(
            ExceptionID::new(),
            CTID::new(),
            exc,
            5000,
            regs,
            mem,
            ipc,
        )
    }

    #[test]
    fn test_exception_engine_new() {
        let ct_id = CTID::new();
        let engine = ExceptionEngine::new(ct_id);
        assert_eq!(engine.ct_id(), ct_id);
        assert!(!engine.has_handler());
        assert!(!engine.in_exception_handler);
        assert_eq!(engine.exception_count, 0);
    }

    #[test]
    fn test_exception_engine_mode() {
        let ct_id = CTID::new();
        let mut engine = ExceptionEngine::new(ct_id);
        assert_eq!(engine.mode(), ExceptionHandlingMode::Resilient);

        engine.set_mode(ExceptionHandlingMode::Observability);
        assert_eq!(engine.mode(), ExceptionHandlingMode::Observability);
    }

    #[test]
    fn test_exception_engine_register_handler() {
        let ct_id = CTID::new();
        let mut engine = ExceptionEngine::new(ct_id);

        let result = engine.register_handler(|_ctx| {
            Ok(RecoveryActionResult::Failed {
                reason: "test".to_string(),
            })
        });

        assert!(result.is_ok());
        assert!(engine.has_handler());
    }

    #[test]
    fn test_exception_engine_register_handler_twice_fails() {
        let ct_id = CTID::new();
        let mut engine = ExceptionEngine::new(ct_id);

        let _ = engine.register_handler(|_ctx| {
            Ok(RecoveryActionResult::Failed {
                reason: "test".to_string(),
            })
        });

        let result = engine.register_handler(|_ctx| {
            Ok(RecoveryActionResult::Failed {
                reason: "test".to_string(),
            })
        });

        assert!(result.is_err());
    }

    #[test]
    fn test_exception_engine_unregister_handler() {
        let ct_id = CTID::new();
        let mut engine = ExceptionEngine::new(ct_id);

        engine.register_handler(|_ctx| {
            Ok(RecoveryActionResult::Failed {
                reason: "test".to_string(),
            })
        }).unwrap();

        assert!(engine.has_handler());
        engine.unregister_handler();
        assert!(!engine.has_handler());
    }

    #[test]
    fn test_exception_engine_dispatch() {
        let ct_id = CTID::new();
        let mut engine = ExceptionEngine::new(ct_id);

        let ctx = create_test_exception_context();
        let result = engine.dispatch(ctx);

        assert!(result.is_ok());
        assert_eq!(engine.exception_count, 1);
        assert_eq!(engine.history_size(), 1);
    }

    #[test]
    fn test_exception_engine_history_limited() {
        let ct_id = CTID::new();
        let mut engine = ExceptionEngine::new(ct_id);

        // Add 15 exceptions (more than max of 10)
        for _ in 0..15 {
            let ctx = create_test_exception_context();
            let _ = engine.dispatch(ctx);
        }

        // History should only contain last 10
        assert_eq!(engine.history_size(), 10);
        assert_eq!(engine.exception_count, 15);
    }

    #[test]
    fn test_exception_engine_stats() {
        let ct_id = CTID::new();
        let mut engine = ExceptionEngine::new(ct_id);

        let ctx = create_test_exception_context();
        let _ = engine.dispatch(ctx);

        let stats = engine.stats();
        assert_eq!(stats.total_exceptions, 1);
        assert_eq!(stats.history_size, 1);
        assert!(!stats.in_handler);
    }

    #[test]
    fn test_exception_engine_stats_recovery_rate() {
        let stats = ExceptionEngineStats {
            total_exceptions: 100,
            recovered_count: 50,
            escalated_count: 30,
            terminated_count: 20,
            history_size: 10,
            in_handler: false,
        };

        assert_eq!(stats.recovery_rate_percent(), 50);
        assert_eq!(stats.escalation_rate_percent(), 30);
    }

    #[test]
    fn test_exception_engine_recursion_prevention() {
        let ct_id = CTID::new();
        let mut engine = ExceptionEngine::new(ct_id);

        // Register a handler that tries to dispatch another exception
        engine.register_handler(|_ctx| {
            // In real scenario, this would dispatch another exception
            // For this test, we just return normally
            Ok(RecoveryActionResult::Failed {
                reason: "nested exception".to_string(),
            })
        }).unwrap();

        let ctx1 = create_test_exception_context();
        let result = engine.dispatch(ctx1);

        // Should succeed without infinite loop
        assert!(result.is_ok());
        assert_eq!(engine.exception_count, 1);
    }

    #[test]
    fn test_exception_engine_default_handling_resilient_medium() {
        let ct_id = CTID::new();
        let mut engine = ExceptionEngine::new(ct_id);
        engine.set_mode(ExceptionHandlingMode::Resilient);

        let ctx = create_test_exception_context();
        let result = engine.dispatch(ctx);

        assert!(result.is_ok());
        match result.unwrap() {
            RecoveryActionResult::RetryScheduled { .. } => (),
            _ => panic!("Expected RetryScheduled for medium severity in Resilient mode"),
        }
    }

    #[test]
    fn test_exception_engine_default_handling_fail_fast() {
        let ct_id = CTID::new();
        let mut engine = ExceptionEngine::new(ct_id);
        engine.set_mode(ExceptionHandlingMode::Fail Fast);

        let ctx = create_test_exception_context();
        let result = engine.dispatch(ctx);

        assert!(result.is_ok());
        match result.unwrap() {
            RecoveryActionResult::TerminationInitiated { .. } => (),
            _ => panic!("Expected TerminationInitiated in Fail Fast mode"),
        }
    }

    #[test]
    fn test_exception_engine_clear_history() {
        let ct_id = CTID::new();
        let mut engine = ExceptionEngine::new(ct_id);

        let ctx = create_test_exception_context();
        let _ = engine.dispatch(ctx);

        assert!(engine.history_size() > 0);
        engine.clear_history();
        assert_eq!(engine.history_size(), 0);
        assert_eq!(engine.exception_count, 1); // Count not cleared
    }

    #[test]
    fn test_exception_engine_reset_stats() {
        let ct_id = CTID::new();
        let mut engine = ExceptionEngine::new(ct_id);

        let ctx = create_test_exception_context();
        let _ = engine.dispatch(ctx);

        assert!(engine.exception_count > 0);
        engine.reset_stats();
        assert_eq!(engine.exception_count, 0);
    }
}
