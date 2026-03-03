// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! GPU Manager state machine and lifecycle.
//!
//! Implements a finite state machine for the GPU Manager to track its operational
//! state across device initialization, resource allocation, kernel execution,
//! checkpointing, and error recovery. Provides observability through transition logs.
//!
//! Reference: Engineering Plan § State Management, Observability

use crate::error::GpuError;
use crate::ids::GpuDeviceID;
use alloc::vec::Vec;
use core::fmt;

/// GPU Manager operational state.
///
/// Defines the possible states the GPU Manager can be in during its lifecycle.
/// State transitions are constrained to maintain safety and consistency.
///
/// Reference: Engineering Plan § State Management
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GpuManagerState {
    /// Idle state: no active workload, ready to accept directives.
    ///
    /// In this state, TPCs and VRAM are unallocated or deallocated.
    /// The GPU Manager can transition to Allocating upon receiving a directive.
    Idle,

    /// Allocating state: in process of allocating TPCs and VRAM for workload.
    ///
    /// Temporary state during resource allocation. Transitions to Executing
    /// on success or Error on failure.
    Allocating,

    /// Executing state: actively running kernel workload.
    ///
    /// Kernels are launching and executing on allocated TPCs.
    /// Can transition to Checkpointing or Error.
    Executing,

    /// Checkpointing state: saving GPU state to persistent storage.
    ///
    /// In this state, GPU state (VRAM, registers, kernel metadata) is being
    /// captured. Transitions to Idle on success or Error on failure.
    Checkpointing,

    /// Recovering state: restoring from a checkpoint or error condition.
    ///
    /// GPU state is being restored from a previous checkpoint.
    /// Transitions to Idle on success.
    Recovering,

    /// Error state: fatal error occurred (isolation violation, driver failure).
    ///
    /// GPU is in undefined state. Can transition to Recovering (recovery attempt)
    /// or Idle (error cleared without recovery).
    Error,
}

impl fmt::Display for GpuManagerState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GpuManagerState::Idle => write!(f, "Idle"),
            GpuManagerState::Allocating => write!(f, "Allocating"),
            GpuManagerState::Executing => write!(f, "Executing"),
            GpuManagerState::Checkpointing => write!(f, "Checkpointing"),
            GpuManagerState::Recovering => write!(f, "Recovering"),
            GpuManagerState::Error => write!(f, "Error"),
        }
    }
}

/// Event that triggers a state transition.
///
/// Events represent external stimuli (directives, kernel completion, errors)
/// that cause the GPU Manager to change state.
///
/// Reference: Engineering Plan § State Management
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GpuManagerEvent {
    /// Scheduler has issued a resource allocation directive.
    SchedulerDirectiveReceived,

    /// Resource allocation (TPCs, VRAM) completed successfully.
    AllocationComplete,

    /// Resource allocation failed (insufficient resources, etc.).
    AllocationFailed(GpuError),

    /// Kernel execution initiated.
    ExecutionStarted,

    /// Kernel execution completed (all queued kernels finished).
    ExecutionComplete,

    /// Checkpoint operation initiated.
    CheckpointInitiated,

    /// Checkpoint operation completed successfully.
    CheckpointComplete,

    /// Checkpoint operation failed.
    CheckpointFailed(GpuError),

    /// Recovery operation initiated.
    RecoveryInitiated,

    /// Recovery operation completed successfully.
    RecoveryComplete,

    /// Error occurred during operation.
    ErrorOccurred(GpuError),

    /// Error condition has been cleared (manual intervention).
    ErrorCleared,
}

impl fmt::Display for GpuManagerEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GpuManagerEvent::SchedulerDirectiveReceived => write!(f, "SchedulerDirectiveReceived"),
            GpuManagerEvent::AllocationComplete => write!(f, "AllocationComplete"),
            GpuManagerEvent::AllocationFailed(err) => write!(f, "AllocationFailed({})", err),
            GpuManagerEvent::ExecutionStarted => write!(f, "ExecutionStarted"),
            GpuManagerEvent::ExecutionComplete => write!(f, "ExecutionComplete"),
            GpuManagerEvent::CheckpointInitiated => write!(f, "CheckpointInitiated"),
            GpuManagerEvent::CheckpointComplete => write!(f, "CheckpointComplete"),
            GpuManagerEvent::CheckpointFailed(err) => write!(f, "CheckpointFailed({})", err),
            GpuManagerEvent::RecoveryInitiated => write!(f, "RecoveryInitiated"),
            GpuManagerEvent::RecoveryComplete => write!(f, "RecoveryComplete"),
            GpuManagerEvent::ErrorOccurred(err) => write!(f, "ErrorOccurred({})", err),
            GpuManagerEvent::ErrorCleared => write!(f, "ErrorCleared"),
        }
    }
}

/// State transition log entry for observability.
///
/// Records each state transition for debugging, monitoring, and audit purposes.
/// Includes timestamp, previous state, new state, triggering event, and optional error.
///
/// Reference: Engineering Plan § Observability
#[derive(Clone, Debug)]
pub struct StateTransitionLog {
    /// Timestamp of transition (nanoseconds since boot).
    pub timestamp_ns: u64,

    /// Previous state.
    pub from_state: GpuManagerState,

    /// New state.
    pub to_state: GpuManagerState,

    /// Triggering event.
    pub event: GpuManagerEvent,

    /// Device ID (if applicable).
    pub device_id: Option<GpuDeviceID>,

    /// Error information (if transition was due to error).
    pub error: Option<GpuError>,
}

impl fmt::Display for StateTransitionLog {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Transition(ts={}ns, {} -> {}, event={}, device={:?}, error={:?})",
            self.timestamp_ns,
            self.from_state,
            self.to_state,
            self.event,
            self.device_id.map(|d| format!("{:?}", d.as_bytes()[0])),
            self.error
        )
    }
}

/// GPU Manager state machine.
///
/// Tracks the current operational state and enforces valid transitions.
/// Maintains transition history for observability.
///
/// Reference: Engineering Plan § State Management
#[derive(Clone, Debug)]
pub struct GpuManagerStateMachine {
    /// Current state.
    pub current_state: GpuManagerState,

    /// Transition history (last N transitions).
    pub transition_log: Vec<StateTransitionLog>,

    /// Maximum log entries to retain.
    pub max_log_entries: usize,

    /// Total transitions performed (lifetime counter).
    pub total_transitions: u64,
}

impl GpuManagerStateMachine {
    /// Create a new state machine (starts in Idle state).
    pub fn new(max_log_entries: usize) -> Self {
        GpuManagerStateMachine {
            current_state: GpuManagerState::Idle,
            transition_log: Vec::new(),
            max_log_entries,
            total_transitions: 0,
        }
    }

    /// Attempt a state transition.
    ///
    /// Validates that the transition is legal according to the state machine rules.
    /// If valid, updates state and records transition in log.
    /// If invalid, returns error without changing state.
    ///
    /// # Arguments
    ///
    /// * `event` - Event triggering the transition
    /// * `timestamp_ns` - Transition timestamp in nanoseconds since boot
    /// * `device_id` - Device ID (optional, for attribution)
    pub fn transition(
        &mut self,
        event: GpuManagerEvent,
        timestamp_ns: u64,
        device_id: Option<GpuDeviceID>,
    ) -> Result<GpuManagerState, GpuError> {
        let new_state = self.compute_next_state(&event)?;

        // Record transition
        let error = match &event {
            GpuManagerEvent::AllocationFailed(e) | GpuManagerEvent::CheckpointFailed(e) | GpuManagerEvent::ErrorOccurred(e) => Some(*e),
            _ => None,
        };

        let log_entry = StateTransitionLog {
            timestamp_ns,
            from_state: self.current_state,
            to_state: new_state,
            event,
            device_id,
            error,
        };

        self.transition_log.push(log_entry);
        if self.transition_log.len() > self.max_log_entries {
            self.transition_log.remove(0);
        }

        self.current_state = new_state;
        self.total_transitions += 1;

        Ok(new_state)
    }

    /// Compute the next state given an event (without side effects).
    ///
    /// Validates the transition is legal.
    fn compute_next_state(&self, event: &GpuManagerEvent) -> Result<GpuManagerState, GpuError> {
        match (self.current_state, event) {
            // Idle -> Allocating
            (GpuManagerState::Idle, GpuManagerEvent::SchedulerDirectiveReceived) => Ok(GpuManagerState::Allocating),

            // Allocating -> Executing (success)
            (GpuManagerState::Allocating, GpuManagerEvent::AllocationComplete) => Ok(GpuManagerState::Executing),

            // Allocating -> Error (failure)
            (GpuManagerState::Allocating, GpuManagerEvent::AllocationFailed(_)) => Ok(GpuManagerState::Error),

            // Executing -> Idle (complete)
            (GpuManagerState::Executing, GpuManagerEvent::ExecutionComplete) => Ok(GpuManagerState::Idle),

            // Executing -> Checkpointing
            (GpuManagerState::Executing, GpuManagerEvent::CheckpointInitiated) => Ok(GpuManagerState::Checkpointing),

            // Executing -> Error
            (GpuManagerState::Executing, GpuManagerEvent::ErrorOccurred(_)) => Ok(GpuManagerState::Error),

            // Checkpointing -> Idle (success)
            (GpuManagerState::Checkpointing, GpuManagerEvent::CheckpointComplete) => Ok(GpuManagerState::Idle),

            // Checkpointing -> Error (failure)
            (GpuManagerState::Checkpointing, GpuManagerEvent::CheckpointFailed(_)) => Ok(GpuManagerState::Error),

            // Error -> Recovering
            (GpuManagerState::Error, GpuManagerEvent::RecoveryInitiated) => Ok(GpuManagerState::Recovering),

            // Error -> Idle (cleared)
            (GpuManagerState::Error, GpuManagerEvent::ErrorCleared) => Ok(GpuManagerState::Idle),

            // Recovering -> Idle (complete)
            (GpuManagerState::Recovering, GpuManagerEvent::RecoveryComplete) => Ok(GpuManagerState::Idle),

            // All other transitions are invalid
            _ => Err(GpuError::DriverError),
        }
    }

    /// Get the current state.
    pub fn state(&self) -> GpuManagerState {
        self.current_state
    }

    /// Check if the state machine is in error state.
    pub fn is_in_error(&self) -> bool {
        self.current_state == GpuManagerState::Error
    }

    /// Get the most recent transition (if any).
    pub fn last_transition(&self) -> Option<&StateTransitionLog> {
        self.transition_log.last()
    }

    /// Get the full transition history (immutable).
    pub fn history(&self) -> &[StateTransitionLog] {
        &self.transition_log
    }

    /// Clear the transition log (for testing).
    pub fn clear_log(&mut self) {
        self.transition_log.clear();
    }
}

impl fmt::Display for GpuManagerStateMachine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "GpuManagerStateMachine(state={}, transitions={}, log_entries={})",
            self.current_state,
            self.total_transitions,
            self.transition_log.len()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::format;

    #[test]
    fn test_state_machine_initialization() {
        let sm = GpuManagerStateMachine::new(10);
        assert_eq!(sm.current_state, GpuManagerState::Idle);
        assert_eq!(sm.total_transitions, 0);
        assert_eq!(sm.transition_log.len(), 0);
    }

    #[test]
    fn test_valid_transition_idle_to_allocating() {
        let mut sm = GpuManagerStateMachine::new(10);
        let result = sm.transition(GpuManagerEvent::SchedulerDirectiveReceived, 1000, None);

        assert!(result.is_ok());
        assert_eq!(sm.current_state, GpuManagerState::Allocating);
        assert_eq!(sm.total_transitions, 1);
    }

    #[test]
    fn test_valid_transition_allocating_to_executing() {
        let mut sm = GpuManagerStateMachine::new(10);
        let _ = sm.transition(GpuManagerEvent::SchedulerDirectiveReceived, 1000, None);
        let result = sm.transition(GpuManagerEvent::AllocationComplete, 2000, None);

        assert!(result.is_ok());
        assert_eq!(sm.current_state, GpuManagerState::Executing);
    }

    #[test]
    fn test_valid_transition_executing_to_idle() {
        let mut sm = GpuManagerStateMachine::new(10);
        let _ = sm.transition(GpuManagerEvent::SchedulerDirectiveReceived, 1000, None);
        let _ = sm.transition(GpuManagerEvent::AllocationComplete, 2000, None);
        let result = sm.transition(GpuManagerEvent::ExecutionComplete, 3000, None);

        assert!(result.is_ok());
        assert_eq!(sm.current_state, GpuManagerState::Idle);
    }

    #[test]
    fn test_valid_transition_executing_to_checkpointing() {
        let mut sm = GpuManagerStateMachine::new(10);
        let _ = sm.transition(GpuManagerEvent::SchedulerDirectiveReceived, 1000, None);
        let _ = sm.transition(GpuManagerEvent::AllocationComplete, 2000, None);
        let result = sm.transition(GpuManagerEvent::CheckpointInitiated, 3000, None);

        assert!(result.is_ok());
        assert_eq!(sm.current_state, GpuManagerState::Checkpointing);
    }

    #[test]
    fn test_valid_transition_checkpointing_to_idle() {
        let mut sm = GpuManagerStateMachine::new(10);
        let _ = sm.transition(GpuManagerEvent::SchedulerDirectiveReceived, 1000, None);
        let _ = sm.transition(GpuManagerEvent::AllocationComplete, 2000, None);
        let _ = sm.transition(GpuManagerEvent::CheckpointInitiated, 3000, None);
        let result = sm.transition(GpuManagerEvent::CheckpointComplete, 4000, None);

        assert!(result.is_ok());
        assert_eq!(sm.current_state, GpuManagerState::Idle);
    }

    #[test]
    fn test_allocation_failure_transitions_to_error() {
        let mut sm = GpuManagerStateMachine::new(10);
        let _ = sm.transition(GpuManagerEvent::SchedulerDirectiveReceived, 1000, None);
        let result = sm.transition(GpuManagerEvent::AllocationFailed(GpuError::VramExhausted), 2000, None);

        assert!(result.is_ok());
        assert_eq!(sm.current_state, GpuManagerState::Error);
    }

    #[test]
    fn test_execution_error_transitions_to_error() {
        let mut sm = GpuManagerStateMachine::new(10);
        let _ = sm.transition(GpuManagerEvent::SchedulerDirectiveReceived, 1000, None);
        let _ = sm.transition(GpuManagerEvent::AllocationComplete, 2000, None);
        let result = sm.transition(GpuManagerEvent::ErrorOccurred(GpuError::IsolationViolation), 3000, None);

        assert!(result.is_ok());
        assert_eq!(sm.current_state, GpuManagerState::Error);
    }

    #[test]
    fn test_error_to_recovering() {
        let mut sm = GpuManagerStateMachine::new(10);
        let _ = sm.transition(GpuManagerEvent::SchedulerDirectiveReceived, 1000, None);
        let _ = sm.transition(GpuManagerEvent::AllocationComplete, 2000, None);
        let _ = sm.transition(GpuManagerEvent::ErrorOccurred(GpuError::DriverError), 3000, None);
        let result = sm.transition(GpuManagerEvent::RecoveryInitiated, 4000, None);

        assert!(result.is_ok());
        assert_eq!(sm.current_state, GpuManagerState::Recovering);
    }

    #[test]
    fn test_error_to_idle_via_clearing() {
        let mut sm = GpuManagerStateMachine::new(10);
        let _ = sm.transition(GpuManagerEvent::SchedulerDirectiveReceived, 1000, None);
        let _ = sm.transition(GpuManagerEvent::AllocationComplete, 2000, None);
        let _ = sm.transition(GpuManagerEvent::ErrorOccurred(GpuError::TpcUnavailable), 3000, None);
        let result = sm.transition(GpuManagerEvent::ErrorCleared, 4000, None);

        assert!(result.is_ok());
        assert_eq!(sm.current_state, GpuManagerState::Idle);
    }

    #[test]
    fn test_invalid_transition_idle_to_executing() {
        let mut sm = GpuManagerStateMachine::new(10);
        let result = sm.transition(GpuManagerEvent::ExecutionComplete, 1000, None);

        assert!(result.is_err());
        assert_eq!(sm.current_state, GpuManagerState::Idle); // State unchanged
    }

    #[test]
    fn test_invalid_transition_allocating_to_checkpointing() {
        let mut sm = GpuManagerStateMachine::new(10);
        let _ = sm.transition(GpuManagerEvent::SchedulerDirectiveReceived, 1000, None);
        let result = sm.transition(GpuManagerEvent::CheckpointInitiated, 2000, None);

        assert!(result.is_err());
        assert_eq!(sm.current_state, GpuManagerState::Allocating); // State unchanged
    }

    #[test]
    fn test_transition_log_recording() {
        let mut sm = GpuManagerStateMachine::new(10);
        let device_id = GpuDeviceID::from_bytes([1u8; 16]);

        sm.transition(GpuManagerEvent::SchedulerDirectiveReceived, 1000, Some(device_id))
            .unwrap();
        sm.transition(GpuManagerEvent::AllocationComplete, 2000, Some(device_id))
            .unwrap();

        assert_eq!(sm.transition_log.len(), 2);
        assert_eq!(sm.transition_log[0].from_state, GpuManagerState::Idle);
        assert_eq!(sm.transition_log[0].to_state, GpuManagerState::Allocating);
        assert_eq!(sm.transition_log[1].from_state, GpuManagerState::Allocating);
        assert_eq!(sm.transition_log[1].to_state, GpuManagerState::Executing);
    }

    #[test]
    fn test_transition_log_max_entries() {
        let mut sm = GpuManagerStateMachine::new(3);

        for i in 0..5 {
            let event = if i % 2 == 0 {
                GpuManagerEvent::SchedulerDirectiveReceived
            } else {
                GpuManagerEvent::AllocationComplete
            };

            // Only toggle between states
            if sm.current_state == GpuManagerState::Idle {
                let _ = sm.transition(GpuManagerEvent::SchedulerDirectiveReceived, i as u64 * 1000, None);
            } else if sm.current_state == GpuManagerState::Allocating {
                let _ = sm.transition(GpuManagerEvent::AllocationComplete, i as u64 * 1000, None);
            } else if sm.current_state == GpuManagerState::Executing {
                let _ = sm.transition(GpuManagerEvent::ExecutionComplete, i as u64 * 1000, None);
            }
        }

        // Log should only contain last 3 entries
        assert!(sm.transition_log.len() <= 3);
    }

    #[test]
    fn test_is_in_error() {
        let mut sm = GpuManagerStateMachine::new(10);
        assert!(!sm.is_in_error());

        let _ = sm.transition(GpuManagerEvent::SchedulerDirectiveReceived, 1000, None);
        let _ = sm.transition(GpuManagerEvent::AllocationComplete, 2000, None);
        let _ = sm.transition(GpuManagerEvent::ErrorOccurred(GpuError::DriverError), 3000, None);

        assert!(sm.is_in_error());
    }

    #[test]
    fn test_last_transition() {
        let mut sm = GpuManagerStateMachine::new(10);
        assert!(sm.last_transition().is_none());

        sm.transition(GpuManagerEvent::SchedulerDirectiveReceived, 1000, None)
            .unwrap();

        let last = sm.last_transition();
        assert!(last.is_some());
        assert_eq!(last.unwrap().from_state, GpuManagerState::Idle);
        assert_eq!(last.unwrap().to_state, GpuManagerState::Allocating);
    }

    #[test]
    fn test_state_machine_display() {
        let sm = GpuManagerStateMachine::new(10);
        let display_str = format!("{}", sm);
        assert!(display_str.contains("GpuManagerStateMachine"));
        assert!(display_str.contains("Idle"));
    }

    #[test]
    fn test_state_display() {
        assert_eq!(format!("{}", GpuManagerState::Idle), "Idle");
        assert_eq!(format!("{}", GpuManagerState::Executing), "Executing");
        assert_eq!(format!("{}", GpuManagerState::Error), "Error");
    }

    #[test]
    fn test_event_display() {
        assert_eq!(format!("{}", GpuManagerEvent::SchedulerDirectiveReceived), "SchedulerDirectiveReceived");
        assert_eq!(format!("{}", GpuManagerEvent::ExecutionComplete), "ExecutionComplete");
    }
}
