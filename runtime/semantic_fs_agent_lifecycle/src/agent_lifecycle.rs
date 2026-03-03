// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.

//! Agent lifecycle state machine, unit file parsing, health checks, and restart policies.
//!
//! Provides core agent lifecycle management including state transitions,
//! health monitoring, and automatic restart capabilities.


/// Agent lifecycle state enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentState {
    /// Agent created but not started
    Created,
    /// Agent starting
    Starting,
    /// Agent running normally
    Running,
    /// Agent stopping
    Stopping,
    /// Agent stopped
    Stopped,
    /// Agent in error state
    Failed,
}

/// Agent unit file configuration
#[derive(Debug, Clone)]
pub struct AgentUnit {
    /// Unit name
    pub name: String,
    /// Agent framework type
    pub framework: String,
    /// Current state
    pub state: AgentState,
}

impl AgentUnit {
    /// Create a new agent unit
    pub fn new(name: String, framework: String) -> Self {
        AgentUnit {
            name,
            framework,
            state: AgentState::Created,
        }
    }
}

/// Health check probe for agent monitoring
#[derive(Debug, Clone)]
pub struct HealthProbe {
    /// Probe name
    pub name: String,
    /// Probe type (http, tcp, exec, etc.)
    pub probe_type: String,
    /// Timeout in milliseconds
    pub timeout_ms: u32,
}

impl HealthProbe {
    /// Create a new health probe
    pub fn new(name: String, probe_type: String, timeout_ms: u32) -> Self {
        HealthProbe {
            name,
            probe_type,
            timeout_ms,
        }
    }
}

/// Restart policy for agent failure handling
#[derive(Debug, Clone)]
pub enum RestartPolicy {
    /// Never restart the agent
    Never,
    /// Always restart the agent
    Always,
    /// Restart only on failure
    OnFailure,
}

/// State machine for agent lifecycle management
pub struct StateMachine {
    state: AgentState,
}

impl StateMachine {
    /// Create a new state machine
    pub fn new() -> Self {
        StateMachine {
            state: AgentState::Created,
        }
    }

    /// Transition to starting state
    pub fn start(&mut self) -> Result<(), String> {
        if self.state != AgentState::Created && self.state != AgentState::Stopped {
            return Err("Invalid state for starting".into());
        }
        self.state = AgentState::Starting;
        Ok(())
    }

    /// Transition to running state
    pub fn run(&mut self) -> Result<(), String> {
        if self.state != AgentState::Starting {
            return Err("Invalid state for running".into());
        }
        self.state = AgentState::Running;
        Ok(())
    }

    /// Transition to stopping state
    pub fn stop(&mut self) -> Result<(), String> {
        if self.state != AgentState::Running {
            return Err("Invalid state for stopping".into());
        }
        self.state = AgentState::Stopping;
        Ok(())
    }

    /// Complete the stop transition
    pub fn stopped(&mut self) -> Result<(), String> {
        if self.state != AgentState::Stopping {
            return Err("Invalid state for stopped".into());
        }
        self.state = AgentState::Stopped;
        Ok(())
    }

    /// Mark agent as failed
    pub fn fail(&mut self) -> Result<(), String> {
        self.state = AgentState::Failed;
        Ok(())
    }

    /// Get current state
    pub fn current_state(&self) -> AgentState {
        self.state
    }
}

impl Default for StateMachine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_state_creation() {
        let unit = AgentUnit::new("test".into(), "langchain".into());
        assert_eq!(unit.state, AgentState::Created);
    }

    #[test]
    fn test_health_probe_creation() {
        let probe = HealthProbe::new("health".into(), "http".into(), 5000);
        assert_eq!(probe.name, "health");
        assert_eq!(probe.timeout_ms, 5000);
    }

    #[test]
    fn test_state_machine_transitions() {
        let mut sm = StateMachine::new();
        assert!(sm.start().is_ok());
        assert_eq!(sm.current_state(), AgentState::Starting);
        assert!(sm.run().is_ok());
        assert_eq!(sm.current_state(), AgentState::Running);
    }

    #[test]
    fn test_state_machine_invalid_transition() {
        let mut sm = StateMachine::new();
        let result = sm.run();
        assert!(result.is_err());
    }
}
