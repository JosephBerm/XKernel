// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! Lifecycle error types.
//!
//! Defines the [`LifecycleError`] enum covering all error conditions that may occur
//! during agent lifecycle management, from invalid state transitions to dependency cycles.
//!
//! Reference: Engineering Plan § Agent Lifecycle Management

use thiserror::Error;

/// Agent lifecycle error types.
///
/// Represents all error conditions that may occur during agent lifecycle management,
/// including invalid state transitions, health check failures, dependency issues,
/// and timeout conditions.
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Error Handling
#[derive(Debug, Clone, Error)]
pub enum LifecycleError {
    /// Invalid state transition attempted.
    ///
    /// Occurs when an agent tries to transition to a state that is not allowed
    /// from the current state. Only specific transitions are valid according to
    /// the lifecycle state machine.
    #[error("Invalid state transition from {current:?} to {target:?}")]
    InvalidTransition {
        /// Current state before attempted transition.
        current: String,
        /// Target state that was not allowed.
        target: String,
    },

    /// Health check failed for agent.
    ///
    /// Indicates that a readiness or liveness probe failed after exceeding
    /// the failure threshold. The agent may be degraded or requires restart.
    #[error("Health check failed: {reason}")]
    HealthCheckFailed {
        /// Detailed reason for health check failure.
        reason: String,
    },

    /// Circular dependency detected in agent dependency graph.
    ///
    /// Occurs when dependency specifications form a cycle, which would prevent
    /// proper startup ordering. The cycle must be resolved before agents can start.
    #[error("Dependency cycle detected: {agents:?}")]
    DependencyCycle {
        /// Agent identifiers that form the cycle.
        agents: Vec<String>,
    },

    /// Agent startup exceeded timeout threshold.
    ///
    /// The agent failed to reach the Running state within the configured
    /// `startup_timeout_ms` duration.
    #[error("Startup timeout after {timeout_ms}ms")]
    StartupTimeout {
        /// Timeout duration in milliseconds.
        timeout_ms: u64,
    },

    /// Agent shutdown exceeded timeout threshold.
    ///
    /// The agent failed to reach the Stopped state within the configured
    /// `shutdown_timeout_ms` duration.
    #[error("Shutdown timeout after {timeout_ms}ms")]
    ShutdownTimeout {
        /// Timeout duration in milliseconds.
        timeout_ms: u64,
    },

    /// Restart limit exceeded for agent.
    ///
    /// The agent has exceeded the maximum number of restart attempts as configured
    /// in the backoff policy. Further restarts are not permitted.
    #[error("Restart limit exceeded: {max_retries} retries")]
    RestartLimitExceeded {
        /// Maximum number of retries that was exceeded.
        max_retries: u32,
    },

    /// Generic lifecycle error with custom message.
    ///
    /// Used for general error conditions not covered by specific error types.
    #[error("{0}")]
    GenericError(String),

    /// Health status tracking error.
    ///
    /// Occurs during health status aggregation or update operations.
    #[error("Health status error: {0}")]
    HealthStatusError(String),

    /// Logging infrastructure error.
    ///
    /// Occurs during log entry creation, writing, or querying.
    #[error("Logging error: {0}")]
    LoggingError(String),

    /// CLI command processing error.
    ///
    /// Occurs during command-line argument parsing or execution.
    #[error("CLI error: {0}")]
    CliError(String),

    /// Phase 1 readiness assessment error.
    ///
    /// Occurs during readiness gap, risk, or dependency analysis.
    #[error("Assessment error: {0}")]
    AssessmentError(String),

    /// I/O error during lifecycle operations.
    #[error("I/O error: {0}")]
    IoError(String),
}

impl From<std::io::Error> for LifecycleError {
    fn from(err: std::io::Error) -> Self {
        LifecycleError::IoError(err.to_string())
    }
}

/// Result type for lifecycle operations.
///
/// Convenience type alias for operations that may return [`LifecycleError`].
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Error Handling
pub type Result<T> = core::result::Result<T, LifecycleError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_transition_error() {
        let err = LifecycleError::InvalidTransition {
            current: "Starting".to_string(),
            target: "Initializing".to_string(),
        };
        let msg = format!("{}", err);
        assert!(msg.contains("Invalid state transition"));
        assert!(msg.contains("Starting"));
        assert!(msg.contains("Initializing"));
    }

    #[test]
    fn test_health_check_failed_error() {
        let err = LifecycleError::HealthCheckFailed {
            reason: "HTTP endpoint returned 500".to_string(),
        };
        let msg = format!("{}", err);
        assert!(msg.contains("Health check failed"));
        assert!(msg.contains("HTTP endpoint returned 500"));
    }

    #[test]
    fn test_dependency_cycle_error() {
        let agents = vec!["agent_a".to_string(), "agent_b".to_string(), "agent_a".to_string()];
        let err = LifecycleError::DependencyCycle {
            agents: agents.clone(),
        };
        let msg = format!("{}", err);
        assert!(msg.contains("Dependency cycle detected"));
    }

    #[test]
    fn test_startup_timeout_error() {
        let err = LifecycleError::StartupTimeout {
            timeout_ms: 5000,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("Startup timeout"));
        assert!(msg.contains("5000ms"));
    }

    #[test]
    fn test_shutdown_timeout_error() {
        let err = LifecycleError::ShutdownTimeout {
            timeout_ms: 10000,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("Shutdown timeout"));
        assert!(msg.contains("10000ms"));
    }

    #[test]
    fn test_restart_limit_exceeded_error() {
        let err = LifecycleError::RestartLimitExceeded {
            max_retries: 5,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("Restart limit exceeded"));
        assert!(msg.contains("5 retries"));
    }

    #[test]
    fn test_generic_error() {
        let err = LifecycleError::GenericError("Something went wrong".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("Something went wrong"));
    }

    #[test]
    fn test_health_status_error() {
        let err = LifecycleError::HealthStatusError("Lock contention".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("Health status error"));
        assert!(msg.contains("Lock contention"));
    }

    #[test]
    fn test_logging_error() {
        let err = LifecycleError::LoggingError("Failed to write log".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("Logging error"));
        assert!(msg.contains("Failed to write log"));
    }

    #[test]
    fn test_cli_error() {
        let err = LifecycleError::CliError("Unknown subcommand".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("CLI error"));
        assert!(msg.contains("Unknown subcommand"));
    }

    #[test]
    fn test_assessment_error() {
        let err = LifecycleError::AssessmentError("Duplicate gap ID".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("Assessment error"));
        assert!(msg.contains("Duplicate gap ID"));
    }
}
