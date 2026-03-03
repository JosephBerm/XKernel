// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Execution chain validation for tool invocation sequences.
//!
//! This module validates sequences (chains) of tool invocations to enforce
//! ordering constraints based on effect classes. The primary constraint is:
//!
//! **Irreversible tools must be last in the execution chain** (unless they support
//! PREPARE/COMMIT protocol, in which case they can be followed by compensation logic).
//!
//! This prevents dangerous patterns like:
//! - Delete file (WriteIrreversible), then check if deletion succeeded (ReadOnly)
//! - Send email (WriteIrreversible), then retry on failure (another SendEmail)
//!
//! See Engineering Plan § 2.11.2: Effect Classes.
//! See Engineering Plan § 2.11.6: Commit Protocol.

use crate::effect_class::EffectClass;
use crate::error::{Result, ToolError};
use crate::tool_binding::ToolBinding;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// Chain validation error specific to execution sequence violations.
///
/// Returned when a tool execution sequence violates ordering constraints.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChainValidationError {
    /// Irreversible operation followed by another operation (unless compensatable).
    IrreversibleNotLast {
        /// Index of the irreversible tool in the chain
        irreversible_index: usize,
        /// Index of the following tool
        following_index: usize,
        /// ID of the irreversible tool
        irreversible_tool: String,
        /// ID of the following tool
        following_tool: String,
    },

    /// Execution chain is empty.
    EmptyChain,

    /// Invalid chain configuration.
    InvalidConfiguration {
        /// Reason for invalidity
        reason: String,
    },
}

impl core::fmt::Display for ChainValidationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ChainValidationError::IrreversibleNotLast {
                irreversible_tool,
                following_tool,
                ..
            } => {
                write!(
                    f,
                    "irreversible operation '{}' cannot be followed by '{}'",
                    irreversible_tool, following_tool
                )
            }
            ChainValidationError::EmptyChain => {
                write!(f, "execution chain is empty")
            }
            ChainValidationError::InvalidConfiguration { reason } => {
                write!(f, "invalid chain configuration: {}", reason)
            }
        }
    }
}

/// Execution chain for tool invocation sequences.
///
/// Represents an ordered sequence of tool invocations that will be executed together.
/// Chains are validated to ensure effect class constraints are respected.
///
/// # Example
///
/// ```ignore
/// let chain = ExecutionChain::new()
///     .with_tool(web_search_binding)
///     .with_tool(database_update_binding)
///     .with_tool(email_notification_binding);
///
/// chain.validate()?;
/// ```
#[derive(Clone, Debug)]
pub struct ExecutionChain {
    /// Ordered sequence of tool bindings to execute.
    tools: Vec<ToolBinding>,
}

impl ExecutionChain {
    /// Creates a new empty execution chain.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let chain = ExecutionChain::new();
    /// assert_eq!(chain.length(), 0);
    /// ```
    pub fn new() -> Self {
        ExecutionChain {
            tools: Vec::new(),
        }
    }

    /// Adds a tool binding to the execution chain.
    ///
    /// Tools are added in execution order. Validation is performed during
    /// validate() call, not during construction.
    ///
    /// # Arguments
    ///
    /// - `binding`: Tool binding to add to the chain
    ///
    /// # Returns
    ///
    /// Self for method chaining
    ///
    /// # Example
    ///
    /// ```ignore
    /// let chain = ExecutionChain::new()
    ///     .with_tool(search_binding)
    ///     .with_tool(update_binding);
    /// ```
    pub fn with_tool(mut self, binding: ToolBinding) -> Self {
        self.tools.push(binding);
        self
    }

    /// Adds multiple tools to the execution chain.
    ///
    /// # Arguments
    ///
    /// - `bindings`: Vector of tool bindings to add
    ///
    /// # Returns
    ///
    /// Self for method chaining
    pub fn with_tools(mut self, bindings: Vec<ToolBinding>) -> Self {
        self.tools.extend(bindings);
        self
    }

    /// Returns the number of tools in the chain.
    pub fn length(&self) -> usize {
        self.tools.len()
    }

    /// Returns true if the chain is empty.
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }

    /// Returns a reference to the tools in the chain.
    pub fn tools(&self) -> &[ToolBinding] {
        &self.tools
    }

    /// Returns a tool at the specified index.
    pub fn get_tool(&self, index: usize) -> Option<&ToolBinding> {
        self.tools.get(index)
    }

    /// Validates the execution chain for effect class ordering constraints.
    ///
    /// # Validation Rules
    ///
    /// 1. Chain must not be empty (if called after construction)
    /// 2. WriteIrreversible tools must be last in the chain, UNLESS:
    ///    - They support PREPARE/COMMIT protocol, in which case compensation
    ///      logic can follow, OR
    ///    - They are not truly last (another tool follows after compensation)
    ///
    /// # Returns
    ///
    /// - `Ok(())`: Chain is valid
    /// - `Err(ChainValidationError)`: Validation failed
    ///
    /// # Example
    ///
    /// ```ignore
    /// let chain = ExecutionChain::new()
    ///     .with_tool(search_binding)
    ///     .with_tool(email_binding);  // WriteIrreversible
    ///
    /// assert!(chain.validate().is_ok());
    ///
    /// let bad_chain = ExecutionChain::new()
    ///     .with_tool(email_binding)      // WriteIrreversible
    ///     .with_tool(search_binding);    // ReadOnly after irreversible
    ///
    /// assert!(bad_chain.validate().is_err());
    /// ```
    ///
    /// See Engineering Plan § 2.11.2: Effect Classes - Execution Ordering.
    pub fn validate(&self) -> core::result::Result<(), ChainValidationError> {
        if self.tools.is_empty() {
            return Err(ChainValidationError::EmptyChain);
        }

        // Check for irreversible tools not being last
        for (index, tool) in self.tools.iter().enumerate() {
            if tool.effect_class == EffectClass::WriteIrreversible {
                // If there are tools after this one, check if allowed
                if index < self.tools.len() - 1 {
                    // Check if this irreversible tool has PREPARE/COMMIT protocol
                    if tool.commit_protocol.is_none() {
                        // No commit protocol - cannot have tools after this
                        let following_tool = &self.tools[index + 1];
                        return Err(ChainValidationError::IrreversibleNotLast {
                            irreversible_index: index,
                            following_index: index + 1,
                            irreversible_tool: tool.id.as_str().to_string(),
                            following_tool: following_tool.id.as_str().to_string(),
                        });
                    }
                    // With PREPARE/COMMIT protocol, tools can follow for compensation
                }
            }
        }

        Ok(())
    }

    /// Clears all tools from the chain.
    ///
    /// Used for testing and cleanup.
    pub fn clear(&mut self) {
        self.tools.clear();
    }
}

impl Default for ExecutionChain {
    fn default() -> Self {
        Self::new()
    }
}

/// Chain validator for checking execution sequences.
///
/// Provides additional validation methods beyond basic chain checks,
/// including effect class analysis and optimization hints.
#[derive(Clone, Debug)]
pub struct ChainValidator;

impl ChainValidator {
    /// Validates a chain and returns detailed analysis.
    ///
    /// # Returns
    ///
    /// - `Ok(analysis)`: Chain is valid with analysis details
    /// - `Err(e)`: Chain validation failed
    ///
    /// # Example
    ///
    /// ```ignore
    /// let chain = ExecutionChain::new()
    ///     .with_tool(search_binding)
    ///     .with_tool(update_binding);
    ///
    /// let analysis = ChainValidator::analyze(&chain)?;
    /// println!("Chain contains {} tools", analysis.tool_count);
    /// ```
    pub fn analyze(chain: &ExecutionChain) -> core::result::Result<ChainAnalysis, ChainValidationError> {
        chain.validate()?;

        let mut read_only_count = 0;
        let mut write_reversible_count = 0;
        let mut write_compensable_count = 0;
        let mut write_irreversible_count = 0;
        let mut has_commit_protocol = false;

        for tool in chain.tools() {
            match tool.effect_class {
                EffectClass::ReadOnly => read_only_count += 1,
                EffectClass::WriteReversible => write_reversible_count += 1,
                EffectClass::WriteCompensable => write_compensable_count += 1,
                EffectClass::WriteIrreversible => write_irreversible_count += 1,
            }

            if tool.commit_protocol.is_some() {
                has_commit_protocol = true;
            }
        }

        Ok(ChainAnalysis {
            tool_count: chain.length(),
            read_only_count,
            write_reversible_count,
            write_compensable_count,
            write_irreversible_count,
            has_commit_protocol,
            is_read_only_chain: write_irreversible_count == 0
                && write_reversible_count == 0
                && write_compensable_count == 0,
        })
    }

    /// Checks if a chain can be safely executed in a read-only context.
    ///
    /// A chain is safe for read-only execution if it contains only ReadOnly tools.
    ///
    /// # Arguments
    ///
    /// - `chain`: Execution chain to check
    ///
    /// # Returns
    ///
    /// - `true`: Chain contains only ReadOnly tools
    /// - `false`: Chain contains mutation tools
    pub fn is_read_only_safe(chain: &ExecutionChain) -> bool {
        chain.tools().iter().all(|t| t.is_read_only())
    }

    /// Checks if a chain requires user confirmation.
    ///
    /// A chain requires confirmation if any tool requires confirmation.
    ///
    /// # Arguments
    ///
    /// - `chain`: Execution chain to check
    ///
    /// # Returns
    ///
    /// - `true`: At least one tool requires confirmation
    /// - `false`: No tools require confirmation
    pub fn requires_confirmation(chain: &ExecutionChain) -> bool {
        chain.tools().iter().any(|t| t.requires_confirmation())
    }

    /// Checks if a chain requires transactional support.
    ///
    /// A chain requires transactions if it has WriteCompensable tools.
    ///
    /// # Arguments
    ///
    /// - `chain`: Execution chain to check
    ///
    /// # Returns
    ///
    /// - `true`: Chain has WriteCompensable or explicitly committed tools
    /// - `false`: Chain has no transactional requirements
    pub fn requires_transactions(chain: &ExecutionChain) -> bool {
        chain.tools().iter().any(|t| {
            t.effect_class == EffectClass::WriteCompensable || t.commit_protocol.is_some()
        })
    }
}

/// Analysis results for an execution chain.
///
/// Provides statistics about the tools in a chain and their effect classes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChainAnalysis {
    /// Total number of tools in the chain
    pub tool_count: usize,

    /// Number of ReadOnly tools
    pub read_only_count: usize,

    /// Number of WriteReversible tools
    pub write_reversible_count: usize,

    /// Number of WriteCompensable tools
    pub write_compensable_count: usize,

    /// Number of WriteIrreversible tools
    pub write_irreversible_count: usize,

    /// Whether any tool has a commit protocol
    pub has_commit_protocol: bool,

    /// Whether the chain contains only ReadOnly tools
    pub is_read_only_chain: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{AgentID, CapID, ToolBindingID, ToolID};
    use crate::schema::{SchemaDefinition, TypeSchema};
    use crate::tool_binding::ToolBinding;
    use crate::commit_protocol::{CommitProtocol, RollbackStrategy};
use alloc::string::String;
use alloc::string::ToString;

    fn create_test_binding(id: &str, effect_class: EffectClass) -> ToolBinding {
        let input_schema = SchemaDefinition::new("TestInput");
        let output_schema = SchemaDefinition::new("TestOutput");
        let schema = TypeSchema::new(input_schema, output_schema);

        let capability_bytes = [42u8; 32];

        let mut binding = ToolBinding::new(
            ToolBindingID::new(id),
            ToolID::new("test-tool"),
            AgentID::new("agent-1"),
            CapID::from_bytes(capability_bytes),
            schema,
        );
        binding.effect_class = effect_class;
        binding
    }

    #[test]
    fn test_execution_chain_new() {
        let chain = ExecutionChain::new();
        assert_eq!(chain.length(), 0);
        assert!(chain.is_empty());
    }

    #[test]
    fn test_execution_chain_with_tool() {
        let binding = create_test_binding("tool-1", EffectClass::ReadOnly);
        let chain = ExecutionChain::new().with_tool(binding);

        assert_eq!(chain.length(), 1);
        assert!(!chain.is_empty());
    }

    #[test]
    fn test_execution_chain_with_multiple_tools() {
        let binding1 = create_test_binding("tool-1", EffectClass::ReadOnly);
        let binding2 = create_test_binding("tool-2", EffectClass::ReadOnly);
        let binding3 = create_test_binding("tool-3", EffectClass::ReadOnly);

        let chain = ExecutionChain::new()
            .with_tool(binding1)
            .with_tool(binding2)
            .with_tool(binding3);

        assert_eq!(chain.length(), 3);
    }

    #[test]
    fn test_execution_chain_get_tool() {
        let binding = create_test_binding("tool-1", EffectClass::ReadOnly);
        let chain = ExecutionChain::new().with_tool(binding.clone());

        let retrieved = chain.get_tool(0);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id.as_str(), "tool-1");

        let not_found = chain.get_tool(1);
        assert!(not_found.is_none());
    }

    #[test]
    fn test_execution_chain_validate_empty() {
        let chain = ExecutionChain::new();
        let result = chain.validate();
        assert!(result.is_err());
        match result {
            Err(ChainValidationError::EmptyChain) => {}
            _ => panic!("expected EmptyChain error"),
        }
    }

    #[test]
    fn test_execution_chain_validate_readonly_only() {
        let binding1 = create_test_binding("tool-1", EffectClass::ReadOnly);
        let binding2 = create_test_binding("tool-2", EffectClass::ReadOnly);

        let chain = ExecutionChain::new()
            .with_tool(binding1)
            .with_tool(binding2);

        assert!(chain.validate().is_ok());
    }

    #[test]
    fn test_execution_chain_validate_write_at_end() {
        let binding1 = create_test_binding("tool-1", EffectClass::ReadOnly);
        let binding2 = create_test_binding("tool-2", EffectClass::WriteIrreversible);

        let chain = ExecutionChain::new()
            .with_tool(binding1)
            .with_tool(binding2);

        assert!(chain.validate().is_ok());
    }

    #[test]
    fn test_execution_chain_validate_write_not_at_end() {
        let binding1 = create_test_binding("tool-1", EffectClass::WriteIrreversible);
        let binding2 = create_test_binding("tool-2", EffectClass::ReadOnly);

        let chain = ExecutionChain::new()
            .with_tool(binding1)
            .with_tool(binding2);

        let result = chain.validate();
        assert!(result.is_err());
        match result {
            Err(ChainValidationError::IrreversibleNotLast {
                irreversible_index,
                following_index,
                ..
            }) => {
                assert_eq!(irreversible_index, 0);
                assert_eq!(following_index, 1);
            }
            _ => panic!("expected IrreversibleNotLast error"),
        }
    }

    #[test]
    fn test_execution_chain_validate_with_commit_protocol() {
        let mut binding1 = create_test_binding("tool-1", EffectClass::WriteIrreversible);
        binding1.commit_protocol = Some(CommitProtocol::new(
            1000,
            2000,
            RollbackStrategy::Automatic,
        ));

        let binding2 = create_test_binding("tool-2", EffectClass::ReadOnly);

        let chain = ExecutionChain::new()
            .with_tool(binding1)
            .with_tool(binding2);

        assert!(chain.validate().is_ok());
    }

    #[test]
    fn test_execution_chain_clear() {
        let binding = create_test_binding("tool-1", EffectClass::ReadOnly);
        let mut chain = ExecutionChain::new().with_tool(binding);

        assert_eq!(chain.length(), 1);
        chain.clear();
        assert_eq!(chain.length(), 0);
    }

    #[test]
    fn test_chain_validator_analyze() {
        let binding1 = create_test_binding("tool-1", EffectClass::ReadOnly);
        let binding2 = create_test_binding("tool-2", EffectClass::WriteReversible);
        let binding3 = create_test_binding("tool-3", EffectClass::WriteIrreversible);

        let chain = ExecutionChain::new()
            .with_tool(binding1)
            .with_tool(binding2)
            .with_tool(binding3);

        let analysis = ChainValidator::analyze(&chain).unwrap();

        assert_eq!(analysis.tool_count, 3);
        assert_eq!(analysis.read_only_count, 1);
        assert_eq!(analysis.write_reversible_count, 1);
        assert_eq!(analysis.write_irreversible_count, 1);
        assert!(!analysis.is_read_only_chain);
    }

    #[test]
    fn test_chain_validator_readonly_safe() {
        let binding1 = create_test_binding("tool-1", EffectClass::ReadOnly);
        let binding2 = create_test_binding("tool-2", EffectClass::ReadOnly);

        let chain = ExecutionChain::new()
            .with_tool(binding1)
            .with_tool(binding2);

        assert!(ChainValidator::is_read_only_safe(&chain));
    }

    #[test]
    fn test_chain_validator_readonly_not_safe() {
        let binding1 = create_test_binding("tool-1", EffectClass::ReadOnly);
        let binding2 = create_test_binding("tool-2", EffectClass::WriteReversible);

        let chain = ExecutionChain::new()
            .with_tool(binding1)
            .with_tool(binding2);

        assert!(!ChainValidator::is_read_only_safe(&chain));
    }

    #[test]
    fn test_chain_validator_requires_confirmation() {
        let binding = create_test_binding("tool-1", EffectClass::WriteIrreversible);

        let chain = ExecutionChain::new().with_tool(binding);

        assert!(ChainValidator::requires_confirmation(&chain));
    }

    #[test]
    fn test_chain_validator_no_confirmation_needed() {
        let binding = create_test_binding("tool-1", EffectClass::ReadOnly);

        let chain = ExecutionChain::new().with_tool(binding);

        assert!(!ChainValidator::requires_confirmation(&chain));
    }

    #[test]
    fn test_chain_validator_requires_transactions() {
        let binding = create_test_binding("tool-1", EffectClass::WriteCompensable);

        let chain = ExecutionChain::new().with_tool(binding);

        assert!(ChainValidator::requires_transactions(&chain));
    }

    #[test]
    fn test_chain_validator_no_transactions_needed() {
        let binding = create_test_binding("tool-1", EffectClass::ReadOnly);

        let chain = ExecutionChain::new().with_tool(binding);

        assert!(!ChainValidator::requires_transactions(&chain));
    }

    #[test]
    fn test_chain_analysis_readonly_chain() {
        let binding1 = create_test_binding("tool-1", EffectClass::ReadOnly);
        let binding2 = create_test_binding("tool-2", EffectClass::ReadOnly);

        let chain = ExecutionChain::new()
            .with_tool(binding1)
            .with_tool(binding2);

        let analysis = ChainValidator::analyze(&chain).unwrap();

        assert!(analysis.is_read_only_chain);
        assert_eq!(analysis.read_only_count, 2);
    }

    #[test]
    fn test_chain_validation_error_display() {
        let err = ChainValidationError::IrreversibleNotLast {
            irreversible_index: 0,
            following_index: 1,
            irreversible_tool: "email".to_string(),
            following_tool: "search".to_string(),
        };

        let msg = err.to_string();
        assert!(msg.contains("irreversible"));
        assert!(msg.contains("email"));
        assert!(msg.contains("search"));
    }
}
