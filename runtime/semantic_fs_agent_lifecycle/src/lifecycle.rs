// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! Agent lifecycle configuration and state management.
//!
//! Defines the [`LifecycleConfig`] struct and [`LifecycleState`] enum for comprehensive
//! agent lifecycle management, including startup/shutdown timeouts, health probes,
//! and restart policies. Provides valid state transition checking.
//!
//! Reference: Engineering Plan § Agent Lifecycle Management

use crate::health_check::HealthProbe;
use crate::restart_policy::RestartPolicy;
use crate::{LifecycleError, Result};
use alloc::vec::Vec;

/// Agent lifecycle configuration.
///
/// Comprehensive configuration for agent startup, shutdown, and runtime behavior,
/// including health checks, restart policies, and dependency ordering constraints.
/// This is a core component of the agent unit file specification.
///
/// # Fields
///
/// - `startup_timeout_ms`: Maximum time (milliseconds) agent has to reach Running state
/// - `shutdown_timeout_ms`: Maximum time (milliseconds) to gracefully stop agent
/// - `health_check`: Overall health check configuration for the agent
/// - `readiness_probe`: Probe determining if agent is ready to receive requests
/// - `liveness_probe`: Probe determining if agent is still alive and responsive
/// - `restart_policy`: Policy controlling restart behavior on failure
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Configuration
#[derive(Debug, Clone)]
pub struct LifecycleConfig {
    /// Maximum time in milliseconds for agent to reach Running state.
    pub startup_timeout_ms: u64,

    /// Maximum time in milliseconds for agent to reach Stopped state during shutdown.
    pub shutdown_timeout_ms: u64,

    /// Optional overall health check configuration.
    pub health_check: Option<()>,

    /// Readiness probe: determines if agent is ready to accept work.
    pub readiness_probe: Option<HealthProbe>,

    /// Liveness probe: determines if agent is still alive.
    pub liveness_probe: Option<HealthProbe>,

    /// Restart policy determining behavior on agent failure.
    pub restart_policy: RestartPolicy,
}

impl LifecycleConfig {
    /// Creates a new lifecycle configuration with default safe values.
    ///
    /// Defaults:
    /// - startup_timeout_ms: 30000 (30 seconds)
    /// - shutdown_timeout_ms: 15000 (15 seconds)
    /// - restart_policy: Never (no automatic restarts)
    /// - health_check, readiness_probe, liveness_probe: None
    ///
    /// Reference: Engineering Plan § Agent Lifecycle Management § Configuration
    pub fn new() -> Self {
        Self {
            startup_timeout_ms: 30000,
            shutdown_timeout_ms: 15000,
            health_check: None,
            readiness_probe: None,
            liveness_probe: None,
            restart_policy: RestartPolicy::Never,
        }
    }

    /// Sets the startup timeout in milliseconds.
    pub fn with_startup_timeout(mut self, timeout_ms: u64) -> Self {
        self.startup_timeout_ms = timeout_ms;
        self
    }

    /// Sets the shutdown timeout in milliseconds.
    pub fn with_shutdown_timeout(mut self, timeout_ms: u64) -> Self {
        self.shutdown_timeout_ms = timeout_ms;
        self
    }

    /// Sets the readiness probe.
    pub fn with_readiness_probe(mut self, probe: HealthProbe) -> Self {
        self.readiness_probe = Some(probe);
        self
    }

    /// Sets the liveness probe.
    pub fn with_liveness_probe(mut self, probe: HealthProbe) -> Self {
        self.liveness_probe = Some(probe);
        self
    }

    /// Sets the restart policy.
    pub fn with_restart_policy(mut self, policy: RestartPolicy) -> Self {
        self.restart_policy = policy;
        self
    }
}

impl Default for LifecycleConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Agent lifecycle state.
///
/// Represents the possible states of an agent throughout its lifecycle.
/// Valid state transitions are limited and enforced by the type system where possible.
///
/// # State Machine
///
/// ```text
/// Initializing ---> Starting ---> Running
///     |                |             |
///     +--- Failed      |             +--- Degraded
///                      |             |
///                  Stopping <--------+
///                      |
///                   Stopped
/// ```
///
/// Key invariants:
/// - Only `Running` or `Degraded` agents can process work
/// - `Failed` is a terminal state (typically triggers restart policy)
/// - `Stopped` is a terminal state after graceful shutdown
/// - `Degraded` indicates reduced capacity but continued operation
///
/// Reference: Engineering Plan § Agent Lifecycle Management § State Machine
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleState {
    /// Agent initialization phase, before startup begins.
    ///
    /// Agent is preparing internal state and resources.
    Initializing,

    /// Agent startup phase, moving toward Running.
    ///
    /// Agent startup procedures are in progress. Readiness probe not yet passing.
    Starting,

    /// Agent is running and ready to process work.
    ///
    /// Agent has passed readiness probe and is nominally operational.
    Running,

    /// Agent is degraded but still operational.
    ///
    /// Agent is still processing work but with reduced capacity or failures
    /// below the liveness probe threshold. May recover or transition to Failed.
    Degraded,

    /// Agent is shutting down.
    ///
    /// Agent is in the process of graceful shutdown. No new work should be
    /// assigned but existing work may continue until shutdown_timeout_ms.
    Stopping,

    /// Agent has stopped.
    ///
    /// Terminal state: agent has completed shutdown and released resources.
    /// A stopped agent will not be restarted unless explicitly requested.
    Stopped,

    /// Agent has failed.
    ///
    /// Terminal state (or triggers restart): agent encountered an unrecoverable
    /// error during startup, shutdown, or execution. Restart policy determines
    /// whether restart is attempted.
    Failed,
}

impl LifecycleState {
    /// Determines if the given state transition is valid.
    ///
    /// Enforces the lifecycle state machine rules:
    /// - Initializing can transition to: Starting, Failed
    /// - Starting can transition to: Running, Failed, Stopping
    /// - Running can transition to: Degraded, Stopping, Failed
    /// - Degraded can transition to: Running, Stopping, Failed
    /// - Stopping can transition to: Stopped, Failed
    /// - Stopped and Failed are terminal states
    ///
    /// # Arguments
    ///
    /// - `target`: The target state to transition to
    ///
    /// # Returns
    ///
    /// - `Ok(())` if transition is valid
    /// - `Err(LifecycleError::InvalidTransition)` if transition is invalid
    ///
    /// Reference: Engineering Plan § Agent Lifecycle Management § State Machine
    pub fn validate_transition(&self, target: LifecycleState) -> Result<()> {
        let is_valid = match (*self, target) {
            // From Initializing
            (Self::Initializing, Self::Starting) => true,
            (Self::Initializing, Self::Failed) => true,

            // From Starting
            (Self::Starting, Self::Running) => true,
            (Self::Starting, Self::Failed) => true,
            (Self::Starting, Self::Stopping) => true,

            // From Running
            (Self::Running, Self::Degraded) => true,
            (Self::Running, Self::Stopping) => true,
            (Self::Running, Self::Failed) => true,

            // From Degraded
            (Self::Degraded, Self::Running) => true,
            (Self::Degraded, Self::Stopping) => true,
            (Self::Degraded, Self::Failed) => true,

            // From Stopping
            (Self::Stopping, Self::Stopped) => true,
            (Self::Stopping, Self::Failed) => true,

            // Terminal states: no transitions allowed
            (Self::Stopped, _) => false,
            (Self::Failed, _) => false,

            // All other transitions are invalid
            _ => false,
        };

        if is_valid {
            Ok(())
        } else {
            Err(LifecycleError::InvalidTransition {
                current: format!("{:?}", self),
                target: format!("{:?}", target),
            })
        }
    }

    /// Checks if this state represents an operational state.
    ///
    /// Returns `true` if agent can process work in this state.
    pub fn is_operational(&self) -> bool {
        matches!(self, Self::Running | Self::Degraded)
    }

    /// Checks if this state is terminal (no further transitions allowed).
    ///
    /// Returns `true` for Stopped and Failed states.
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Stopped | Self::Failed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::format;

    #[test]
    fn test_lifecycle_config_default() {
        let config = LifecycleConfig::default();
        assert_eq!(config.startup_timeout_ms, 30000);
        assert_eq!(config.shutdown_timeout_ms, 15000);
        assert_eq!(config.restart_policy, RestartPolicy::Never);
        assert!(config.readiness_probe.is_none());
        assert!(config.liveness_probe.is_none());
    }

    #[test]
    fn test_lifecycle_config_builder() {
        let config = LifecycleConfig::new()
            .with_startup_timeout(10000)
            .with_shutdown_timeout(5000)
            .with_restart_policy(RestartPolicy::Always);

        assert_eq!(config.startup_timeout_ms, 10000);
        assert_eq!(config.shutdown_timeout_ms, 5000);
        assert_eq!(config.restart_policy, RestartPolicy::Always);
    }

    #[test]
    fn test_valid_transitions_from_initializing() {
        let state = LifecycleState::Initializing;
        assert!(state.validate_transition(LifecycleState::Starting).is_ok());
        assert!(state.validate_transition(LifecycleState::Failed).is_ok());
    }

    #[test]
    fn test_invalid_transitions_from_initializing() {
        let state = LifecycleState::Initializing;
        assert!(state.validate_transition(LifecycleState::Running).is_err());
        assert!(state.validate_transition(LifecycleState::Stopped).is_err());
        assert!(state.validate_transition(LifecycleState::Degraded).is_err());
    }

    #[test]
    fn test_valid_transitions_from_starting() {
        let state = LifecycleState::Starting;
        assert!(state.validate_transition(LifecycleState::Running).is_ok());
        assert!(state.validate_transition(LifecycleState::Failed).is_ok());
        assert!(state.validate_transition(LifecycleState::Stopping).is_ok());
    }

    #[test]
    fn test_valid_transitions_from_running() {
        let state = LifecycleState::Running;
        assert!(state.validate_transition(LifecycleState::Degraded).is_ok());
        assert!(state.validate_transition(LifecycleState::Stopping).is_ok());
        assert!(state.validate_transition(LifecycleState::Failed).is_ok());
    }

    #[test]
    fn test_valid_transitions_from_degraded() {
        let state = LifecycleState::Degraded;
        assert!(state.validate_transition(LifecycleState::Running).is_ok());
        assert!(state.validate_transition(LifecycleState::Stopping).is_ok());
        assert!(state.validate_transition(LifecycleState::Failed).is_ok());
    }

    #[test]
    fn test_valid_transitions_from_stopping() {
        let state = LifecycleState::Stopping;
        assert!(state.validate_transition(LifecycleState::Stopped).is_ok());
        assert!(state.validate_transition(LifecycleState::Failed).is_ok());
    }

    #[test]
    fn test_terminal_states_no_transitions() {
        assert!(LifecycleState::Stopped.validate_transition(LifecycleState::Running).is_err());
        assert!(LifecycleState::Stopped.validate_transition(LifecycleState::Starting).is_err());
        assert!(LifecycleState::Failed.validate_transition(LifecycleState::Running).is_err());
    }

    #[test]
    fn test_is_operational() {
        assert!(LifecycleState::Running.is_operational());
        assert!(LifecycleState::Degraded.is_operational());
        assert!(!LifecycleState::Initializing.is_operational());
        assert!(!LifecycleState::Starting.is_operational());
        assert!(!LifecycleState::Stopping.is_operational());
        assert!(!LifecycleState::Stopped.is_operational());
        assert!(!LifecycleState::Failed.is_operational());
    }

    #[test]
    fn test_is_terminal() {
        assert!(LifecycleState::Stopped.is_terminal());
        assert!(LifecycleState::Failed.is_terminal());
        assert!(!LifecycleState::Initializing.is_terminal());
        assert!(!LifecycleState::Starting.is_terminal());
        assert!(!LifecycleState::Running.is_terminal());
        assert!(!LifecycleState::Degraded.is_terminal());
        assert!(!LifecycleState::Stopping.is_terminal());
    }
}
