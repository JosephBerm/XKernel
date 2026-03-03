// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! Agent stop operation - Graceful shutdown and resource cleanup.
//!
//! Implements the agent shutdown sequence: graceful termination signal, timeout-based
//! forced termination, resource cleanup, and state updates. Handles both graceful and
//! forceful shutdown paths with configurable timeouts.
//!
//! Reference: Engineering Plan § Agent Lifecycle Manager § Stop Operation

use crate::lifecycle_manager::LifecycleManager;
use crate::{LifecycleError, LifecycleState, Result};

/// Signal types for agent termination.
///
/// Represents different termination signals sent to agents during shutdown.
///
/// Reference: Engineering Plan § Agent Lifecycle Manager § Stop Operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminationSignal {
    /// SIGTERM - Request graceful shutdown
    Terminate,

    /// SIGKILL - Forceful termination (kill -9)
    Kill,

    /// Custom signal (for future extensions)
    Custom(u32),
}

/// Result of sending termination signal.
///
/// Tracks the outcome of attempting to send a signal to a running agent.
///
/// Reference: Engineering Plan § Agent Lifecycle Manager § Stop Operation
#[derive(Debug, Clone)]
pub struct SignalResult {
    /// Whether signal was sent successfully.
    pub sent: bool,

    /// Error message if signal failed to send.
    pub error_message: Option<String>,

    /// Time signal was sent (Unix timestamp in ms).
    pub sent_at_ms: u64,
}

impl SignalResult {
    /// Creates a successful signal result.
    pub fn success(sent_at_ms: u64) -> Self {
        Self {
            sent: true,
            error_message: None,
            sent_at_ms,
        }
    }

    /// Creates a failed signal result.
    pub fn failure(error: impl Into<String>, sent_at_ms: u64) -> Self {
        Self {
            sent: false,
            error_message: Some(error.into()),
            sent_at_ms,
        }
    }
}

/// Parameters for stopping an agent.
///
/// Encapsulates all information needed to stop an agent, including process ID,
/// timeout configuration, and timing information.
///
/// Reference: Engineering Plan § Agent Lifecycle Manager § Stop Operation
#[derive(Debug, Clone)]
pub struct AgentStopParams {
    /// Unique agent identifier.
    pub agent_id: String,

    /// CT process ID to stop.
    pub ct_process_id: u64,

    /// Graceful shutdown timeout in milliseconds.
    pub graceful_timeout_ms: u64,

    /// Forced termination timeout in milliseconds.
    pub force_timeout_ms: u64,

    /// Current time in milliseconds since Unix epoch.
    pub current_time_ms: u64,
}

impl AgentStopParams {
    /// Creates new agent stop parameters.
    ///
    /// # Arguments
    ///
    /// - `agent_id`: Unique identifier for the agent
    /// - `ct_process_id`: Platform process ID
    /// - `graceful_timeout_ms`: Time to wait for graceful shutdown
    /// - `force_timeout_ms`: Time to wait for forced termination
    /// - `current_time_ms`: Current time in milliseconds
    ///
    /// Reference: Engineering Plan § Agent Lifecycle Manager § Stop Operation
    pub fn new(
        agent_id: impl Into<String>,
        ct_process_id: u64,
        graceful_timeout_ms: u64,
        force_timeout_ms: u64,
        current_time_ms: u64,
    ) -> Self {
        Self {
            agent_id: agent_id.into(),
            ct_process_id,
            graceful_timeout_ms,
            force_timeout_ms,
            current_time_ms,
        }
    }
}

/// Agent stop operation handler.
///
/// Implements the full shutdown sequence for agents: graceful termination,
/// timeout handling, forced killing, and resource cleanup.
///
/// Reference: Engineering Plan § Agent Lifecycle Manager § Stop Operation
pub struct AgentStopHandler;

impl AgentStopHandler {
    /// Sends termination signal to agent process.
    ///
    /// Simulates sending a signal to the agent's CT process. In production,
    /// this would invoke actual kernel signal mechanisms.
    ///
    /// # Arguments
    ///
    /// - `agent_id`: Agent identifier
    /// - `pid`: Process ID to signal
    /// - `signal`: Signal type to send
    /// - `current_time_ms`: Current time in milliseconds
    ///
    /// # Returns
    ///
    /// `SignalResult` indicating success or failure of signal delivery.
    ///
    /// Reference: Engineering Plan § Agent Lifecycle Manager § Stop Operation
    pub fn send_signal(
        agent_id: &str,
        pid: u64,
        signal: TerminationSignal,
        current_time_ms: u64,
    ) -> SignalResult {
        // Validate parameters
        if agent_id.is_empty() {
            return SignalResult::failure("Agent ID cannot be empty", current_time_ms);
        }

        if pid == 0 {
            return SignalResult::failure("Process ID cannot be zero", current_time_ms);
        }

        // In production, this would invoke kernel signal mechanisms
        // For now, we simulate successful signal delivery
        SignalResult::success(current_time_ms)
    }

    /// Waits for agent process to terminate after signal.
    ///
    /// Simulates polling or waiting for a process to exit after receiving a signal.
    /// In production, this would use kernel process monitoring.
    ///
    /// # Arguments
    ///
    /// - `pid`: Process ID to monitor
    /// - `timeout_ms`: Maximum time to wait in milliseconds
    /// - `current_time_ms`: Current time in milliseconds
    ///
    /// # Returns
    ///
    /// - `Ok(true)` if process terminated within timeout
    /// - `Ok(false)` if timeout expired
    /// - `Err` if monitoring failed
    pub fn wait_for_process_termination(
        pid: u64,
        timeout_ms: u64,
        current_time_ms: u64,
    ) -> Result<bool> {
        if pid == 0 {
            return Err(LifecycleError::GenericError(
                "Process ID cannot be zero".to_string(),
            ));
        }

        // In production, this would check /proc/{pid} or use waitpid(2)
        // For simulation, we assume process terminates if given sufficient time
        // This is deterministic: small timeouts fail, larger ones succeed
        if timeout_ms >= 100 {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Performs graceful shutdown of agent.
    ///
    /// Sends SIGTERM, waits for graceful timeout, returns status of termination.
    /// If agent doesn't respond within timeout, caller should attempt forced shutdown.
    ///
    /// # Arguments
    ///
    /// - `agent_id`: Agent identifier
    /// - `pid`: Process ID to terminate
    /// - `graceful_timeout_ms`: Time to wait for graceful shutdown
    /// - `current_time_ms`: Current time in milliseconds
    ///
    /// # Returns
    ///
    /// - `Ok(true)` if agent terminated gracefully
    /// - `Ok(false)` if timeout expired (forced termination needed)
    /// - `Err` if operation failed
    ///
    /// Reference: Engineering Plan § Agent Lifecycle Manager § Stop Operation
    pub fn graceful_shutdown(
        agent_id: &str,
        pid: u64,
        graceful_timeout_ms: u64,
        current_time_ms: u64,
    ) -> Result<bool> {
        // Send SIGTERM
        let signal_result = Self::send_signal(agent_id, pid, TerminationSignal::Terminate, current_time_ms);

        if !signal_result.sent {
            return Err(LifecycleError::GenericError(
                signal_result.error_message.unwrap_or_else(|| "Failed to send signal".to_string()),
            ));
        }

        // Wait for process to terminate
        Self::wait_for_process_termination(pid, graceful_timeout_ms, current_time_ms)
    }

    /// Performs forced shutdown of agent.
    ///
    /// Sends SIGKILL to forcefully terminate agent. This bypasses graceful shutdown.
    ///
    /// # Arguments
    ///
    /// - `agent_id`: Agent identifier
    /// - `pid`: Process ID to kill
    /// - `current_time_ms`: Current time in milliseconds
    ///
    /// # Returns
    ///
    /// - `Ok(())` if kill signal sent successfully
    /// - `Err` if kill failed
    ///
    /// Reference: Engineering Plan § Agent Lifecycle Manager § Stop Operation
    pub fn forced_shutdown(agent_id: &str, pid: u64, current_time_ms: u64) -> Result<()> {
        let signal_result = Self::send_signal(agent_id, pid, TerminationSignal::Kill, current_time_ms);

        if signal_result.sent {
            Ok(())
        } else {
            Err(LifecycleError::GenericError(
                signal_result.error_message.unwrap_or_else(|| "Failed to send kill signal".to_string()),
            ))
        }
    }

    /// Cleans up agent resources.
    ///
    /// Releases resources held by the agent, including memory, file handles, etc.
    /// In production, this would invoke kernel resource management.
    ///
    /// # Arguments
    ///
    /// - `agent_id`: Agent identifier
    /// - `pid`: Process ID whose resources to clean up
    ///
    /// # Returns
    ///
    /// - `Ok(())` on successful cleanup
    /// - `Err` if cleanup failed
    pub fn cleanup_resources(agent_id: &str, _pid: u64) -> Result<()> {
        if agent_id.is_empty() {
            return Err(LifecycleError::GenericError(
                "Agent ID cannot be empty".to_string(),
            ));
        }

        // In production, this would:
        // - Release memory allocations
        // - Close open file descriptors
        // - Clean up temporary files
        // - Release capabilities
        // - Update resource accounting

        Ok(())
    }

    /// Performs complete agent stop operation.
    ///
    /// Orchestrates the full shutdown sequence: graceful termination attempt,
    /// forced termination if needed, resource cleanup, and state updates.
    ///
    /// # Arguments
    ///
    /// - `lifecycle_manager`: Lifecycle manager for state updates
    /// - `params`: Agent stop parameters
    ///
    /// # Returns
    ///
    /// - `Ok(bool)` - true if graceful shutdown, false if forced
    /// - `Err(LifecycleError::...)` if stop failed
    ///
    /// Reference: Engineering Plan § Agent Lifecycle Manager § Stop Operation
    pub fn stop_agent(
        lifecycle_manager: &LifecycleManager,
        params: AgentStopParams,
    ) -> Result<bool> {
        // Step 1: Transition to Stopping state
        lifecycle_manager.transition_agent(&params.agent_id, LifecycleState::Stopping, params.current_time_ms)?;

        // Step 2: Attempt graceful shutdown
        let graceful_success = Self::graceful_shutdown(
            &params.agent_id,
            params.ct_process_id,
            params.graceful_timeout_ms,
            params.current_time_ms,
        )?;

        // Step 3: If graceful shutdown timed out, force termination
        if !graceful_success {
            Self::forced_shutdown(&params.agent_id, params.ct_process_id, params.current_time_ms)?;
        }

        // Step 4: Clean up resources
        Self::cleanup_resources(&params.agent_id, params.ct_process_id)?;

        // Step 5: Transition to Stopped state
        lifecycle_manager.transition_agent(&params.agent_id, LifecycleState::Stopped, params.current_time_ms)?;

        Ok(graceful_success)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lifecycle_manager::LifecycleManager;
    use crate::unit_file::AgentUnitFile;

    fn create_test_unit_file() -> AgentUnitFile {
        AgentUnitFile::new("test-agent", "1.0.0", "Test agent")
    }

    #[test]
    fn test_termination_signal_equality() {
        assert_eq!(TerminationSignal::Terminate, TerminationSignal::Terminate);
        assert_eq!(TerminationSignal::Kill, TerminationSignal::Kill);
        assert_ne!(TerminationSignal::Terminate, TerminationSignal::Kill);
    }

    #[test]
    fn test_signal_result_success() {
        let result = SignalResult::success(1000);
        assert!(result.sent);
        assert!(result.error_message.is_none());
        assert_eq!(result.sent_at_ms, 1000);
    }

    #[test]
    fn test_signal_result_failure() {
        let result = SignalResult::failure("Test error", 1000);
        assert!(!result.sent);
        assert_eq!(result.error_message, Some("Test error".to_string()));
    }

    #[test]
    fn test_agent_stop_params_creation() {
        let params = AgentStopParams::new("agent-1", 12345, 5000, 1000, 2000);

        assert_eq!(params.agent_id, "agent-1");
        assert_eq!(params.ct_process_id, 12345);
        assert_eq!(params.graceful_timeout_ms, 5000);
        assert_eq!(params.force_timeout_ms, 1000);
        assert_eq!(params.current_time_ms, 2000);
    }

    #[test]
    fn test_send_signal_valid() {
        let result = AgentStopHandler::send_signal("agent-1", 12345, TerminationSignal::Terminate, 1000);
        assert!(result.sent);
    }

    #[test]
    fn test_send_signal_empty_agent_id() {
        let result = AgentStopHandler::send_signal("", 12345, TerminationSignal::Terminate, 1000);
        assert!(!result.sent);
    }

    #[test]
    fn test_send_signal_zero_pid() {
        let result = AgentStopHandler::send_signal("agent-1", 0, TerminationSignal::Terminate, 1000);
        assert!(!result.sent);
    }

    #[test]
    fn test_wait_for_process_termination_success() {
        let result = AgentStopHandler::wait_for_process_termination(12345, 200, 1000);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_wait_for_process_termination_timeout() {
        let result = AgentStopHandler::wait_for_process_termination(12345, 50, 1000);
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    fn test_wait_for_process_termination_zero_pid() {
        let result = AgentStopHandler::wait_for_process_termination(0, 100, 1000);
        assert!(result.is_err());
    }

    #[test]
    fn test_graceful_shutdown_success() {
        let result = AgentStopHandler::graceful_shutdown("agent-1", 12345, 200, 1000);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_graceful_shutdown_timeout() {
        let result = AgentStopHandler::graceful_shutdown("agent-1", 12345, 50, 1000);
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    fn test_forced_shutdown_success() {
        let result = AgentStopHandler::forced_shutdown("agent-1", 12345, 1000);
        assert!(result.is_ok());
    }

    #[test]
    fn test_forced_shutdown_zero_pid() {
        let result = AgentStopHandler::forced_shutdown("agent-1", 0, 1000);
        assert!(result.is_err());
    }

    #[test]
    fn test_cleanup_resources_success() {
        let result = AgentStopHandler::cleanup_resources("agent-1", 12345);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cleanup_resources_empty_agent_id() {
        let result = AgentStopHandler::cleanup_resources("", 12345);
        assert!(result.is_err());
    }

    #[test]
    fn test_stop_agent_graceful() {
        let manager = LifecycleManager::new();
        let unit_file = create_test_unit_file();

        // Register and transition agent to Running state
        manager.register_agent("agent-1", unit_file, 1000).unwrap();
        manager.transition_agent("agent-1", LifecycleState::Starting, 1100).unwrap();
        manager.transition_agent("agent-1", LifecycleState::Running, 1200).unwrap();
        manager.set_agent_ct_process_id("agent-1", 12345).unwrap();

        let params = AgentStopParams::new("agent-1", 12345, 200, 100, 1300);
        let result = AgentStopHandler::stop_agent(&manager, params);

        assert!(result.is_ok());
        assert!(result.unwrap()); // Should be graceful

        // Verify agent is in Stopped state
        assert_eq!(manager.get_agent_state("agent-1").unwrap(), LifecycleState::Stopped);
    }

    #[test]
    fn test_stop_agent_forced() {
        let manager = LifecycleManager::new();
        let unit_file = create_test_unit_file();

        manager.register_agent("agent-1", unit_file, 1000).unwrap();
        manager.transition_agent("agent-1", LifecycleState::Running, 1100).unwrap();
        manager.set_agent_ct_process_id("agent-1", 12345).unwrap();

        let params = AgentStopParams::new("agent-1", 12345, 50, 100, 1200);
        let result = AgentStopHandler::stop_agent(&manager, params);

        assert!(result.is_ok());
        assert!(!result.unwrap()); // Should be forced

        assert_eq!(manager.get_agent_state("agent-1").unwrap(), LifecycleState::Stopped);
    }

    #[test]
    fn test_stop_agent_not_found() {
        let manager = LifecycleManager::new();
        let params = AgentStopParams::new("agent-1", 12345, 5000, 1000, 1000);

        let result = AgentStopHandler::stop_agent(&manager, params);
        assert!(result.is_err());
    }
}
