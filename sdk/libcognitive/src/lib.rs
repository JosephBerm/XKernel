// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! # Cognitive Library (libcognitive)
//!
//! The libcognitive crate provides high-level abstractions for reasoning task lifecycle management,
//! syscall interfacing, and reasoning pattern execution within the Cognitive Substrate.
//!
//! ## Architecture
//!
//! This crate defines:
//! - **CognitiveRuntime**: Main runtime struct for managing Cognitive Tasks (CTs)
//! - **CT Lifecycle**: Task spawning, yielding, checkpointing, and resumption
//! - **Reasoning Patterns**: chain_of_thought, react_loop, tree_of_thought, plan_and_execute, self_refine
//! - **Error Handling**: LibcogError for detailed error reporting
//!
//! ## No Std
//!
//! This crate is `#![no_std]` to support kernel and embedded environments.

#![no_std]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

extern crate alloc;

use alloc::vec::Vec;
use alloc::string::String;
use alloc::collections::BTreeMap;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use ulid::Ulid;

/// Cognitive Task Identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Ord, PartialOrd)]
pub struct CTID(Ulid);

impl CTID {
    /// Generate a new Cognitive Task ID
    pub fn new() -> Self {
        CTID(Ulid::new())
    }
}

impl Default for CTID {
    fn default() -> Self {
        Self::new()
    }
}

/// Checkpoint identifier for task state snapshots
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Ord, PartialOrd)]
pub struct CheckpointID(Ulid);

impl CheckpointID {
    /// Generate a new Checkpoint ID
    pub fn new() -> Self {
        CheckpointID(Ulid::new())
    }
}

impl Default for CheckpointID {
    fn default() -> Self {
        Self::new()
    }
}

/// Result type for libcognitive operations
pub type LibcogResult<T> = Result<T, LibcogError>;

/// Error types for libcognitive operations
#[derive(Debug, Clone, Error, Serialize, Deserialize)]
pub enum LibcogError {
    /// Task not found
    #[error("Task not found: {0}")]
    TaskNotFound(String),

    /// Invalid task configuration
    #[error("Invalid task configuration: {0}")]
    InvalidConfig(String),

    /// Task execution failed
    #[error("Task execution failed: {0}")]
    ExecutionFailed(String),

    /// Checkpoint operation failed
    #[error("Checkpoint operation failed: {0}")]
    CheckpointFailed(String),

    /// Reasoning pattern error
    #[error("Reasoning pattern error: {0}")]
    PatternError(String),

    /// Syscall interface error
    #[error("Syscall interface error: {0}")]
    SyscallError(String),

    /// Memory allocation failed
    #[error("Memory allocation failed: {0}")]
    MemoryError(String),

    /// Invalid state transition
    #[error("Invalid state transition: {0}")]
    InvalidStateTransition(String),
}

/// Cognitive Task configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskConfig {
    /// Task name
    pub name: String,
    /// Task priority (0-255)
    pub priority: u8,
    /// Timeout in milliseconds
    pub timeout_ms: u64,
    /// Enable checkpointing
    pub enable_checkpoint: bool,
    /// Custom metadata
    pub metadata: BTreeMap<String, String>,
}

impl Default for TaskConfig {
    fn default() -> Self {
        TaskConfig {
            name: "default_task".to_string(),
            priority: 128,
            timeout_ms: 30000,
            enable_checkpoint: true,
            metadata: BTreeMap::new(),
        }
    }
}

/// Task state enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskState {
    /// Task created and ready to run
    Ready,
    /// Task currently executing
    Running,
    /// Task suspended/yielded
    Suspended,
    /// Task waiting for checkpoint restore
    WaitingRestore,
    /// Task completed successfully
    Completed,
    /// Task failed
    Failed,
}

/// Result from a reasoning pattern execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningResult {
    /// Pattern name
    pub pattern_name: String,
    /// Output text
    pub output: String,
    /// Intermediate steps (if applicable)
    pub steps: Vec<String>,
    /// Reasoning metadata
    pub metadata: BTreeMap<String, String>,
}

/// Cognitive Runtime - main entry point for CT lifecycle management
#[derive(Debug)]
pub struct CognitiveRuntime {
    /// Active tasks indexed by CTID
    active_tasks: BTreeMap<CTID, (TaskConfig, TaskState)>,
    /// Syscall interface handle (opaque)
    syscall_interface: u64,
    /// Runtime configuration
    config: RuntimeConfig,
    /// Checkpoint storage (CTID -> CheckpointID -> checkpoint data)
    checkpoints: BTreeMap<CTID, BTreeMap<CheckpointID, Vec<u8>>>,
}

/// Runtime configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    /// Maximum concurrent tasks
    pub max_tasks: usize,
    /// Enable global checkpointing
    pub enable_global_checkpoint: bool,
    /// Default task timeout in milliseconds
    pub default_timeout_ms: u64,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        RuntimeConfig {
            max_tasks: 1024,
            enable_global_checkpoint: true,
            default_timeout_ms: 30000,
        }
    }
}

impl CognitiveRuntime {
    /// Create a new CognitiveRuntime with default configuration
    pub fn new() -> Self {
        CognitiveRuntime {
            active_tasks: BTreeMap::new(),
            syscall_interface: 0,
            config: RuntimeConfig::default(),
            checkpoints: BTreeMap::new(),
        }
    }

    /// Create a new CognitiveRuntime with custom configuration
    pub fn with_config(config: RuntimeConfig) -> Self {
        CognitiveRuntime {
            active_tasks: BTreeMap::new(),
            syscall_interface: 0,
            config,
            checkpoints: BTreeMap::new(),
        }
    }

    /// Initialize syscall interface
    pub fn init_syscall_interface(&mut self, interface_handle: u64) -> LibcogResult<()> {
        if interface_handle == 0 {
            return Err(LibcogError::SyscallError("Invalid interface handle".to_string()));
        }
        self.syscall_interface = interface_handle;
        Ok(())
    }

    /// Spawn a new Cognitive Task with the given configuration
    ///
    /// # Arguments
    /// * `config` - Task configuration parameters
    ///
    /// # Returns
    /// * `Ok(CTID)` - The ID of the spawned task
    /// * `Err(LibcogError)` - If task limit exceeded or config invalid
    pub fn spawn_task(&mut self, config: TaskConfig) -> LibcogResult<CTID> {
        if self.active_tasks.len() >= self.config.max_tasks {
            return Err(LibcogError::InvalidConfig(
                format!("Task limit {} reached", self.config.max_tasks)
            ));
        }

        if config.name.is_empty() {
            return Err(LibcogError::InvalidConfig("Task name cannot be empty".to_string()));
        }

        let task_id = CTID::new();
        self.active_tasks.insert(task_id, (config, TaskState::Ready));
        Ok(task_id)
    }

    /// Yield the current task, suspending execution
    ///
    /// # Arguments
    /// * `task_id` - The task to yield
    ///
    /// # Returns
    /// * `Ok(())` - If yield successful
    /// * `Err(LibcogError)` - If task not found or invalid state
    pub fn yield_task(&mut self, task_id: CTID) -> LibcogResult<()> {
        let task = self.active_tasks.get_mut(&task_id)
            .ok_or_else(|| LibcogError::TaskNotFound(format!("{:?}", task_id)))?;

        match task.1 {
            TaskState::Running => {
                task.1 = TaskState::Suspended;
                Ok(())
            }
            _ => Err(LibcogError::InvalidStateTransition(
                format!("Cannot yield task in state {:?}", task.1)
            ))
        }
    }

    /// Create a checkpoint of a task's state
    ///
    /// # Arguments
    /// * `task_id` - The task to checkpoint
    /// * `state_data` - Serialized task state
    ///
    /// # Returns
    /// * `Ok(CheckpointID)` - The ID of the created checkpoint
    /// * `Err(LibcogError)` - If checkpoint failed
    pub fn checkpoint(&mut self, task_id: CTID, state_data: Vec<u8>) -> LibcogResult<CheckpointID> {
        let task = self.active_tasks.get(&task_id)
            .ok_or_else(|| LibcogError::TaskNotFound(format!("{:?}", task_id)))?;

        if !task.0.enable_checkpoint {
            return Err(LibcogError::CheckpointFailed(
                "Checkpointing disabled for this task".to_string()
            ));
        }

        let checkpoint_id = CheckpointID::new();
        self.checkpoints
            .entry(task_id)
            .or_insert_with(BTreeMap::new)
            .insert(checkpoint_id, state_data);

        Ok(checkpoint_id)
    }

    /// Resume a task from a checkpoint
    ///
    /// # Arguments
    /// * `task_id` - The task to resume
    /// * `checkpoint_id` - The checkpoint to restore from
    ///
    /// # Returns
    /// * `Ok(Vec<u8>)` - The checkpoint state data
    /// * `Err(LibcogError)` - If resume failed
    pub fn resume(&mut self, task_id: CTID, checkpoint_id: CheckpointID) -> LibcogResult<Vec<u8>> {
        let task = self.active_tasks.get_mut(&task_id)
            .ok_or_else(|| LibcogError::TaskNotFound(format!("{:?}", task_id)))?;

        if task.1 != TaskState::WaitingRestore && task.1 != TaskState::Suspended {
            return Err(LibcogError::InvalidStateTransition(
                format!("Cannot resume task in state {:?}", task.1)
            ));
        }

        let checkpoints = self.checkpoints.get(&task_id)
            .ok_or_else(|| LibcogError::CheckpointFailed(
                format!("No checkpoints for task {:?}", task_id)
            ))?;

        let state_data = checkpoints.get(&checkpoint_id)
            .cloned()
            .ok_or_else(|| LibcogError::CheckpointFailed(
                format!("Checkpoint not found: {:?}", checkpoint_id)
            ))?;

        task.1 = TaskState::Running;
        Ok(state_data)
    }

    /// Get task state
    pub fn get_task_state(&self, task_id: CTID) -> LibcogResult<TaskState> {
        self.active_tasks.get(&task_id)
            .map(|(_, state)| *state)
            .ok_or_else(|| LibcogError::TaskNotFound(format!("{:?}", task_id)))
    }

    /// List all active task IDs
    pub fn list_active_tasks(&self) -> Vec<CTID> {
        self.active_tasks.keys().copied().collect()
    }

    // ========== REASONING PATTERNS ==========

    /// Chain of Thought pattern: decompose problem into sequential steps
    pub fn chain_of_thought(&self, problem: &str) -> LibcogResult<ReasoningResult> {
        if problem.is_empty() {
            return Err(LibcogError::PatternError("Problem statement cannot be empty".to_string()));
        }

        let mut steps = Vec::new();
        steps.push(format!("Step 1: Analyze problem: {}", problem));
        steps.push("Step 2: Identify key components".to_string());
        steps.push("Step 3: Decompose into sub-problems".to_string());
        steps.push("Step 4: Solve each component".to_string());
        steps.push("Step 5: Verify solution consistency".to_string());

        Ok(ReasoningResult {
            pattern_name: "chain_of_thought".to_string(),
            output: format!("Decomposed problem: {}", problem),
            steps,
            metadata: BTreeMap::new(),
        })
    }

    /// ReAct pattern: Reason + Act loop
    pub fn react_loop(&self, task: &str) -> LibcogResult<ReasoningResult> {
        if task.is_empty() {
            return Err(LibcogError::PatternError("Task cannot be empty".to_string()));
        }

        let mut steps = Vec::new();
        steps.push(format!("Thought: Analyzing task: {}", task));
        steps.push("Action: Execute reasoning step 1".to_string());
        steps.push("Observation: Check results".to_string());
        steps.push("Thought: Refine approach".to_string());
        steps.push("Action: Execute refined step".to_string());

        Ok(ReasoningResult {
            pattern_name: "react_loop".to_string(),
            output: format!("ReAct execution for task: {}", task),
            steps,
            metadata: BTreeMap::new(),
        })
    }

    /// Tree of Thought pattern: explore multiple reasoning paths
    pub fn tree_of_thought(&self, problem: &str) -> LibcogResult<ReasoningResult> {
        if problem.is_empty() {
            return Err(LibcogError::PatternError("Problem statement cannot be empty".to_string()));
        }

        let mut steps = Vec::new();
        steps.push(format!("Root: {}", problem));
        steps.push("Branch 1: Approach A - analyze path".to_string());
        steps.push("Branch 2: Approach B - analyze path".to_string());
        steps.push("Branch 3: Approach C - analyze path".to_string());
        steps.push("Evaluate all branches and select best path".to_string());

        Ok(ReasoningResult {
            pattern_name: "tree_of_thought".to_string(),
            output: format!("Tree exploration for: {}", problem),
            steps,
            metadata: BTreeMap::new(),
        })
    }

    /// Plan and Execute pattern: create plan then execute
    pub fn plan_and_execute(&self, goal: &str) -> LibcogResult<ReasoningResult> {
        if goal.is_empty() {
            return Err(LibcogError::PatternError("Goal cannot be empty".to_string()));
        }

        let mut steps = Vec::new();
        steps.push(format!("Goal: {}", goal));
        steps.push("Plan Phase: Create execution plan".to_string());
        steps.push("Plan Phase: Identify dependencies".to_string());
        steps.push("Execute Phase: Step 1".to_string());
        steps.push("Execute Phase: Step 2".to_string());
        steps.push("Verify Phase: Check goal completion".to_string());

        Ok(ReasoningResult {
            pattern_name: "plan_and_execute".to_string(),
            output: format!("Planned execution for goal: {}", goal),
            steps,
            metadata: BTreeMap::new(),
        })
    }

    /// Self-Refine pattern: iterative refinement
    pub fn self_refine(&self, initial_solution: &str) -> LibcogResult<ReasoningResult> {
        if initial_solution.is_empty() {
            return Err(LibcogError::PatternError("Initial solution cannot be empty".to_string()));
        }

        let mut steps = Vec::new();
        steps.push(format!("Initial Solution: {}", initial_solution));
        steps.push("Iteration 1: Identify improvements".to_string());
        steps.push("Iteration 1: Apply refinements".to_string());
        steps.push("Iteration 2: Further analysis".to_string());
        steps.push("Iteration 2: Enhanced refinement".to_string());
        steps.push("Convergence: Solution optimized".to_string());

        Ok(ReasoningResult {
            pattern_name: "self_refine".to_string(),
            output: format!("Refined solution from: {}", initial_solution),
            steps,
            metadata: BTreeMap::new(),
        })
    }
}

impl Default for CognitiveRuntime {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::format;
use alloc::string::ToString;
use alloc::vec;

    #[test]
    fn test_ctid_generation() {
        let id1 = CTID::new();
        let id2 = CTID::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_checkpoint_id_generation() {
        let cp1 = CheckpointID::new();
        let cp2 = CheckpointID::new();
        assert_ne!(cp1, cp2);
    }

    #[test]
    fn test_runtime_creation() {
        let rt = CognitiveRuntime::new();
        assert_eq!(rt.list_active_tasks().len(), 0);
    }

    #[test]
    fn test_spawn_task() {
        let mut rt = CognitiveRuntime::new();
        let config = TaskConfig::default();
        let task_id = rt.spawn_task(config).unwrap();
        assert_eq!(rt.list_active_tasks().len(), 1);
        assert_eq!(rt.get_task_state(task_id).unwrap(), TaskState::Ready);
    }

    #[test]
    fn test_spawn_task_empty_name() {
        let mut rt = CognitiveRuntime::new();
        let config = TaskConfig {
            name: String::new(),
            ..Default::default()
        };
        assert!(rt.spawn_task(config).is_err());
    }

    #[test]
    fn test_yield_task() {
        let mut rt = CognitiveRuntime::new();
        let config = TaskConfig::default();
        let task_id = rt.spawn_task(config).unwrap();

        // Change state to Running first
        rt.active_tasks.get_mut(&task_id).unwrap().1 = TaskState::Running;

        rt.yield_task(task_id).unwrap();
        assert_eq!(rt.get_task_state(task_id).unwrap(), TaskState::Suspended);
    }

    #[test]
    fn test_yield_non_running_task() {
        let mut rt = CognitiveRuntime::new();
        let config = TaskConfig::default();
        let task_id = rt.spawn_task(config).unwrap();
        assert!(rt.yield_task(task_id).is_err());
    }

    #[test]
    fn test_checkpoint_and_resume() {
        let mut rt = CognitiveRuntime::new();
        let config = TaskConfig::default();
        let task_id = rt.spawn_task(config).unwrap();

        let state_data = alloc::vec![1, 2, 3, 4];
        let checkpoint_id = rt.checkpoint(task_id, state_data.clone()).unwrap();

        rt.active_tasks.get_mut(&task_id).unwrap().1 = TaskState::Suspended;
        let restored = rt.resume(task_id, checkpoint_id).unwrap();
        assert_eq!(restored, state_data);
    }

    #[test]
    fn test_checkpoint_disabled() {
        let mut rt = CognitiveRuntime::new();
        let config = TaskConfig {
            enable_checkpoint: false,
            ..Default::default()
        };
        let task_id = rt.spawn_task(config).unwrap();
        assert!(rt.checkpoint(task_id, alloc::vec![]).is_err());
    }

    #[test]
    fn test_chain_of_thought() {
        let rt = CognitiveRuntime::new();
        let result = rt.chain_of_thought("solve problem X").unwrap();
        assert_eq!(result.pattern_name, "chain_of_thought");
        assert!(!result.steps.is_empty());
    }

    #[test]
    fn test_chain_of_thought_empty() {
        let rt = CognitiveRuntime::new();
        assert!(rt.chain_of_thought("").is_err());
    }

    #[test]
    fn test_react_loop() {
        let rt = CognitiveRuntime::new();
        let result = rt.react_loop("test task").unwrap();
        assert_eq!(result.pattern_name, "react_loop");
        assert!(!result.steps.is_empty());
    }

    #[test]
    fn test_tree_of_thought() {
        let rt = CognitiveRuntime::new();
        let result = rt.tree_of_thought("complex problem").unwrap();
        assert_eq!(result.pattern_name, "tree_of_thought");
        assert!(!result.steps.is_empty());
    }

    #[test]
    fn test_plan_and_execute() {
        let rt = CognitiveRuntime::new();
        let result = rt.plan_and_execute("achieve goal").unwrap();
        assert_eq!(result.pattern_name, "plan_and_execute");
        assert!(!result.steps.is_empty());
    }

    #[test]
    fn test_self_refine() {
        let rt = CognitiveRuntime::new();
        let result = rt.self_refine("initial approach").unwrap();
        assert_eq!(result.pattern_name, "self_refine");
        assert!(!result.steps.is_empty());
    }

    #[test]
    fn test_runtime_with_custom_config() {
        let config = RuntimeConfig {
            max_tasks: 512,
            enable_global_checkpoint: false,
            default_timeout_ms: 60000,
        };
        let rt = CognitiveRuntime::with_config(config);
        assert_eq!(rt.config.max_tasks, 512);
    }

    #[test]
    fn test_max_tasks_limit() {
        let config = RuntimeConfig {
            max_tasks: 2,
            ..Default::default()
        };
        let mut rt = CognitiveRuntime::with_config(config);

        let _id1 = rt.spawn_task(TaskConfig::default()).unwrap();
        let _id2 = rt.spawn_task(TaskConfig::default()).unwrap();
        assert!(rt.spawn_task(TaskConfig::default()).is_err());
    }

    #[test]
    fn test_nonexistent_task() {
        let rt = CognitiveRuntime::new();
        let task_id = CTID::new();
        assert!(rt.get_task_state(task_id).is_err());
    }
}
