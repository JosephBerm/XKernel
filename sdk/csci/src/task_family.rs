// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! # CSCI Task Family Syscalls
//!
//! Task family syscalls manage the lifecycle of cognitive tasks:
//! - **ct_spawn**: Create a new task
//! - **ct_yield**: Voluntarily yield task execution
//! - **ct_checkpoint**: Create a state checkpoint
//! - **ct_resume**: Resume task from checkpoint
//!
//! # Engineering Plan Reference
//! Section 7: Task Family Specification.

use crate::error_codes::CsciErrorCode;
use crate::syscall::{ParamType, ReturnType, SyscallDefinition, SyscallFamily, SyscallParam};
use crate::types::{
    AgentID, CapabilitySet, CheckpointID, CheckpointType, CTConfig, ResourceQuota, CTID,
    YieldHint,
};

/// Task family syscall numbers.
pub mod number {
    /// ct_spawn syscall number within Task family.
    pub const CT_SPAWN: u8 = 0;
    /// ct_yield syscall number within Task family.
    pub const CT_YIELD: u8 = 1;
    /// ct_checkpoint syscall number within Task family.
    pub const CT_CHECKPOINT: u8 = 2;
    /// ct_resume syscall number within Task family.
    pub const CT_RESUME: u8 = 3;
}

/// Get the definition of the ct_spawn syscall.
///
/// **ct_spawn**: Create a new cognitive task.
///
/// Creates a new cognitive task with the specified configuration, capabilities, and budget.
/// The task is created in the Spawn phase and is ready to begin execution.
///
/// # Parameters
/// - `parent_agent`: (AgentID) Agent creating this task
/// - `config`: (CTConfig) Task configuration (name, timeout, priority)
/// - `capabilities`: (CapabilitySet) Capabilities granted to the task
/// - `budget`: (ResourceQuota) Resource budget constraints
///
/// # Returns
/// - Success: CTID of the newly created task
/// - Error: CS_EPERM (no Task capability), CS_ENOMEM (insufficient memory),
///          CS_EINVAL (invalid configuration), CS_EBUDGET (parent budget exceeded),
///          CS_ECYCLE (dependency cycle)
///
/// # Preconditions
/// - Caller must have Task family capability (CAP_TASK_FAMILY)
/// - `parent_agent` must be a valid, existing agent
/// - `budget` must not exceed parent agent's remaining quota
/// - Task configuration must not create circular dependencies
///
/// # Postconditions
/// - CT is created in Spawn phase
/// - CT has immutable CTID
/// - CT owns the specified resource budget
/// - Parent agent's quota is reduced by CT's budget
///
/// # Engineering Plan Reference
/// Section 7.1: ct_spawn specification.
pub fn ct_spawn_definition() -> SyscallDefinition {
    SyscallDefinition::new(
        "ct_spawn",
        SyscallFamily::Task,
        number::CT_SPAWN,
        ReturnType::Identifier,
        CapabilitySet::CAP_TASK_FAMILY,
        "Create a new cognitive task with configuration and budget",
    )
    .with_param(SyscallParam::new(
        "parent_agent",
        ParamType::Identifier,
        "Agent creating this task",
        false,
    ))
    .with_param(SyscallParam::new(
        "config",
        ParamType::Config,
        "Task configuration (name, timeout, priority)",
        false,
    ))
    .with_param(SyscallParam::new(
        "capabilities",
        ParamType::Capability,
        "Capabilities granted to the task",
        false,
    ))
    .with_param(SyscallParam::new(
        "budget",
        ParamType::Config,
        "Resource quota (memory, compute, children)",
        false,
    ))
    .with_error(CsciErrorCode::CsSuccess)
    .with_error(CsciErrorCode::CsEperm)
    .with_error(CsciErrorCode::CsEnomem)
    .with_error(CsciErrorCode::CsEinval)
    .with_error(CsciErrorCode::CsEbudget)
    .with_error(CsciErrorCode::CsEcycle)
    .with_preconditions(
        "Caller has Task family capability; parent_agent is valid; budget within parent quota; no circular dependencies",
    )
    .with_postconditions("CT created in Spawn phase; CT assigned immutable CTID; parent budget reduced")
}

/// Get the definition of the ct_yield syscall.
///
/// **ct_yield**: Voluntarily yield task execution.
///
/// Allows a task to voluntarily suspend execution and return control to the scheduler.
/// The task provides a hint about why it's yielding to help the scheduler make
/// scheduling decisions.
///
/// # Parameters
/// - `ct_id`: (CTID) Task ID to yield
/// - `yield_hint`: (YieldHint) Hint about why yielding (more thinking, waiting for event, etc.)
///
/// # Returns
/// - Success: Unit (operation successful)
/// - Error: CS_EPERM (task not owned by caller), CS_ENOENT (task not found),
///          CS_EBUSY (task not in reason/act/reflect phase)
///
/// # Preconditions
/// - Caller must own or have capability to yield this task
/// - CT must exist
/// - CT must be in reason, act, or reflect phase (not in Spawn, Complete, or Failed)
///
/// # Postconditions
/// - CT execution is suspended
/// - CT retains its state
/// - Scheduler will use yield_hint for re-scheduling
///
/// # Engineering Plan Reference
/// Section 7.2: ct_yield specification.
pub fn ct_yield_definition() -> SyscallDefinition {
    SyscallDefinition::new(
        "ct_yield",
        SyscallFamily::Task,
        number::CT_YIELD,
        ReturnType::Unit,
        CapabilitySet::CAP_TASK_FAMILY,
        "Voluntarily yield task execution",
    )
    .with_param(SyscallParam::new(
        "ct_id",
        ParamType::Identifier,
        "Task ID to yield",
        false,
    ))
    .with_param(SyscallParam::new(
        "yield_hint",
        ParamType::Enum,
        "Hint about reason for yielding",
        false,
    ))
    .with_error(CsciErrorCode::CsSuccess)
    .with_error(CsciErrorCode::CsEperm)
    .with_error(CsciErrorCode::CsEnoent)
    .with_error(CsciErrorCode::CsEbusy)
    .with_preconditions("Caller owns or can yield this CT; CT exists; CT in reason/act/reflect phase")
    .with_postconditions("CT execution suspended; CT state retained; scheduler uses hint for re-scheduling")
}

/// Get the definition of the ct_checkpoint syscall.
///
/// **ct_checkpoint**: Create a checkpoint of task state.
///
/// Creates a point-in-time snapshot of a task's state, allowing later resumption
/// from this point. Different checkpoint types capture different levels of state.
///
/// # Parameters
/// - `ct_id`: (CTID) Task ID to checkpoint
/// - `checkpoint_type`: (CheckpointType) What state to preserve (Full, ReasoningOnly, MemoryOnly)
///
/// # Returns
/// - Success: CheckpointID of the created checkpoint
/// - Error: CS_EPERM (insufficient capability), CS_ENOENT (task not found),
///          CS_ENOMEM (insufficient memory for checkpoint), CS_EBUSY (task in Failed state)
///
/// # Preconditions
/// - Caller must have Task family capability
/// - CT must exist
/// - CT must not be in Failed state (cannot checkpoint failed tasks)
/// - Sufficient memory available for checkpoint storage
///
/// # Postconditions
/// - Checkpoint created with unique CheckpointID
/// - Checkpoint is immutable
/// - Task state unchanged by checkpoint operation
/// - Checkpoint can be resumed at any later time
///
/// # Engineering Plan Reference
/// Section 7.3: ct_checkpoint specification.
pub fn ct_checkpoint_definition() -> SyscallDefinition {
    SyscallDefinition::new(
        "ct_checkpoint",
        SyscallFamily::Task,
        number::CT_CHECKPOINT,
        ReturnType::Identifier,
        CapabilitySet::CAP_TASK_FAMILY,
        "Create checkpoint of task state",
    )
    .with_param(SyscallParam::new(
        "ct_id",
        ParamType::Identifier,
        "Task ID to checkpoint",
        false,
    ))
    .with_param(SyscallParam::new(
        "checkpoint_type",
        ParamType::Enum,
        "Type of checkpoint (Full, ReasoningOnly, MemoryOnly)",
        false,
    ))
    .with_error(CsciErrorCode::CsSuccess)
    .with_error(CsciErrorCode::CsEperm)
    .with_error(CsciErrorCode::CsEnoent)
    .with_error(CsciErrorCode::CsEnomem)
    .with_error(CsciErrorCode::CsEbusy)
    .with_preconditions("Caller has Task capability; CT exists; CT not in Failed state; sufficient memory")
    .with_postconditions("Checkpoint created with immutable ID; checkpoint is persistent; task state unchanged")
}

/// Get the definition of the ct_resume syscall.
///
/// **ct_resume**: Resume task from a checkpoint.
///
/// Resumes a task from a previously saved checkpoint, restoring its state to
/// the point when the checkpoint was created. Used for recovery after failure
/// or for exploring alternative execution paths.
///
/// # Parameters
/// - `ct_id`: (CTID) Task ID to resume
/// - `checkpoint_id`: (CheckpointID) Checkpoint to restore from
///
/// # Returns
/// - Success: Unit (task resumed)
/// - Error: CS_EPERM (insufficient capability), CS_ENOENT (task or checkpoint not found),
///          CS_EINVAL (checkpoint incompatible with task), CS_EBUSY (task not in terminal state)
///
/// # Preconditions
/// - Caller must have Task family capability
/// - CT must exist
/// - CT must be in Failed or Complete state (cannot resume running tasks)
/// - CheckpointID must be valid and belong to this task
/// - Checkpoint type must be compatible with task structure
///
/// # Postconditions
/// - Task state restored to checkpoint
/// - Task resumed in appropriate phase
/// - Resources reallocated as needed
/// - Task lifecycle clock advances
///
/// # Engineering Plan Reference
/// Section 7.4: ct_resume specification.
pub fn ct_resume_definition() -> SyscallDefinition {
    SyscallDefinition::new(
        "ct_resume",
        SyscallFamily::Task,
        number::CT_RESUME,
        ReturnType::Unit,
        CapabilitySet::CAP_TASK_FAMILY,
        "Resume task from checkpoint",
    )
    .with_param(SyscallParam::new(
        "ct_id",
        ParamType::Identifier,
        "Task ID to resume",
        false,
    ))
    .with_param(SyscallParam::new(
        "checkpoint_id",
        ParamType::Identifier,
        "Checkpoint to restore from",
        false,
    ))
    .with_error(CsciErrorCode::CsSuccess)
    .with_error(CsciErrorCode::CsEperm)
    .with_error(CsciErrorCode::CsEnoent)
    .with_error(CsciErrorCode::CsEinval)
    .with_error(CsciErrorCode::CsEbusy)
    .with_preconditions(
        "Caller has Task capability; CT exists; CT in Failed/Complete state; checkpoint valid and compatible",
    )
    .with_postconditions("Task state restored to checkpoint; task resumed; resources reallocated")
}

/// Get all Task family syscall definitions.
///
/// Returns a vector of all four syscall definitions in the Task family.
pub fn all_definitions() -> Vec<SyscallDefinition> {
    vec![
        ct_spawn_definition(),
        ct_yield_definition(),
        ct_checkpoint_definition(),
        ct_resume_definition(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;



    #[test]
    fn test_ct_spawn_definition() {
        let def = ct_spawn_definition();
        assert_eq!(def.name, "ct_spawn");
        assert_eq!(def.family, SyscallFamily::Task);
        assert_eq!(def.number, number::CT_SPAWN);
        assert_eq!(def.return_type, ReturnType::Identifier);
        assert_eq!(def.parameters.len(), 4);
        assert!(def.error_codes.len() > 0);
    }

    #[test]
    fn test_ct_yield_definition() {
        let def = ct_yield_definition();
        assert_eq!(def.name, "ct_yield");
        assert_eq!(def.family, SyscallFamily::Task);
        assert_eq!(def.number, number::CT_YIELD);
        assert_eq!(def.return_type, ReturnType::Unit);
        assert_eq!(def.parameters.len(), 2);
    }

    #[test]
    fn test_ct_checkpoint_definition() {
        let def = ct_checkpoint_definition();
        assert_eq!(def.name, "ct_checkpoint");
        assert_eq!(def.family, SyscallFamily::Task);
        assert_eq!(def.number, number::CT_CHECKPOINT);
        assert_eq!(def.return_type, ReturnType::Identifier);
        assert_eq!(def.parameters.len(), 2);
    }

    #[test]
    fn test_ct_resume_definition() {
        let def = ct_resume_definition();
        assert_eq!(def.name, "ct_resume");
        assert_eq!(def.family, SyscallFamily::Task);
        assert_eq!(def.number, number::CT_RESUME);
        assert_eq!(def.return_type, ReturnType::Unit);
        assert_eq!(def.parameters.len(), 2);
    }

    #[test]
    fn test_all_task_definitions() {
        let defs = all_definitions();
        assert_eq!(defs.len(), 4);
        assert_eq!(defs[0].name, "ct_spawn");
        assert_eq!(defs[1].name, "ct_yield");
        assert_eq!(defs[2].name, "ct_checkpoint");
        assert_eq!(defs[3].name, "ct_resume");
    }

    #[test]
    fn test_ct_spawn_parameters() {
        let def = ct_spawn_definition();
        assert_eq!(def.parameters[0].name, "parent_agent");
        assert_eq!(def.parameters[1].name, "config");
        assert_eq!(def.parameters[2].name, "capabilities");
        assert_eq!(def.parameters[3].name, "budget");
    }

    #[test]
    fn test_ct_spawn_errors() {
        let def = ct_spawn_definition();
        let error_codes = &def.error_codes;
        assert!(error_codes.contains(&CsciErrorCode::CsEperm));
        assert!(error_codes.contains(&CsciErrorCode::CsEnomem));
    }

    #[test]
    fn test_syscall_definitions_have_preconditions() {
        let defs = all_definitions();
        for def in defs {
            assert!(!def.preconditions.is_empty());
            assert!(!def.postconditions.is_empty());
        }
    }
}
