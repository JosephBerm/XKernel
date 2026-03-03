// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! # CSCI Signals, Exceptions, Telemetry, and Crew Management Syscalls
//!
//! This family manages signal handlers, exception handlers, telemetry emission,
//! and agent crew operations:
//! - **sig_register**: Register signal handler
//! - **exc_register**: Register exception handler
//! - **trace_emit**: Emit CEF telemetry event
//! - **crew_create**: Create agent crew
//! - **crew_join**: Join existing crew
//!
//! # Engineering Plan Reference
//! Section 12: Signals, Context, and Crew Family Specification.

use crate::error_codes::CsciErrorCode;
use crate::syscall::{ParamType, ReturnType, SyscallDefinition, SyscallFamily, SyscallParam};
use crate::types::{AgentID, CapabilitySet, CrewID, CTID};

/// Signals, context, and crew family syscall numbers.
pub mod number {
    /// sig_register syscall number within Signals family.
    pub const SIG_REGISTER: u8 = 0;
    /// exc_register syscall number within Signals family.
    pub const EXC_REGISTER: u8 = 1;
    /// trace_emit syscall number within Telemetry family.
    pub const TRACE_EMIT: u8 = 2;
    /// crew_create syscall number within Crew family.
    pub const CREW_CREATE: u8 = 3;
    /// crew_join syscall number within Crew family.
    pub const CREW_JOIN: u8 = 4;
}

/// Get the definition of the sig_register syscall.
///
/// **sig_register**: Register signal handler.
///
/// Registers a signal handler function to be invoked when a specified signal
/// is generated. Signals include task completion, channel messages, timeouts,
/// and resource warnings. The handler is invoked asynchronously.
///
/// # Parameters
/// - `signal_type`: (Enum) Signal type to handle (TaskComplete, ChannelMessage, Timeout, ResourceWarning)
/// - `handler_ct`: (Identifier) CT ID of handler task to invoke
/// - `filter_data`: (Memory) Optional filter/context data for signal matching
///
/// # Returns
/// - Success: Unit (handler registered)
/// - Error: CS_EPERM (no signal capability), CS_ENOENT (handler task not found),
///          CS_EINVAL (invalid signal type)
///
/// # Preconditions
/// - Caller must have Signal handling capability
/// - `signal_type` must be a valid signal type
/// - `handler_ct` must be a valid, existing task
/// - Handler task must be in a state that can accept signals
///
/// # Postconditions
/// - Signal handler is registered
/// - Handler will be invoked when signal is generated
/// - Handler registration persists until task completion or explicit deregistration
///
/// # Engineering Plan Reference
/// Section 12.1: sig_register specification.
pub fn sig_register_definition() -> SyscallDefinition {
    SyscallDefinition::new(
        "sig_register",
        SyscallFamily::Context,
        number::SIG_REGISTER,
        ReturnType::Unit,
        CapabilitySet::CAP_CONTEXT_FAMILY,
        "Register signal handler",
    )
    .with_param(SyscallParam::new(
        "signal_type",
        ParamType::Enum,
        "Signal type to handle",
        false,
    ))
    .with_param(SyscallParam::new(
        "handler_ct",
        ParamType::Identifier,
        "Handler task CT ID",
        false,
    ))
    .with_param(SyscallParam::new(
        "filter_data",
        ParamType::Memory,
        "Optional filter/context data",
        true,
    ))
    .with_error(CsciErrorCode::CsSuccess)
    .with_error(CsciErrorCode::CsEperm)
    .with_error(CsciErrorCode::CsEnoent)
    .with_error(CsciErrorCode::CsEinval)
    .with_preconditions(
        "Caller has Context capability; signal_type valid; handler_ct exists and ready",
    )
    .with_postconditions("Handler registered; will be invoked when signal occurs; persists until deregistration or task completion")
}

/// Get the definition of the exc_register syscall.
///
/// **exc_register**: Register exception handler.
///
/// Registers an exception handler to catch and handle exceptions raised during
/// task execution. Exception types include memory violations, capability violations,
/// timeouts, assertion failures, and stack overflows. The handler can inspect the
/// exception and optionally recover or suppress it.
///
/// # Parameters
/// - `exception_type`: (Enum) Exception type to handle
/// - `handler_ct`: (Identifier) CT ID of handler task
/// - `catch_and_recover`: (Numeric) Whether handler can suppress exception (0=no, 1=yes)
///
/// # Returns
/// - Success: Unit (handler registered)
/// - Error: CS_EPERM (no exception handling capability), CS_ENOENT (handler not found),
///          CS_EINVAL (invalid exception type)
///
/// # Preconditions
/// - Caller must have Exception handling capability
/// - `exception_type` must be valid
/// - `handler_ct` must be valid and ready
/// - If `catch_and_recover` is true, caller must have exception recovery capability
///
/// # Postconditions
/// - Exception handler is registered
/// - Handler will be invoked when exception is raised
/// - Handler can optionally suppress exception and allow recovery
/// - Registration persists until task completion
///
/// # Engineering Plan Reference
/// Section 12.2: exc_register specification.
pub fn exc_register_definition() -> SyscallDefinition {
    SyscallDefinition::new(
        "exc_register",
        SyscallFamily::Context,
        number::EXC_REGISTER,
        ReturnType::Unit,
        CapabilitySet::CAP_CONTEXT_FAMILY,
        "Register exception handler",
    )
    .with_param(SyscallParam::new(
        "exception_type",
        ParamType::Enum,
        "Exception type to handle",
        false,
    ))
    .with_param(SyscallParam::new(
        "handler_ct",
        ParamType::Identifier,
        "Handler task CT ID",
        false,
    ))
    .with_param(SyscallParam::new(
        "catch_and_recover",
        ParamType::Numeric,
        "Whether handler can suppress exception",
        false,
    ))
    .with_error(CsciErrorCode::CsSuccess)
    .with_error(CsciErrorCode::CsEperm)
    .with_error(CsciErrorCode::CsEnoent)
    .with_error(CsciErrorCode::CsEinval)
    .with_preconditions(
        "Caller has Context capability; exception_type valid; handler_ct exists; recovery capability if needed",
    )
    .with_postconditions(
        "Handler registered; invoked when exception raised; can optionally suppress exception",
    )
}

/// Get the definition of the trace_emit syscall.
///
/// **trace_emit**: Emit Cognitive Event Format (CEF) telemetry event.
///
/// Emits a structured telemetry event for system monitoring and tracing.
/// Events include information about system state, task execution, resource usage,
/// and other observable phenomena. Events are timestamped and classified by severity.
///
/// # Parameters
/// - `event_type`: (Numeric) Event type identifier
/// - `severity`: (Numeric) Severity level (0=info, 1=warn, 2=error)
/// - `message`: (Memory) Event message and metadata
///
/// # Returns
/// - Success: Unit (event emitted)
/// - Error: CS_EPERM (no telemetry capability), CS_EINVAL (invalid event data),
///          CS_EBUDGET (telemetry budget exhausted)
///
/// # Preconditions
/// - Caller must have Telemetry capability
/// - `event_type` must be valid
/// - `severity` must be 0-2
/// - `message` must be valid UTF-8
/// - Telemetry budget must not be exceeded
///
/// # Postconditions
/// - Event is recorded and timestamped
/// - Event is buffered or streamed to telemetry backend
/// - Event is available for monitoring and audit
///
/// # Engineering Plan Reference
/// Section 12.3: trace_emit specification.
pub fn trace_emit_definition() -> SyscallDefinition {
    SyscallDefinition::new(
        "trace_emit",
        SyscallFamily::Context,
        number::TRACE_EMIT,
        ReturnType::Unit,
        CapabilitySet::CAP_CONTEXT_FAMILY,
        "Emit CEF telemetry event",
    )
    .with_param(SyscallParam::new(
        "event_type",
        ParamType::Numeric,
        "Event type identifier",
        false,
    ))
    .with_param(SyscallParam::new(
        "severity",
        ParamType::Numeric,
        "Severity level (0=info, 1=warn, 2=error)",
        false,
    ))
    .with_param(SyscallParam::new(
        "message",
        ParamType::Memory,
        "Event message and metadata",
        false,
    ))
    .with_error(CsciErrorCode::CsSuccess)
    .with_error(CsciErrorCode::CsEperm)
    .with_error(CsciErrorCode::CsEinval)
    .with_error(CsciErrorCode::CsEbudget)
    .with_preconditions(
        "Caller has Telemetry capability; event_type valid; severity 0-2; message valid; budget available",
    )
    .with_postconditions(
        "Event recorded with timestamp; buffered/streamed to backend; available for monitoring",
    )
}

/// Get the definition of the crew_create syscall.
///
/// **crew_create**: Create agent crew.
///
/// Creates a new crew (group) of agents that will collaborate on a task.
/// A crew provides a namespace for shared resources and coordination mechanisms.
/// The creating agent becomes the crew owner and can manage crew membership.
///
/// # Parameters
/// - `crew_name`: (Memory) Name of the crew
/// - `owner_agent`: (Identifier) Agent ID of crew owner
/// - `max_members`: (Numeric) Maximum crew size (0 = unlimited)
///
/// # Returns
/// - Success: CrewID of the newly created crew
/// - Error: CS_EPERM (no crew management capability), CS_ENOMEM (insufficient memory),
///          CS_EINVAL (invalid parameters)
///
/// # Preconditions
/// - Caller must have Crew management capability
/// - `owner_agent` must be valid
/// - `crew_name` must be non-empty and unique
/// - `max_members` constraint must be reasonable
///
/// # Postconditions
/// - Crew is created with immutable CrewID
/// - Owner agent is added as initial member
/// - Crew is ready to accept new members
/// - Crew has empty shared resource namespace
///
/// # Engineering Plan Reference
/// Section 12.4: crew_create specification.
pub fn crew_create_definition() -> SyscallDefinition {
    SyscallDefinition::new(
        "crew_create",
        SyscallFamily::Context,
        number::CREW_CREATE,
        ReturnType::Identifier,
        CapabilitySet::CAP_CONTEXT_FAMILY,
        "Create agent crew",
    )
    .with_param(SyscallParam::new(
        "crew_name",
        ParamType::Memory,
        "Crew name",
        false,
    ))
    .with_param(SyscallParam::new(
        "owner_agent",
        ParamType::Identifier,
        "Crew owner agent ID",
        false,
    ))
    .with_param(SyscallParam::new(
        "max_members",
        ParamType::Numeric,
        "Maximum crew size (0 = unlimited)",
        false,
    ))
    .with_error(CsciErrorCode::CsSuccess)
    .with_error(CsciErrorCode::CsEperm)
    .with_error(CsciErrorCode::CsEnomem)
    .with_error(CsciErrorCode::CsEinval)
    .with_preconditions(
        "Caller has Crew capability; owner_agent valid; crew_name unique and non-empty; max_members reasonable",
    )
    .with_postconditions(
        "Crew created with immutable CrewID; owner is initial member; ready for membership; empty resource namespace",
    )
}

/// Get the definition of the crew_join syscall.
///
/// **crew_join**: Join existing crew.
///
/// Joins an agent to an existing crew. Once joined, the agent can access
/// crew-shared resources and participate in crew coordination. The crew
/// owner can accept or reject join requests (depending on crew policy).
///
/// # Parameters
/// - `crew_id`: (Identifier) Crew ID to join
/// - `agent_id`: (Identifier) Agent ID of the joining agent
/// - `role`: (Memory) Requested role in crew (optional)
///
/// # Returns
/// - Success: Unit (agent joined crew)
/// - Error: CS_EPERM (join denied or no capability), CS_ENOENT (crew not found),
///          CS_EEXIST (agent already in crew), CS_EINVAL (invalid parameters)
///
/// # Preconditions
/// - Caller must have Crew management capability
/// - `crew_id` must reference an existing crew
/// - `agent_id` must be valid
/// - Agent must not already be in crew
/// - Crew must not be at max_members limit
/// - Crew owner must not reject join request
///
/// # Postconditions
/// - Agent is added to crew membership
/// - Agent can access crew-shared resources
/// - Agent has specified role in crew
/// - Crew size is incremented
///
/// # Engineering Plan Reference
/// Section 12.5: crew_join specification.
pub fn crew_join_definition() -> SyscallDefinition {
    SyscallDefinition::new(
        "crew_join",
        SyscallFamily::Context,
        number::CREW_JOIN,
        ReturnType::Unit,
        CapabilitySet::CAP_CONTEXT_FAMILY,
        "Join existing crew",
    )
    .with_param(SyscallParam::new(
        "crew_id",
        ParamType::Identifier,
        "Crew ID to join",
        false,
    ))
    .with_param(SyscallParam::new(
        "agent_id",
        ParamType::Identifier,
        "Agent ID of joining agent",
        false,
    ))
    .with_param(SyscallParam::new(
        "role",
        ParamType::Memory,
        "Requested role in crew",
        true,
    ))
    .with_error(CsciErrorCode::CsSuccess)
    .with_error(CsciErrorCode::CsEperm)
    .with_error(CsciErrorCode::CsEnoent)
    .with_error(CsciErrorCode::CsEexist)
    .with_error(CsciErrorCode::CsEinval)
    .with_preconditions(
        "Caller has Crew capability; crew_id exists; agent_id valid; agent not already in crew; not at max_members; owner accepts",
    )
    .with_postconditions(
        "Agent added to crew; can access crew resources; has specified role; crew size incremented",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sig_register_definition() {
        let def = sig_register_definition();
        assert_eq!(def.name, "sig_register");
        assert_eq!(def.family, SyscallFamily::Context);
        assert_eq!(def.return_type, ReturnType::Unit);
        assert_eq!(def.parameters.len(), 3);
    }

    #[test]
    fn test_sig_register_parameters() {
        let def = sig_register_definition();
        assert_eq!(def.parameters[0].name, "signal_type");
        assert_eq!(def.parameters[1].name, "handler_ct");
        assert_eq!(def.parameters[2].name, "filter_data");
        assert!(def.parameters[2].optional);
    }

    #[test]
    fn test_exc_register_definition() {
        let def = exc_register_definition();
        assert_eq!(def.name, "exc_register");
        assert_eq!(def.family, SyscallFamily::Context);
        assert_eq!(def.return_type, ReturnType::Unit);
        assert_eq!(def.parameters.len(), 3);
    }

    #[test]
    fn test_exc_register_parameters() {
        let def = exc_register_definition();
        assert_eq!(def.parameters[0].name, "exception_type");
        assert_eq!(def.parameters[1].name, "handler_ct");
        assert_eq!(def.parameters[2].name, "catch_and_recover");
    }

    #[test]
    fn test_trace_emit_definition() {
        let def = trace_emit_definition();
        assert_eq!(def.name, "trace_emit");
        assert_eq!(def.family, SyscallFamily::Context);
        assert_eq!(def.return_type, ReturnType::Unit);
        assert_eq!(def.parameters.len(), 3);
    }

    #[test]
    fn test_trace_emit_parameters() {
        let def = trace_emit_definition();
        assert_eq!(def.parameters[0].name, "event_type");
        assert_eq!(def.parameters[1].name, "severity");
        assert_eq!(def.parameters[2].name, "message");
    }

    #[test]
    fn test_crew_create_definition() {
        let def = crew_create_definition();
        assert_eq!(def.name, "crew_create");
        assert_eq!(def.family, SyscallFamily::Context);
        assert_eq!(def.return_type, ReturnType::Identifier);
        assert_eq!(def.parameters.len(), 3);
    }

    #[test]
    fn test_crew_create_parameters() {
        let def = crew_create_definition();
        assert_eq!(def.parameters[0].name, "crew_name");
        assert_eq!(def.parameters[1].name, "owner_agent");
        assert_eq!(def.parameters[2].name, "max_members");
    }

    #[test]
    fn test_crew_join_definition() {
        let def = crew_join_definition();
        assert_eq!(def.name, "crew_join");
        assert_eq!(def.family, SyscallFamily::Context);
        assert_eq!(def.return_type, ReturnType::Unit);
        assert_eq!(def.parameters.len(), 3);
    }

    #[test]
    fn test_crew_join_parameters() {
        let def = crew_join_definition();
        assert_eq!(def.parameters[0].name, "crew_id");
        assert_eq!(def.parameters[1].name, "agent_id");
        assert_eq!(def.parameters[2].name, "role");
        assert!(def.parameters[2].optional);
    }

    #[test]
    fn test_syscall_numbers_unique() {
        let nums = [
            number::SIG_REGISTER,
            number::EXC_REGISTER,
            number::TRACE_EMIT,
            number::CREW_CREATE,
            number::CREW_JOIN,
        ];
        for i in 0..nums.len() {
            for j in (i + 1)..nums.len() {
                assert_ne!(
                    nums[i], nums[j],
                    "Syscall numbers must be unique"
                );
            }
        }
    }

    #[test]
    fn test_all_definitions_have_preconditions() {
        assert!(!sig_register_definition().preconditions.is_empty());
        assert!(!exc_register_definition().preconditions.is_empty());
        assert!(!trace_emit_definition().preconditions.is_empty());
        assert!(!crew_create_definition().preconditions.is_empty());
        assert!(!crew_join_definition().preconditions.is_empty());
    }

    #[test]
    fn test_all_definitions_have_postconditions() {
        assert!(!sig_register_definition().postconditions.is_empty());
        assert!(!exc_register_definition().postconditions.is_empty());
        assert!(!trace_emit_definition().postconditions.is_empty());
        assert!(!crew_create_definition().postconditions.is_empty());
        assert!(!crew_join_definition().postconditions.is_empty());
    }
}
