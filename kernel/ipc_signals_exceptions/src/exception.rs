// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! # Exception Types and Exception Handling
//!
//! This module defines the exception hierarchy for the cognitive substrate system.
//! Exceptions represent error conditions that may require recovery, rollback, escalation,
//! or termination.
//!
//! ## Exception Classification
//!
//! Exceptions are classified by severity with dedicated context structs:
//! - **Critical** (2 types): System integrity threatened
//!   - DeadlineExceeded(DeadlineContext)
//!   - InconsistentState(StateContext)
//! - **High** (2 types): Significant functionality impaired
//!   - IpcFailure(IpcErrorContext)
//!   - CapabilityViolation(CapabilityContext)
//! - **Medium** (2 types): Operation degraded but recoverable
//!   - ToolCallFailed(ToolFailureContext)
//!   - ContextOverflow(MemoryContext)
//! - **Low** (1 type): Minor issue, typically recoverable
//!   - ReasoningDiverged(DivergenceContext)
//! - **Unknown**: Generic exception with fallback handling
//!
//! ## References
//!
//! - Engineering Plan § 6.2 (Exception System)

use crate::ids::{CheckpointID, ExceptionID};
use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

/// Exception severity level.
///
/// Indicates how critical an exception is and constrains the available recovery strategies.
///
/// See Engineering Plan § 6.2 (Exception System)
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExceptionSeverity {
    /// Critical severity - system integrity threatened.
    ///
    /// Recovery options severely limited; immediate escalation or termination typically required.
    Critical,

    /// High severity - significant functionality impaired.
    ///
    /// Recovery requires careful intervention (escalation, rollback, or termination).
    High,

    /// Medium severity - operation degraded but still potentially recoverable.
    ///
    /// Retry, rollback, or escalation may resolve the issue.
    Medium,

    /// Low severity - minor issue, typically recoverable.
    ///
    /// Retry or context adjustment likely sufficient.
    Low,
}

impl ExceptionSeverity {
    /// Check if this severity allows retry as a recovery strategy.
    pub fn allows_retry(&self) -> bool {
        matches!(self, ExceptionSeverity::Medium | ExceptionSeverity::Low)
    }

    /// Check if this severity allows rollback as a recovery strategy.
    pub fn allows_rollback(&self) -> bool {
        !matches!(self, ExceptionSeverity::Critical)
    }

    /// Check if this severity allows escalation as a recovery strategy.
    pub fn allows_escalation(&self) -> bool {
        true
    }

    /// Check if this severity allows termination as a recovery strategy.
    pub fn allows_termination(&self) -> bool {
        true
    }
}

/// Context for tool call failures.
///
/// Provides detailed information about why a tool invocation failed,
/// including the tool identity, error details, and retry feasibility.
///
/// See Engineering Plan § 6.2 (Exception System)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolFailureContext {
    /// Tool identifier
    pub tool_id: String,
    /// Error message from the tool
    pub error_message: String,
    /// Whether the failure is retryable (transient vs. permanent)
    pub is_retryable: bool,
    /// Optional retry count if this is a retry
    pub retry_attempt: u32,
    /// Timestamp of failure (Unix epoch milliseconds)
    pub timestamp_ms: u64,
}

impl ToolFailureContext {
    /// Create a new tool failure context.
    pub fn new(
        tool_id: String,
        error_message: String,
        is_retryable: bool,
        timestamp_ms: u64,
    ) -> Self {
        Self {
            tool_id,
            error_message,
            is_retryable,
            retry_attempt: 0,
            timestamp_ms,
        }
    }
}

/// Context for reasoning divergence.
///
/// Captures information about reasoning that deviated from expected bounds,
/// such as excessive iteration, token consumption, or policy violations.
///
/// See Engineering Plan § 6.2 (Exception System)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DivergenceContext {
    /// Description of how reasoning diverged
    pub divergence_type: String,
    /// Current value that exceeded bounds
    pub current_value: u64,
    /// Maximum allowed value
    pub threshold: u64,
    /// Optional policy that was violated
    pub policy_violated: Option<String>,
    /// Timestamp of detection (Unix epoch milliseconds)
    pub timestamp_ms: u64,
}

impl DivergenceContext {
    /// Create a new divergence context.
    pub fn new(
        divergence_type: String,
        current_value: u64,
        threshold: u64,
        timestamp_ms: u64,
    ) -> Self {
        Self {
            divergence_type,
            current_value,
            threshold,
            policy_violated: None,
            timestamp_ms,
        }
    }
}

/// Context for deadline exceeded exceptions.
///
/// Captures information about deadline violations including the deadline,
/// elapsed time, and remaining work.
///
/// See Engineering Plan § 6.2 (Exception System)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeadlineContext {
    /// Deadline description/name
    pub deadline_name: String,
    /// Deadline timestamp (Unix epoch milliseconds)
    pub deadline_ms: u64,
    /// Current timestamp (Unix epoch milliseconds)
    pub current_ms: u64,
    /// Estimated remaining work (percentage: 0-100)
    pub remaining_work_percent: u32,
    /// Deadline overrun in milliseconds
    pub overrun_ms: u64,
}

impl DeadlineContext {
    /// Create a new deadline context.
    pub fn new(
        deadline_name: String,
        deadline_ms: u64,
        current_ms: u64,
        remaining_work_percent: u32,
    ) -> Self {
        let overrun_ms = if current_ms > deadline_ms {
            current_ms - deadline_ms
        } else {
            0
        };
        Self {
            deadline_name,
            deadline_ms,
            current_ms,
            remaining_work_percent,
            overrun_ms,
        }
    }
}

/// Context for context/memory overflow.
///
/// Tracks buffer usage, capacity, and memory pressure metrics.
///
/// See Engineering Plan § 6.2 (Exception System)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MemoryContext {
    /// Current memory usage in bytes
    pub current_bytes: u64,
    /// Maximum capacity in bytes
    pub max_bytes: u64,
    /// Percentage of capacity used
    pub usage_percent: u32,
    /// Optional breakdown by memory type
    pub breakdown: Vec<(String, u64)>,
}

impl MemoryContext {
    /// Create a new memory context.
    pub fn new(current_bytes: u64, max_bytes: u64) -> Self {
        let usage_percent = if max_bytes > 0 {
            ((current_bytes as f64 / max_bytes as f64) * 100.0) as u32
        } else {
            100
        };
        Self {
            current_bytes,
            max_bytes,
            usage_percent,
            breakdown: Vec::new(),
        }
    }
}

/// Context for IPC failures.
///
/// Captures information about inter-process communication failures.
///
/// See Engineering Plan § 6.2 (Exception System)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IpcErrorContext {
    /// Channel or endpoint identifier
    pub channel_id: String,
    /// Type of IPC failure
    pub failure_type: String,
    /// Detailed error message
    pub error_message: String,
    /// Whether the failure is transient
    pub is_transient: bool,
    /// Timestamp of failure (Unix epoch milliseconds)
    pub timestamp_ms: u64,
}

impl IpcErrorContext {
    /// Create a new IPC error context.
    pub fn new(
        channel_id: String,
        failure_type: String,
        error_message: String,
        is_transient: bool,
        timestamp_ms: u64,
    ) -> Self {
        Self {
            channel_id,
            failure_type,
            error_message,
            is_transient,
            timestamp_ms,
        }
    }
}

/// Context for capability violations.
///
/// Records capability authorization failures and their details.
///
/// See Engineering Plan § 6.2 (Exception System)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CapabilityContext {
    /// Capability that was violated/missing
    pub capability_name: String,
    /// Required capability level
    pub required_level: u32,
    /// Actual capability level held
    pub actual_level: u32,
    /// Optional required action/resource
    pub required_action: Option<String>,
    /// Timestamp of violation (Unix epoch milliseconds)
    pub timestamp_ms: u64,
}

impl CapabilityContext {
    /// Create a new capability context.
    pub fn new(
        capability_name: String,
        required_level: u32,
        actual_level: u32,
        timestamp_ms: u64,
    ) -> Self {
        Self {
            capability_name,
            required_level,
            actual_level,
            required_action: None,
            timestamp_ms,
        }
    }
}

/// Context for inconsistent state exceptions.
///
/// Records system state inconsistencies and invariant violations.
///
/// See Engineering Plan § 6.2 (Exception System)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StateContext {
    /// Description of the inconsistency
    pub inconsistency_description: String,
    /// Expected state
    pub expected_state: String,
    /// Actual observed state
    pub actual_state: String,
    /// Affected component
    pub component: String,
    /// Timestamp of detection (Unix epoch milliseconds)
    pub timestamp_ms: u64,
}

impl StateContext {
    /// Create a new state context.
    pub fn new(
        inconsistency_description: String,
        expected_state: String,
        actual_state: String,
        component: String,
        timestamp_ms: u64,
    ) -> Self {
        Self {
            inconsistency_description,
            expected_state,
            actual_state,
            component,
            timestamp_ms,
        }
    }
}

/// Cognitive exception type.
///
/// Represents exceptional conditions during cognitive task execution that require
/// error handling. Each exception type carries detailed context for recovery decisions.
///
/// See Engineering Plan § 6.2 (Exception System)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CognitiveException {
    /// Tool call failed (Medium severity).
    ///
    /// An external tool (e.g., API, database, code execution) failed to execute.
    /// Recovery: Retry (if transient), use fallback tool, or escalate.
    ///
    /// See Engineering Plan § 6.2 (Exception System)
    ToolCallFailed(ToolFailureContext),

    /// Reasoning diverged (Low severity).
    ///
    /// The reasoning trajectory diverged from acceptable bounds (e.g., token limit,
    /// iteration count, or policy constraints).
    /// Recovery: Prune reasoning branch, rollback, or escalate.
    ///
    /// See Engineering Plan § 6.2 (Exception System)
    ReasoningDiverged(DivergenceContext),

    /// Deadline exceeded (Critical severity).
    ///
    /// A time deadline has been exceeded (e.g., task timeout, SLA violation).
    /// Recovery: Escalate with partial results, or terminate immediately.
    ///
    /// See Engineering Plan § 6.2 (Exception System)
    DeadlineExceeded(DeadlineContext),

    /// Context/memory overflow (Medium severity).
    ///
    /// The context (variables, reasoning state, etc.) exceeded available buffer capacity.
    /// Recovery: Trim context, switch to ReadOnly mode, or escalate for external cleanup.
    ///
    /// See Engineering Plan § 6.2 (Exception System)
    ContextOverflow(MemoryContext),

    /// IPC communication failure (High severity).
    ///
    /// Inter-process communication failed (e.g., channel error, message delivery failure).
    /// Recovery: Retry if transient, escalate, or terminate.
    ///
    /// See Engineering Plan § 6.2 (Exception System)
    IpcFailure(IpcErrorContext),

    /// Capability violation (High severity).
    ///
    /// A required capability is missing or insufficient (e.g., permission denied).
    /// Recovery: Escalate to acquire capability or terminate.
    ///
    /// See Engineering Plan § 6.2 (Exception System)
    CapabilityViolation(CapabilityContext),

    /// Inconsistent state (Critical severity).
    ///
    /// System invariant violation or internal state inconsistency detected.
    /// Recovery: Escalate with full system state or terminate.
    ///
    /// See Engineering Plan § 6.2 (Exception System)
    InconsistentState(StateContext),

    /// Unknown/generic exception.
    ///
    /// Exception that doesn't fit other categories. Carries a generic message.
    /// Severity must be inferred from context.
    Unknown(String),
}

impl CognitiveException {
    /// Get the exception severity level.
    ///
    /// Determines the severity of this exception based on its type.
    /// This constrains available recovery strategies.
    ///
    /// See Engineering Plan § 6.2 (Exception System)
    pub fn severity(&self) -> ExceptionSeverity {
        match self {
            CognitiveException::ToolCallFailed(_) => ExceptionSeverity::Medium,
            CognitiveException::ReasoningDiverged(_) => ExceptionSeverity::Low,
            CognitiveException::DeadlineExceeded(_) => ExceptionSeverity::Critical,
            CognitiveException::ContextOverflow(_) => ExceptionSeverity::Medium,
            CognitiveException::IpcFailure(_) => ExceptionSeverity::High,
            CognitiveException::CapabilityViolation(_) => ExceptionSeverity::High,
            CognitiveException::InconsistentState(_) => ExceptionSeverity::Critical,
            CognitiveException::Unknown(_) => ExceptionSeverity::Medium,
        }
    }

    /// Get a human-readable exception type name.
    ///
    /// Returns the string name of this exception type.
    pub fn exception_type(&self) -> &'static str {
        match self {
            CognitiveException::ToolCallFailed(_) => "ToolCallFailed",
            CognitiveException::ReasoningDiverged(_) => "ReasoningDiverged",
            CognitiveException::DeadlineExceeded(_) => "DeadlineExceeded",
            CognitiveException::ContextOverflow(_) => "ContextOverflow",
            CognitiveException::IpcFailure(_) => "IpcFailure",
            CognitiveException::CapabilityViolation(_) => "CapabilityViolation",
            CognitiveException::InconsistentState(_) => "InconsistentState",
            CognitiveException::Unknown(_) => "Unknown",
        }
    }

    /// Check if this exception is recoverable (Low or Medium severity).
    ///
    /// Exceptions with Low or Medium severity can potentially be recovered
    /// through retry or rollback strategies.
    pub fn is_recoverable(&self) -> bool {
        matches!(self.severity(), ExceptionSeverity::Low | ExceptionSeverity::Medium)
    }

    /// Check if this exception is critical (Critical severity).
    ///
    /// Critical exceptions indicate system integrity threats and require
    /// immediate escalation or termination.
    pub fn is_critical(&self) -> bool {
        self.severity() == ExceptionSeverity::Critical
    }
}

/// Agent reference for escalation.
///
/// Identifies an agent that can handle escalated exceptions.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentRef {
    /// Agent identifier
    pub agent_id: String,
}

/// Partial results from a failed operation.
///
/// When an exception causes termination, partial results can be preserved.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PartialResults {
    /// Results completed before failure
    pub completed: alloc::string::String,
    /// Failure point
    pub failure_point: alloc::string::String,
}

/// Exception handler return type.
///
/// Specifies the recovery action to take in response to an exception.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ExceptionHandler {
    /// Retry the operation.
    ///
    /// Valid only for Recoverable exceptions.
    /// The operation will be retried, potentially with backoff.
    Retry,

    /// Rollback to a checkpoint.
    ///
    /// Valid for Recoverable and NonRecoverable exceptions.
    /// Restores state to a previous checkpoint and resumes execution from that point.
    Rollback(CheckpointID),

    /// Escalate to a parent/supervisor agent.
    ///
    /// Valid for all exception severities.
    /// Passes the exception to the identified agent for handling.
    Escalate(AgentRef),

    /// Terminate with partial results.
    ///
    /// Valid for all exception severities.
    /// Stops execution and returns partial results completed before failure.
    Terminate(PartialResults),
}

impl ExceptionHandler {
    /// Check if this handler is valid for the given exception severity.
    pub fn is_valid_for(&self, severity: ExceptionSeverity) -> bool {
        match self {
            ExceptionHandler::Retry => severity.allows_retry(),
            ExceptionHandler::Rollback(_) => severity.allows_rollback(),
            ExceptionHandler::Escalate(_) => severity.allows_escalation(),
            ExceptionHandler::Terminate(_) => severity.allows_termination(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================================
    // Exception Severity Tests
    // ============================================================================

    #[test]
    fn test_exception_severity_critical() {
        let sev = ExceptionSeverity::Critical;
        assert!(!sev.allows_retry());
        assert!(!sev.allows_rollback());
        assert!(sev.allows_escalation());
        assert!(sev.allows_termination());
    }

    #[test]
    fn test_exception_severity_high() {
        let sev = ExceptionSeverity::High;
        assert!(!sev.allows_retry());
        assert!(sev.allows_rollback());
        assert!(sev.allows_escalation());
        assert!(sev.allows_termination());
    }

    #[test]
    fn test_exception_severity_medium() {
        let sev = ExceptionSeverity::Medium;
        assert!(sev.allows_retry());
        assert!(sev.allows_rollback());
        assert!(sev.allows_escalation());
        assert!(sev.allows_termination());
    }

    #[test]
    fn test_exception_severity_low() {
        let sev = ExceptionSeverity::Low;
        assert!(sev.allows_retry());
        assert!(sev.allows_rollback());
        assert!(sev.allows_escalation());
        assert!(sev.allows_termination());
    }

    // ============================================================================
    // Context Struct Tests
    // ============================================================================

    #[test]
    fn test_tool_failure_context_new() {
        let ctx = ToolFailureContext::new(
            alloc::string::String::from("api_tool"),
            alloc::string::String::from("timeout"),
            true,
            1000,
        );
        assert_eq!(ctx.tool_id, "api_tool");
        assert_eq!(ctx.error_message, "timeout");
        assert!(ctx.is_retryable);
        assert_eq!(ctx.retry_attempt, 0);
    }

    #[test]
    fn test_divergence_context_new() {
        let ctx = DivergenceContext::new(
            alloc::string::String::from("token_limit"),
            1500,
            1000,
            5000,
        );
        assert_eq!(ctx.divergence_type, "token_limit");
        assert_eq!(ctx.current_value, 1500);
        assert_eq!(ctx.threshold, 1000);
        assert!(ctx.policy_violated.is_none());
    }

    #[test]
    fn test_deadline_context_new() {
        let deadline_ms = 10000;
        let current_ms = 12000;
        let ctx = DeadlineContext::new(
            alloc::string::String::from("task_timeout"),
            deadline_ms,
            current_ms,
            30,
        );
        assert_eq!(ctx.deadline_name, "task_timeout");
        assert_eq!(ctx.overrun_ms, 2000);
        assert_eq!(ctx.remaining_work_percent, 30);
    }

    #[test]
    fn test_memory_context_new() {
        let ctx = MemoryContext::new(950, 1000);
        assert_eq!(ctx.current_bytes, 950);
        assert_eq!(ctx.max_bytes, 1000);
        assert_eq!(ctx.usage_percent, 95);
    }

    #[test]
    fn test_ipc_error_context_new() {
        let ctx = IpcErrorContext::new(
            alloc::string::String::from("channel_123"),
            alloc::string::String::from("delivery"),
            alloc::string::String::from("receiver offline"),
            true,
            5000,
        );
        assert_eq!(ctx.channel_id, "channel_123");
        assert!(ctx.is_transient);
    }

    #[test]
    fn test_capability_context_new() {
        let ctx = CapabilityContext::new(
            alloc::string::String::from("execute_tool"),
            2,
            1,
            5000,
        );
        assert_eq!(ctx.capability_name, "execute_tool");
        assert_eq!(ctx.required_level, 2);
        assert_eq!(ctx.actual_level, 1);
    }

    #[test]
    fn test_state_context_new() {
        let ctx = StateContext::new(
            alloc::string::String::from("inconsistency detected"),
            alloc::string::String::from("executing"),
            alloc::string::String::from("failed"),
            alloc::string::String::from("task_state"),
            5000,
        );
        assert_eq!(ctx.inconsistency_description, "inconsistency detected");
        assert_eq!(ctx.expected_state, "executing");
        assert_eq!(ctx.actual_state, "failed");
    }

    // ============================================================================
    // Exception Type Tests (Critical Severity)
    // ============================================================================

    #[test]
    fn test_deadline_exceeded_critical() {
        let ctx = DeadlineContext::new(
            alloc::string::String::from("deadline"),
            5000,
            7000,
            0,
        );
        let exc = CognitiveException::DeadlineExceeded(ctx);
        assert_eq!(exc.exception_type(), "DeadlineExceeded");
        assert_eq!(exc.severity(), ExceptionSeverity::Critical);
        assert!(!exc.is_recoverable());
        assert!(exc.is_critical());
    }

    #[test]
    fn test_inconsistent_state_critical() {
        let ctx = StateContext::new(
            alloc::string::String::from("state mismatch"),
            alloc::string::String::from("expected"),
            alloc::string::String::from("actual"),
            alloc::string::String::from("component"),
            5000,
        );
        let exc = CognitiveException::InconsistentState(ctx);
        assert_eq!(exc.exception_type(), "InconsistentState");
        assert_eq!(exc.severity(), ExceptionSeverity::Critical);
        assert!(exc.is_critical());
    }

    // ============================================================================
    // Exception Type Tests (High Severity)
    // ============================================================================

    #[test]
    fn test_ipc_failure_high() {
        let ctx = IpcErrorContext::new(
            alloc::string::String::from("ch1"),
            alloc::string::String::from("delivery_failed"),
            alloc::string::String::from("msg"),
            false,
            5000,
        );
        let exc = CognitiveException::IpcFailure(ctx);
        assert_eq!(exc.exception_type(), "IpcFailure");
        assert_eq!(exc.severity(), ExceptionSeverity::High);
        assert!(!exc.is_recoverable());
    }

    #[test]
    fn test_capability_violation_high() {
        let ctx = CapabilityContext::new(
            alloc::string::String::from("cap"),
            2,
            0,
            5000,
        );
        let exc = CognitiveException::CapabilityViolation(ctx);
        assert_eq!(exc.exception_type(), "CapabilityViolation");
        assert_eq!(exc.severity(), ExceptionSeverity::High);
        assert!(!exc.is_recoverable());
    }

    // ============================================================================
    // Exception Type Tests (Medium Severity)
    // ============================================================================

    #[test]
    fn test_tool_call_failed_medium() {
        let ctx = ToolFailureContext::new(
            alloc::string::String::from("tool_1"),
            alloc::string::String::from("timeout"),
            true,
            5000,
        );
        let exc = CognitiveException::ToolCallFailed(ctx);
        assert_eq!(exc.exception_type(), "ToolCallFailed");
        assert_eq!(exc.severity(), ExceptionSeverity::Medium);
        assert!(exc.is_recoverable());
        assert!(!exc.is_critical());
    }

    #[test]
    fn test_context_overflow_medium() {
        let ctx = MemoryContext::new(950, 1000);
        let exc = CognitiveException::ContextOverflow(ctx);
        assert_eq!(exc.exception_type(), "ContextOverflow");
        assert_eq!(exc.severity(), ExceptionSeverity::Medium);
        assert!(exc.is_recoverable());
    }

    // ============================================================================
    // Exception Type Tests (Low Severity)
    // ============================================================================

    #[test]
    fn test_reasoning_diverged_low() {
        let ctx = DivergenceContext::new(
            alloc::string::String::from("token_limit"),
            1200,
            1000,
            5000,
        );
        let exc = CognitiveException::ReasoningDiverged(ctx);
        assert_eq!(exc.exception_type(), "ReasoningDiverged");
        assert_eq!(exc.severity(), ExceptionSeverity::Low);
        assert!(exc.is_recoverable());
        assert!(!exc.is_critical());
    }

    // ============================================================================
    // Unknown Exception Tests
    // ============================================================================

    #[test]
    fn test_unknown_exception() {
        let exc = CognitiveException::Unknown(alloc::string::String::from("generic error"));
        assert_eq!(exc.exception_type(), "Unknown");
        assert_eq!(exc.severity(), ExceptionSeverity::Medium);
        assert!(exc.is_recoverable());
    }

    // ============================================================================
    // Exception Handler Tests
    // ============================================================================

    #[test]
    fn test_exception_handler_retry() {
        let handler = ExceptionHandler::Retry;
        assert!(handler.is_valid_for(ExceptionSeverity::Medium));
        assert!(handler.is_valid_for(ExceptionSeverity::Low));
        assert!(!handler.is_valid_for(ExceptionSeverity::High));
        assert!(!handler.is_valid_for(ExceptionSeverity::Critical));
    }

    #[test]
    fn test_exception_handler_rollback() {
        let ckpt_id = CheckpointID::new();
        let handler = ExceptionHandler::Rollback(ckpt_id);
        assert!(handler.is_valid_for(ExceptionSeverity::Medium));
        assert!(handler.is_valid_for(ExceptionSeverity::Low));
        assert!(handler.is_valid_for(ExceptionSeverity::High));
        assert!(!handler.is_valid_for(ExceptionSeverity::Critical));
    }

    #[test]
    fn test_exception_handler_escalate() {
        let agent = AgentRef {
            agent_id: alloc::string::String::from("supervisor"),
        };
        let handler = ExceptionHandler::Escalate(agent);
        assert!(handler.is_valid_for(ExceptionSeverity::Medium));
        assert!(handler.is_valid_for(ExceptionSeverity::Low));
        assert!(handler.is_valid_for(ExceptionSeverity::High));
        assert!(handler.is_valid_for(ExceptionSeverity::Critical));
    }

    #[test]
    fn test_exception_handler_terminate() {
        let results = PartialResults {
            completed: alloc::string::String::from("steps completed"),
            failure_point: alloc::string::String::from("step N"),
        };
        let handler = ExceptionHandler::Terminate(results);
        assert!(handler.is_valid_for(ExceptionSeverity::Medium));
        assert!(handler.is_valid_for(ExceptionSeverity::Low));
        assert!(handler.is_valid_for(ExceptionSeverity::High));
        assert!(handler.is_valid_for(ExceptionSeverity::Critical));
    }

    #[test]
    fn test_agent_ref() {
        let agent = AgentRef {
            agent_id: alloc::string::String::from("supervisor_agent"),
        };
        assert_eq!(agent.agent_id, "supervisor_agent");
    }
}
