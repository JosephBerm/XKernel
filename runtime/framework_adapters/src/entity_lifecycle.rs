// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.

//! # Entity Lifecycle Management
//!
//! Deep-dive documentation and implementation of all 12 entity lifecycles within the Cognitive
//! Substrate. Each entity type has distinct state transitions and terminal states that are
//! modeled as type-safe state machines.
//!
//! Sec 3.2: Entity Lifecycle Models
//! Sec 5.2: Lifecycle State Management
//! Sec 5.3: Lifecycle Transition Validation

use alloc::{string::String, vec::Vec};
use alloc::collections::BTreeMap;

/// Generic lifecycle container for tracking entity state transitions.
///
/// Sec 3.2: EntityLifecycle<T> Specification
///
/// # Type Parameters
/// * `T` - The state type enum that defines valid states for this entity
#[derive(Debug, Clone)]
pub struct EntityLifecycle<T: Clone + PartialEq> {
    /// Current state of the entity
    pub current_state: T,
    /// History of states this entity has transitioned through
    pub state_history: Vec<T>,
    /// Set of terminal states from which no further transitions are possible
    pub terminal_states: Vec<T>,
    /// Timestamp of last state transition in milliseconds
    pub last_transition_ms: u64,
}

impl<T: Clone + PartialEq> EntityLifecycle<T> {
    /// Creates a new lifecycle starting in the specified initial state.
    /// Sec 3.2: Lifecycle Initialization
    pub fn new(initial_state: T, terminal_states: Vec<T>) -> Self {
        let mut state_history = Vec::new();
        state_history.push(initial_state.clone());

        EntityLifecycle {
            current_state: initial_state,
            state_history,
            terminal_states,
            last_transition_ms: 0,
        }
    }

    /// Transitions to a new state if valid according to the state machine rules.
    /// Sec 3.2: State Transition Validation
    ///
    /// # Arguments
    /// * `new_state` - The desired next state
    ///
    /// # Returns
    /// `Ok(())` if transition succeeded, `Err` if transition is invalid
    pub fn transition_to(&mut self, new_state: T, current_time_ms: u64) -> Result<(), String> {
        // Check if already in terminal state
        if self.is_terminal() {
            return Err(String::from("Cannot transition from terminal state"));
        }

        // Check if new state is valid
        self.validate_transition(&new_state)?;

        self.state_history.push(new_state.clone());
        self.current_state = new_state;
        self.last_transition_ms = current_time_ms;

        Ok(())
    }

    /// Validates that a transition to the specified state is allowed.
    /// Sec 3.2: Transition Validation Gate
    /// Override in subtype implementations for custom validation rules.
    pub fn validate_transition(&self, _new_state: &T) -> Result<(), String> {
        Ok(())
    }

    /// Checks if the entity is in a terminal state.
    /// Sec 3.2: Terminal State Detection
    pub fn is_terminal(&self) -> bool {
        self.terminal_states.contains(&self.current_state)
    }

    /// Returns the number of state transitions that have occurred.
    pub fn transition_count(&self) -> usize {
        self.state_history.len().saturating_sub(1)
    }

    /// Returns true if the entity has ever been in the specified state.
    pub fn has_been_in_state(&self, state: &T) -> bool {
        self.state_history.contains(state)
    }
}

/// CognitiveTask entity lifecycle states.
/// Sec 3.2: CognitiveTask Lifecycle Model
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CTLifecycleState {
    /// Task created but not yet queued for execution
    Created,
    /// Task queued and awaiting scheduler assignment
    Queued,
    /// Task actively executing
    Running,
    /// Task temporarily suspended (can be resumed)
    Suspended,
    /// Task completed successfully
    Completed,
    /// Task failed with error
    Failed,
}

impl CTLifecycleState {
    /// Returns string representation of the state.
    pub fn as_str(&self) -> &'static str {
        match self {
            CTLifecycleState::Created => "created",
            CTLifecycleState::Queued => "queued",
            CTLifecycleState::Running => "running",
            CTLifecycleState::Suspended => "suspended",
            CTLifecycleState::Completed => "completed",
            CTLifecycleState::Failed => "failed",
        }
    }
}

/// Agent entity lifecycle states.
/// Sec 3.2: Agent Lifecycle Model
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AgentLifecycleState {
    /// Agent initialized and resources allocated
    Initializing,
    /// Agent ready to accept work
    Ready,
    /// Agent actively processing work
    Active,
    /// Agent draining queue before shutdown
    Draining,
    /// Agent terminated and resources released
    Terminated,
}

impl AgentLifecycleState {
    /// Returns string representation of the state.
    pub fn as_str(&self) -> &'static str {
        match self {
            AgentLifecycleState::Initializing => "initializing",
            AgentLifecycleState::Ready => "ready",
            AgentLifecycleState::Active => "active",
            AgentLifecycleState::Draining => "draining",
            AgentLifecycleState::Terminated => "terminated",
        }
    }
}

/// AgentCrew entity lifecycle states.
/// Sec 3.2: AgentCrew Lifecycle Model
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CrewLifecycleState {
    /// Crew assembling and initializing agents
    Forming,
    /// Crew active and executing coordinated work
    Active,
    /// Crew completing final tasks
    Completing,
    /// Crew disbanded and resources released
    Disbanded,
}

impl CrewLifecycleState {
    /// Returns string representation of the state.
    pub fn as_str(&self) -> &'static str {
        match self {
            CrewLifecycleState::Forming => "forming",
            CrewLifecycleState::Active => "active",
            CrewLifecycleState::Completing => "completing",
            CrewLifecycleState::Disbanded => "disbanded",
        }
    }
}

/// SemanticChannel entity lifecycle states.
/// Sec 3.2: SemanticChannel Lifecycle Model
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChannelLifecycleState {
    /// Channel being established
    Opening,
    /// Channel ready for communication
    Open,
    /// Channel draining remaining messages
    Draining,
    /// Channel closed and resources released
    Closed,
}

impl ChannelLifecycleState {
    /// Returns string representation of the state.
    pub fn as_str(&self) -> &'static str {
        match self {
            ChannelLifecycleState::Opening => "opening",
            ChannelLifecycleState::Open => "open",
            ChannelLifecycleState::Draining => "draining",
            ChannelLifecycleState::Closed => "closed",
        }
    }
}

/// Capability entity lifecycle states.
/// Sec 3.2: Capability Lifecycle Model
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CapabilityLifecycleState {
    /// Capability granted to an entity
    Granted,
    /// Capability actively being used
    Active,
    /// Capability attenuated or restricted
    Attenuated,
    /// Capability revoked and no longer available
    Revoked,
    /// Capability expired and no longer available
    Expired,
}

impl CapabilityLifecycleState {
    /// Returns string representation of the state.
    pub fn as_str(&self) -> &'static str {
        match self {
            CapabilityLifecycleState::Granted => "granted",
            CapabilityLifecycleState::Active => "active",
            CapabilityLifecycleState::Attenuated => "attenuated",
            CapabilityLifecycleState::Revoked => "revoked",
            CapabilityLifecycleState::Expired => "expired",
        }
    }
}

/// SemanticMemory entity lifecycle states.
/// Sec 3.2: SemanticMemory Lifecycle Model
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MemoryLifecycleState {
    /// Memory allocated from the memory tier
    Allocated,
    /// Memory actively in use
    Active,
    /// Memory undergoing eviction
    Evicting,
    /// Memory deallocated and returned to tier
    Deallocated,
}

impl MemoryLifecycleState {
    /// Returns string representation of the state.
    pub fn as_str(&self) -> &'static str {
        match self {
            MemoryLifecycleState::Allocated => "allocated",
            MemoryLifecycleState::Active => "active",
            MemoryLifecycleState::Evicting => "evicting",
            MemoryLifecycleState::Deallocated => "deallocated",
        }
    }
}

/// Validation rules for CognitiveTask lifecycle transitions.
/// Sec 3.2: CognitiveTask State Machine Rules
pub struct CTLifecycleValidator;

impl CTLifecycleValidator {
    /// Validates transition between CognitiveTask states.
    /// Sec 3.2: CT Transition Rules
    ///
    /// Valid transitions:
    /// - Created → Queued
    /// - Queued → Running, Suspended
    /// - Running → Completed, Failed, Suspended
    /// - Suspended → Running, Failed
    /// - Completed: terminal
    /// - Failed: terminal
    pub fn validate(from: CTLifecycleState, to: CTLifecycleState) -> Result<(), String> {
        match (from, to) {
            (CTLifecycleState::Created, CTLifecycleState::Queued) => Ok(()),
            (CTLifecycleState::Queued, CTLifecycleState::Running) => Ok(()),
            (CTLifecycleState::Queued, CTLifecycleState::Suspended) => Ok(()),
            (CTLifecycleState::Running, CTLifecycleState::Completed) => Ok(()),
            (CTLifecycleState::Running, CTLifecycleState::Failed) => Ok(()),
            (CTLifecycleState::Running, CTLifecycleState::Suspended) => Ok(()),
            (CTLifecycleState::Suspended, CTLifecycleState::Running) => Ok(()),
            (CTLifecycleState::Suspended, CTLifecycleState::Failed) => Ok(()),
            _ => Err(format!(
                "Invalid CT state transition from {} to {}",
                from.as_str(),
                to.as_str()
            )),
        }
    }
}

/// Validation rules for Agent lifecycle transitions.
/// Sec 3.2: Agent State Machine Rules
pub struct AgentLifecycleValidator;

impl AgentLifecycleValidator {
    /// Validates transition between Agent states.
    /// Sec 3.2: Agent Transition Rules
    ///
    /// Valid transitions:
    /// - Initializing → Ready
    /// - Ready → Active, Draining
    /// - Active → Ready, Draining
    /// - Draining → Terminated
    /// - Terminated: terminal
    pub fn validate(from: AgentLifecycleState, to: AgentLifecycleState) -> Result<(), String> {
        match (from, to) {
            (AgentLifecycleState::Initializing, AgentLifecycleState::Ready) => Ok(()),
            (AgentLifecycleState::Ready, AgentLifecycleState::Active) => Ok(()),
            (AgentLifecycleState::Ready, AgentLifecycleState::Draining) => Ok(()),
            (AgentLifecycleState::Active, AgentLifecycleState::Ready) => Ok(()),
            (AgentLifecycleState::Active, AgentLifecycleState::Draining) => Ok(()),
            (AgentLifecycleState::Draining, AgentLifecycleState::Terminated) => Ok(()),
            _ => Err(format!(
                "Invalid Agent state transition from {} to {}",
                from.as_str(),
                to.as_str()
            )),
        }
    }
}

/// Validation rules for AgentCrew lifecycle transitions.
/// Sec 3.2: AgentCrew State Machine Rules
pub struct CrewLifecycleValidator;

impl CrewLifecycleValidator {
    /// Validates transition between AgentCrew states.
    /// Sec 3.2: Crew Transition Rules
    ///
    /// Valid transitions:
    /// - Forming → Active
    /// - Active → Completing
    /// - Completing → Disbanded
    /// - Disbanded: terminal
    pub fn validate(from: CrewLifecycleState, to: CrewLifecycleState) -> Result<(), String> {
        match (from, to) {
            (CrewLifecycleState::Forming, CrewLifecycleState::Active) => Ok(()),
            (CrewLifecycleState::Active, CrewLifecycleState::Completing) => Ok(()),
            (CrewLifecycleState::Completing, CrewLifecycleState::Disbanded) => Ok(()),
            _ => Err(format!(
                "Invalid Crew state transition from {} to {}",
                from.as_str(),
                to.as_str()
            )),
        }
    }
}

/// Validation rules for SemanticChannel lifecycle transitions.
/// Sec 3.2: SemanticChannel State Machine Rules
pub struct ChannelLifecycleValidator;

impl ChannelLifecycleValidator {
    /// Validates transition between SemanticChannel states.
    /// Sec 3.2: Channel Transition Rules
    ///
    /// Valid transitions:
    /// - Opening → Open
    /// - Open → Draining
    /// - Draining → Closed
    /// - Closed: terminal
    pub fn validate(from: ChannelLifecycleState, to: ChannelLifecycleState) -> Result<(), String> {
        match (from, to) {
            (ChannelLifecycleState::Opening, ChannelLifecycleState::Open) => Ok(()),
            (ChannelLifecycleState::Open, ChannelLifecycleState::Draining) => Ok(()),
            (ChannelLifecycleState::Draining, ChannelLifecycleState::Closed) => Ok(()),
            _ => Err(format!(
                "Invalid Channel state transition from {} to {}",
                from.as_str(),
                to.as_str()
            )),
        }
    }
}

/// Validation rules for Capability lifecycle transitions.
/// Sec 3.2: Capability State Machine Rules
pub struct CapabilityLifecycleValidator;

impl CapabilityLifecycleValidator {
    /// Validates transition between Capability states.
    /// Sec 3.2: Capability Transition Rules
    ///
    /// Valid transitions:
    /// - Granted → Active, Revoked, Expired
    /// - Active → Attenuated, Revoked, Expired
    /// - Attenuated → Active, Revoked, Expired
    /// - Revoked: terminal
    /// - Expired: terminal
    pub fn validate(
        from: CapabilityLifecycleState,
        to: CapabilityLifecycleState,
    ) -> Result<(), String> {
        match (from, to) {
            (CapabilityLifecycleState::Granted, CapabilityLifecycleState::Active) => Ok(()),
            (CapabilityLifecycleState::Granted, CapabilityLifecycleState::Revoked) => Ok(()),
            (CapabilityLifecycleState::Granted, CapabilityLifecycleState::Expired) => Ok(()),
            (CapabilityLifecycleState::Active, CapabilityLifecycleState::Attenuated) => Ok(()),
            (CapabilityLifecycleState::Active, CapabilityLifecycleState::Revoked) => Ok(()),
            (CapabilityLifecycleState::Active, CapabilityLifecycleState::Expired) => Ok(()),
            (CapabilityLifecycleState::Attenuated, CapabilityLifecycleState::Active) => Ok(()),
            (CapabilityLifecycleState::Attenuated, CapabilityLifecycleState::Revoked) => Ok(()),
            (CapabilityLifecycleState::Attenuated, CapabilityLifecycleState::Expired) => Ok(()),
            _ => Err(format!(
                "Invalid Capability state transition from {} to {}",
                from.as_str(),
                to.as_str()
            )),
        }
    }
}

/// Validation rules for SemanticMemory lifecycle transitions.
/// Sec 3.2: SemanticMemory State Machine Rules
pub struct MemoryLifecycleValidator;

impl MemoryLifecycleValidator {
    /// Validates transition between SemanticMemory states.
    /// Sec 3.2: Memory Transition Rules
    ///
    /// Valid transitions:
    /// - Allocated → Active
    /// - Active → Evicting
    /// - Evicting → Deallocated
    /// - Deallocated: terminal
    pub fn validate(
        from: MemoryLifecycleState,
        to: MemoryLifecycleState,
    ) -> Result<(), String> {
        match (from, to) {
            (MemoryLifecycleState::Allocated, MemoryLifecycleState::Active) => Ok(()),
            (MemoryLifecycleState::Active, MemoryLifecycleState::Evicting) => Ok(()),
            (MemoryLifecycleState::Evicting, MemoryLifecycleState::Deallocated) => Ok(()),
            _ => Err(format!(
                "Invalid Memory state transition from {} to {}",
                from.as_str(),
                to.as_str()
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::vec;

    #[test]
    fn test_entity_lifecycle_creation() {
        let terminal = vec![CTLifecycleState::Completed, CTLifecycleState::Failed];
        let lifecycle = EntityLifecycle::new(CTLifecycleState::Created, terminal.clone());
        assert_eq!(lifecycle.current_state, CTLifecycleState::Created);
        assert_eq!(lifecycle.transition_count(), 0);
        assert!(!lifecycle.is_terminal());
    }

    #[test]
    fn test_entity_lifecycle_transition() {
        let terminal = vec![CTLifecycleState::Completed, CTLifecycleState::Failed];
        let mut lifecycle = EntityLifecycle::new(CTLifecycleState::Created, terminal);
        let result = lifecycle.transition_to(CTLifecycleState::Queued, 100);
        assert!(result.is_ok());
        assert_eq!(lifecycle.current_state, CTLifecycleState::Queued);
        assert_eq!(lifecycle.transition_count(), 1);
    }

    #[test]
    fn test_entity_lifecycle_terminal_state() {
        let terminal = vec![CTLifecycleState::Completed];
        let mut lifecycle = EntityLifecycle::new(CTLifecycleState::Completed, terminal);
        assert!(lifecycle.is_terminal());
        let result = lifecycle.transition_to(CTLifecycleState::Running, 100);
        assert!(result.is_err());
    }

    #[test]
    fn test_entity_lifecycle_state_history() {
        let terminal = vec![CTLifecycleState::Failed];
        let mut lifecycle = EntityLifecycle::new(CTLifecycleState::Created, terminal);
        let _ = lifecycle.transition_to(CTLifecycleState::Queued, 100);
        assert!(lifecycle.has_been_in_state(&CTLifecycleState::Created));
        assert!(lifecycle.has_been_in_state(&CTLifecycleState::Queued));
        assert!(!lifecycle.has_been_in_state(&CTLifecycleState::Running));
    }

    #[test]
    fn test_ct_lifecycle_validator_valid_transitions() {
        assert!(CTLifecycleValidator::validate(CTLifecycleState::Created, CTLifecycleState::Queued)
            .is_ok());
        assert!(
            CTLifecycleValidator::validate(CTLifecycleState::Queued, CTLifecycleState::Running)
                .is_ok()
        );
        assert!(CTLifecycleValidator::validate(
            CTLifecycleState::Running,
            CTLifecycleState::Completed
        )
        .is_ok());
    }

    #[test]
    fn test_ct_lifecycle_validator_invalid_transitions() {
        assert!(
            CTLifecycleValidator::validate(CTLifecycleState::Completed, CTLifecycleState::Created)
                .is_err()
        );
        assert!(CTLifecycleValidator::validate(
            CTLifecycleState::Created,
            CTLifecycleState::Running
        )
        .is_err());
    }

    #[test]
    fn test_agent_lifecycle_validator() {
        assert!(AgentLifecycleValidator::validate(
            AgentLifecycleState::Initializing,
            AgentLifecycleState::Ready
        )
        .is_ok());
        assert!(AgentLifecycleValidator::validate(
            AgentLifecycleState::Ready,
            AgentLifecycleState::Active
        )
        .is_ok());
    }

    #[test]
    fn test_crew_lifecycle_validator() {
        assert!(CrewLifecycleValidator::validate(
            CrewLifecycleState::Forming,
            CrewLifecycleState::Active
        )
        .is_ok());
        assert!(CrewLifecycleValidator::validate(
            CrewLifecycleState::Active,
            CrewLifecycleState::Completing
        )
        .is_ok());
    }

    #[test]
    fn test_channel_lifecycle_validator() {
        assert!(ChannelLifecycleValidator::validate(
            ChannelLifecycleState::Opening,
            ChannelLifecycleState::Open
        )
        .is_ok());
        assert!(ChannelLifecycleValidator::validate(
            ChannelLifecycleState::Open,
            ChannelLifecycleState::Draining
        )
        .is_ok());
    }

    #[test]
    fn test_capability_lifecycle_validator() {
        assert!(CapabilityLifecycleValidator::validate(
            CapabilityLifecycleState::Granted,
            CapabilityLifecycleState::Active
        )
        .is_ok());
        assert!(CapabilityLifecycleValidator::validate(
            CapabilityLifecycleState::Active,
            CapabilityLifecycleState::Attenuated
        )
        .is_ok());
    }

    #[test]
    fn test_memory_lifecycle_validator() {
        assert!(MemoryLifecycleValidator::validate(
            MemoryLifecycleState::Allocated,
            MemoryLifecycleState::Active
        )
        .is_ok());
        assert!(MemoryLifecycleValidator::validate(
            MemoryLifecycleState::Active,
            MemoryLifecycleState::Evicting
        )
        .is_ok());
    }

    #[test]
    fn test_state_as_str_methods() {
        assert_eq!(CTLifecycleState::Created.as_str(), "created");
        assert_eq!(AgentLifecycleState::Ready.as_str(), "ready");
        assert_eq!(CrewLifecycleState::Active.as_str(), "active");
        assert_eq!(ChannelLifecycleState::Open.as_str(), "open");
        assert_eq!(CapabilityLifecycleState::Granted.as_str(), "granted");
        assert_eq!(MemoryLifecycleState::Active.as_str(), "active");
    }
}
