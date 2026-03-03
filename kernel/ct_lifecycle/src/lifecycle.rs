// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 XKernal Contributors
//! Task lifecycle state machine for cognitive task phase transitions

use crate::types::TaskPhase;
use alloc::vec::Vec;
use thiserror::Error;

/// Errors that can occur during task lifecycle transitions
#[derive(Debug, Clone, Error)]
pub enum LifecycleError {
    /// Invalid state transition attempted
    #[error("invalid transition from {from:?} to {to:?}")]
    InvalidTransition { from: TaskPhase, to: TaskPhase },
    /// Task not found
    #[error("task {0} not found")]
    TaskNotFound(u64),
    /// State machine is in an inconsistent state
    #[error("state machine inconsistent: {0}")]
    InconsistentState(alloc::string::String),
}

pub type Result<T> = core::result::Result<T, LifecycleError>;

/// Represents a transition between task phases
#[derive(Debug, Clone, Copy)]
pub struct TaskTransition {
    /// Source phase
    pub from: TaskPhase,
    /// Destination phase
    pub to: TaskPhase,
}

impl TaskTransition {
    /// Create a new transition
    pub fn new(from: TaskPhase, to: TaskPhase) -> Self {
        Self { from, to }
    }

    /// Check if this transition is valid according to CT phase rules
    pub fn is_valid(&self) -> bool {
        use TaskPhase::*;
        matches!(
            (self.from, self.to),
            (Init, Ready)
                | (Ready, Running)
                | (Running, Waiting)
                | (Running, Checkpointed)
                | (Running, Completed)
                | (Waiting, Ready)
                | (Checkpointed, Ready)
                | (Completed, _)
                | (Failed, _)
        )
    }
}

/// Current state type for the state machine
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    /// Task has been initialized
    Init,
    /// Task is ready to run
    Ready,
    /// Task is currently executing
    Running,
    /// Task is waiting for a resource or event
    Waiting,
    /// Task has been checkpointed
    Checkpointed,
    /// Task has completed successfully
    Completed,
    /// Task has encountered a failure
    Failed,
}

impl From<TaskPhase> for TaskState {
    fn from(phase: TaskPhase) -> Self {
        use TaskPhase::*;
        match phase {
            Init => TaskState::Init,
            Ready => TaskState::Ready,
            Running => TaskState::Running,
            Waiting => TaskState::Waiting,
            Checkpointed => TaskState::Checkpointed,
            Completed => TaskState::Completed,
            Failed => TaskState::Failed,
        }
    }
}

impl From<TaskState> for TaskPhase {
    fn from(state: TaskState) -> Self {
        match state {
            TaskState::Init => TaskPhase::Init,
            TaskState::Ready => TaskPhase::Ready,
            TaskState::Running => TaskPhase::Running,
            TaskState::Waiting => TaskPhase::Waiting,
            TaskState::Checkpointed => TaskPhase::Checkpointed,
            TaskState::Completed => TaskPhase::Completed,
            TaskState::Failed => TaskPhase::Failed,
        }
    }
}

/// Task lifecycle state machine
#[derive(Debug)]
pub struct TaskStateMachine {
    task_id: u64,
    current_state: TaskState,
    transition_history: Vec<TaskTransition>,
}

impl TaskStateMachine {
    /// Create a new task state machine
    pub fn new(task_id: u64) -> Self {
        Self {
            task_id,
            current_state: TaskState::Init,
            transition_history: Vec::new(),
        }
    }

    /// Get the current state
    pub fn current_state(&self) -> TaskState {
        self.current_state
    }

    /// Attempt to transition to a new state
    pub fn transition(&mut self, new_state: TaskState) -> Result<()> {
        let transition = TaskTransition {
            from: self.current_state.into(),
            to: match new_state {
                TaskState::Init => TaskPhase::Init,
                TaskState::Ready => TaskPhase::Ready,
                TaskState::Running => TaskPhase::Running,
                TaskState::Waiting => TaskPhase::Waiting,
                TaskState::Checkpointed => TaskPhase::Checkpointed,
                TaskState::Completed => TaskPhase::Completed,
                TaskState::Failed => TaskPhase::Failed,
            },
        };

        if !transition.is_valid() {
            return Err(LifecycleError::InvalidTransition {
                from: transition.from,
                to: transition.to,
            });
        }

        self.current_state = new_state;
        self.transition_history.push(transition);
        Ok(())
    }

    /// Get the complete transition history
    pub fn history(&self) -> &[TaskTransition] {
        &self.transition_history
    }

    /// Check if the task is in a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(self.current_state, TaskState::Completed | TaskState::Failed)
    }

    /// Get the task ID
    pub fn task_id(&self) -> u64 {
        self.task_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_transition() {
        let mut sm = TaskStateMachine::new(1);
        assert_eq!(sm.current_state(), TaskState::Init);
        assert!(sm.transition(TaskState::Ready).is_ok());
        assert_eq!(sm.current_state(), TaskState::Ready);
    }

    #[test]
    fn test_invalid_transition() {
        let mut sm = TaskStateMachine::new(1);
        assert!(sm.transition(TaskState::Completed).is_err());
    }

    #[test]
    fn test_history_tracking() {
        let mut sm = TaskStateMachine::new(1);
        let _ = sm.transition(TaskState::Ready);
        let _ = sm.transition(TaskState::Running);
        assert_eq!(sm.history().len(), 2);
    }
}
