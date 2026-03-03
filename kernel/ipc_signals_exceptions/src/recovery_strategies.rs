// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! # Exception Recovery Strategies
//!
//! This module implements the four primary exception recovery strategies:
//! 1. **Retry**: Exponential backoff with configurable retry policy
//! 2. **Rollback**: Restore CT state from a checkpoint and resume
//! 3. **Escalate**: Bubble exception to supervisor CT via IPC
//! 4. **Terminate**: Graceful shutdown with partial results
//!
//! ## Recovery Strategy Selection
//!
//! The appropriate strategy depends on:
//! - Exception severity (critical exceptions cannot retry/rollback)
//! - Exception type (tool failures can retry; state corruption cannot)
//! - Availability of checkpoints (rollback requires valid checkpoint)
//! - Supervisor availability (escalation requires parent CT)
//!
//! ## References
//!
//! - Engineering Plan § 6.4 (Exception Handling & Recovery)
//! - Engineering Plan § 6.6 (Recovery Strategies)

use crate::ids::CheckpointID;
use crate::exception::PartialResults;
use alloc::string::String;
use serde::{Deserialize, Serialize};

/// Retry strategy configuration.
///
/// Controls how retries are attempted after a transient failure.
/// Uses exponential backoff to avoid overwhelming the system.
///
/// See Engineering Plan § 6.4 (Exception Handling & Recovery)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RetryStrategy {
    /// Initial backoff delay in milliseconds
    pub backoff_ms: u32,

    /// Maximum number of retry attempts (0 = no retries)
    pub max_retries: u32,

    /// Backoff multiplier for exponential growth (typically 2)
    pub backoff_multiplier: f64,

    /// Current retry attempt (0-indexed)
    pub current_attempt: u32,

    /// Whether to vary parameters on retry (adaptive retry)
    pub adaptive_retry: bool,
}

impl RetryStrategy {
    /// Create a new retry strategy with exponential backoff.
    ///
    /// # Arguments
    /// * `backoff_ms` - Initial delay in milliseconds
    /// * `max_retries` - Maximum number of attempts
    /// * `backoff_multiplier` - Exponential growth factor (typically 2.0)
    /// * `adaptive_retry` - Whether to adapt parameters between attempts
    ///
    /// See Engineering Plan § 6.6 (Recovery Strategies)
    pub fn new(
        backoff_ms: u32,
        max_retries: u32,
        backoff_multiplier: f64,
        adaptive_retry: bool,
    ) -> Self {
        Self {
            backoff_ms,
            max_retries,
            backoff_multiplier,
            current_attempt: 0,
            adaptive_retry,
        }
    }

    /// Default retry strategy: exponential backoff with 3 retries.
    pub fn default_exponential() -> Self {
        Self::new(100, 3, 2.0, false)
    }

    /// Aggressive retry strategy: more retries with faster backoff.
    pub fn aggressive() -> Self {
        Self::new(50, 5, 1.5, true)
    }

    /// Conservative retry strategy: fewer retries with longer backoff.
    pub fn conservative() -> Self {
        Self::new(500, 2, 2.0, false)
    }

    /// Check if retries are exhausted.
    pub fn is_exhausted(&self) -> bool {
        self.current_attempt >= self.max_retries
    }

    /// Get the delay for the next retry in milliseconds.
    pub fn next_delay_ms(&self) -> u32 {
        let delay = (self.backoff_ms as f64
            * self.backoff_multiplier.powi(self.current_attempt as i32))
            as u32;
        // Cap at 60 seconds to prevent overflow
        delay.min(60_000)
    }

    /// Advance to the next retry attempt.
    pub fn advance_attempt(&mut self) {
        self.current_attempt = self.current_attempt.saturating_add(1);
    }

    /// Get retry attempt count (1-indexed for user display).
    pub fn attempt_number(&self) -> u32 {
        self.current_attempt + 1
    }

    /// Check if this is the final retry attempt.
    pub fn is_final_attempt(&self) -> bool {
        self.current_attempt + 1 >= self.max_retries
    }
}

/// Rollback strategy configuration.
///
/// Restores CT state from a saved checkpoint and resumes execution.
/// Useful for recovering from transient failures or state corruption.
///
/// See Engineering Plan § 6.4 (Exception Handling & Recovery)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RollbackStrategy {
    /// Checkpoint to restore from
    pub checkpoint_id: CheckpointID,

    /// Whether to clear tool execution state after rollback
    pub clear_tool_state: bool,

    /// Whether to clear IPC buffers/queues after rollback
    pub clear_ipc_state: bool,

    /// Whether to preserve reasoning trace for analysis
    pub preserve_reasoning_trace: bool,
}

impl RollbackStrategy {
    /// Create a new rollback strategy.
    pub fn new(checkpoint_id: CheckpointID) -> Self {
        Self {
            checkpoint_id,
            clear_tool_state: false,
            clear_ipc_state: false,
            preserve_reasoning_trace: true,
        }
    }

    /// Set whether to clear tool state.
    pub fn with_clear_tool_state(mut self, clear: bool) -> Self {
        self.clear_tool_state = clear;
        self
    }

    /// Set whether to clear IPC state.
    pub fn with_clear_ipc_state(mut self, clear: bool) -> Self {
        self.clear_ipc_state = clear;
        self
    }

    /// Set whether to preserve reasoning trace.
    pub fn with_preserve_trace(mut self, preserve: bool) -> Self {
        self.preserve_reasoning_trace = preserve;
        self
    }
}

/// Escalation strategy configuration.
///
/// Bubbles the exception up to a supervisor CT for handling.
/// The supervisor can make informed decisions about recovery or escalation.
///
/// See Engineering Plan § 6.4 (Exception Handling & Recovery)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EscalationStrategy {
    /// Supervisor CT ID to escalate to
    pub supervisor_ct_id: String,

    /// Whether to suspend the current CT while waiting for response
    pub suspend_current_ct: bool,

    /// Timeout for supervisor response in milliseconds
    pub response_timeout_ms: u32,

    /// Optional escalation context/reason
    pub escalation_reason: String,
}

impl EscalationStrategy {
    /// Create a new escalation strategy.
    pub fn new(supervisor_ct_id: String, escalation_reason: String) -> Self {
        Self {
            supervisor_ct_id,
            suspend_current_ct: true,
            response_timeout_ms: 30_000, // 30 second default timeout
            escalation_reason,
        }
    }

    /// Set whether to suspend the current CT.
    pub fn with_suspend(mut self, suspend: bool) -> Self {
        self.suspend_current_ct = suspend;
        self
    }

    /// Set the response timeout.
    pub fn with_timeout_ms(mut self, timeout_ms: u32) -> Self {
        self.response_timeout_ms = timeout_ms;
        self
    }
}

/// Termination strategy configuration.
///
/// Gracefully shuts down the CT with optional partial results.
/// Used when recovery is impossible or not warranted.
///
/// See Engineering Plan § 6.4 (Exception Handling & Recovery)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TerminationStrategy {
    /// Partial results to return
    pub partial_results: PartialResults,

    /// Exit code/status
    pub exit_status: u32,

    /// Whether to preserve execution state for post-mortem analysis
    pub preserve_state: bool,

    /// Optional cleanup actions to perform
    pub cleanup_actions: Vec<String>,
}

impl TerminationStrategy {
    /// Create a new termination strategy.
    pub fn new(partial_results: PartialResults) -> Self {
        Self {
            partial_results,
            exit_status: 1, // Non-zero indicates error
            preserve_state: true,
            cleanup_actions: alloc::vec![],
        }
    }

    /// Set the exit status.
    pub fn with_exit_status(mut self, status: u32) -> Self {
        self.exit_status = status;
        self
    }

    /// Set whether to preserve state.
    pub fn with_preserve_state(mut self, preserve: bool) -> Self {
        self.preserve_state = preserve;
        self
    }

    /// Add a cleanup action.
    pub fn add_cleanup_action(&mut self, action: String) {
        self.cleanup_actions.push(action);
    }
}

/// Recovery action result.
///
/// Describes the result of executing a recovery strategy.
/// Used by the exception handler to determine next steps.
///
/// See Engineering Plan § 6.6 (Recovery Strategies)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RecoveryActionResult {
    /// Retry scheduled - CT will be resumed after delay
    RetryScheduled {
        /// Delay before retry in milliseconds
        delay_ms: u32,
        /// Retry attempt number
        attempt: u32,
    },

    /// Rollback initiated - CT state restored from checkpoint
    RollbackInitiated {
        /// Checkpoint ID that was restored
        checkpoint_id: CheckpointID,
        /// Estimated recovery time in milliseconds
        recovery_time_ms: u32,
    },

    /// Exception escalated to supervisor
    EscalationInitiated {
        /// Supervisor CT ID
        supervisor_id: String,
        /// Escalation request ID for correlation
        request_id: String,
    },

    /// CT terminated gracefully
    TerminationInitiated {
        /// Exit status
        exit_status: u32,
        /// Cleanup actions scheduled
        cleanup_count: u32,
    },

    /// Recovery action failed
    Failed {
        /// Error reason
        reason: String,
    },
}

impl RecoveryActionResult {
    /// Check if recovery was successful.
    pub fn is_success(&self) -> bool {
        !matches!(self, RecoveryActionResult::Failed { .. })
    }

    /// Check if recovery requires external action (escalation/termination).
    pub fn requires_external_action(&self) -> bool {
        matches!(
            self,
            RecoveryActionResult::EscalationInitiated { .. }
                | RecoveryActionResult::TerminationInitiated { .. }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::string::ToString;
use alloc::vec::Vec;
use alloc::vec;

    #[test]
    fn test_retry_strategy_new() {
        let strategy = RetryStrategy::new(100, 3, 2.0, false);
        assert_eq!(strategy.backoff_ms, 100);
        assert_eq!(strategy.max_retries, 3);
        assert_eq!(strategy.current_attempt, 0);
    }

    #[test]
    fn test_retry_strategy_default_exponential() {
        let strategy = RetryStrategy::default_exponential();
        assert_eq!(strategy.backoff_ms, 100);
        assert_eq!(strategy.max_retries, 3);
    }

    #[test]
    fn test_retry_strategy_aggressive() {
        let strategy = RetryStrategy::aggressive();
        assert_eq!(strategy.backoff_ms, 50);
        assert_eq!(strategy.max_retries, 5);
    }

    #[test]
    fn test_retry_strategy_conservative() {
        let strategy = RetryStrategy::conservative();
        assert_eq!(strategy.backoff_ms, 500);
        assert_eq!(strategy.max_retries, 2);
    }

    #[test]
    fn test_retry_strategy_next_delay_exponential() {
        let strategy = RetryStrategy::new(100, 3, 2.0, false);
        assert_eq!(strategy.next_delay_ms(), 100); // attempt 0: 100 * 2^0 = 100
    }

    #[test]
    fn test_retry_strategy_next_delay_growth() {
        let mut strategy = RetryStrategy::new(100, 5, 2.0, false);
        assert_eq!(strategy.next_delay_ms(), 100); // 100 * 2^0 = 100
        strategy.advance_attempt();
        assert_eq!(strategy.next_delay_ms(), 200); // 100 * 2^1 = 200
        strategy.advance_attempt();
        assert_eq!(strategy.next_delay_ms(), 400); // 100 * 2^2 = 400
    }

    #[test]
    fn test_retry_strategy_exhausted() {
        let mut strategy = RetryStrategy::new(100, 3, 2.0, false);
        assert!(!strategy.is_exhausted());
        strategy.current_attempt = 3;
        assert!(strategy.is_exhausted());
    }

    #[test]
    fn test_retry_strategy_is_final_attempt() {
        let mut strategy = RetryStrategy::new(100, 3, 2.0, false);
        strategy.current_attempt = 1;
        assert!(!strategy.is_final_attempt());
        strategy.current_attempt = 2;
        assert!(strategy.is_final_attempt());
    }

    #[test]
    fn test_rollback_strategy_new() {
        let ckpt = CheckpointID::new();
        let strategy = RollbackStrategy::new(ckpt);
        assert_eq!(strategy.checkpoint_id, ckpt);
        assert!(!strategy.clear_tool_state);
        assert!(!strategy.clear_ipc_state);
        assert!(strategy.preserve_reasoning_trace);
    }

    #[test]
    fn test_rollback_strategy_builder() {
        let ckpt = CheckpointID::new();
        let strategy = RollbackStrategy::new(ckpt)
            .with_clear_tool_state(true)
            .with_clear_ipc_state(true);
        assert!(strategy.clear_tool_state);
        assert!(strategy.clear_ipc_state);
    }

    #[test]
    fn test_escalation_strategy_new() {
        let strategy = EscalationStrategy::new(
            "supervisor".to_string(),
            "Recovery failed".to_string(),
        );
        assert_eq!(strategy.supervisor_ct_id, "supervisor");
        assert!(strategy.suspend_current_ct);
        assert_eq!(strategy.response_timeout_ms, 30_000);
    }

    #[test]
    fn test_escalation_strategy_builder() {
        let strategy = EscalationStrategy::new(
            "supervisor".to_string(),
            "Recovery failed".to_string(),
        )
        .with_suspend(false)
        .with_timeout_ms(5000);
        assert!(!strategy.suspend_current_ct);
        assert_eq!(strategy.response_timeout_ms, 5000);
    }

    #[test]
    fn test_termination_strategy_new() {
        let results = PartialResults {
            completed: "done".to_string(),
            failure_point: "tool_call".to_string(),
        };
        let strategy = TerminationStrategy::new(results.clone());
        assert_eq!(strategy.partial_results.completed, "done");
        assert_eq!(strategy.exit_status, 1);
        assert!(strategy.preserve_state);
    }

    #[test]
    fn test_termination_strategy_builder() {
        let results = PartialResults {
            completed: "done".to_string(),
            failure_point: "tool_call".to_string(),
        };
        let mut strategy = TerminationStrategy::new(results)
            .with_exit_status(42)
            .with_preserve_state(false);
        strategy.add_cleanup_action("cleanup1".to_string());
        assert_eq!(strategy.exit_status, 42);
        assert!(!strategy.preserve_state);
        assert_eq!(strategy.cleanup_actions.len(), 1);
    }

    #[test]
    fn test_recovery_action_result_success() {
        let result = RecoveryActionResult::RetryScheduled {
            delay_ms: 100,
            attempt: 1,
        };
        assert!(result.is_success());
    }

    #[test]
    fn test_recovery_action_result_failed() {
        let result = RecoveryActionResult::Failed {
            reason: "handler error".to_string(),
        };
        assert!(!result.is_success());
    }

    #[test]
    fn test_recovery_action_result_requires_external() {
        let result = RecoveryActionResult::EscalationInitiated {
            supervisor_id: "sup".to_string(),
            request_id: "req123".to_string(),
        };
        assert!(result.requires_external_action());
    }
}
