// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! Agent Lifecycle Manager - Core state machine and agent registry.
//!
//! Implements the central lifecycle management for agents, maintaining state transitions,
//! agent registry, and enforcing state machine invariants. Provides the foundation for
//! agent start/stop operations and lifecycle event tracking.
//!
//! # State Machine
//!
//! ```text
//! Undefined ---> Loading ---> Running
//!    |              |             |
//!    +-- Failed     |             +--- Stopping
//!                   |             |
//!               Stopped <--------+
//! ```
//!
//! Reference: Engineering Plan § Agent Lifecycle Manager § State Machine

use crate::unit_file::AgentUnitFile;
use crate::{LifecycleError, LifecycleState, Result};
use std::collections::BTreeMap;
use core::cell::RefCell;

/// Agent lifecycle manager state.
///
/// Represents the internal state tracked for each agent by the lifecycle manager.
/// Includes agent identity, configuration, current state, and metadata.
///
/// Reference: Engineering Plan § Agent Lifecycle Manager § State Machine
#[derive(Debug, Clone)]
pub struct AgentLifecycleState {
    /// Unique agent identifier (name/version).
    pub agent_id: String,

    /// Current lifecycle state.
    pub state: LifecycleState,

    /// Agent unit file configuration.
    pub unit_file: AgentUnitFile,

    /// Time of last state transition (Unix timestamp in ms).
    pub last_state_change_ms: u64,

    /// Number of restart attempts made.
    pub restart_count: u32,

    /// Optional error message if in Failed state.
    pub error_message: Option<String>,

    /// CT process identifier if running (platform-specific).
    pub ct_process_id: Option<u64>,
}

impl AgentLifecycleState {
    /// Creates a new lifecycle state for an agent.
    ///
    /// # Arguments
    ///
    /// - `agent_id`: Unique identifier for the agent
    /// - `unit_file`: Agent unit file configuration
    /// - `current_time_ms`: Current time in milliseconds since Unix epoch
    ///
    /// # Returns
    ///
    /// New `AgentLifecycleState` in Undefined state.
    ///
    /// Reference: Engineering Plan § Agent Lifecycle Manager § State Machine
    pub fn new(agent_id: String, unit_file: AgentUnitFile, current_time_ms: u64) -> Self {
        Self {
            agent_id,
            state: LifecycleState::Initializing,
            unit_file,
            last_state_change_ms: current_time_ms,
            restart_count: 0,
            error_message: None,
            ct_process_id: None,
        }
    }

    /// Transitions this agent to a new state.
    ///
    /// Validates the state transition according to the lifecycle state machine,
    /// updates the timestamp, and clears error messages on successful transition.
    ///
    /// # Arguments
    ///
    /// - `target_state`: The target lifecycle state
    /// - `current_time_ms`: Current time in milliseconds since Unix epoch
    ///
    /// # Returns
    ///
    /// - `Ok(())` if transition succeeded
    /// - `Err(LifecycleError::InvalidTransition)` if transition is invalid
    ///
    /// Reference: Engineering Plan § Agent Lifecycle Manager § State Machine
    pub fn transition_to(&mut self, target_state: LifecycleState, current_time_ms: u64) -> Result<()> {
        self.state.validate_transition(target_state)?;
        self.state = target_state;
        self.last_state_change_ms = current_time_ms;

        // Clear error message on successful transition
        if target_state != LifecycleState::Failed {
            self.error_message = None;
        }

        Ok(())
    }

    /// Sets error message when transitioning to Failed state.
    ///
    /// Should be called before or after transition to Failed state to record
    /// the reason for failure.
    ///
    /// # Arguments
    ///
    /// - `error`: Error message describing the failure reason
    pub fn set_error(&mut self, error: impl Into<String>) {
        self.error_message = Some(error.into());
    }

    /// Sets the CT process ID for this agent.
    ///
    /// Called after agent is spawned to track the platform process ID.
    ///
    /// # Arguments
    ///
    /// - `pid`: Platform-specific process identifier
    pub fn set_ct_process_id(&mut self, pid: u64) {
        self.ct_process_id = Some(pid);
    }

    /// Increments the restart count.
    ///
    /// Called when the agent is being restarted.
    pub fn increment_restart_count(&mut self) {
        self.restart_count = self.restart_count.saturating_add(1);
    }

    /// Checks if agent is in an operational state.
    ///
    /// Returns true if the agent can process work.
    pub fn is_operational(&self) -> bool {
        self.state.is_operational()
    }

    /// Checks if agent is in a terminal state.
    ///
    /// Returns true if no further state transitions are allowed.
    pub fn is_terminal(&self) -> bool {
        self.state.is_terminal()
    }
}

/// Agent Lifecycle Manager.
///
/// Maintains the registry of agents and their lifecycle states, enforces state
/// machine transitions, and provides operations for state querying and updates.
/// This is the central coordination point for all agent lifecycle management.
///
/// # Invariants
///
/// - Each agent_id maps to exactly one AgentLifecycleState
/// - State transitions must be valid according to the state machine
/// - Error messages are tracked for failed agents
/// - Restart counts are non-decreasing per restart attempt
///
/// Reference: Engineering Plan § Agent Lifecycle Manager
#[derive(Debug)]
pub struct LifecycleManager {
    /// Registry mapping agent IDs to their lifecycle states.
    agents: RefCell<BTreeMap<String, AgentLifecycleState>>,
}

impl LifecycleManager {
    /// Creates a new lifecycle manager.
    ///
    /// Initializes with an empty agent registry. Agents are registered as they
    /// are started or loaded.
    ///
    /// Reference: Engineering Plan § Agent Lifecycle Manager
    pub fn new() -> Self {
        Self {
            agents: RefCell::new(BTreeMap::new()),
        }
    }

    /// Registers a new agent with the lifecycle manager.
    ///
    /// Creates initial agent state in Initializing state. Returns error if agent
    /// with this ID is already registered.
    ///
    /// # Arguments
    ///
    /// - `agent_id`: Unique agent identifier
    /// - `unit_file`: Agent configuration
    /// - `current_time_ms`: Current time in milliseconds
    ///
    /// # Returns
    ///
    /// - `Ok(())` if agent was registered successfully
    /// - `Err(LifecycleError::...)` if registration failed
    ///
    /// Reference: Engineering Plan § Agent Lifecycle Manager § Agent Registry
    pub fn register_agent(
        &self,
        agent_id: impl Into<String>,
        unit_file: AgentUnitFile,
        current_time_ms: u64,
    ) -> Result<()> {
        let agent_id = agent_id.into();
        let mut agents = self.agents.borrow_mut();

        if agents.contains_key(&agent_id) {
            return Err(LifecycleError::GenericError(format!(
                "Agent {} already registered",
                agent_id
            )));
        }

        let lifecycle_state = AgentLifecycleState::new(agent_id.clone(), unit_file, current_time_ms);
        agents.insert(agent_id, lifecycle_state);
        Ok(())
    }

    /// Unregisters an agent from the lifecycle manager.
    ///
    /// Removes the agent from the registry. Typically called after a stopped or
    /// failed agent is cleaned up.
    ///
    /// # Arguments
    ///
    /// - `agent_id`: ID of agent to unregister
    ///
    /// # Returns
    ///
    /// - `Ok(())` if agent was unregistered
    /// - `Err` if agent was not found
    ///
    /// Reference: Engineering Plan § Agent Lifecycle Manager § Agent Registry
    pub fn unregister_agent(&self, agent_id: &str) -> Result<()> {
        let mut agents = self.agents.borrow_mut();

        if agents.remove(agent_id).is_some() {
            Ok(())
        } else {
            Err(LifecycleError::GenericError(format!(
                "Agent {} not found",
                agent_id
            )))
        }
    }

    /// Gets the current state of an agent.
    ///
    /// # Arguments
    ///
    /// - `agent_id`: Agent identifier
    ///
    /// # Returns
    ///
    /// - `Ok(LifecycleState)` if agent exists
    /// - `Err` if agent not found
    pub fn get_agent_state(&self, agent_id: &str) -> Result<LifecycleState> {
        let agents = self.agents.borrow();
        agents
            .get(agent_id)
            .map(|a| a.state)
            .ok_or_else(|| LifecycleError::GenericError(format!("Agent {} not found", agent_id)))
    }

    /// Transitions an agent to a new state.
    ///
    /// Validates the transition and updates the agent's state timestamp.
    ///
    /// # Arguments
    ///
    /// - `agent_id`: Agent identifier
    /// - `target_state`: Target lifecycle state
    /// - `current_time_ms`: Current time in milliseconds
    ///
    /// # Returns
    ///
    /// - `Ok(())` if transition succeeded
    /// - `Err(LifecycleError::...)` if transition failed or agent not found
    ///
    /// Reference: Engineering Plan § Agent Lifecycle Manager § State Transitions
    pub fn transition_agent(
        &self,
        agent_id: &str,
        target_state: LifecycleState,
        current_time_ms: u64,
    ) -> Result<()> {
        let mut agents = self.agents.borrow_mut();

        let agent = agents
            .get_mut(agent_id)
            .ok_or_else(|| LifecycleError::GenericError(format!("Agent {} not found", agent_id)))?;

        agent.transition_to(target_state, current_time_ms)
    }

    /// Sets error message for a failed agent.
    ///
    /// # Arguments
    ///
    /// - `agent_id`: Agent identifier
    /// - `error`: Error message
    ///
    /// # Returns
    ///
    /// - `Ok(())` on success
    /// - `Err` if agent not found
    pub fn set_agent_error(&self, agent_id: &str, error: impl Into<String>) -> Result<()> {
        let mut agents = self.agents.borrow_mut();

        let agent = agents
            .get_mut(agent_id)
            .ok_or_else(|| LifecycleError::GenericError(format!("Agent {} not found", agent_id)))?;

        agent.set_error(error);
        Ok(())
    }

    /// Sets CT process ID for an agent.
    ///
    /// # Arguments
    ///
    /// - `agent_id`: Agent identifier
    /// - `pid`: Platform process ID
    ///
    /// # Returns
    ///
    /// - `Ok(())` on success
    /// - `Err` if agent not found
    pub fn set_agent_ct_process_id(&self, agent_id: &str, pid: u64) -> Result<()> {
        let mut agents = self.agents.borrow_mut();

        let agent = agents
            .get_mut(agent_id)
            .ok_or_else(|| LifecycleError::GenericError(format!("Agent {} not found", agent_id)))?;

        agent.set_ct_process_id(pid);
        Ok(())
    }

    /// Increments restart count for an agent.
    ///
    /// # Arguments
    ///
    /// - `agent_id`: Agent identifier
    ///
    /// # Returns
    ///
    /// - `Ok(u32)` - new restart count
    /// - `Err` if agent not found
    pub fn increment_restart_count(&self, agent_id: &str) -> Result<u32> {
        let mut agents = self.agents.borrow_mut();

        let agent = agents
            .get_mut(agent_id)
            .ok_or_else(|| LifecycleError::GenericError(format!("Agent {} not found", agent_id)))?;

        agent.increment_restart_count();
        Ok(agent.restart_count)
    }

    /// Lists all registered agents.
    ///
    /// # Returns
    ///
    /// Vector of agent IDs currently registered.
    pub fn list_agents(&self) -> Vec<String> {
        self.agents.borrow().keys().cloned().collect()
    }

    /// Lists agents in a specific state.
    ///
    /// # Arguments
    ///
    /// - `state`: Lifecycle state to filter by
    ///
    /// # Returns
    ///
    /// Vector of agent IDs in the specified state.
    pub fn agents_in_state(&self, state: LifecycleState) -> Vec<String> {
        self.agents
            .borrow()
            .iter()
            .filter(|(_, agent)| agent.state == state)
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Counts agents in all states.
    ///
    /// # Returns
    ///
    /// Tuple of (initializing, starting, running, degraded, stopping, stopped, failed)
    pub fn count_agents_by_state(&self) -> (u32, u32, u32, u32, u32, u32, u32) {
        let agents = self.agents.borrow();

        let mut counts = (0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32);

        for agent in agents.values() {
            match agent.state {
                LifecycleState::Initializing => counts.0 += 1,
                LifecycleState::Starting => counts.1 += 1,
                LifecycleState::Running => counts.2 += 1,
                LifecycleState::Degraded => counts.3 += 1,
                LifecycleState::Stopping => counts.4 += 1,
                LifecycleState::Stopped => counts.5 += 1,
                LifecycleState::Failed => counts.6 += 1,
            }
        }

        counts
    }

    /// Gets full agent lifecycle state info.
    ///
    /// # Arguments
    ///
    /// - `agent_id`: Agent identifier
    ///
    /// # Returns
    ///
    /// - `Ok(AgentLifecycleState)` if agent exists
    /// - `Err` if agent not found
    pub fn get_agent_info(&self, agent_id: &str) -> Result<AgentLifecycleState> {
        let agents = self.agents.borrow();
        agents
            .get(agent_id)
            .cloned()
            .ok_or_else(|| LifecycleError::GenericError(format!("Agent {} not found", agent_id)))
    }

    /// Returns total number of registered agents.
    pub fn total_agents(&self) -> u32 {
        self.agents.borrow().len() as u32
    }
}

impl Default for LifecycleManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_unit_file() -> AgentUnitFile {
        AgentUnitFile::new("test-agent", "1.0.0", "Test agent")
    }

    #[test]
    fn test_agent_lifecycle_state_creation() {
        let unit_file = create_test_unit_file();
        let state = AgentLifecycleState::new("agent-1".to_string(), unit_file, 1000);

        assert_eq!(state.agent_id, "agent-1");
        assert_eq!(state.state, LifecycleState::Initializing);
        assert_eq!(state.restart_count, 0);
        assert!(state.error_message.is_none());
        assert!(state.ct_process_id.is_none());
    }

    #[test]
    fn test_agent_lifecycle_state_transition() {
        let unit_file = create_test_unit_file();
        let mut state = AgentLifecycleState::new("agent-1".to_string(), unit_file, 1000);

        assert!(state.transition_to(LifecycleState::Starting, 1100).is_ok());
        assert_eq!(state.state, LifecycleState::Starting);
        assert_eq!(state.last_state_change_ms, 1100);

        assert!(state.transition_to(LifecycleState::Running, 1200).is_ok());
        assert_eq!(state.state, LifecycleState::Running);
    }

    #[test]
    fn test_agent_lifecycle_state_invalid_transition() {
        let unit_file = create_test_unit_file();
        let mut state = AgentLifecycleState::new("agent-1".to_string(), unit_file, 1000);

        assert!(state.transition_to(LifecycleState::Running, 1100).is_err());
    }

    #[test]
    fn test_agent_lifecycle_state_error_message() {
        let unit_file = create_test_unit_file();
        let mut state = AgentLifecycleState::new("agent-1".to_string(), unit_file, 1000);

        state.set_error("Test error");
        assert_eq!(state.error_message, Some("Test error".to_string()));
    }

    #[test]
    fn test_lifecycle_manager_creation() {
        let manager = LifecycleManager::new();
        assert_eq!(manager.total_agents(), 0);
        assert_eq!(manager.list_agents().len(), 0);
    }

    #[test]
    fn test_lifecycle_manager_register_agent() {
        let manager = LifecycleManager::new();
        let unit_file = create_test_unit_file();

        assert!(manager.register_agent("agent-1", unit_file.clone(), 1000).is_ok());
        assert_eq!(manager.total_agents(), 1);

        // Duplicate registration should fail
        assert!(manager.register_agent("agent-1", unit_file, 1000).is_err());
    }

    #[test]
    fn test_lifecycle_manager_unregister_agent() {
        let manager = LifecycleManager::new();
        let unit_file = create_test_unit_file();

        manager.register_agent("agent-1", unit_file, 1000).unwrap();
        assert_eq!(manager.total_agents(), 1);

        assert!(manager.unregister_agent("agent-1").is_ok());
        assert_eq!(manager.total_agents(), 0);

        // Unregistering non-existent agent should fail
        assert!(manager.unregister_agent("agent-1").is_err());
    }

    #[test]
    fn test_lifecycle_manager_get_agent_state() {
        let manager = LifecycleManager::new();
        let unit_file = create_test_unit_file();

        manager.register_agent("agent-1", unit_file, 1000).unwrap();

        assert_eq!(manager.get_agent_state("agent-1").unwrap(), LifecycleState::Initializing);
        assert!(manager.get_agent_state("agent-2").is_err());
    }

    #[test]
    fn test_lifecycle_manager_transition_agent() {
        let manager = LifecycleManager::new();
        let unit_file = create_test_unit_file();

        manager.register_agent("agent-1", unit_file, 1000).unwrap();

        assert!(manager
            .transition_agent("agent-1", LifecycleState::Starting, 1100)
            .is_ok());
        assert_eq!(manager.get_agent_state("agent-1").unwrap(), LifecycleState::Starting);

        // Invalid transition should fail
        assert!(manager
            .transition_agent("agent-1", LifecycleState::Initializing, 1200)
            .is_err());
    }

    #[test]
    fn test_lifecycle_manager_set_agent_error() {
        let manager = LifecycleManager::new();
        let unit_file = create_test_unit_file();

        manager.register_agent("agent-1", unit_file, 1000).unwrap();
        assert!(manager.set_agent_error("agent-1", "Test error").is_ok());

        let info = manager.get_agent_info("agent-1").unwrap();
        assert_eq!(info.error_message, Some("Test error".to_string()));
    }

    #[test]
    fn test_lifecycle_manager_agents_in_state() {
        let manager = LifecycleManager::new();

        for i in 0..3 {
            let unit_file = create_test_unit_file();
            manager.register_agent(format!("agent-{}", i), unit_file, 1000).unwrap();
        }

        manager.transition_agent("agent-0", LifecycleState::Starting, 1100).unwrap();
        manager.transition_agent("agent-1", LifecycleState::Starting, 1100).unwrap();

        let initializing = manager.agents_in_state(LifecycleState::Initializing);
        let starting = manager.agents_in_state(LifecycleState::Starting);

        assert_eq!(initializing.len(), 1);
        assert_eq!(starting.len(), 2);
    }

    #[test]
    fn test_lifecycle_manager_count_agents_by_state() {
        let manager = LifecycleManager::new();

        for i in 0..5 {
            let unit_file = create_test_unit_file();
            manager.register_agent(format!("agent-{}", i), unit_file, 1000).unwrap();
        }

        manager.transition_agent("agent-0", LifecycleState::Starting, 1100).unwrap();
        manager.transition_agent("agent-1", LifecycleState::Running, 1100).unwrap();
        manager.transition_agent("agent-2", LifecycleState::Stopping, 1100).unwrap();

        let counts = manager.count_agents_by_state();
        assert_eq!(counts.0, 2); // Initializing
        assert_eq!(counts.1, 1); // Starting
        assert_eq!(counts.2, 1); // Running
        assert_eq!(counts.4, 1); // Stopping
    }

    #[test]
    fn test_lifecycle_manager_restart_count() {
        let manager = LifecycleManager::new();
        let unit_file = create_test_unit_file();

        manager.register_agent("agent-1", unit_file, 1000).unwrap();

        assert_eq!(manager.increment_restart_count("agent-1").unwrap(), 1);
        assert_eq!(manager.increment_restart_count("agent-1").unwrap(), 2);
        assert_eq!(manager.increment_restart_count("agent-1").unwrap(), 3);

        let info = manager.get_agent_info("agent-1").unwrap();
        assert_eq!(info.restart_count, 3);
    }
}
