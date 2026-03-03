// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 XKernal Contributors
//! Core domain types for cognitive task management

use core::cmp::Ordering;
use serde::{Deserialize, Serialize};

/// Unique identifier for a cognitive task
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskId(pub u64);

impl TaskId {
    /// Create a new task ID
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get the raw ID value
    pub fn inner(&self) -> u64 {
        self.0
    }
}

/// Phase of a cognitive task in its lifecycle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskPhase {
    /// Task initialized but not yet ready
    Init,
    /// Task ready for execution
    Ready,
    /// Task is currently executing
    Running,
    /// Task is waiting for a resource or event
    Waiting,
    /// Task has been checkpointed for recovery
    Checkpointed,
    /// Task completed successfully
    Completed,
    /// Task encountered a failure
    Failed,
}

impl TaskPhase {
    /// Check if this phase is a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(self, TaskPhase::Completed | TaskPhase::Failed)
    }

    /// Get a human-readable description
    pub fn description(&self) -> &'static str {
        match self {
            TaskPhase::Init => "Initialized",
            TaskPhase::Ready => "Ready for execution",
            TaskPhase::Running => "Currently running",
            TaskPhase::Waiting => "Waiting for resource",
            TaskPhase::Checkpointed => "Checkpointed",
            TaskPhase::Completed => "Successfully completed",
            TaskPhase::Failed => "Failed",
        }
    }
}

/// Task priority combining multiple dimensions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Priority {
    /// Chain criticality level (0-255)
    pub chain_criticality: u8,
    /// Resource efficiency score (0-255)
    pub resource_efficiency: u8,
    /// Deadline pressure (0-255)
    pub deadline_pressure: u8,
    /// Capability cost (0-255)
    pub capability_cost: u8,
}

impl Priority {
    /// Create a new priority with individual components
    pub fn new(
        chain_criticality: u8,
        resource_efficiency: u8,
        deadline_pressure: u8,
        capability_cost: u8,
    ) -> Self {
        Self {
            chain_criticality,
            resource_efficiency,
            deadline_pressure,
            capability_cost,
        }
    }

    /// Get the overall priority score (weighted sum)
    pub fn score(&self) -> u32 {
        (self.chain_criticality as u32 * 4)
            + (self.deadline_pressure as u32 * 3)
            + (self.resource_efficiency as u32 * 2)
            + (self.capability_cost as u32)
    }

    /// Create a high-priority task
    pub fn high() -> Self {
        Self::new(200, 200, 200, 50)
    }

    /// Create a normal-priority task
    pub fn normal() -> Self {
        Self::new(100, 100, 100, 100)
    }

    /// Create a low-priority task
    pub fn low() -> Self {
        Self::new(50, 50, 50, 150)
    }
}

impl Ord for Priority {
    fn cmp(&self, other: &Self) -> Ordering {
        self.score().cmp(&other.score())
    }
}

impl PartialOrd for Priority {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Cognitive Task - core entity in the XKernal microkernel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveTask {
    /// Unique task identifier
    pub id: TaskId,
    /// Current phase in the task lifecycle
    pub phase: TaskPhase,
    /// Task priority
    pub priority: Priority,
    /// Capability set (bitfield of granted capabilities)
    pub capabilities: u64,
    /// Parent task ID (for task hierarchy)
    pub parent: Option<TaskId>,
    /// Memory footprint in bytes
    pub memory_bytes: usize,
    /// Number of child tasks
    pub child_count: usize,
    /// Timestamps: (created, started, last_update)
    pub timestamps: (u64, u64, u64),
}

impl CognitiveTask {
    /// Create a new cognitive task
    pub fn new(
        id: TaskId,
        priority: Priority,
        capabilities: u64,
        parent: Option<TaskId>,
    ) -> Self {
        Self {
            id,
            phase: TaskPhase::Init,
            priority,
            capabilities,
            parent,
            memory_bytes: 0,
            child_count: 0,
            timestamps: (0, 0, 0),
        }
    }

    /// Check if this task is in a terminal phase
    pub fn is_complete(&self) -> bool {
        self.phase.is_terminal()
    }

    /// Check if this task has a specific capability
    pub fn has_capability(&self, cap_mask: u64) -> bool {
        (self.capabilities & cap_mask) != 0
    }

    /// Grant a capability to this task
    pub fn grant_capability(&mut self, cap_mask: u64) {
        self.capabilities |= cap_mask;
    }

    /// Revoke a capability from this task
    pub fn revoke_capability(&mut self, cap_mask: u64) {
        self.capabilities &= !cap_mask;
    }

    /// Transition to a new phase
    pub fn transition_phase(&mut self, new_phase: TaskPhase) {
        self.phase = new_phase;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_id_equality() {
        let id1 = TaskId::new(42);
        let id2 = TaskId::new(42);
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_phase_terminal() {
        assert!(TaskPhase::Completed.is_terminal());
        assert!(TaskPhase::Failed.is_terminal());
        assert!(!TaskPhase::Running.is_terminal());
    }

    #[test]
    fn test_priority_comparison() {
        let high = Priority::high();
        let low = Priority::low();
        assert!(high > low);
    }

    #[test]
    fn test_capability_operations() {
        let mut task = CognitiveTask::new(TaskId::new(1), Priority::normal(), 0, None);
        assert!(!task.has_capability(0x01));
        task.grant_capability(0x01);
        assert!(task.has_capability(0x01));
        task.revoke_capability(0x01);
        assert!(!task.has_capability(0x01));
    }
}
