// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! # Exception Handler Framework
//!
//! This module defines the exception handler framework for automated recovery strategies.
//! Handlers map exception types to recovery actions and implement retry policies.
//!
//! ## Exception Recovery Actions
//!
//! Four primary recovery strategies:
//! - **Retry**: Attempt the operation again (for transient failures)
//! - **Rollback**: Restore to a previous checkpoint and resume
//! - **Escalate**: Pass the exception to a supervisor for handling
//! - **Terminate**: Stop execution and return partial results
//!
//! ## Retry Policies
//!
//! Backoff strategies for retry attempts:
//! - **Constant**: Fixed delay between retries
//! - **Linear**: Delay increases linearly with attempt number
//! - **Exponential**: Delay grows exponentially (2^attempt)
//!
//! ## References
//!
//! - Engineering Plan § 6.2 (Exception System)
//! - Engineering Plan § 6.4 (Exception Handling & Recovery)

use crate::exception::{AgentRef, CognitiveException, ExceptionSeverity, PartialResults};
use crate::ids::CheckpointID;
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use serde::{Deserialize, Serialize};

/// Retry backoff strategy.
///
/// Determines how delays grow between retry attempts.
///
/// See Engineering Plan § 6.4 (Exception Handling & Recovery)
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum BackoffStrategy {
    /// Constant delay (specified in milliseconds) between retries.
    Constant(u32),

    /// Linear backoff: delay = base_ms * attempt_number.
    Linear(u32),

    /// Exponential backoff: delay = base_ms * 2^attempt_number.
    Exponential(u32),
}

impl BackoffStrategy {
    /// Calculate the delay for the given attempt number (0-indexed).
    pub fn calculate_delay_ms(&self, attempt: u32) -> u32 {
        match self {
            BackoffStrategy::Constant(ms) => *ms,
            BackoffStrategy::Linear(base_ms) => base_ms.saturating_mul(attempt + 1),
            BackoffStrategy::Exponential(base_ms) => base_ms.saturating_mul(
                2_u32.saturating_pow(attempt.min(30)), // Limit exponent to prevent overflow
            ),
        }
    }
}

/// Retry policy configuration.
///
/// Controls retry behavior for transient failures.
///
/// See Engineering Plan § 6.4 (Exception Handling & Recovery)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RetryPolicy {
    /// Maximum number of retry attempts (0 = no retries)
    pub max_retries: u32,

    /// Backoff strategy between retries
    pub backoff_strategy: BackoffStrategy,

    /// Current retry attempt (0-indexed)
    pub current_attempt: u32,

    /// Whether to retry with modified parameters (e.g., different prompt)
    pub adaptive_retry: bool,
}

impl RetryPolicy {
    /// Create a new retry policy.
    pub fn new(
        max_retries: u32,
        backoff_strategy: BackoffStrategy,
        adaptive_retry: bool,
    ) -> Self {
        Self {
            max_retries,
            backoff_strategy,
            current_attempt: 0,
            adaptive_retry,
        }
    }

    /// Check if retries are exhausted.
    pub fn is_exhausted(&self) -> bool {
        self.current_attempt >= self.max_retries
    }

    /// Get the next retry delay in milliseconds.
    pub fn next_delay_ms(&self) -> u32 {
        self.backoff_strategy.calculate_delay_ms(self.current_attempt)
    }
}

/// Exception recovery action.
///
/// Specifies what action to take in response to an exception.
/// This extends the basic ExceptionHandler enum with retry policies.
///
/// See Engineering Plan § 6.4 (Exception Handling & Recovery)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ExceptionRecoveryAction {
    /// Retry the operation with the given policy.
    Retry(RetryPolicy),

    /// Rollback to a checkpoint.
    Rollback(CheckpointID),

    /// Escalate to a supervisor agent.
    Escalate(AgentRef),

    /// Terminate with partial results.
    Terminate(PartialResults),
}

impl ExceptionRecoveryAction {
    /// Check if this action is valid for the given exception severity.
    pub fn is_valid_for(&self, severity: ExceptionSeverity) -> bool {
        match self {
            ExceptionRecoveryAction::Retry(_) => severity.allows_retry(),
            ExceptionRecoveryAction::Rollback(_) => severity.allows_rollback(),
            ExceptionRecoveryAction::Escalate(_) => severity.allows_escalation(),
            ExceptionRecoveryAction::Terminate(_) => severity.allows_termination(),
        }
    }
}

/// Default exception handler with severity-based recovery strategies.
///
/// Provides sensible defaults for exception recovery based on severity level.
///
/// See Engineering Plan § 6.4 (Exception Handling & Recovery)
pub trait DefaultExceptionHandler {
    /// Get the default recovery action for an exception.
    ///
    /// Based on the exception type and severity, returns a recommended recovery action.
    fn default_action(&self, exception: &CognitiveException) -> ExceptionRecoveryAction;
}

/// Standard exception handler implementation.
///
/// Provides default recovery strategies for all exception types.
pub struct StandardExceptionHandler;

impl DefaultExceptionHandler for StandardExceptionHandler {
    fn default_action(&self, exception: &CognitiveException) -> ExceptionRecoveryAction {
        match exception.severity() {
            ExceptionSeverity::Critical => {
                // Critical exceptions: escalate or terminate immediately
                ExceptionRecoveryAction::Terminate(PartialResults {
                    completed: alloc::string::String::from("critical exception triggered"),
                    failure_point: exception.exception_type().to_string(),
                })
            }

            ExceptionSeverity::High => {
                // High severity: attempt escalation
                ExceptionRecoveryAction::Escalate(AgentRef {
                    agent_id: alloc::string::String::from("supervisor_agent"),
                })
            }

            ExceptionSeverity::Medium => {
                // Medium severity: try retry with exponential backoff
                let policy = RetryPolicy::new(
                    3, // max 3 retries
                    BackoffStrategy::Exponential(100),
                    true, // allow adaptive retry
                );
                ExceptionRecoveryAction::Retry(policy)
            }

            ExceptionSeverity::Low => {
                // Low severity: retry with linear backoff
                let policy = RetryPolicy::new(
                    5, // max 5 retries
                    BackoffStrategy::Linear(50),
                    false,
                );
                ExceptionRecoveryAction::Retry(policy)
            }
        }
    }
}

/// Exception handler table mapping exception types to handlers.
///
/// Allows dispatching of exceptions to appropriate handlers with
/// severity-based recovery strategies.
///
/// See Engineering Plan § 6.4 (Exception Handling & Recovery)
pub struct ExceptionHandlerTable {
    /// Map from exception type to recovery action
    handlers: BTreeMap<String, Box<dyn Fn(&CognitiveException) -> ExceptionRecoveryAction>>,
}

impl ExceptionHandlerTable {
    /// Create a new empty exception handler table.
    pub fn new() -> Self {
        Self {
            handlers: BTreeMap::new(),
        }
    }

    /// Register a handler for a specific exception type.
    pub fn register<F>(&mut self, exception_type: String, handler: F)
    where
        F: Fn(&CognitiveException) -> ExceptionRecoveryAction + 'static,
    {
        self.handlers.insert(exception_type, Box::new(handler));
    }

    /// Get the recovery action for an exception.
    ///
    /// If a specific handler is registered, uses it. Otherwise falls back
    /// to the standard exception handler.
    pub fn get_action(&self, exception: &CognitiveException) -> ExceptionRecoveryAction {
        if let Some(handler) = self.handlers.get(exception.exception_type()) {
            handler(exception)
        } else {
            StandardExceptionHandler.default_action(exception)
        }
    }

    /// Check if a handler is registered for a specific exception type.
    pub fn has_handler(&self, exception_type: &str) -> bool {
        self.handlers.contains_key(exception_type)
    }
}

impl Default for ExceptionHandlerTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::string::ToString;

    // ============================================================================
    // Backoff Strategy Tests
    // ============================================================================

    #[test]
    fn test_backoff_constant() {
        let strategy = BackoffStrategy::Constant(100);
        assert_eq!(strategy.calculate_delay_ms(0), 100);
        assert_eq!(strategy.calculate_delay_ms(1), 100);
        assert_eq!(strategy.calculate_delay_ms(5), 100);
    }

    #[test]
    fn test_backoff_linear() {
        let strategy = BackoffStrategy::Linear(50);
        assert_eq!(strategy.calculate_delay_ms(0), 50);
        assert_eq!(strategy.calculate_delay_ms(1), 100);
        assert_eq!(strategy.calculate_delay_ms(2), 150);
        assert_eq!(strategy.calculate_delay_ms(5), 300);
    }

    #[test]
    fn test_backoff_exponential() {
        let strategy = BackoffStrategy::Exponential(10);
        assert_eq!(strategy.calculate_delay_ms(0), 10); // 10 * 2^0 = 10
        assert_eq!(strategy.calculate_delay_ms(1), 20); // 10 * 2^1 = 20
        assert_eq!(strategy.calculate_delay_ms(2), 40); // 10 * 2^2 = 40
        assert_eq!(strategy.calculate_delay_ms(3), 80); // 10 * 2^3 = 80
    }

    // ============================================================================
    // Retry Policy Tests
    // ============================================================================

    #[test]
    fn test_retry_policy_new() {
        let policy = RetryPolicy::new(3, BackoffStrategy::Constant(100), true);
        assert_eq!(policy.max_retries, 3);
        assert_eq!(policy.current_attempt, 0);
        assert!(!policy.is_exhausted());
    }

    #[test]
    fn test_retry_policy_exhausted() {
        let mut policy = RetryPolicy::new(2, BackoffStrategy::Constant(100), false);
        policy.current_attempt = 2;
        assert!(policy.is_exhausted());
    }

    #[test]
    fn test_retry_policy_next_delay() {
        let policy = RetryPolicy::new(3, BackoffStrategy::Exponential(100), false);
        assert_eq!(policy.next_delay_ms(), 100); // attempt 0
    }

    // ============================================================================
    // Exception Recovery Action Tests
    // ============================================================================

    #[test]
    fn test_recovery_action_retry_validity() {
        let policy = RetryPolicy::new(3, BackoffStrategy::Constant(100), false);
        let action = ExceptionRecoveryAction::Retry(policy);
        assert!(action.is_valid_for(ExceptionSeverity::Medium));
        assert!(action.is_valid_for(ExceptionSeverity::Low));
        assert!(!action.is_valid_for(ExceptionSeverity::High));
        assert!(!action.is_valid_for(ExceptionSeverity::Critical));
    }

    #[test]
    fn test_recovery_action_rollback_validity() {
        let ckpt_id = CheckpointID::new();
        let action = ExceptionRecoveryAction::Rollback(ckpt_id);
        assert!(action.is_valid_for(ExceptionSeverity::Medium));
        assert!(action.is_valid_for(ExceptionSeverity::Low));
        assert!(action.is_valid_for(ExceptionSeverity::High));
        assert!(!action.is_valid_for(ExceptionSeverity::Critical));
    }

    #[test]
    fn test_recovery_action_escalate_validity() {
        let agent = AgentRef {
            agent_id: alloc::string::String::from("supervisor"),
        };
        let action = ExceptionRecoveryAction::Escalate(agent);
        assert!(action.is_valid_for(ExceptionSeverity::Medium));
        assert!(action.is_valid_for(ExceptionSeverity::Low));
        assert!(action.is_valid_for(ExceptionSeverity::High));
        assert!(action.is_valid_for(ExceptionSeverity::Critical));
    }

    #[test]
    fn test_recovery_action_terminate_validity() {
        let results = PartialResults {
            completed: alloc::string::String::from("partial"),
            failure_point: alloc::string::String::from("failure"),
        };
        let action = ExceptionRecoveryAction::Terminate(results);
        assert!(action.is_valid_for(ExceptionSeverity::Medium));
        assert!(action.is_valid_for(ExceptionSeverity::Low));
        assert!(action.is_valid_for(ExceptionSeverity::High));
        assert!(action.is_valid_for(ExceptionSeverity::Critical));
    }

    // ============================================================================
    // Standard Exception Handler Tests
    // ============================================================================

    #[test]
    fn test_standard_handler_critical() {
        let handler = StandardExceptionHandler;
        let exc = CognitiveException::DeadlineExceeded(
            crate::exception::DeadlineContext::new(
                alloc::string::String::from("deadline"),
                1000,
                2000,
                0,
            ),
        );
        let action = handler.default_action(&exc);
        match action {
            ExceptionRecoveryAction::Terminate(_) => (),
            _ => panic!("Expected Terminate for critical exception"),
        }
    }

    #[test]
    fn test_standard_handler_high() {
        let handler = StandardExceptionHandler;
        let exc = CognitiveException::IpcFailure(
            crate::exception::IpcErrorContext::new(
                alloc::string::String::from("ch1"),
                alloc::string::String::from("delivery"),
                alloc::string::String::from("msg"),
                false,
                5000,
            ),
        );
        let action = handler.default_action(&exc);
        match action {
            ExceptionRecoveryAction::Escalate(_) => (),
            _ => panic!("Expected Escalate for high severity exception"),
        }
    }

    #[test]
    fn test_standard_handler_medium() {
        let handler = StandardExceptionHandler;
        let exc = CognitiveException::ToolCallFailed(
            crate::exception::ToolFailureContext::new(
                alloc::string::String::from("tool"),
                alloc::string::String::from("error"),
                true,
                5000,
            ),
        );
        let action = handler.default_action(&exc);
        match action {
            ExceptionRecoveryAction::Retry(policy) => {
                assert_eq!(policy.max_retries, 3);
                assert!(policy.adaptive_retry);
            }
            _ => panic!("Expected Retry for medium severity exception"),
        }
    }

    #[test]
    fn test_standard_handler_low() {
        let handler = StandardExceptionHandler;
        let exc = CognitiveException::ReasoningDiverged(
            crate::exception::DivergenceContext::new(
                alloc::string::String::from("divergence"),
                1200,
                1000,
                5000,
            ),
        );
        let action = handler.default_action(&exc);
        match action {
            ExceptionRecoveryAction::Retry(policy) => {
                assert_eq!(policy.max_retries, 5);
                assert!(!policy.adaptive_retry);
            }
            _ => panic!("Expected Retry for low severity exception"),
        }
    }

    // ============================================================================
    // Exception Handler Table Tests
    // ============================================================================

    #[test]
    fn test_handler_table_new() {
        let table = ExceptionHandlerTable::new();
        assert!(!table.has_handler("ToolCallFailed"));
    }

    #[test]
    fn test_handler_table_register() {
        let mut table = ExceptionHandlerTable::new();
        table.register(
            alloc::string::String::from("ToolCallFailed"),
            |_exc| {
                ExceptionRecoveryAction::Escalate(AgentRef {
                    agent_id: alloc::string::String::from("custom_handler"),
                })
            },
        );
        assert!(table.has_handler("ToolCallFailed"));
    }

    #[test]
    fn test_handler_table_get_action_custom() {
        let mut table = ExceptionHandlerTable::new();
        table.register(
            alloc::string::String::from("ToolCallFailed"),
            |_exc| {
                ExceptionRecoveryAction::Escalate(AgentRef {
                    agent_id: alloc::string::String::from("custom_handler"),
                })
            },
        );

        let exc = CognitiveException::ToolCallFailed(
            crate::exception::ToolFailureContext::new(
                alloc::string::String::from("tool"),
                alloc::string::String::from("error"),
                true,
                5000,
            ),
        );
        let action = table.get_action(&exc);
        match action {
            ExceptionRecoveryAction::Escalate(agent) => {
                assert_eq!(agent.agent_id, "custom_handler");
            }
            _ => panic!("Expected custom Escalate action"),
        }
    }

    #[test]
    fn test_handler_table_default_action() {
        let table = ExceptionHandlerTable::new();
        let exc = CognitiveException::ToolCallFailed(
            crate::exception::ToolFailureContext::new(
                alloc::string::String::from("tool"),
                alloc::string::String::from("error"),
                true,
                5000,
            ),
        );
        let action = table.get_action(&exc);
        match action {
            ExceptionRecoveryAction::Retry(_) => (),
            _ => panic!("Expected default Retry action"),
        }
    }
}
