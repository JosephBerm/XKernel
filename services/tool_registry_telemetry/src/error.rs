// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Error types for tool registry and telemetry operations.
//!
//! This module defines all error conditions that can occur across tool binding,
//! effect class validation, and telemetry operations.
//!
//! See Engineering Plan § 2.11 (ToolBinding Entity) and § 2.12 (CEF Telemetry).

use alloc::string::String;
use core::fmt;

/// Result type alias for tool registry operations.
///
/// All public tool registry operations return `Result<T>` using this type.
/// See Engineering Plan § 2.11: Tool Registry Error Handling.
pub type Result<T> = core::result::Result<T, ToolError>;

/// Errors that may occur during tool registry and telemetry operations.
///
/// Per Engineering Plan § 2.11, all errors must be recoverable without panics.
///
/// # Variants
///
/// The error variants are organized by operation category:
/// - Binding resolution and validation
/// - Effect class violations
/// - Sandbox and security violations
/// - Schema and type validation
/// - Caching and transaction errors
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ToolError {
    /// Tool binding not found in registry.
    ///
    /// Occurs when attempting to resolve a tool binding that does not exist.
    /// See Engineering Plan § 2.11: Tool Registry.
    #[error("binding not found: {binding_id}")]
    BindingNotFound {
        /// ID of the binding that was not found
        binding_id: String,
    },

    /// Effect class violation detected.
    ///
    /// Occurs when an operation violates the declared effect class constraints.
    /// For example, attempting to write with a ReadOnly binding.
    /// See Engineering Plan § 2.11.2: Effect Classes.
    #[error("effect class violation: {reason}")]
    EffectClassViolation {
        /// Description of which effect class constraint was violated
        reason: String,
    },

    /// Sandbox constraint violation.
    ///
    /// Occurs when an operation violates sandbox security constraints,
    /// such as network access denial or filesystem restriction.
    /// See Engineering Plan § 2.11.4: Sandbox Configuration.
    #[error("sandbox violation: {reason}")]
    SandboxViolation {
        /// Description of which sandbox constraint was violated
        reason: String,
    },

    /// Cache operation failed or returned stale data.
    ///
    /// Occurs when cache lookup fails or freshness policy is violated.
    /// See Engineering Plan § 2.11.5: Response Caching.
    #[error("cache miss: {reason}")]
    CacheMiss {
        /// Description of the cache failure
        reason: String,
    },

    /// Schema validation failed for tool input or output.
    ///
    /// Occurs when input data does not conform to the tool's input schema
    /// or output data does not conform to the output schema.
    /// See Engineering Plan § 2.11.3: Type Schema.
    #[error("schema validation failed: {reason}")]
    SchemaValidationFailed {
        /// Description of the validation failure
        reason: String,
    },

    /// Commit protocol operation failed.
    ///
    /// Occurs when a commit, prepare, or rollback operation fails.
    /// See Engineering Plan § 2.11.6: Commit Protocol.
    #[error("commit failed: {reason}")]
    CommitFailed {
        /// Description of the commit failure
        reason: String,
    },

    /// Operation timed out.
    ///
    /// Occurs when a tool invocation or sandbox operation exceeds timeout limits.
    /// See Engineering Plan § 2.11.4: Sandbox Configuration (max_execution_time_ms).
    #[error("timeout exceeded: {reason}")]
    TimeoutExceeded {
        /// Description of the timeout
        reason: String,
    },

    /// Generic tool registry operation error.
    #[error("tool registry operation failed: {0}")]
    Other(String),
}

impl ToolError {
    /// Returns true if this error is due to a missing binding.
    pub fn is_binding_not_found(&self) -> bool {
        matches!(self, ToolError::BindingNotFound { .. })
    }

    /// Returns true if this error is due to effect class violation.
    pub fn is_effect_class_violation(&self) -> bool {
        matches!(self, ToolError::EffectClassViolation { .. })
    }

    /// Returns true if this error is due to sandbox violation.
    pub fn is_sandbox_violation(&self) -> bool {
        matches!(self, ToolError::SandboxViolation { .. })
    }

    /// Returns true if this error is due to schema validation.
    pub fn is_schema_error(&self) -> bool {
        matches!(self, ToolError::SchemaValidationFailed { .. })
    }

    /// Returns true if this error is due to commit failure.
    pub fn is_commit_error(&self) -> bool {
        matches!(self, ToolError::CommitFailed { .. })
    }

    /// Returns true if this error is due to timeout.
    pub fn is_timeout(&self) -> bool {
        matches!(self, ToolError::TimeoutExceeded { .. })
    }

    /// Returns true if this is a security-related error (sandbox or effect class).
    pub fn is_security_error(&self) -> bool {
        matches!(
            self,
            ToolError::SandboxViolation { .. } | ToolError::EffectClassViolation { .. }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::string::ToString;

    #[test]
    fn test_binding_not_found_display() {
        let err = ToolError::BindingNotFound {
            binding_id: "binding-001".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("binding not found"));
        assert!(msg.contains("binding-001"));
    }

    #[test]
    fn test_effect_class_violation_display() {
        let err = ToolError::EffectClassViolation {
            reason: "write on read-only binding".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("effect class violation"));
        assert!(msg.contains("write on read-only"));
    }

    #[test]
    fn test_sandbox_violation_display() {
        let err = ToolError::SandboxViolation {
            reason: "network access denied".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("sandbox violation"));
        assert!(msg.contains("network access"));
    }

    #[test]
    fn test_cache_miss_display() {
        let err = ToolError::CacheMiss {
            reason: "stale entry".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("cache miss"));
        assert!(msg.contains("stale"));
    }

    #[test]
    fn test_schema_validation_failed_display() {
        let err = ToolError::SchemaValidationFailed {
            reason: "missing required field 'url'".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("schema validation failed"));
        assert!(msg.contains("url"));
    }

    #[test]
    fn test_commit_failed_display() {
        let err = ToolError::CommitFailed {
            reason: "rollback required".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("commit failed"));
        assert!(msg.contains("rollback"));
    }

    #[test]
    fn test_timeout_exceeded_display() {
        let err = ToolError::TimeoutExceeded {
            reason: "exceeded 5000ms limit".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("timeout exceeded"));
        assert!(msg.contains("5000ms"));
    }

    #[test]
    fn test_is_binding_not_found() {
        let err = ToolError::BindingNotFound {
            binding_id: "test".to_string(),
        };
        assert!(err.is_binding_not_found());

        let err = ToolError::SandboxViolation {
            reason: "test".to_string(),
        };
        assert!(!err.is_binding_not_found());
    }

    #[test]
    fn test_is_effect_class_violation() {
        let err = ToolError::EffectClassViolation {
            reason: "test".to_string(),
        };
        assert!(err.is_effect_class_violation());

        let err = ToolError::CacheMiss {
            reason: "test".to_string(),
        };
        assert!(!err.is_effect_class_violation());
    }

    #[test]
    fn test_is_sandbox_violation() {
        let err = ToolError::SandboxViolation {
            reason: "test".to_string(),
        };
        assert!(err.is_sandbox_violation());

        let err = ToolError::SchemaValidationFailed {
            reason: "test".to_string(),
        };
        assert!(!err.is_sandbox_violation());
    }

    #[test]
    fn test_is_schema_error() {
        let err = ToolError::SchemaValidationFailed {
            reason: "test".to_string(),
        };
        assert!(err.is_schema_error());

        let err = ToolError::CommitFailed {
            reason: "test".to_string(),
        };
        assert!(!err.is_schema_error());
    }

    #[test]
    fn test_is_commit_error() {
        let err = ToolError::CommitFailed {
            reason: "test".to_string(),
        };
        assert!(err.is_commit_error());

        let err = ToolError::TimeoutExceeded {
            reason: "test".to_string(),
        };
        assert!(!err.is_commit_error());
    }

    #[test]
    fn test_is_timeout() {
        let err = ToolError::TimeoutExceeded {
            reason: "test".to_string(),
        };
        assert!(err.is_timeout());

        let err = ToolError::BindingNotFound {
            binding_id: "test".to_string(),
        };
        assert!(!err.is_timeout());
    }

    #[test]
    fn test_is_security_error() {
        let sandbox_err = ToolError::SandboxViolation {
            reason: "test".to_string(),
        };
        assert!(sandbox_err.is_security_error());

        let effect_err = ToolError::EffectClassViolation {
            reason: "test".to_string(),
        };
        assert!(effect_err.is_security_error());

        let cache_err = ToolError::CacheMiss {
            reason: "test".to_string(),
        };
        assert!(!cache_err.is_security_error());
    }

    #[test]
    fn test_error_equality() {
        let err1 = ToolError::BindingNotFound {
            binding_id: "test".to_string(),
        };
        let err2 = ToolError::BindingNotFound {
            binding_id: "test".to_string(),
        };
        assert_eq!(err1, err2);
    }
}
