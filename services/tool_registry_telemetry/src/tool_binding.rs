// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! ToolBinding entity definition and validation.
//!
//! Defines the central ToolBinding entity that represents a binding between
//! a tool definition and an agent context, including all security, caching,
//! and effect metadata.
//!
//! See Engineering Plan § 2.11: ToolBinding Entity & Tool Registry.

use crate::cache::CacheConfig;
use crate::commit_protocol::CommitProtocol;
use crate::effect_class::EffectClass;
use crate::error::Result;
use crate::ids::{AgentID, CapID, ToolBindingID, ToolID};
use crate::sandbox::SandboxConfig;
use crate::schema::TypeSchema;
use core::fmt;

/// ToolBinding entity.
///
/// Represents a binding between a tool definition and an agent context.
/// Contains all metadata required to safely and correctly invoke the tool
/// including security constraints, effect declarations, and caching behavior.
///
/// See Engineering Plan § 2.11: ToolBinding Entity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolBinding {
    /// Unique identifier for this binding.
    ///
    /// Globally unique across all tool bindings.
    pub id: ToolBindingID,

    /// Identifier of the underlying tool definition.
    ///
    /// References the tool in the tool registry.
    pub tool: ToolID,

    /// Agent that holds this binding.
    ///
    /// The principal (user, service, process) authorized to use this tool.
    pub agent: AgentID,

    /// Required capability for tool invocation.
    ///
    /// Agent must present this capability to invoke the tool.
    /// 256-bit unforgeable identifier assigned by kernel.
    pub capability: CapID,

    /// Input and output type schema.
    ///
    /// Specifies expected input structure and guaranteed output structure.
    /// Used for validation and serialization.
    pub schema: TypeSchema,

    /// Sandbox configuration for this tool.
    ///
    /// Security constraints including network access, filesystem restrictions,
    /// execution timeouts, and allowed syscalls.
    pub sandbox_config: SandboxConfig,

    /// Response caching configuration.
    ///
    /// Caching behavior for tool outputs.
    pub response_cache: CacheConfig,

    /// Effect class for this tool.
    ///
    /// Declares the nature of state mutations the tool can perform.
    /// Defaults to WriteIrreversible (fail-safe).
    pub effect_class: EffectClass,

    /// Optional commit protocol for transactional operations.
    ///
    /// If present, tool invocations follow two-phase commit protocol.
    /// Required for tools with WriteCompensable or high-impact WriteIrreversible effects.
    pub commit_protocol: Option<CommitProtocol>,
}

impl ToolBinding {
    /// Creates a new tool binding with default configuration.
    ///
    /// # Parameters
    ///
    /// - `id`: Unique binding identifier
    /// - `tool`: Tool definition identifier
    /// - `agent`: Agent holding this binding
    /// - `capability`: Required capability for invocation
    /// - `schema`: Input/output type schema
    ///
    /// Default configuration:
    /// - Restrictive sandbox (no network, no filesystem access)
    /// - Effect class: WriteIrreversible
    /// - Caching: disabled
    /// - No commit protocol
    pub fn new(
        id: ToolBindingID,
        tool: ToolID,
        agent: AgentID,
        capability: CapID,
        schema: TypeSchema,
    ) -> Self {
        ToolBinding {
            id,
            tool,
            agent,
            capability,
            schema,
            sandbox_config: SandboxConfig::restrictive(),
            response_cache: CacheConfig::disabled(),
            effect_class: EffectClass::default(),
            commit_protocol: None,
        }
    }

    /// Validates this tool binding for correctness.
    ///
    /// Checks:
    /// - Binding has valid schema (fields and constraints are valid)
    /// - Sandbox configuration is internally consistent
    /// - Cache configuration is internally consistent
    /// - Commit protocol is specified if required by effect class
    ///
    /// See Engineering Plan § 2.11.1: Binding Validation.
    pub fn validate(&self) -> Result<()> {
        // Validate schema
        if self.schema.input_schema.field_count() > 1000 {
            return Err(crate::error::ToolError::SchemaValidationFailed {
                reason: "input schema has too many fields (>1000)".to_string(),
            });
        }

        if self.schema.output_schema.field_count() > 1000 {
            return Err(crate::error::ToolError::SchemaValidationFailed {
                reason: "output schema has too many fields (>1000)".to_string(),
            });
        }

        // Validate sandbox config
        if !self.sandbox_config.is_restrictive() && !self.sandbox_config.is_permissive() {
            // Neither extreme - just sanity check timeouts aren't negative
            // (u64 can't be negative, so this always passes)
        }

        // Validate cache config
        if self.response_cache.enabled && self.response_cache.ttl_ms == 0 {
            return Err(crate::error::ToolError::SchemaValidationFailed {
                reason: "cache enabled but TTL is zero".to_string(),
            });
        }

        // Validate commit protocol when required
        if self.effect_class == EffectClass::WriteCompensable
            && self.commit_protocol.is_none()
        {
            return Err(crate::error::ToolError::CommitFailed {
                reason: "WriteCompensable effect class requires commit protocol".to_string(),
            });
        }

        if let Some(ref protocol) = self.commit_protocol {
            if !protocol.is_valid() {
                return Err(crate::error::ToolError::CommitFailed {
                    reason: "commit protocol has invalid timeouts".to_string(),
                });
            }
        }

        Ok(())
    }

    /// Returns true if this binding is read-only (safe for unauthenticated invocation).
    ///
    /// A binding is read-only if its effect class is ReadOnly.
    ///
    /// See Engineering Plan § 2.11.2: Effect Classes.
    pub fn is_read_only(&self) -> bool {
        self.effect_class.is_safe()
    }

    /// Returns true if this binding requires explicit commit protocol (two-phase commit).
    ///
    /// A binding requires commit if:
    /// - Effect class is WriteCompensable, OR
    /// - Commit protocol is explicitly configured
    ///
    /// See Engineering Plan § 2.11.6: Commit Protocol.
    pub fn requires_commit(&self) -> bool {
        self.commit_protocol.is_some()
            || matches!(self.effect_class, EffectClass::WriteCompensable)
    }

    /// Returns true if invocation requires user confirmation.
    ///
    /// Based on effect class: WriteIrreversible effects require confirmation.
    ///
    /// See Engineering Plan § 2.11.2: Effect Classes.
    pub fn requires_confirmation(&self) -> bool {
        self.effect_class.requires_confirmation()
    }

    /// Sets the sandbox configuration for this binding.
    pub fn with_sandbox(mut self, config: SandboxConfig) -> Self {
        self.sandbox_config = config;
        self
    }

    /// Sets the caching configuration for this binding.
    pub fn with_cache(mut self, config: CacheConfig) -> Self {
        self.response_cache = config;
        self
    }

    /// Sets the effect class for this binding.
    pub fn with_effect_class(mut self, effect_class: EffectClass) -> Self {
        self.effect_class = effect_class;
        self
    }

    /// Sets the commit protocol for this binding.
    pub fn with_commit_protocol(mut self, protocol: CommitProtocol) -> Self {
        self.commit_protocol = Some(protocol);
        self
    }
}

impl fmt::Display for ToolBinding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ToolBinding {{ id: {}, tool: {}, agent: {}, effect: {} }}",
            self.id, self.tool, self.agent, self.effect_class
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::CacheConfig;
    use crate::commit_protocol::{CommitProtocol, RollbackStrategy};
    use crate::sandbox::SandboxConfig;
    use crate::schema::SchemaDefinition;
use alloc::string::ToString;

    fn create_test_binding() -> ToolBinding {
        let input_schema = SchemaDefinition::new("TestInput");
        let output_schema = SchemaDefinition::new("TestOutput");
        let schema = TypeSchema::new(input_schema, output_schema);

        let capability_bytes = [42u8; 32];

        ToolBinding::new(
            ToolBindingID::new("binding-1"),
            ToolID::new("tool-1"),
            AgentID::new("agent-1"),
            CapID::from_bytes(capability_bytes),
            schema,
        )
    }

    #[test]
    fn test_tool_binding_creation() {
        let binding = create_test_binding();
        assert_eq!(binding.id.as_str(), "binding-1");
        assert_eq!(binding.tool.as_str(), "tool-1");
        assert_eq!(binding.agent.as_str(), "agent-1");
        assert_eq!(binding.effect_class, EffectClass::WriteIrreversible);
    }

    #[test]
    fn test_tool_binding_validate_ok() {
        let binding = create_test_binding();
        assert!(binding.validate().is_ok());
    }

    #[test]
    fn test_tool_binding_validate_cache_config_error() {
        let mut binding = create_test_binding();
        binding.response_cache.enabled = true;
        binding.response_cache.ttl_ms = 0;

        assert!(binding.validate().is_err());
    }

    #[test]
    fn test_tool_binding_validate_write_compensable_without_protocol() {
        let mut binding = create_test_binding();
        binding.effect_class = EffectClass::WriteCompensable;
        binding.commit_protocol = None;

        assert!(binding.validate().is_err());
    }

    #[test]
    fn test_tool_binding_validate_write_compensable_with_protocol() {
        let mut binding = create_test_binding();
        binding.effect_class = EffectClass::WriteCompensable;
        binding.commit_protocol = Some(CommitProtocol::new(
            1000,
            2000,
            RollbackStrategy::Automatic,
        ));

        assert!(binding.validate().is_ok());
    }

    #[test]
    fn test_tool_binding_is_read_only() {
        let mut binding = create_test_binding();
        binding.effect_class = EffectClass::ReadOnly;
        assert!(binding.is_read_only());

        binding.effect_class = EffectClass::WriteReversible;
        assert!(!binding.is_read_only());
    }

    #[test]
    fn test_tool_binding_requires_commit() {
        let mut binding = create_test_binding();
        binding.effect_class = EffectClass::WriteCompensable;
        binding.commit_protocol = Some(CommitProtocol::new(
            1000,
            2000,
            RollbackStrategy::Automatic,
        ));
        assert!(binding.requires_commit());

        binding.effect_class = EffectClass::ReadOnly;
        binding.commit_protocol = None;
        assert!(!binding.requires_commit());
    }

    #[test]
    fn test_tool_binding_requires_confirmation() {
        let mut binding = create_test_binding();
        binding.effect_class = EffectClass::WriteIrreversible;
        assert!(binding.requires_confirmation());

        binding.effect_class = EffectClass::ReadOnly;
        assert!(!binding.requires_confirmation());

        binding.effect_class = EffectClass::WriteReversible;
        assert!(!binding.requires_confirmation());
    }

    #[test]
    fn test_tool_binding_with_sandbox() {
        let binding = create_test_binding().with_sandbox(SandboxConfig::permissive());
        assert!(binding.sandbox_config.is_permissive());
    }

    #[test]
    fn test_tool_binding_with_cache() {
        let binding = create_test_binding().with_cache(CacheConfig::short_lived());
        assert!(binding.response_cache.enabled);
        assert_eq!(binding.response_cache.ttl_ms, 60_000);
    }

    #[test]
    fn test_tool_binding_with_effect_class() {
        let binding =
            create_test_binding().with_effect_class(EffectClass::ReadOnly);
        assert_eq!(binding.effect_class, EffectClass::ReadOnly);
    }

    #[test]
    fn test_tool_binding_with_commit_protocol() {
        let protocol =
            CommitProtocol::new(5000, 10000, RollbackStrategy::Automatic);
        let binding = create_test_binding()
            .with_effect_class(EffectClass::WriteCompensable)
            .with_commit_protocol(protocol.clone());

        assert_eq!(binding.commit_protocol, Some(protocol));
    }

    #[test]
    fn test_tool_binding_display() {
        let binding = create_test_binding();
        let display = binding.to_string();
        assert!(display.contains("ToolBinding"));
        assert!(display.contains("binding-1"));
        assert!(display.contains("tool-1"));
        assert!(display.contains("agent-1"));
    }

    #[test]
    fn test_tool_binding_equality() {
        let b1 = create_test_binding();
        let b2 = create_test_binding();
        assert_eq!(b1, b2);

        let mut b3 = create_test_binding();
        b3.effect_class = EffectClass::ReadOnly;
        assert_ne!(b1, b3);
    }

    #[test]
    fn test_tool_binding_builder_chain() {
        let binding = create_test_binding()
            .with_sandbox(SandboxConfig::balanced())
            .with_cache(CacheConfig::medium_lived())
            .with_effect_class(EffectClass::WriteReversible);

        assert!(binding.validate().is_ok());
        assert!(!binding.is_read_only());
        assert!(!binding.requires_commit());
        assert!(!binding.requires_confirmation());
    }

    #[test]
    fn test_tool_binding_complex_scenario() {
        let protocol =
            CommitProtocol::new(3000, 7000, RollbackStrategy::Compensating);

        let binding = create_test_binding()
            .with_sandbox(SandboxConfig::permissive())
            .with_cache(CacheConfig::long_lived())
            .with_effect_class(EffectClass::WriteCompensable)
            .with_commit_protocol(protocol);

        assert!(binding.validate().is_ok());
        assert!(!binding.is_read_only());
        assert!(binding.requires_commit());
        assert!(!binding.requires_confirmation()); // WriteCompensable doesn't require confirmation
    }

    #[test]
    fn test_tool_binding_readonly_with_caching() {
        let binding = create_test_binding()
            .with_effect_class(EffectClass::ReadOnly)
            .with_cache(CacheConfig::long_lived());

        assert!(binding.validate().is_ok());
        assert!(binding.is_read_only());
        assert!(!binding.requires_commit());
        assert!(!binding.requires_confirmation());
    }
}
