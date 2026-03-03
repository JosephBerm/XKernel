// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! # CSCI Tool Family Syscalls
//!
//! Tool family syscalls enable invocation of external tools and services:
//! - **tool_bind**: Bind external tool with sandbox configuration
//! - **tool_invoke**: Invoke bound tool with arguments
//!
//! # Engineering Plan Reference
//! Section 11: Tool Family Specification.

use crate::error_codes::CsciErrorCode;
use crate::syscall::{ParamType, ReturnType, SyscallDefinition, SyscallFamily, SyscallParam};
use crate::types::{CapabilitySet, SandboxConfig, ToolArguments, ToolBindingID, ToolResult, ToolSpec};

/// Tool family syscall numbers.
pub mod number {
    /// tool_bind syscall number within Tool family.
    pub const TOOL_BIND: u8 = 0;
    /// tool_invoke syscall number within Tool family.
    pub const TOOL_INVOKE: u8 = 1;
}

/// Get the definition of the tool_bind syscall.
///
/// **tool_bind**: Bind external tool with sandbox configuration.
///
/// Binds an external tool (MCP tool, web search, code executor, etc.) into the
/// agent's execution environment with specified sandbox constraints. The sandbox
/// configuration enforces resource limits and policy restrictions on the tool.
///
/// # Parameters
/// - `tool_spec`: (Config) Tool specification (name, version, type)
/// - `sandbox_config`: (Config) Sandbox configuration (memory, timeout, network, etc.)
///
/// # Returns
/// - Success: ToolBindingID for the bound tool
/// - Error: CS_EINVAL (invalid spec or config), CS_ENOMEM (insufficient memory),
///          CS_ESANDBOX (sandbox configuration error)
///
/// # Preconditions
/// - Caller must have Tool family capability
/// - `tool_spec` must be valid and reference an available tool
/// - `sandbox_config` must be valid and enforceable
/// - Tool must be compatible with current execution environment
/// - Resource budget must accommodate sandbox overhead
///
/// # Postconditions
/// - Tool is bound with immutable ToolBindingID
/// - Sandbox is initialized and ready
/// - Tool cannot be invoked without the binding ID
/// - Binding lifetime matches caller's task lifetime (unless explicitly closed)
///
/// # Engineering Plan Reference
/// Section 11.1: tool_bind specification.
pub fn tool_bind_definition() -> SyscallDefinition {
    SyscallDefinition::new(
        "tool_bind",
        SyscallFamily::Tool,
        number::TOOL_BIND,
        ReturnType::Identifier,
        CapabilitySet::CAP_TOOL_FAMILY,
        "Bind external tool with sandbox configuration",
    )
    .with_param(SyscallParam::new(
        "tool_spec",
        ParamType::Config,
        "Tool specification",
        false,
    ))
    .with_param(SyscallParam::new(
        "sandbox_config",
        ParamType::Config,
        "Sandbox configuration",
        false,
    ))
    .with_error(CsciErrorCode::CsSuccess)
    .with_error(CsciErrorCode::CsEinval)
    .with_error(CsciErrorCode::CsEnomem)
    .with_error(CsciErrorCode::CsEsandbox)
    .with_preconditions(
        "Caller has Tool capability; tool_spec valid; tool available; sandbox_config valid and enforceable",
    )
    .with_postconditions(
        "Tool bound with immutable ToolBindingID; sandbox initialized; ready for invocation",
    )
}

/// Get the definition of the tool_invoke syscall.
///
/// **tool_invoke**: Invoke bound tool with arguments.
///
/// Invokes a previously bound tool with specified arguments. The tool executes
/// in its configured sandbox environment with enforced resource limits. The
/// invocation may timeout if exceeding the specified deadline.
///
/// # Parameters
/// - `binding_id`: (Identifier) Tool binding ID from tool_bind
/// - `arguments`: (Config) Tool arguments (key-value pairs)
/// - `timeout_ms`: (Numeric) Execution timeout in milliseconds, or 0 for no timeout
///
/// # Returns
/// - Success: ToolResult containing tool output and exit status
/// - Error: CS_EINVAL (invalid binding or arguments), CS_ETIMEOUT (exceeded timeout),
///          CS_ETOOLERR (tool execution error), CS_EBUDGET (would exceed budget)
///
/// # Preconditions
/// - `binding_id` must reference a valid, active binding
/// - `arguments` must be valid and match tool expectations
/// - Tool must not already be executing (non-concurrent invocation)
/// - Sufficient budget must remain for tool execution
/// - Timeout must be non-negative
///
/// # Postconditions
/// - Tool has executed in sandbox
/// - ToolResult returned with exit status and output
/// - Tool output size capped by memory limit or result buffer size
/// - Tool cannot access resources outside sandbox
/// - Execution logged for audit trail
///
/// # Engineering Plan Reference
/// Section 11.2: tool_invoke specification.
pub fn tool_invoke_definition() -> SyscallDefinition {
    SyscallDefinition::new(
        "tool_invoke",
        SyscallFamily::Tool,
        number::TOOL_INVOKE,
        ReturnType::Memory,
        CapabilitySet::CAP_TOOL_FAMILY,
        "Invoke bound tool with arguments",
    )
    .with_param(SyscallParam::new(
        "binding_id",
        ParamType::Identifier,
        "Tool binding ID from tool_bind",
        false,
    ))
    .with_param(SyscallParam::new(
        "arguments",
        ParamType::Config,
        "Tool arguments",
        false,
    ))
    .with_param(SyscallParam::new(
        "timeout_ms",
        ParamType::Numeric,
        "Execution timeout in milliseconds (0 = no timeout)",
        true,
    ))
    .with_error(CsciErrorCode::CsSuccess)
    .with_error(CsciErrorCode::CsEinval)
    .with_error(CsciErrorCode::CsEtimeout)
    .with_error(CsciErrorCode::CsEtoolerr)
    .with_error(CsciErrorCode::CsEbudget)
    .with_preconditions(
        "Binding valid and active; arguments valid; tool not executing; budget sufficient; timeout >= 0",
    )
    .with_postconditions(
        "Tool executed in sandbox; ToolResult returned; tool restricted to sandbox; execution logged",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_bind_definition() {
        let def = tool_bind_definition();
        assert_eq!(def.name, "tool_bind");
        assert_eq!(def.family, SyscallFamily::Tool);
        assert_eq!(def.number, number::TOOL_BIND);
        assert_eq!(def.return_type, ReturnType::Identifier);
        assert_eq!(def.parameters.len(), 2);
    }

    #[test]
    fn test_tool_bind_parameters() {
        let def = tool_bind_definition();
        assert_eq!(def.parameters[0].name, "tool_spec");
        assert_eq!(def.parameters[1].name, "sandbox_config");
        assert!(!def.parameters[0].optional);
        assert!(!def.parameters[1].optional);
    }

    #[test]
    fn test_tool_bind_errors() {
        let def = tool_bind_definition();
        assert!(def.error_codes.len() >= 4);
        assert!(def.error_codes.contains(&CsciErrorCode::CsEinval));
        assert!(def.error_codes.contains(&CsciErrorCode::CsEnomem));
        assert!(def.error_codes.contains(&CsciErrorCode::CsEsandbox));
    }

    #[test]
    fn test_tool_invoke_definition() {
        let def = tool_invoke_definition();
        assert_eq!(def.name, "tool_invoke");
        assert_eq!(def.family, SyscallFamily::Tool);
        assert_eq!(def.number, number::TOOL_INVOKE);
        assert_eq!(def.return_type, ReturnType::Memory);
        assert_eq!(def.parameters.len(), 3);
    }

    #[test]
    fn test_tool_invoke_parameters() {
        let def = tool_invoke_definition();
        assert_eq!(def.parameters[0].name, "binding_id");
        assert_eq!(def.parameters[1].name, "arguments");
        assert_eq!(def.parameters[2].name, "timeout_ms");
        assert!(!def.parameters[0].optional);
        assert!(!def.parameters[1].optional);
        assert!(def.parameters[2].optional);
    }

    #[test]
    fn test_tool_invoke_errors() {
        let def = tool_invoke_definition();
        assert!(def.error_codes.len() >= 5);
        assert!(def.error_codes.contains(&CsciErrorCode::CsEtimeout));
        assert!(def.error_codes.contains(&CsciErrorCode::CsEtoolerr));
        assert!(def.error_codes.contains(&CsciErrorCode::CsEbudget));
    }

    #[test]
    fn test_tool_family_syscall_numbers_unique() {
        assert_ne!(number::TOOL_BIND, number::TOOL_INVOKE);
    }

    #[test]
    fn test_all_definitions_have_preconditions() {
        assert!(!tool_bind_definition().preconditions.is_empty());
        assert!(!tool_invoke_definition().preconditions.is_empty());
    }

    #[test]
    fn test_all_definitions_have_postconditions() {
        assert!(!tool_bind_definition().postconditions.is_empty());
        assert!(!tool_invoke_definition().postconditions.is_empty());
    }
}
