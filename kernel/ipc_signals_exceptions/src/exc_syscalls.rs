// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//\!
//\! # Exception Syscalls
//\!
//\! This module defines the syscall interface for exception handling operations.
//\! These syscalls allow user-space code to register exception handlers, query
//\! exception history, and manage exception engine state.
//\!
//\! ## Syscall Interface
//\!
//\! - `exc_register` - Register an exception handler for the current CT
//\! - `exc_unregister` - Unregister the current exception handler
//\! - `exc_get_history` - Query exception history (last N exceptions)
//\! - `exc_get_stats` - Get exception engine statistics
//\! - `exc_set_mode` - Set the exception handling mode
//\!
//\! ## Handler Validation
//\!
//\! Handlers are validated to ensure:
//\! - Handler pointer is valid and readable
//\! - Handler is not already registered
//\! - Handler respects the exception handling contract (no panics)
//\!
//\! ## References
//\!
//\! - Engineering Plan § 6.8 (Exception Syscalls)
//\! - Engineering Plan § 6.2 (Exception System)

use crate::exception_engine::{ExceptionEngine, ExceptionHandlingMode, ExceptionEngineStats};
use crate::exception_context::ExceptionContext;
use crate::error::{CsError, Result};
use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

/// Exception syscall error types.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExceptionSyscallError {
    /// Invalid handler pointer (not readable or null)
    InvalidHandlerPointer,

    /// Handler already registered for this CT
    HandlerAlreadyRegistered,

    /// No handler registered for this CT
    NoHandlerRegistered,

    /// Exception engine not found for CT
    EngineNotFound,

    /// Invalid exception history query
    InvalidHistoryQuery,

    /// Handler validation failed
    HandlerValidationFailed(String),
}

/// Result of registering an exception handler.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExceptionRegisterResult {
    /// Whether registration succeeded
    pub success: bool,

    /// Error message if failed
    pub error_message: Option<String>,

    /// Handler ID for later reference
    pub handler_id: Option<String>,
}

impl ExceptionRegisterResult {
    /// Create a successful registration result.
    pub fn success(handler_id: String) -> Self {
        Self {
            success: true,
            error_message: None,
            handler_id: Some(handler_id),
        }
    }

    /// Create a failed registration result.
    pub fn failure(error: String) -> Self {
        Self {
            success: false,
            error_message: Some(error),
            handler_id: None,
        }
    }
}

/// Result of unregistering an exception handler.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExceptionUnregisterResult {
    /// Whether unregistration succeeded
    pub success: bool,

    /// Error message if failed
    pub error_message: Option<String>,

    /// Number of pending exceptions discarded
    pub pending_exceptions_discarded: u32,
}

impl ExceptionUnregisterResult {
    /// Create a successful unregistration result.
    pub fn success(discarded: u32) -> Self {
        Self {
            success: true,
            error_message: None,
            pending_exceptions_discarded: discarded,
        }
    }

    /// Create a failed unregistration result.
    pub fn failure(error: String) -> Self {
        Self {
            success: false,
            error_message: Some(error),
            pending_exceptions_discarded: 0,
        }
    }
}

/// Exception history query parameters.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HistoryQuery {
    /// Maximum number of exceptions to return
    pub limit: u32,

    /// Only include exceptions after this timestamp (milliseconds since epoch)
    pub since_timestamp_ms: Option<u64>,

    /// Only include exceptions of this severity (optional filter)
    pub severity_filter: Option<String>,
}

impl HistoryQuery {
    /// Create a new history query with default parameters.
    pub fn new(limit: u32) -> Self {
        Self {
            limit: limit.min(100), // Cap at 100 to prevent memory exhaustion
            since_timestamp_ms: None,
            severity_filter: None,
        }
    }

    /// Add a timestamp filter.
    pub fn with_since(mut self, timestamp_ms: u64) -> Self {
        self.since_timestamp_ms = Some(timestamp_ms);
        self
    }

    /// Add a severity filter.
    pub fn with_severity(mut self, severity: String) -> Self {
        self.severity_filter = Some(severity);
        self
    }

    /// Validate the query.
    pub fn validate(&self) -> Result<()> {
        if self.limit == 0 {
            return Err(CsError::Other(
                "History query limit must be greater than 0".to_string(),
            ));
        }
        if self.limit > 100 {
            return Err(CsError::Other(
                "History query limit must not exceed 100".to_string(),
            ));
        }
        Ok(())
    }
}

/// Result of querying exception history.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HistoryQueryResult {
    /// Matched exceptions
    pub exceptions: Vec<ExceptionContextSummary>,

    /// Total exceptions in history
    pub total_in_history: u32,

    /// Matching exceptions count
    pub matched_count: u32,
}

/// Summary of an exception context (for history queries).
///
/// A lighter version of ExceptionContext that omits large fields
/// like full register dumps to reduce memory usage in queries.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExceptionContextSummary {
    /// Exception ID
    pub exception_id: String,

    /// CT ID
    pub ct_id: String,

    /// Exception type
    pub exception_type: String,

    /// Exception severity
    pub severity: String,

    /// Timestamp
    pub timestamp_ms: u64,

    /// Whether exception was recoverable
    pub is_recoverable: bool,

    /// Retry count at time of capture
    pub retry_count: u32,
}

impl ExceptionContextSummary {
    /// Create a summary from an exception context.
    pub fn from_context(ctx: &ExceptionContext) -> Self {
        Self {
            exception_id: ctx.exception_id.to_string(),
            ct_id: ctx.ct_id.to_string(),
            exception_type: ctx.exception.exception_type().to_string(),
            severity: format\!("{:?}", ctx.severity()),
            timestamp_ms: ctx.timestamp_ms,
            is_recoverable: ctx.is_recoverable(),
            retry_count: ctx.retry_count,
        }
    }
}

/// Exception syscall handler.
///
/// This trait encapsulates syscall implementations for exception handling.
/// Implementations should validate all inputs and handle errors gracefully.
///
/// See Engineering Plan § 6.8 (Exception Syscalls)
pub trait ExceptionSyscallHandler {
    /// Register an exception handler for a CT.
    ///
    /// Validates the handler and rejects if one is already registered.
    ///
    /// # Arguments
    /// * `ct_id` - ID of the CT registering the handler
    /// * `handler_name` - Human-readable handler name
    ///
    /// # Returns
    /// Result with handler ID on success
    fn exc_register(
        &mut self,
        ct_id: &str,
        handler_name: &str,
    ) -> Result<ExceptionRegisterResult>;

    /// Unregister the exception handler for a CT.
    ///
    /// Removes the handler and discards any pending exceptions.
    ///
    /// # Arguments
    /// * `ct_id` - ID of the CT unregistering the handler
    ///
    /// # Returns
    /// Result with count of discarded pending exceptions
    fn exc_unregister(&mut self, ct_id: &str) -> Result<ExceptionUnregisterResult>;

    /// Query exception history for a CT.
    ///
    /// Returns recent exceptions matching the query criteria.
    /// Memory-efficient: returns summaries instead of full contexts.
    ///
    /// # Arguments
    /// * `ct_id` - ID of the CT
    /// * `query` - History query parameters
    ///
    /// # Returns
    /// Result with matching exception summaries
    fn exc_get_history(
        &self,
        ct_id: &str,
        query: &HistoryQuery,
    ) -> Result<HistoryQueryResult>;

    /// Get exception engine statistics for a CT.
    ///
    /// Returns aggregate statistics about exception handling.
    ///
    /// # Arguments
    /// * `ct_id` - ID of the CT
    ///
    /// # Returns
    /// Result with statistics
    fn exc_get_stats(&self, ct_id: &str) -> Result<ExceptionEngineStats>;

    /// Set the exception handling mode for a CT.
    ///
    /// Changes how the exception engine handles exceptions.
    ///
    /// # Arguments
    /// * `ct_id` - ID of the CT
    /// * `mode` - New handling mode
    ///
    /// # Returns
    /// Result with success status
    fn exc_set_mode(&mut self, ct_id: &str, mode: ExceptionHandlingMode) -> Result<()>;
}

/// Syscall flags for exception operations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExceptionSyscallFlags(u32);

impl ExceptionSyscallFlags {
    /// Flag to preserve pending exceptions when unregistering
    pub const PRESERVE_PENDING: u32 = 1 << 0;

    /// Flag to clear history after unregistering
    pub const CLEAR_HISTORY: u32 = 1 << 1;

    /// Flag to async handler invocation (non-blocking)
    pub const ASYNC: u32 = 1 << 2;

    /// Create flags from raw bits.
    pub fn new(bits: u32) -> Self {
        Self(bits)
    }

    /// Check if a flag is set.
    pub fn contains(&self, flag: u32) -> bool {
        (self.0 & flag) \!= 0
    }

    /// Set a flag.
    pub fn insert(&mut self, flag: u32) {
        self.0 |= flag;
    }

    /// Clear a flag.
    pub fn remove(&mut self, flag: u32) {
        self.0 &= \!flag;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::string::ToString;

    #[test]
    fn test_exception_register_result_success() {
        let result = ExceptionRegisterResult::success("handler_1".to_string());
        assert\!(result.success);
        assert\!(result.error_message.is_none());
        assert_eq\!(result.handler_id, Some("handler_1".to_string()));
    }

    #[test]
    fn test_exception_register_result_failure() {
        let result = ExceptionRegisterResult::failure("Already registered".to_string());
        assert\!(\!result.success);
        assert\!(result.error_message.is_some());
        assert\!(result.handler_id.is_none());
    }

    #[test]
    fn test_exception_unregister_result_success() {
        let result = ExceptionUnregisterResult::success(5);
        assert\!(result.success);
        assert_eq\!(result.pending_exceptions_discarded, 5);
    }

    #[test]
    fn test_exception_unregister_result_failure() {
        let result = ExceptionUnregisterResult::failure("Not found".to_string());
        assert\!(\!result.success);
        assert_eq\!(result.pending_exceptions_discarded, 0);
    }

    #[test]
    fn test_history_query_new() {
        let query = HistoryQuery::new(10);
        assert_eq\!(query.limit, 10);
        assert\!(query.since_timestamp_ms.is_none());
        assert\!(query.severity_filter.is_none());
    }

    #[test]
    fn test_history_query_limit_capped() {
        let query = HistoryQuery::new(200);
        assert_eq\!(query.limit, 100); // Should be capped at 100
    }

    #[test]
    fn test_history_query_builder() {
        let query = HistoryQuery::new(10)
            .with_since(5000)
            .with_severity("High".to_string());
        assert_eq\!(query.limit, 10);
        assert_eq\!(query.since_timestamp_ms, Some(5000));
        assert_eq\!(query.severity_filter, Some("High".to_string()));
    }

    #[test]
    fn test_history_query_validate() {
        let query = HistoryQuery::new(10);
        assert\!(query.validate().is_ok());

        let invalid_query = HistoryQuery::new(0);
        assert\!(invalid_query.validate().is_err());

        let capped_query = HistoryQuery::new(200);
        assert\!(capped_query.validate().is_ok()); // After capping to 100
    }

    #[test]
    fn test_history_query_result() {
        let result = HistoryQueryResult {
            exceptions: Vec::new(),
            total_in_history: 10,
            matched_count: 0,
        };
        assert_eq\!(result.total_in_history, 10);
        assert_eq\!(result.matched_count, 0);
    }

    #[test]
    fn test_exception_syscall_flags() {
        let mut flags = ExceptionSyscallFlags::new(0);
        assert\!(\!flags.contains(ExceptionSyscallFlags::PRESERVE_PENDING));

        flags.insert(ExceptionSyscallFlags::PRESERVE_PENDING);
        assert\!(flags.contains(ExceptionSyscallFlags::PRESERVE_PENDING));

        flags.remove(ExceptionSyscallFlags::PRESERVE_PENDING);
        assert\!(\!flags.contains(ExceptionSyscallFlags::PRESERVE_PENDING));
    }

    #[test]
    fn test_exception_syscall_flags_multiple() {
        let mut flags = ExceptionSyscallFlags::new(0);
        flags.insert(ExceptionSyscallFlags::PRESERVE_PENDING);
        flags.insert(ExceptionSyscallFlags::CLEAR_HISTORY);

        assert\!(flags.contains(ExceptionSyscallFlags::PRESERVE_PENDING));
        assert\!(flags.contains(ExceptionSyscallFlags::CLEAR_HISTORY));
        assert\!(\!flags.contains(ExceptionSyscallFlags::ASYNC));
    }
}
