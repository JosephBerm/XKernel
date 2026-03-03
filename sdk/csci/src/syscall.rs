// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! # CSCI Syscall Definitions
//!
//! Core types for defining syscalls including families, definitions, and parameters.
//!
//! # Engineering Plan Reference
//! Section 6: CSCI Syscall Definitions.

use crate::error_codes::CsciErrorCode;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

/// Syscall family classification.
///
/// Syscalls are organized into families based on functional area.
/// Each family is protected by its own capability bit.
///
/// # Engineering Plan Reference
/// Section 6.1: Syscall families.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SyscallFamily {
    /// Task lifecycle: spawn, yield, checkpoint, resume.
    Task,
    /// Semantic memory operations.
    Memory,
    /// Tool invocation and registry.
    Tool,
    /// Inter-task communication.
    Channel,
    /// Execution context management.
    Context,
    /// Capability granting and revocation.
    Capability,
    /// Signal and exception handling.
    Signals,
    /// Agent crew management.
    Crew,
    /// Telemetry and tracing.
    Telemetry,
}

impl fmt::Display for SyscallFamily {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Task => write!(f, "Task"),
            Self::Memory => write!(f, "Memory"),
            Self::Tool => write!(f, "Tool"),
            Self::Channel => write!(f, "Channel"),
            Self::Context => write!(f, "Context"),
            Self::Capability => write!(f, "Capability"),
            Self::Signals => write!(f, "Signals"),
            Self::Crew => write!(f, "Crew"),
            Self::Telemetry => write!(f, "Telemetry"),
        }
    }
}

/// Parameter type classification.
///
/// Describes the semantic type of a syscall parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamType {
    /// Numeric value (u64 or similar).
    Numeric,
    /// Identifier (CTID, AgentID, MemoryRegionID, etc.).
    Identifier,
    /// Configuration structure.
    Config,
    /// Memory data.
    Memory,
    /// Capability set.
    Capability,
    /// Enumeration value.
    Enum,
}

impl fmt::Display for ParamType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Numeric => write!(f, "Numeric"),
            Self::Identifier => write!(f, "Identifier"),
            Self::Config => write!(f, "Config"),
            Self::Memory => write!(f, "Memory"),
            Self::Capability => write!(f, "Capability"),
            Self::Enum => write!(f, "Enum"),
        }
    }
}

/// Parameter definition for a syscall.
///
/// Documents a single parameter including its name, type, purpose, and optionality.
///
/// # Engineering Plan Reference
/// Section 6.2: Syscall parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyscallParam {
    /// Parameter name (snake_case).
    pub name: String,
    /// Parameter type classification.
    pub param_type: ParamType,
    /// Purpose and usage documentation.
    pub purpose: String,
    /// Whether this parameter is optional (default behavior specified).
    pub optional: bool,
}

impl SyscallParam {
    /// Create a new syscall parameter definition.
    pub fn new(name: &str, param_type: ParamType, purpose: &str, optional: bool) -> Self {
        Self {
            name: String::from(name),
            param_type,
            purpose: String::from(purpose),
            optional,
        }
    }
}

/// Return type classification for a syscall.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReturnType {
    /// Unit return type (no return value, just success/error).
    Unit,
    /// Numeric return value.
    Numeric,
    /// Identifier return (CTID, MemoryRegionID, etc.).
    Identifier,
    /// Memory data return.
    Memory,
}

impl fmt::Display for ReturnType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unit => write!(f, "Unit"),
            Self::Numeric => write!(f, "Numeric"),
            Self::Identifier => write!(f, "Identifier"),
            Self::Memory => write!(f, "Memory"),
        }
    }
}

/// Full definition of a CSCI syscall.
///
/// Specifies the complete interface of a syscall including its parameters,
/// return type, error codes, preconditions, postconditions, and capability
/// requirements.
///
/// # Engineering Plan Reference
/// Section 6.3: Syscall definitions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyscallDefinition {
    /// Syscall name (snake_case, e.g., "ct_spawn").
    pub name: String,
    /// Syscall family classification.
    pub family: SyscallFamily,
    /// Numeric syscall number (0-255 within family).
    pub number: u8,
    /// Parameter definitions in order.
    pub parameters: Vec<SyscallParam>,
    /// Return type.
    pub return_type: ReturnType,
    /// Possible error codes this syscall can return.
    pub error_codes: Vec<CsciErrorCode>,
    /// Preconditions (in prose).
    pub preconditions: String,
    /// Postconditions (in prose).
    pub postconditions: String,
    /// Capability bit required to invoke this syscall.
    pub capability_required: u32,
    /// Human-readable description.
    pub description: String,
}

impl SyscallDefinition {
    /// Create a new syscall definition.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        name: &str,
        family: SyscallFamily,
        number: u8,
        return_type: ReturnType,
        capability_required: u32,
        description: &str,
    ) -> Self {
        Self {
            name: String::from(name),
            family,
            number,
            parameters: Vec::new(),
            return_type,
            error_codes: Vec::new(),
            preconditions: String::new(),
            postconditions: String::new(),
            capability_required,
            description: String::from(description),
        }
    }

    /// Add a parameter to this syscall definition.
    pub fn with_param(mut self, param: SyscallParam) -> Self {
        self.parameters.push(param);
        self
    }

    /// Add an error code to this syscall definition.
    pub fn with_error(mut self, error: CsciErrorCode) -> Self {
        self.error_codes.push(error);
        self
    }

    /// Set the preconditions for this syscall.
    pub fn with_preconditions(mut self, preconditions: &str) -> Self {
        self.preconditions = String::from(preconditions);
        self
    }

    /// Set the postconditions for this syscall.
    pub fn with_postconditions(mut self, postconditions: &str) -> Self {
        self.postconditions = String::from(postconditions);
        self
    }

    /// Get the full syscall identifier (family + number).
    pub fn identifier(&self) -> String {
        format!("{}-{:03}", self.family, self.number)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::format;
use alloc::string::ToString;

    #[test]
    fn test_syscall_family_display() {
        assert_eq!(SyscallFamily::Task.to_string(), "Task");
        assert_eq!(SyscallFamily::Memory.to_string(), "Memory");
        assert_eq!(SyscallFamily::Tool.to_string(), "Tool");
    }

    #[test]
    fn test_param_type_display() {
        assert_eq!(ParamType::Numeric.to_string(), "Numeric");
        assert_eq!(ParamType::Identifier.to_string(), "Identifier");
        assert_eq!(ParamType::Config.to_string(), "Config");
    }

    #[test]
    fn test_syscall_param_creation() {
        let param = SyscallParam::new(
            "parent_agent",
            ParamType::Identifier,
            "Parent agent creating the task",
            false,
        );

        assert_eq!(param.name, "parent_agent");
        assert_eq!(param.param_type, ParamType::Identifier);
        assert!(!param.optional);
    }

    #[test]
    fn test_syscall_param_optional() {
        let param = SyscallParam::new(
            "optional_param",
            ParamType::Numeric,
            "Optional numeric parameter",
            true,
        );

        assert!(param.optional);
    }

    #[test]
    fn test_return_type_display() {
        assert_eq!(ReturnType::Unit.to_string(), "Unit");
        assert_eq!(ReturnType::Numeric.to_string(), "Numeric");
        assert_eq!(ReturnType::Identifier.to_string(), "Identifier");
    }

    #[test]
    fn test_syscall_definition_creation() {
        let def = SyscallDefinition::new(
            "ct_spawn",
            SyscallFamily::Task,
            0,
            ReturnType::Identifier,
            0,
            "Create a new cognitive task",
        );

        assert_eq!(def.name, "ct_spawn");
        assert_eq!(def.family, SyscallFamily::Task);
        assert_eq!(def.number, 0);
        assert!(def.parameters.is_empty());
    }

    #[test]
    fn test_syscall_definition_builder() {
        let def = SyscallDefinition::new(
            "ct_spawn",
            SyscallFamily::Task,
            0,
            ReturnType::Identifier,
            0,
            "Create a new cognitive task",
        )
        .with_param(SyscallParam::new(
            "parent_agent",
            ParamType::Identifier,
            "Parent agent",
            false,
        ))
        .with_error(CsciErrorCode::CsEperm)
        .with_error(CsciErrorCode::CsEnomem)
        .with_preconditions("parent agent must be valid")
        .with_postconditions("CT in Spawn phase");

        assert_eq!(def.parameters.len(), 1);
        assert_eq!(def.error_codes.len(), 2);
        assert!(!def.preconditions.is_empty());
        assert!(!def.postconditions.is_empty());
    }

    #[test]
    fn test_syscall_definition_identifier() {
        let def = SyscallDefinition::new(
            "ct_spawn",
            SyscallFamily::Task,
            0,
            ReturnType::Identifier,
            0,
            "Create a new cognitive task",
        );

        let id = def.identifier();
        assert!(id.contains("Task"));
        assert!(id.contains("000"));
    }

    #[test]
    fn test_syscall_family_equality() {
        assert_eq!(SyscallFamily::Task, SyscallFamily::Task);
        assert_ne!(SyscallFamily::Task, SyscallFamily::Memory);
    }
}
